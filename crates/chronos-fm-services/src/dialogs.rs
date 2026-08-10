//! Native file/directory picker via the XDG desktop portal (T039 gate 1).
//!
//! Used for S3 transfer local source/destination selection — the user
//! picks a local file or directory through the system's own dialog
//! (rather than a homemade one) via `org.freedesktop.portal.FileChooser`.
//! `ashpd` is already present transitively (via `gpui_linux`/`oo7`); this
//! module is the compile probe + thin wrapper the T039 design spec's gate
//! 1 required before the transfer subsystem itself is speced.

use std::path::PathBuf;

/// Open a single-file picker. Returns `None` if the user cancelled or the
/// portal declined (both are ordinary outcomes, not errors — see
/// [`GitError`]-style honesty rule: no fake path on cancel).
pub async fn pick_file(title: &str) -> Result<Option<PathBuf>, ashpd::Error> {
    let files = ashpd::desktop::file_chooser::SelectedFiles::open_file()
        .title(title)
        .modal(true)
        .multiple(false)
        .send()
        .await?
        .response()?;
    Ok(files.uris().first().and_then(uri_to_path))
}

/// Open a directory picker (S3 upload source / download destination).
pub async fn pick_directory(title: &str) -> Result<Option<PathBuf>, ashpd::Error> {
    let files = ashpd::desktop::file_chooser::SelectedFiles::open_file()
        .title(title)
        .modal(true)
        .multiple(false)
        .directory(true)
        .send()
        .await?
        .response()?;
    Ok(files.uris().first().and_then(uri_to_path))
}

/// Convert a `file://`-scheme portal response URI to a local path. Any
/// other scheme (the portal can theoretically return non-local URIs, e.g.
/// an MTP device) is honestly reported as `None` rather than mangled into
/// a bogus path. `ashpd::Uri` is a thin string wrapper (not `url::Url`),
/// so the `file://` prefix and percent-encoding are handled by hand here.
fn uri_to_path(uri: &ashpd::Uri) -> Option<PathBuf> {
    let rest = uri.as_str().strip_prefix("file://")?;
    let decoded = percent_encoding::percent_decode_str(rest)
        .decode_utf8()
        .ok()?;
    Some(PathBuf::from(decoded.into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_to_path_accepts_file_scheme() {
        let uri = ashpd::Uri::parse("file:///home/user/docs/report.pdf").unwrap();
        assert_eq!(
            uri_to_path(&uri),
            Some(PathBuf::from("/home/user/docs/report.pdf"))
        );
    }

    #[test]
    fn uri_to_path_rejects_non_file_scheme() {
        // The portal spec allows other schemes in principle; we must not
        // silently mangle e.g. an mtp:// URI into a bogus local path.
        let uri = ashpd::Uri::parse("mtp://device/DCIM/photo.jpg").unwrap();
        assert_eq!(uri_to_path(&uri), None);
    }

    #[test]
    fn uri_to_path_percent_decodes_spaces() {
        let uri = ashpd::Uri::parse("file:///home/user/My%20Documents/a.txt").unwrap();
        assert_eq!(
            uri_to_path(&uri),
            Some(PathBuf::from("/home/user/My Documents/a.txt"))
        );
    }
}
