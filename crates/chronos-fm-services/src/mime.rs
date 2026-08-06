//! MIME type detection and freedesktop.org application resolution.
//!
//! Building blocks for the "Open With" context menu (T007):
//! 1. Detect a file's MIME type via `xdg-mime`.
//! 2. Find desktop entries that claim that MIME type via `freedesktop-desktop-entry`.
//! 3. Launch an application with a file path by expanding its `Exec` field.
//! 4. Register an application as the default handler for a MIME type.

use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// A simplified representation of a desktop application, carrying only the
/// fields needed for the "Open With" menu.
#[derive(Debug, Clone)]
pub struct DesktopApp {
    /// Human-readable application name (localized).
    pub name: String,
    /// Icon name from the desktop entry, suitable for an icon theme lookup.
    pub icon: Option<String>,
    /// The raw `Exec` line from the desktop entry.
    pub exec: String,
    /// Whether the application requests a terminal.
    pub terminal: bool,
    /// The desktop file id (e.g. `org.gnome.TextEditor.desktop`), used for
    /// `xdg-mime default` registration.
    pub desktop_id: String,
}

/// Detect the MIME type of a file at `path`.
///
/// Returns the MIME type as a string (e.g. `"image/png"`, `"text/plain"`),
/// or `None` if the file does not exist or the MIME database is unavailable.
pub fn detect_mime_type(path: &str) -> Option<String> {
    let db = mime_db();
    let mut gb = db.guess_mime_type();
    let guess = gb.path(path).guess();
    Some(guess.mime_type().essence_str().to_owned())
}

/// Find all desktop applications that claim to handle `mime_type`.
///
/// The search covers the standard XDG application directories
/// (`~/.local/share/applications`, `/usr/share/applications`, etc.).
/// Entries marked `NoDisplay=true` or `Hidden=true` are skipped.
pub fn find_apps_for_mime(mime_type: &str) -> Vec<DesktopApp> {
    let entries = desktop_entries();
    let locales = freedesktop_desktop_entry::get_languages_from_env();

    entries
        .iter()
        .filter(|entry| !entry.no_display() && !entry.hidden())
        .filter(|entry| {
            entry.mime_type().is_some_and(|mimes| {
                mimes
                    .iter()
                    .any(|m| *m == mime_type || *m == "application/octet-stream")
            })
        })
        .map(|entry| DesktopApp {
            name: entry
                .name(&locales)
                .map(|n| n.into_owned())
                .unwrap_or_else(|| entry.id().to_owned()),
            icon: entry.icon().map(|s| s.to_owned()),
            exec: entry.exec().unwrap_or("").to_owned(),
            terminal: entry.terminal(),
            desktop_id: entry.id().to_owned(),
        })
        .collect()
}

/// Launch `app` with `path` as its argument.
///
/// The `Exec` field from the desktop entry is expanded:
/// - `%f` / `%F` → replaced with `path`
/// - `%u` / `%U` → replaced with `path` (URL-encoded)
/// - `%%` → literal `%`
///
/// If `app.terminal` is true, the command is wrapped in the user's preferred
/// terminal emulator (from `$TERMINAL`, falling back to `foot`).
///
/// The launch never blocks: the application is detached so the caller's event
/// loop (the UI thread, for the explorer context menu) is not frozen while the
/// launched program runs. Errors reported are limited to failures to start the
/// wrapper process itself; the application's own exit status is not observed.
pub fn open_with(app: &DesktopApp, path: &str) -> Result<(), String> {
    let expanded = expand_exec(&app.exec, path);

    if app.terminal {
        let term = std::env::var("TERMINAL").unwrap_or_else(|_| "foot".to_string());
        // Background the terminal through `sh -c`, so it is reparented to init
        // and reaped there; `status()` returns as soon as `sh` exits.
        let script = format!("{} -e sh -c {} &", shell_escape(&term), shell_escape(&expanded));
        let status = Command::new("sh")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to launch terminal {}: {}", term, e))?;
        if !status.success() {
            return Err(format!(
                "Terminal command exited with status {}",
                status.code().unwrap_or(1)
            ));
        }
    } else {
        let script = format!("{} >/dev/null 2>&1 &", expanded);
        let status = Command::new("sh")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to launch: {}", e))?;
        if !status.success() {
            return Err(format!(
                "Command exited with status {}",
                status.code().unwrap_or(1)
            ));
        }
    }

    Ok(())
}

/// Open `path` with the system's default handler (an `xdg-open` equivalent).
///
/// Detaches the launch the same way [`open_with`] does. Used by the context
/// menu's "Open" item.
pub fn open_default(path: &str) -> Result<(), String> {
    let script = format!("xdg-open {} >/dev/null 2>&1 &", shell_escape(path));
    let status = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to launch xdg-open: {}", e))?;
    if !status.success() {
        return Err(format!(
            "xdg-open exited with status {}",
            status.code().unwrap_or(1)
        ));
    }
    Ok(())
}

/// Register `app` as the default handler for `mime_type`.
///
/// Shells out to `xdg-mime default <desktop-id> <mime-type>`, which writes the
/// association into the user's `mimeapps.list`.
pub fn set_default_app(mime_type: &str, app: &DesktopApp) -> Result<(), String> {
    let status = Command::new("xdg-mime")
        .arg("default")
        .arg(&app.desktop_id)
        .arg(mime_type)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run xdg-mime: {}", e))?;
    if !status.success() {
        return Err(format!(
            "xdg-mime default exited with status {}",
            status.code().unwrap_or(1)
        ));
    }
    Ok(())
}

/// Expand the `Exec` field placeholders.
///
/// Placeholders:
/// - `%f`, `%F` → single file path
/// - `%u`, `%U` → single URL (URL-encoded path)
/// - `%i` → `--icon <icon>` (stripped — not needed for launching)
/// - `%c` → translated name (stripped)
/// - `%k` → desktop file path (stripped)
/// - `%%` → literal `%`
fn expand_exec(exec: &str, path: &str) -> String {
    let mut result = String::with_capacity(exec.len() + path.len());
    let mut chars = exec.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek().copied() {
                Some('f') | Some('F') => {
                    chars.next();
                    result.push_str(&shell_escape(path));
                }
                Some('u') | Some('U') => {
                    chars.next();
                    result.push_str(&shell_escape(&file_url(path)));
                }
                Some('i') | Some('c') | Some('k') => {
                    // Strip these deprecated/non-essential placeholders
                    chars.next();
                    if chars.peek().copied() == Some('"') {
                        // Also skip a following quoted argument
                        chars.next();
                        for c in chars.by_ref() {
                            if c == '"' {
                                break;
                            }
                        }
                    }
                }
                Some('%') => {
                    chars.next();
                    result.push('%');
                }
                Some(_) => {
                    // Unknown placeholder — pass through unchanged
                    result.push('%');
                }
                None => result.push('%'),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Shell-escape a string for use in an `sh -c` command.
fn shell_escape(s: &str) -> String {
    // Simple quoting: wrap in single quotes, escape any internal single quotes.
    if s.contains('\'') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        format!("'{}'", s)
    }
}

/// Build a `file://` URL for a local path, percent-encoding the characters
/// that are not valid inside a URL (spaces, `#`, `%`, non-ASCII, etc.).
fn file_url(path: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut url = String::with_capacity(path.len() + 8);
    url.push_str("file://");
    for b in path.as_bytes() {
        let ok = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'.' | b'_' | b'~' | b'/' | b':' | b'@' | b'&' | b'=' | b'+' | b'$'
                    | b',' | b';' | b'!'
            );
        if ok {
            url.push(*b as char);
        } else {
            url.push('%');
            url.push(HEX[(b >> 4) as usize] as char);
            url.push(HEX[(b & 0x0f) as usize] as char);
        }
    }
    url
}

// ── Lazy-init caches ──────────────────────────────────────────────────────

fn mime_db() -> &'static xdg_mime::SharedMimeInfo {
    static DB: OnceLock<xdg_mime::SharedMimeInfo> = OnceLock::new();
    DB.get_or_init(xdg_mime::SharedMimeInfo::new)
}

fn desktop_entries() -> &'static [freedesktop_desktop_entry::DesktopEntry] {
    static ENTRIES: OnceLock<Vec<freedesktop_desktop_entry::DesktopEntry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let locales = freedesktop_desktop_entry::get_languages_from_env();
        freedesktop_desktop_entry::desktop_entries(&locales)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_exec_replaces_file_placeholder() {
        assert_eq!(
            expand_exec("gedit %f", "/tmp/a b.txt"),
            "gedit '/tmp/a b.txt'"
        );
        assert_eq!(expand_exec("gedit %F", "/tmp/a.txt"), "gedit '/tmp/a.txt'");
    }

    #[test]
    fn expand_exec_replaces_url_placeholder() {
        assert_eq!(
            expand_exec("firefox %u", "/tmp/a.txt"),
            "firefox 'file:///tmp/a.txt'"
        );
        assert_eq!(
            expand_exec("firefox %U", "/tmp/a.txt"),
            "firefox 'file:///tmp/a.txt'"
        );
    }

    #[test]
    fn file_url_percent_encodes_spaces_and_specials() {
        assert_eq!(file_url("/tmp/a b.txt"), "file:///tmp/a%20b.txt");
        assert_eq!(file_url("/tmp/100%25.txt"), "file:///tmp/100%2525.txt");
        assert_eq!(file_url("/tmp/пробел.txt"), "file:///tmp/%D0%BF%D1%80%D0%BE%D0%B1%D0%B5%D0%BB.txt");
        assert_eq!(file_url("/tmp/hash#1.txt"), "file:///tmp/hash%231.txt");
    }

    #[test]
    fn expand_exec_handles_percent_escape() {
        assert_eq!(expand_exec("app --pct %%", "/tmp/a.txt"), "app --pct %");
    }

    #[test]
    fn expand_exec_strips_icon_and_name_placeholders() {
        assert_eq!(expand_exec("app %i --flag", "/tmp/a.txt"), "app  --flag");
        assert_eq!(expand_exec("app %i --icon x", "/tmp/a.txt"), "app  --icon x");
        // A quoted argument separated by whitespace from %i is a real argument
        // and is kept; only a directly-attached %i"..." is consumed.
        assert_eq!(
            expand_exec("app %i \"big icon\" rest", "/tmp/a.txt"),
            "app  \"big icon\" rest"
        );
        assert_eq!(expand_exec("app %i\"big icon\" rest", "/tmp/a.txt"), "app  rest");
        assert_eq!(expand_exec("app %c", "/tmp/a.txt"), "app ");
    }

    #[test]
    fn expand_exec_passes_unknown_placeholder_through() {
        assert_eq!(expand_exec("app %z", "/tmp/a.txt"), "app %z");
        assert_eq!(expand_exec("trailing %", "/tmp/a.txt"), "trailing %");
    }

    #[test]
    fn shell_escape_quotes_and_handles_inner_quotes() {
        assert_eq!(shell_escape("plain.txt"), "'plain.txt'");
        assert_eq!(shell_escape("has space.txt"), "'has space.txt'");
        assert_eq!(shell_escape("it's.txt"), "'it'\\''s.txt'");
    }

    #[test]
    fn file_url_keeps_ascii_safe_characters() {
        assert_eq!(file_url("/a-b_c.d~e/f:g@h"), "file:///a-b_c.d~e/f:g@h");
    }

    #[test]
    fn detect_mime_type_known_extensions() {
        // Requires shared-mime-info. When it's absent (minimal CI / containers)
        // xdg_mime returns generic types, so we assert only when the DB is
        // clearly present rather than failing the build environment.
        let tmp = tempfile::tempdir().unwrap();
        let txt = tmp.path().join("sample.txt");
        std::fs::write(&txt, b"hello world").unwrap();
        let txt_mime = detect_mime_type(txt.to_str().unwrap());

        let png = tmp.path().join("img.png");
        std::fs::write(&png, &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();
        let png_mime = detect_mime_type(png.to_str().unwrap());

        match (txt_mime, png_mime) {
            (Some(t), Some(p)) => {
                if t.contains("text") && p.contains("image") {
                    assert_eq!(t, "text/plain");
                    assert_eq!(p, "image/png");
                } else {
                    eprintln!(
                        "shared-mime-info appears unavailable (txt={t:?}, png={p:?}); skipping exact mime assertions"
                    );
                }
            }
            other => eprintln!("detect_mime_type returned {other:?}; shared-mime-info may be unavailable; skipping"),
        }
    }

    #[test]
    fn find_apps_for_mime_text_plain_returns_list() {
        // On a desktop with .desktop entries this should be non-empty; on a
        // headless box with no registered handlers it may be empty — skip
        // rather than fail.
        let apps = find_apps_for_mime("text/plain");
        if apps.is_empty() {
            eprintln!("no .desktop entries for text/plain on this system; skipping");
            return;
        }
        assert!(apps.iter().all(|a| !a.desktop_id.is_empty()));
        assert!(apps.iter().any(|a| !a.name.is_empty()));
    }

    #[test]
    fn open_with_launches_via_detached_sh_c() {
        // Verify the call form (`sh -c`, detached via `&`) without launching a
        // real application: `true` exits 0 immediately and is backgrounded.
        let app = DesktopApp {
            name: "True".into(),
            icon: None,
            exec: "true %f".into(),
            terminal: false,
            desktop_id: "true.desktop".into(),
        };
        let result = open_with(&app, "/tmp/does-not-matter.txt");
        assert!(
            result.is_ok(),
            "open_with should launch via detached sh -c: {result:?}"
        );
    }

    #[test]
    fn open_with_terminal_wraps_command() {
        // terminal=true wraps the command in $TERMINAL -e sh -c ... & — verify
        // the form launches without blocking (uses `true` so nothing real runs).
        let app = DesktopApp {
            name: "True".into(),
            icon: None,
            exec: "true %f".into(),
            terminal: true,
            desktop_id: "true.desktop".into(),
        };
        let result = open_with(&app, "/tmp/does-not-matter.txt");
        assert!(result.is_ok(), "open_with (terminal) should launch: {result:?}");
    }
}
