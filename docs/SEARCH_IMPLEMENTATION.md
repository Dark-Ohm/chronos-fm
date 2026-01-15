# Implementation Notes for Root Search

## Background

The conventional root search (`SearchScope::Root`) utilized `ripgrep` (via the `grep` crate) to traverse the entire file system (`/`). However, this approach presented the following challenges:

* Low search precision (unnecessary files were matched, or expected files were missed).
* Performance issues (scanning the entire file system is extremely resource-intensive).

## Changes

The root search backend for macOS environments has been updated to utilize the system-native Spotlight search (`mdfind`).

### Architecture

A `SearchBackend` trait is used to abstract the search backend.

```rust
pub trait SearchBackend: Send + Sync {
    fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
}

```

The `SearchEngine` initializes the appropriate backend based on the OS.

* **macOS**: `SpotlightBackend` (`src/services/search/spotlight.rs`)
* Uses the `mdfind` command to search for file paths quickly.
* Scans the beginning of the matched files to identify lines containing the query.


* **Others (Windows/Linux)**: `RipgrepBackend` (`src/services/search/ripgrep.rs`)
* Continues to use file traversal with the `ignore` and `grep` crates.
* Future replacements with Windows Search Indexer or `locate` / `plocate` (Linux) can be considered.



## Future Extensibility

When implementing support for Windows or Linux, you can create OS-specific backends (e.g., `windows_search.rs`) under `src/services/search/` and switch between them in `SearchEngine::new` using conditional compilation (`#[cfg(target_os = "windows")]`).

## Notes

* After receiving the results (file paths) from `mdfind`, `SpotlightBackend` opens each file to verify its content. Binary files, extremely large files, or files without access permissions may be skipped.
* Home directory search (`SearchScope::Home`) continues to use `IndexManager` (a proprietary index based on Tantivy) and remains unchanged.
