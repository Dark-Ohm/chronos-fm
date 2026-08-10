use std::sync::Arc;

use chronos_fm_core::config;
use gpui::{AppContext, AsyncWindowContext, Context, Image, ImageFormat, Window};

use super::view::preview::editor::PreviewEditor;
use super::ExplorerPane;

/// Result of reading a file for preview off the UI thread.
enum PreviewOutcome {
    TooLarge,
    /// UTF-8 text (source code, HTML source, plain notes, …).
    Text { body: String, language: String },
    /// Filesystem path for `gpui::img(PathBuf)` (raster + SVG).
    ImagePath(String),
    /// In-memory image (archive member or when path cannot be used).
    ImageBytes { format: ImageFormat, bytes: Vec<u8> },
    Unsupported,
}

fn extension_of(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn detect_language(path: &str) -> String {
    match extension_of(path).as_str() {
        "rs" => "rust",
        "md" | "markdown" => "markdown",
        "json" => "json",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "html" | "htm" | "xhtml" => "html",
        "go" => "go",
        "zig" => "zig",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "css" | "scss" => "css",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "xml" | "svg" => "xml",
        "py" => "python",
        "sh" | "bash" | "zsh" => "shell",
        _ => "plain",
    }
    .to_string()
}

fn is_html_path(path: &str) -> bool {
    matches!(extension_of(path).as_str(), "html" | "htm" | "xhtml")
}

/// Raster + vector formats we preview as images (not as text).
fn is_image_path(path: &str) -> bool {
    matches!(
        extension_of(path).as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tif" | "tiff"
    )
}

fn image_format_for_path(path: &str) -> Option<ImageFormat> {
    match extension_of(path).as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "gif" => Some(ImageFormat::Gif),
        "bmp" => Some(ImageFormat::Bmp),
        "svg" => Some(ImageFormat::Svg),
        "webp" => Some(ImageFormat::Webp),
        "ico" => Some(ImageFormat::Ico),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        _ => None,
    }
}

/// Try to read a file from an archive. Returns `None` if the path is not an
/// archive path or if the read fails.
fn read_from_archive(path: &str) -> Option<Vec<u8>> {
    let (archive_path, inner_path) = chronos_fm_services::archive::split_archive_path(path)?;
    chronos_fm_services::archive::read_file(&archive_path, &inner_path).ok()
}

/// Strip tags/scripts for a plain-text reading of HTML (no browser engine).
/// Source view still uses language="html" for syntax highlighting.
fn html_to_readable_text(html: &str) -> String {
    let mut s = html.to_string();
    // Drop script/style blocks (case-insensitive, non-greedy-ish via line scans).
    for tag in ["script", "style", "noscript"] {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        let lower = s.to_lowercase();
        let mut out = String::with_capacity(s.len());
        let mut rest = s.as_str();
        let mut rest_lower = lower.as_str();
        loop {
            if let Some(i) = rest_lower.find(&open) {
                out.push_str(&rest[..i]);
                let after_open = &rest[i..];
                let after_open_l = &rest_lower[i..];
                if let Some(j) = after_open_l.find(&close) {
                    let skip = j + close.len();
                    rest = &after_open[skip..];
                    rest_lower = &after_open_l[skip..];
                } else {
                    // Unclosed — drop the rest of the open tag content.
                    break;
                }
            } else {
                out.push_str(rest);
                break;
            }
        }
        s = out;
    }

    // Replace block-ish tags with newlines, then strip remaining tags.
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(end) = s[i..].find('>') {
                let tag = s[i + 1..i + end].trim().to_lowercase();
                let name = tag.trim_start_matches('/').split_whitespace().next().unwrap_or("");
                if matches!(
                    name,
                    "p" | "div" | "br" | "tr" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                        | "hr" | "section" | "article" | "header" | "footer" | "table"
                ) {
                    out.push('\n');
                }
                i += end + 1;
                continue;
            }
        }
        out.push(s[i..].chars().next().unwrap());
        i += s[i..].chars().next().unwrap().len_utf8();
    }

    // Basic entities.
    let out = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Collapse whitespace runs but keep paragraph breaks.
    let mut collapsed = String::new();
    let mut newline_run = 0;
    let mut space = false;
    for ch in out.chars() {
        if ch == '\n' {
            space = false;
            newline_run += 1;
            if newline_run <= 2 {
                collapsed.push('\n');
            }
        } else if ch.is_whitespace() {
            if !space && !collapsed.ends_with('\n') && !collapsed.is_empty() {
                collapsed.push(' ');
                space = true;
            }
        } else {
            newline_run = 0;
            space = false;
            collapsed.push(ch);
        }
    }
    collapsed.trim().to_string()
}

/// Reads `path` and classifies it for preview. Runs on a background thread, so
/// it must not touch any GPUI state.
#[allow(clippy::disallowed_methods)]
fn read_preview(path: &str) -> PreviewOutcome {
    // Archive paths bypass filesystem metadata checks.
    if let Some(bytes) = read_from_archive(path) {
        if bytes.len() as u64 > config::PREVIEW_MAX_FILE_SIZE {
            return PreviewOutcome::TooLarge;
        }
        if is_image_path(path) {
            if let Some(format) = image_format_for_path(path) {
                return PreviewOutcome::ImageBytes { format, bytes };
            }
        }
        return match String::from_utf8(bytes) {
            Ok(body) if is_html_path(path) => html_outcome(path, body),
            Ok(body) => PreviewOutcome::Text {
                language: detect_language(path),
                body,
            },
            Err(_) => PreviewOutcome::Unsupported,
        };
    }

    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return PreviewOutcome::Unsupported,
    };
    if !metadata.is_file() {
        return PreviewOutcome::Unsupported;
    }
    if metadata.len() > config::PREVIEW_MAX_FILE_SIZE {
        return PreviewOutcome::TooLarge;
    }

    // Disk images: gpui loads from PathBuf (not String) in the view.
    if is_image_path(path) {
        return PreviewOutcome::ImagePath(path.to_string());
    }

    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(body) if is_html_path(path) => html_outcome(path, body),
            Ok(body) => PreviewOutcome::Text {
                language: detect_language(path),
                body,
            },
            Err(_) => PreviewOutcome::Unsupported,
        },
        Err(_) => PreviewOutcome::Unsupported,
    }
}

/// HTML: readable text extract first; if extract is empty, fall back to source
/// with `language=html` for syntax highlighting.
fn html_outcome(path: &str, body: String) -> PreviewOutcome {
    let readable = html_to_readable_text(&body);
    if readable.chars().count() >= 8 {
        // Prefix a short banner so the user knows this is not a browser render.
        let mut text = String::from(
            "── HTML preview (text extract; open externally for full render) ──\n\n",
        );
        text.push_str(&readable);
        PreviewOutcome::Text {
            language: "plain".into(),
            body: text,
        }
    } else {
        PreviewOutcome::Text {
            language: detect_language(path),
            body,
        }
    }
}

/// Computes the byte offset of the start of the given 0-based line index,
/// accounting for `\n` and `\r\n` line endings.
fn line_start_offset(text: &str, target_line: usize) -> Option<usize> {
    let mut current_off = 0;
    for (i, line) in text.lines().enumerate() {
        if i == target_line {
            return Some(current_off);
        }
        let consumed = line.len();
        let remainder = &text[current_off + consumed..];
        let newline_len = if remainder.starts_with("\r\n") {
            2
        } else if remainder.starts_with('\n') {
            1
        } else {
            0
        };
        current_off += consumed + newline_len;
    }
    None
}

impl ExplorerPane {
    pub(crate) fn open_preview(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preview_editor = None;
        self.preview_image_path = None;
        self.preview_image_data = None;
        self.preview_message = None;
        self.preview_text = None;
        // Record the path being loaded so that out-of-order async completions
        // (the user clicking another file before this read finishes) can be
        // detected and discarded below.
        self.preview_path = Some(path.clone());
        cx.notify();

        let read_task = cx.background_spawn({
            let path = path.clone();
            async move { read_preview(&path) }
        });

        cx.spawn_in(
            window,
            move |this: gpui::WeakEntity<ExplorerPane>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let outcome = read_task.await;
                    if let Err(error) = this.update_in(&mut cx, |this, window, cx| {
                        // Skip if a newer preview was requested while we were reading.
                        if this.preview_path.as_deref() != Some(path.as_str()) {
                            return;
                        }
                        this.apply_preview_outcome(path, outcome, window, cx);
                    }) {
                        tracing::debug!("Preview update skipped: {error}");
                    }
                }
            },
        )
        .detach();
    }

    fn apply_preview_outcome(
        &mut self,
        path: String,
        outcome: PreviewOutcome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match outcome {
            PreviewOutcome::TooLarge => {
                self.preview_path = Some(path);
                self.preview_message = Some("(File too large to preview)".to_string());
            }
            PreviewOutcome::ImagePath(image_path) => {
                self.preview_path = Some(path);
                self.preview_image_path = Some(image_path);
                self.preview_image_data = None;
            }
            PreviewOutcome::ImageBytes { format, bytes } => {
                self.preview_path = Some(path);
                self.preview_image_path = None;
                self.preview_image_data = Some(Arc::new(Image::from_bytes(format, bytes)));
            }
            PreviewOutcome::Unsupported => {
                self.preview_path = Some(path);
                self.preview_message = Some("(Preview not available for this file)".to_string());
            }
            PreviewOutcome::Text { body, language } => {
                self.preview_path = Some(path.clone());
                self.preview_text = Some(body.clone());

                let editor_view = cx.new(|cx| PreviewEditor::new(window, cx));
                editor_view.update(cx, |editor, cx| {
                    editor.set_text(body.clone(), window, cx);
                    if language != "plain" {
                        editor.set_language(language, window, cx);
                    }
                });
                self.preview_editor = Some(editor_view);

                // Highlights (search only; syntax handled by editor)
                self.update_editor_search(window, cx);

                // Scroll to the first match for the active query, if any.
                if let Some(results) = &self.search_results {
                    if let Some(file_result) = results.iter().find(|r| r.path == path) {
                        if let Some(first_match) = file_result.matches.first() {
                            // `line_number` is 1-based; `line_start_offset` takes a 0-based index.
                            if let Some(offset) =
                                line_start_offset(&body, first_match.line_number.saturating_sub(1))
                            {
                                if let Some(editor) = self.preview_editor.clone() {
                                    editor.update(cx, |editor, cx| {
                                        editor.scroll_to(offset, window, cx);
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    pub(crate) fn update_editor_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor_entity) = self.preview_editor.clone() {
            let query = self.search_query.clone();
            editor_entity.update(cx, |editor, cx| {
                editor.set_search_query(query, window, cx);
            });
        }
    }

    pub(crate) fn scroll_to_line(
        &mut self,
        line: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = self.preview_text.clone() else {
            return;
        };
        // 1-based line number to 0-based index
        let target_idx = line.saturating_sub(1);
        if let Some(offset) = line_start_offset(&text, target_idx) {
            if let Some(editor) = self.preview_editor.clone() {
                editor.update(cx, |editor, cx| {
                    editor.scroll_to(offset, window, cx);
                });
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_maps_known_extensions() {
        for (path, expected) in [
            ("a.rs", "rust"),
            ("a.md", "markdown"),
            ("a.json", "json"),
            ("a.js", "javascript"),
            ("a.ts", "typescript"),
            ("a.html", "html"),
            ("a.htm", "html"),
            ("a.xhtml", "html"),
            ("a.go", "go"),
            ("a.zig", "zig"),
            ("a.toml", "toml"),
            ("a.yaml", "yaml"),
            ("a.yml", "yaml"),
            ("a.css", "css"),
            ("a.c", "c"),
            ("a.cpp", "cpp"),
        ] {
            assert_eq!(detect_language(path), expected, "for {path}");
        }
    }

    #[test]
    fn detect_language_is_case_insensitive_and_defaults_to_plain() {
        assert_eq!(detect_language("MAIN.RS"), "rust");
        assert_eq!(detect_language("notes.unknown"), "plain");
        assert_eq!(detect_language("noext"), "plain");
    }

    #[test]
    fn image_path_classification() {
        for ext in ["png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "ico"] {
            assert!(is_image_path(&format!("a.{ext}")), "{ext} is an image");
        }
        assert!(is_image_path("PHOTO.PNG"));
        assert!(!is_image_path("a.txt"));
        assert!(!is_image_path("a.html"));
    }

    #[test]
    fn html_paths_detected() {
        assert!(is_html_path("index.html"));
        assert!(is_html_path("INDEX.HTM"));
        assert!(is_html_path("page.xhtml"));
        assert!(!is_html_path("page.txt"));
    }

    #[test]
    fn html_to_readable_strips_tags_and_scripts() {
        let html = r#"
        <html><head><style>body{color:red}</style><script>alert(1)</script></head>
        <body><h1>Hello</h1><p>World &amp; friends</p></body></html>
        "#;
        let text = html_to_readable_text(html);
        assert!(text.contains("Hello"), "{text}");
        assert!(text.contains("World & friends"), "{text}");
        assert!(!text.contains("alert"), "{text}");
        assert!(!text.contains("color:red"), "{text}");
    }

    #[test]
    fn read_preview_classifies_files() {
        let dir = tempfile::tempdir().unwrap();

        let text_path = dir.path().join("note.txt");
        std::fs::write(&text_path, "hello\nworld").unwrap();
        match read_preview(&text_path.to_string_lossy()) {
            PreviewOutcome::Text { body, .. } => assert_eq!(body, "hello\nworld"),
            _ => panic!("expected Text"),
        }

        // Disk image → ImagePath (view loads via PathBuf).
        let png_path = dir.path().join("pic.png");
        std::fs::write(&png_path, [0u8, 1, 2, 3]).unwrap();
        match read_preview(&png_path.to_string_lossy()) {
            PreviewOutcome::ImagePath(p) => assert!(p.ends_with("pic.png")),
            _ => panic!("expected ImagePath"),
        }

        // HTML → readable extract.
        let html_path = dir.path().join("page.html");
        std::fs::write(
            &html_path,
            "<html><body><h1>Title</h1><p>Paragraph text here.</p></body></html>",
        )
        .unwrap();
        match read_preview(&html_path.to_string_lossy()) {
            PreviewOutcome::Text { body, language } => {
                assert_eq!(language, "plain");
                assert!(body.contains("Title"), "{body}");
                assert!(body.contains("Paragraph text here"), "{body}");
                assert!(body.contains("HTML preview"), "{body}");
            }
            _ => panic!("expected Text for html"),
        }

        // Non-UTF-8, non-image -> Unsupported.
        let bin_path = dir.path().join("blob.bin");
        std::fs::write(&bin_path, [0xff, 0xfe, 0xfd]).unwrap();
        assert!(matches!(
            read_preview(&bin_path.to_string_lossy()),
            PreviewOutcome::Unsupported
        ));

        // A directory and a missing path are both Unsupported.
        assert!(matches!(
            read_preview(&dir.path().to_string_lossy()),
            PreviewOutcome::Unsupported
        ));
        assert!(matches!(
            read_preview("/nonexistent/path/here"),
            PreviewOutcome::Unsupported
        ));
    }

    #[test]
    fn line_start_offset_handles_lf_crlf_and_bounds() {
        let lf = "a\nbb\nccc";
        assert_eq!(line_start_offset(lf, 0), Some(0));
        assert_eq!(line_start_offset(lf, 1), Some(2));
        assert_eq!(line_start_offset(lf, 2), Some(5));
        assert_eq!(line_start_offset(lf, 3), None);

        let crlf = "a\r\nbb";
        assert_eq!(line_start_offset(crlf, 1), Some(3));
    }
}
