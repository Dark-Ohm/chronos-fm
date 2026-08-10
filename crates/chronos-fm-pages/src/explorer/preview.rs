use std::path::{Path, PathBuf};
use std::sync::Arc;

use chronos_fm_core::config;
use gpui::{
    AppContext, AsyncWindowContext, Context, Entity, Image, ImageFormat, Window,
};
use gpui_wry::WebView;
use wry::WebViewBuilder;

use super::view::preview::editor::PreviewEditor;
use super::ExplorerPane;

/// Result of reading a file for preview off the UI thread.
enum PreviewOutcome {
    TooLarge,
    /// UTF-8 text (source code, plain notes, …).
    Text { body: String, language: String },
    /// Filesystem path for `gpui::img(PathBuf)` (raster + SVG).
    ImagePath(String),
    /// In-memory image (archive member or when path cannot be used).
    ImageBytes { format: ImageFormat, bytes: Vec<u8> },
    /// HTML document to show in the embedded WebKit webview.
    HtmlFile { path: PathBuf },
    /// HTML body without a stable disk path (e.g. archive member).
    HtmlBody { html: String },
    Unsupported,
}

fn extension_of(path: &str) -> String {
    Path::new(path)
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
            Ok(html) if is_html_path(path) => PreviewOutcome::HtmlBody { html },
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

    // HTML: real WebKit preview via wry (file://), not raw source.
    if is_html_path(path) {
        return PreviewOutcome::HtmlFile {
            path: PathBuf::from(path),
        };
    }

    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(body) => PreviewOutcome::Text {
                language: detect_language(path),
                body,
            },
            Err(_) => PreviewOutcome::Unsupported,
        },
        Err(_) => PreviewOutcome::Unsupported,
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
    /// Create the HTML webview on first use. Uses wry as a child of the GPUI
    /// window (`build_as_child`). On failure, returns an error string for status.
    fn ensure_html_webview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Entity<WebView>, String> {
        if let Some(view) = &self.preview_webview {
            return Ok(view.clone());
        }

        let webview = {
            let builder = WebViewBuilder::new();
            // Child of the Chronos-FM window — positions via gpui-wry bounds.
            builder
                .build_as_child(window)
                .map_err(|e| format!("HTML webview failed to start: {e}"))?
        };

        let entity = cx.new(|cx| WebView::new(webview, window, cx));
        self.preview_webview = Some(entity.clone());
        Ok(entity)
    }

    fn hide_html_webview(&mut self, cx: &mut Context<Self>) {
        self.preview_html_active = false;
        if let Some(view) = &self.preview_webview {
            view.update(cx, |wv, _| wv.hide());
        }
    }

    fn show_html_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.ensure_html_webview(window, cx) {
            Ok(view) => {
                let url = match path_to_file_url(&path) {
                    Ok(u) => u,
                    Err(msg) => {
                        self.preview_message = Some(msg);
                        self.hide_html_webview(cx);
                        return;
                    }
                };
                view.update(cx, |wv, _| {
                    wv.show();
                    wv.load_url(&url);
                });
                self.preview_html_active = true;
                self.preview_editor = None;
                self.preview_image_path = None;
                self.preview_image_data = None;
                self.preview_message = None;
            }
            Err(msg) => {
                // Fallback: show HTML source so the user still sees something.
                self.hide_html_webview(cx);
                self.preview_message = Some(format!(
                    "{msg}. Showing source. (WebKit/wry child webview is required for rendered HTML.)"
                ));
                if let Ok(body) = std::fs::read_to_string(&path) {
                    self.apply_text_preview(path.display().to_string(), body, "html".into(), window, cx);
                }
            }
        }
    }

    fn show_html_body(
        &mut self,
        html: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.ensure_html_webview(window, cx) {
            Ok(view) => {
                view.update(cx, |wv, _| {
                    wv.show();
                    let _ = wv.raw().load_html(&html);
                });
                self.preview_html_active = true;
                self.preview_editor = None;
                self.preview_image_path = None;
                self.preview_image_data = None;
                self.preview_message = None;
            }
            Err(msg) => {
                self.hide_html_webview(cx);
                self.preview_message = Some(msg);
                self.apply_text_preview(
                    self.preview_path.clone().unwrap_or_else(|| "page.html".into()),
                    html,
                    "html".into(),
                    window,
                    cx,
                );
            }
        }
    }

    fn apply_text_preview(
        &mut self,
        path: String,
        body: String,
        language: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hide_html_webview(cx);
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
        self.update_editor_search(window, cx);

        if let Some(results) = &self.search_results {
            if let Some(file_result) = results.iter().find(|r| r.path == path) {
                if let Some(first_match) = file_result.matches.first() {
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
        // Hide previous HTML surface until the new outcome decides.
        self.hide_html_webview(cx);
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
                self.hide_html_webview(cx);
                self.preview_path = Some(path);
                self.preview_message = Some("(File too large to preview)".to_string());
            }
            PreviewOutcome::ImagePath(image_path) => {
                self.hide_html_webview(cx);
                self.preview_path = Some(path);
                self.preview_image_path = Some(image_path);
                self.preview_image_data = None;
            }
            PreviewOutcome::ImageBytes { format, bytes } => {
                self.hide_html_webview(cx);
                self.preview_path = Some(path);
                self.preview_image_path = None;
                self.preview_image_data = Some(Arc::new(Image::from_bytes(format, bytes)));
            }
            PreviewOutcome::HtmlFile { path: file_path } => {
                self.preview_path = Some(path);
                self.show_html_file(file_path, window, cx);
            }
            PreviewOutcome::HtmlBody { html } => {
                self.preview_path = Some(path);
                self.show_html_body(html, window, cx);
            }
            PreviewOutcome::Unsupported => {
                self.hide_html_webview(cx);
                self.preview_path = Some(path);
                self.preview_message = Some("(Preview not available for this file)".to_string());
            }
            PreviewOutcome::Text { body, language } => {
                self.apply_text_preview(path, body, language, window, cx);
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

fn path_to_file_url(path: &Path) -> Result<String, String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let url = url::Url::from_file_path(&abs)
        .map_err(|_| format!("cannot form file URL for {}", abs.display()))?;
    Ok(url.to_string())
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

        // HTML → HtmlFile for webview load.
        let html_path = dir.path().join("page.html");
        std::fs::write(
            &html_path,
            "<html><body><h1>Title</h1><p>Paragraph text here.</p></body></html>",
        )
        .unwrap();
        match read_preview(&html_path.to_string_lossy()) {
            PreviewOutcome::HtmlFile { path } => {
                assert_eq!(path, html_path);
            }
            _ => panic!("expected HtmlFile for html"),
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
    fn path_to_file_url_uses_file_scheme() {
        let p = PathBuf::from("/tmp/example.html");
        let url = path_to_file_url(&p).unwrap();
        assert!(url.starts_with("file://"), "{url}");
        assert!(url.contains("example.html"), "{url}");
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
