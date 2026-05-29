//! Centralized configuration constants shared across the application.
//!
//! Values live here (rather than inline literals) so behaviour can be tuned in
//! one place and so the upcoming `nohrs-core` extraction (#48) has a single home
//! for tunables.

use std::time::Duration;

/// Default window dimensions on first launch (logical pixels).
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 780.0;

/// Initial widths of the explorer table columns (logical pixels).
pub const COL_NAME_WIDTH: f32 = 400.0;
pub const COL_TYPE_WIDTH: f32 = 120.0;
pub const COL_SIZE_WIDTH: f32 = 120.0;
pub const COL_MODIFIED_WIDTH: f32 = 180.0;
pub const COL_ACTION_WIDTH: f32 = 60.0;

/// Smallest width a column may be resized to (logical pixels).
pub const MIN_COLUMN_WIDTH: f32 = 80.0;

/// Extra horizontal padding added to the sum of column widths to obtain the
/// total table width (left + right row padding).
pub const TABLE_HORIZONTAL_PADDING: f32 = 48.0;

/// Heights of the rows rendered in the explorer table (logical pixels).
pub const HEADER_ROW_HEIGHT: f32 = 48.0;
pub const BASE_ROW_HEIGHT: f32 = 32.0;
pub const SNIPPET_ROW_HEIGHT: f32 = 24.0;

/// Maximum number of match snippets shown under an expanded search result.
pub const MAX_SNIPPETS: usize = 10;

/// Files larger than this are not previewed inline (bytes).
pub const PREVIEW_MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

/// Maximum number of entries fetched in a single directory listing page.
pub const DIR_LISTING_LIMIT: usize = 1000;

/// Window during which a `Confirm` event following a double-click is suppressed
/// so a double-click does not both preview and re-activate the same row.
pub const CONFIRM_SUPPRESS_WINDOW: Duration = Duration::from_millis(300);

/// Lines longer than this are skipped when grepping file contents, to bound
/// memory use on minified or binary-ish files.
pub const SEARCH_MAX_LINE_LEN: usize = 1000;

/// Upper bound on matches collected from a single file during a content search.
pub const SEARCH_MAX_MATCHES_PER_FILE: usize = 1000;
