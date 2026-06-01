use nohrs_core::errors::Result;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

// `FileEntryDto` is a pure data type and lives in `nohrs-models`. It is
// re-exported here so existing `nohrs_services::fs::listing::FileEntryDto`
// references keep resolving without forcing callers through `nohrs-models`.
pub use nohrs_models::file_entry::FileEntryDto;

pub struct ListParams<'a> {
    pub path: &'a str,
    pub limit: usize,
    pub cursor: Option<&'a str>,
}

pub struct ListResult {
    pub entries: Vec<FileEntryDto>,
    pub next_cursor: Option<String>,
}

/// Lists a directory. The work is synchronous filesystem IO; callers that must
/// keep the UI thread responsive should run it on GPUI's background executor via
/// `cx.background_spawn` (async-runtime.md §2).
pub fn list_dir_sync(params: ListParams<'_>) -> Result<ListResult> {
    list_dir_impl(params.path, params.limit, params.cursor)
}

fn list_dir_impl(path: &str, limit: usize, cursor: Option<&str>) -> Result<ListResult> {
    let dir = Path::new(path);
    let mut names: Vec<(String, PathBuf)> = Vec::new();

    // Read directory entries: collect names and paths only (cheap), then sort by name for stable paging.
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = os_str_to_string(entry.file_name());
        names.push((file_name, entry.path()));
    }
    names.sort_by_key(|a| a.0.to_lowercase());

    let total = names.len();
    let offset = cursor
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
        .min(total);

    let end = (offset + limit).min(total);
    let slice = &names[offset..end];

    let mut entries = Vec::with_capacity(slice.len());
    for (name, path) in slice.iter() {
        let md = fs::symlink_metadata(path);
        let (kind, size, modified) = match md {
            Ok(md) => {
                let modified = md
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if md.file_type().is_dir() {
                    ("dir".to_string(), 0, modified)
                } else if md.file_type().is_file() {
                    ("file".to_string(), md.len(), modified)
                } else if md.file_type().is_symlink() {
                    ("symlink".to_string(), 0, modified)
                } else {
                    ("other".to_string(), 0, modified)
                }
            }
            Err(_) => ("unknown".to_string(), 0, 0),
        };

        entries.push(FileEntryDto {
            name: name.clone(),
            path: path.to_string_lossy().to_string(),
            kind,
            size,
            modified,
        });
    }

    let next_cursor = if end < total {
        Some(end.to_string())
    } else {
        None
    };

    Ok(ListResult {
        entries,
        next_cursor,
    })
}

fn os_str_to_string(s: impl AsRef<OsStr>) -> String {
    s.as_ref().to_string_lossy().into_owned()
}
