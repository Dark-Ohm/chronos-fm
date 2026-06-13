//! Directory listing with stable, case-insensitive ordering and cursor paging,
//! plus synchronous file mutation operations (copy/move/rename/delete/trash).

/// Synchronous directory listing.
pub mod listing;
/// Synchronous file mutation operations with cross-volume and conflict handling.
pub mod ops;
