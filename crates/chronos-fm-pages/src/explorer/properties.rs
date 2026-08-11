//! Properties dialog for files and directories — metadata display.
//!
//! Shows file name, path, kind, size (recursive for directories), modification
//! time, owner/group, and Unix permissions (rwx + octal).

use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::{
    prelude::*, App, AppContext, Context, EventEmitter, FocusHandle, Focusable, Render,
    Styled, Window, div, px,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Status of recursive size computation.
#[derive(Debug, Clone)]
enum SizeStatus {
    Idle,
    Counting {
        files: u64,
        bytes: u64,
    },
    Done {
        bytes: u64,
        files: u64,
        dirs: u64,
        errors: u64,
    },
}

/// The Properties modal dialog.
pub struct PropertiesDialog {
    item: FileEntryDto,
    owner_name: String,
    group_name: String,
    modified_str: String,
    created_str: String,
    permissions_rwx: String,
    permissions_octal: String,
    size_status: Arc<Mutex<SizeStatus>>,
    focus_handle: FocusHandle,
}

impl PropertiesDialog {
    /// Create a new Properties dialog for the given file entry.
    /// Starts recursive size computation in the background for directories.
    pub fn new(item: FileEntryDto, cx: &mut Context<Self>) -> Self {
        let metadata = std::fs::metadata(&item.path).ok();
        let (owner_name, group_name) = Self::resolve_owner_group(&metadata);
        let (modified_str, created_str) = Self::format_times(&metadata);
        let (permissions_rwx, permissions_octal) = Self::format_permissions(&metadata);
        let size_status = Arc::new(Mutex::new(SizeStatus::Idle));

        // Start recursive size computation for directories. The walk lives in
        // the GPUI-free `compute_recursive_size`, so the background task just
        // flips to Counting and applies its result (T020).
        if item.kind == "dir" {
            let status = size_status.clone();
            let path: PathBuf = item.path.clone().into();
            cx.background_spawn(async move {
                *status.lock().unwrap() = SizeStatus::Counting { files: 0, bytes: 0 };
                *status.lock().unwrap() = compute_recursive_size(&path);
            })
            .detach();
        } else {
            *size_status.lock().unwrap() = SizeStatus::Done {
                bytes: item.size,
                files: 1,
                dirs: 0,
                errors: 0,
            };
        }

        Self {
            item: item.clone(),
            owner_name,
            group_name,
            modified_str,
            created_str,
            permissions_rwx,
            permissions_octal,
            size_status,
            focus_handle: cx.focus_handle(),
        }
    }

    fn resolve_owner_group(metadata: &Option<std::fs::Metadata>) -> (String, String) {
        let Some(_md) = metadata else {
            return ("unknown".into(), "unknown".into());
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let owner = users::get_user_by_uid(_md.uid())
                .map(|u| u.name().to_string_lossy().to_string())
                .unwrap_or_else(|| _md.uid().to_string());
            let group = users::get_group_by_gid(_md.gid())
                .map(|g| g.name().to_string_lossy().to_string())
                .unwrap_or_else(|| _md.gid().to_string());
            (owner, group)
        }
        #[cfg(not(unix))]
        {
            ("N/A".into(), "N/A".into())
        }
    }

    fn format_times(metadata: &Option<std::fs::Metadata>) -> (String, String) {
        let Some(md) = metadata else {
            return ("unknown".into(), "unknown".into());
        };
        let fmt = |t: std::io::Result<std::time::SystemTime>| -> String {
            t.ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| {
                    let secs = d.as_secs();
                    let dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
                        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
                    format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}",
                        dt.year(),
                        dt.month() as u8,
                        dt.day(),
                        dt.hour(),
                        dt.minute()
                    )
                })
                .unwrap_or_else(|| "unknown".into())
        };
        (fmt(md.modified()), fmt(md.created()))
    }

    fn format_permissions(metadata: &Option<std::fs::Metadata>) -> (String, String) {
        let Some(md) = metadata else {
            return ("---------".into(), "0000".into());
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = md.permissions().mode();
            let perm_bits = mode & 0o7777;
            let octal = format!("{:04o}", perm_bits);
            let rwx = mode_to_rwx(perm_bits);
            (rwx, octal)
        }
        #[cfg(not(unix))]
        {
            ("rw-r--r--".into(), "0644".into())
        }
    }
}

/// Recursively sum the bytes of every regular file under `path` (a
/// directory) and count files and directories, `path` itself included as a
/// directory.
///
/// Never aborts on unreadable entries: a failed read increments `errors` and
/// the walk continues, so the readable remainder is still accounted for. This
/// is the tolerance the live UI relies on (verified in T016) — extracted into
/// a GPUI-free function so it is covered by headless unit tests instead of
/// only manual smoke runs (T020).
fn compute_recursive_size(path: &Path) -> SizeStatus {
    let mut files: u64 = 0;
    let mut dirs: u64 = 0;
    let mut bytes: u64 = 0;
    let mut errors: u64 = 0;

    for entry in walkdir::WalkDir::new(path) {
        match entry {
            Ok(e) => {
                if e.file_type().is_dir() {
                    dirs += 1;
                } else if e.file_type().is_file() {
                    files += 1;
                    // A file whose metadata cannot be read still counts as a
                    // file but contributes 0 bytes.
                    bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
            Err(_) => {
                errors += 1;
            }
        }
    }

    SizeStatus::Done { bytes, files, dirs, errors }
}

/// Convert a Unix mode to an rwx string like "rwxr-xr-x".
fn mode_to_rwx(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    let bits = [(0o4, 'r'), (0o2, 'w'), (0o1, 'x')];
    for shift in [6u32, 3, 0] {
        for (bit, ch) in &bits {
            s.push(if mode & (bit << shift) != 0 { *ch } else { '-' });
        }
    }
    s
}

impl EventEmitter<()> for PropertiesDialog {}

impl Focusable for PropertiesDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PropertiesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let kind_label = match self.item.kind.as_str() {
            "dir" => "Directory",
            "symlink" => "Symlink",
            _ => "File",
        };
        let path = Path::new(&self.item.path)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let size_text = {
            let status = self.size_status.lock().unwrap();
            match &*status {
                SizeStatus::Idle => String::from("—"),
                SizeStatus::Counting { files, bytes } => {
                    format!("Counting… {} files, {}", files, human_size(*bytes))
                }
                SizeStatus::Done { bytes, files, dirs, errors } => {
                    let mut s = human_size(*bytes);
                    if *dirs > 0 || *files > 0 {
                        s.push_str(&format!(" ({} files, {} dirs", files, dirs));
                        if *errors > 0 {
                            s.push_str(&format!(", {} unreadable", errors));
                        }
                        s.push(')');
                    }
                    s
                }
            }
        };

        elevated_card(cx)
            .w(px(420.))
            .child(section_header(cx, "Properties", &self.item.name))
            .child(prop_row(cx, "Name", &self.item.name))
            .child(prop_row(cx, "Path", &path))
            .child(prop_row(cx, "Kind", kind_label))
            .child(prop_row(cx, "Size", &size_text))
            .child(prop_row(cx, "Modified", &self.modified_str))
            .child(prop_row(cx, "Created", &self.created_str))
            .child(prop_row(cx, "Owner", &self.owner_name))
            .child(prop_row(cx, "Group", &self.group_name))
            .child(prop_row(cx, "Permissions", &format!("{} ({})", self.permissions_rwx, self.permissions_octal)))
            .into_any_element()
    }
}

fn prop_row(cx: &App, label: &str, value: &str) -> impl IntoElement {
    let label = label.to_string();
    let value = value.to_string();
    div()
        .flex()
        .gap(px(8.))
        .child(
            div()
                .w(px(100.))
                .text_color(theme::muted(cx))
                .text_xs()
                .child(label),
        )
        .child(div().flex_1().text_color(theme::fg(cx)).text_sm().child(value))
}

pub(crate) fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_to_rwx_known_modes() {
        assert_eq!(mode_to_rwx(0o755), "rwxr-xr-x");
        assert_eq!(mode_to_rwx(0o644), "rw-r--r--");
        assert_eq!(mode_to_rwx(0o000), "---------");
        assert_eq!(mode_to_rwx(0o777), "rwxrwxrwx");
        assert_eq!(mode_to_rwx(0o700), "rwx------");
    }

    #[test]
    fn human_size_rounds_to_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
        assert_eq!(human_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn metadata_formatters_handle_missing_metadata() {
        assert_eq!(
            PropertiesDialog::resolve_owner_group(&None),
            ("unknown".to_string(), "unknown".to_string())
        );
        assert_eq!(
            PropertiesDialog::format_times(&None),
            ("unknown".to_string(), "unknown".to_string())
        );
        assert_eq!(
            PropertiesDialog::format_permissions(&None),
            ("---------".to_string(), "0000".to_string())
        );
    }

    /// T020: a small tree with files of known sizes and nested directories
    /// must reach `Done` with the exact byte sum and file/dir counts. `dirs`
    /// includes the root directory itself (WalkDir yields it first).
    #[test]
    fn compute_recursive_size_sums_tree() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.bin"), vec![0u8; 1000]).unwrap();
        std::fs::write(tmp.path().join("b.bin"), vec![0u8; 500]).unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("c.bin"), vec![0u8; 250]).unwrap();
        let deep = sub.join("deep");
        std::fs::create_dir(&deep).unwrap();
        std::fs::write(deep.join("d.bin"), vec![0u8; 100]).unwrap();

        let SizeStatus::Done { bytes, files, dirs, errors } =
            compute_recursive_size(tmp.path())
        else {
            panic!("expected Done status");
        };
        assert_eq!(bytes, 1000 + 500 + 250 + 100);
        assert_eq!(files, 4);
        assert_eq!(dirs, 3, "root + sub + deep");
        assert_eq!(errors, 0);
    }

    /// T020: an unreadable subdirectory must not abort the walk — it counts
    /// as one error and the readable part of the tree still sums correctly.
    /// Skipped under root, where chmod 000 does not restrict access (same
    /// probe as the T016 tests).
    #[cfg(unix)]
    #[test]
    fn compute_recursive_size_tolerates_unreadable_subdir() {
        use std::os::unix::fs::PermissionsExt;

        // Skip under root: chmod 000 does not restrict root (see T016).
        let probe = std::env::temp_dir()
            .join(format!("cfm_props_root_probe_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&probe);
        let _ = std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o000));
        let can_still_read = std::fs::read_dir(&probe).is_ok();
        let _ = std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o755));
        let _ = std::fs::remove_dir_all(&probe);
        if can_still_read {
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.bin"), vec![0u8; 100]).unwrap();

        let locked = tmp.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("secret.bin"), vec![0u8; 900]).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let SizeStatus::Done { bytes, files, dirs, errors } =
            compute_recursive_size(tmp.path())
        else {
            panic!("expected Done status");
        };
        // The locked dir's own entry is still yielded (stat succeeds); only
        // reading its contents fails, which increments `errors`.
        assert_eq!(bytes, 100, "only the readable file contributes bytes");
        assert_eq!(files, 1);
        assert_eq!(dirs, 2, "root + locked dir entry itself");
        assert_eq!(errors, 1, "unreadable subdir is tolerated, not fatal");

        // Restore permissions so TempDir cleanup can remove the tree.
        let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));
    }

    #[cfg(unix)]
    #[test]
    fn format_permissions_reads_real_mode() {
        use std::os::unix::fs::PermissionsExt;
        let base = std::env::temp_dir().join(format!("chrono-props-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let p = base.join("sample.txt");
        std::fs::write(&p, b"hi").unwrap();
        let mut perms = std::fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&p, perms).unwrap();
        let md = std::fs::metadata(&p).ok();
        assert_eq!(
            PropertiesDialog::format_permissions(&md),
            ("rwxr-xr-x".to_string(), "0755".to_string())
        );
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir(&base);
    }
}
