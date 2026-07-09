//! Persistence layer for chronos-fm (see `docs/persistence.md`).
//!
//! Two backends sit behind interface-segregated traits, split by a single
//! question: *does the data ever need to be queried by anything other than an
//! exact key?*
//!
//! * **SQLite** ([`SqliteStore`]) — file metadata and history, which need
//!   range/diff/ordered queries. Implements [`MetadataQuery`], [`MetadataStore`],
//!   and [`HistoryStore`]. Uses bundled SQLite in WAL mode.
//! * **redb** ([`RedbKvStore`]) — host key/value state (window position,
//!   tab/session restore, dynamic settings): pure `key -> blob`. Implements
//!   [`KvStore`].
//!
//! Both backends are synchronous and tokio-free; the UI layer is expected to
//! call them from `cx.background_spawn`. Per-operation performance logging is
//! controlled by [`StoreLogConfig`] (see `docs/persistence.md` §5).

use std::path::PathBuf;

mod redb_kv;
mod sqlite;

pub use redb_kv::RedbKvStore;
pub use sqlite::SqliteStore;

/// Errors returned by the store backends.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A SQLite operation failed.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A redb operation failed. Boxed because `redb::Error` is large and would
    /// otherwise bloat every `Result` in this crate (`clippy::result_large_err`).
    #[error("redb error: {0}")]
    Redb(Box<redb::Error>),
}

// redb surfaces a family of error types from its different stages. Funnel each
// through `redb::Error` so `?` converts directly to [`StoreError`].
macro_rules! redb_error_from {
    ($($error:ty),+ $(,)?) => {$(
        impl From<$error> for StoreError {
            fn from(error: $error) -> Self {
                StoreError::Redb(Box::new(redb::Error::from(error)))
            }
        }
    )+};
}
redb_error_from!(
    redb::Error,
    redb::DatabaseError,
    redb::TransactionError,
    redb::TableError,
    redb::StorageError,
    redb::CommitError,
);

/// Result type used throughout the store crate.
pub type Result<T> = std::result::Result<T, StoreError>;

/// Controls per-operation performance logging for the store backends. All
/// fields default to off, so a default config adds zero overhead (the SQLite
/// profile hook is not even installed). Output is emitted via `tracing` and can
/// be filtered with `RUST_LOG`; see `docs/persistence.md` §5.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreLogConfig {
    /// Log every SQL statement at `debug` (target `chronos_fm_store::sql`).
    pub log_all_queries: bool,
    /// Log SQL statements slower than this many milliseconds at `warn`
    /// (target `chronos_fm_store::sql`). Zero disables slow-query logging.
    pub slow_query_ms: u64,
    /// Log redb `get`/`put`/`delete`/`batch` operations and their durations at
    /// `debug` (target `chronos_fm_store::redb`).
    pub log_redb_ops: bool,
}

/// Primary key of a row in the `files` table.
pub type FileId = i64;

/// A row read back from the `files` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    /// Stable row identifier.
    pub id: FileId,
    /// Absolute path to the entry (unique).
    pub path: PathBuf,
    /// Path of the containing directory.
    pub parent_path: PathBuf,
    /// Filesystem inode number.
    pub inode: u64,
    /// Size in bytes.
    pub size: u64,
    /// Last-modified time in nanoseconds since the Unix epoch.
    pub mtime_ns: i64,
    /// Content hash (blake3 first-N-KB), populated in P3.
    pub content_hash: Option<Vec<u8>>,
    /// Time the entry was last indexed (nanoseconds since epoch), or `None`.
    pub indexed_at: Option<i64>,
    /// Logical-deletion time (nanoseconds since epoch), or `None` if live.
    pub deleted_at: Option<i64>,
}

/// The data needed to insert or update a `files` row. `indexed_at` and
/// `deleted_at` are managed separately via [`MetadataStore::mark_indexed`] and
/// [`MetadataStore::delete_file`], so they are not part of an upsert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUpsert {
    /// Absolute path to the entry (the upsert conflict key).
    pub path: PathBuf,
    /// Path of the containing directory.
    pub parent_path: PathBuf,
    /// Filesystem inode number.
    pub inode: u64,
    /// Size in bytes.
    pub size: u64,
    /// Last-modified time in nanoseconds since the Unix epoch.
    pub mtime_ns: i64,
    /// Content hash, if already computed (otherwise `None`).
    pub content_hash: Option<Vec<u8>>,
}

/// The category of a [`HistoryEntry`], stored as the `kind` text column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    /// A file or directory was opened.
    Open,
    /// A search was run.
    Search,
    /// A command was invoked.
    Command,
}

impl HistoryKind {
    /// The lowercase string stored in the `kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            HistoryKind::Open => "open",
            HistoryKind::Search => "search",
            HistoryKind::Command => "command",
        }
    }

    /// Parse the `kind` column spelling back into a [`HistoryKind`].
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(HistoryKind::Open),
            "search" => Some(HistoryKind::Search),
            "command" => Some(HistoryKind::Command),
            _ => None,
        }
    }
}

/// A single history record (recent files, searches, command usage).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// What kind of event this is.
    pub kind: HistoryKind,
    /// Opaque, caller-defined payload (e.g. a path or query string).
    pub payload: String,
    /// When the event occurred, in nanoseconds since the Unix epoch.
    pub occurred_at: i64,
}

/// A single operation in a [`KvStore::batch`] call, applied atomically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KvOp {
    /// Insert or overwrite `key` with `value`.
    Put {
        /// The key to write.
        key: String,
        /// The value to store.
        value: Vec<u8>,
    },
    /// Remove `key` if present.
    Delete {
        /// The key to remove.
        key: String,
    },
}

/// Read-only access to file metadata. Kept separate from [`MetadataStore`] so
/// callers that only read (e.g. plugins with `read_paths` permission) cannot
/// mutate the table.
pub trait MetadataQuery: Send + Sync {
    /// Fetch the record for `path`, or `None` if it is not tracked.
    fn get_file(&self, path: &std::path::Path) -> Result<Option<FileRecord>>;
    /// List the direct children of directory `parent`.
    fn list_children(&self, parent: &std::path::Path) -> Result<Vec<FileRecord>>;
    /// List records changed after `ts_ns` (modified or logically deleted since),
    /// for incremental re-indexing.
    fn list_changed_since(&self, ts_ns: i64) -> Result<Vec<FileRecord>>;
    /// Find a record by its filesystem inode, or `None` if untracked.
    fn find_by_inode(&self, inode: u64) -> Result<Option<FileRecord>>;
}

/// Read/write access to file metadata.
pub trait MetadataStore: MetadataQuery {
    /// Insert `entry`, or update the existing row with the same path. Returns
    /// the row id.
    fn upsert_file(&self, entry: &FileUpsert) -> Result<FileId>;
    /// Remove the row for `path` (hard delete in P2).
    fn delete_file(&self, path: &std::path::Path) -> Result<()>;
    /// Record that the file `id` was indexed at `indexed_at_ns`.
    fn mark_indexed(&self, id: FileId, indexed_at_ns: i64) -> Result<()>;
}

/// A simple key/value blob store (host KV, backed by redb).
pub trait KvStore: Send + Sync {
    /// Fetch the value for `key`, or `None` if absent.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    /// Insert or overwrite `key` with `value`.
    fn put(&self, key: &str, value: &[u8]) -> Result<()>;
    /// Remove `key` if present.
    fn delete(&self, key: &str) -> Result<()>;
    /// Return every `(key, value)` whose key begins with `prefix`.
    fn list_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>>;
    /// Apply `ops` atomically in a single transaction.
    fn batch(&self, ops: Vec<KvOp>) -> Result<()>;
}

/// Append-only history of recent files, searches and commands.
pub trait HistoryStore: Send + Sync {
    /// Append `entry` to the history.
    fn record(&self, entry: HistoryEntry) -> Result<()>;
    /// Return up to `limit` entries of `kind`, most recent first.
    fn list(&self, kind: HistoryKind, limit: usize) -> Result<Vec<HistoryEntry>>;
}

/// Current time in nanoseconds since the Unix epoch, saturating to 0 if the
/// clock is set before the epoch.
pub(crate) fn now_ns() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_nanos()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::Path;

    // A consumer that only needs to read metadata depends on the narrow
    // `MetadataQuery` trait, so it can be exercised against a hand-written mock
    // without a real database — the point of the interface segregation.
    fn newest_child(query: &dyn MetadataQuery, parent: &Path) -> Result<Option<FileRecord>> {
        Ok(query
            .list_children(parent)?
            .into_iter()
            .max_by_key(|record| record.mtime_ns))
    }

    #[derive(Default)]
    struct MockMetadataQuery {
        children: Vec<FileRecord>,
    }

    impl MetadataQuery for MockMetadataQuery {
        fn get_file(&self, path: &Path) -> Result<Option<FileRecord>> {
            Ok(self
                .children
                .iter()
                .find(|record| record.path == path)
                .cloned())
        }
        fn list_children(&self, _parent: &Path) -> Result<Vec<FileRecord>> {
            Ok(self.children.clone())
        }
        fn list_changed_since(&self, _ts_ns: i64) -> Result<Vec<FileRecord>> {
            Ok(self.children.clone())
        }
        fn find_by_inode(&self, inode: u64) -> Result<Option<FileRecord>> {
            Ok(self
                .children
                .iter()
                .find(|record| record.inode == inode)
                .cloned())
        }
    }

    fn record(path: &str, inode: u64, mtime_ns: i64) -> FileRecord {
        FileRecord {
            id: inode as FileId,
            path: PathBuf::from(path),
            parent_path: PathBuf::from("/home/user"),
            inode,
            size: 0,
            mtime_ns,
            content_hash: None,
            indexed_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn consumer_works_against_metadata_query_mock() {
        let mock = MockMetadataQuery {
            children: vec![
                record("/home/user/a.txt", 1, 100),
                record("/home/user/b.txt", 2, 300),
                record("/home/user/c.txt", 3, 200),
            ],
        };
        let newest = newest_child(&mock, Path::new("/home/user"))
            .unwrap()
            .unwrap();
        assert_eq!(newest.path, PathBuf::from("/home/user/b.txt"));
    }

    #[test]
    fn history_kind_round_trips_through_text() {
        for kind in [HistoryKind::Open, HistoryKind::Search, HistoryKind::Command] {
            assert_eq!(HistoryKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(HistoryKind::parse("unknown"), None);
    }
}
