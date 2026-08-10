use chronos_fm_core::config;
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_services::fs::provider::FileSystemProvider;
use chronos_fm_services::search::{SearchScope, SearchService};
use chronos_fm_services::syntax::SyntaxService;
use chronos_fm_ui::components::file_list::FileListDelegate;

use gpui::{
    AppContext, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable, Modifiers, Pixels,
    Point, Window, point, px, size,
};
use gpui_component::VirtualListScrollHandle;
use gpui_component::input::InputState;
use gpui_component::list::ListState;
use gpui_component::resizable::ResizableState;
use std::{collections::BTreeMap, rc::Rc, sync::Arc, time::Instant};

use super::entries;
use super::marquee::{
    GeometryToken, MarqueeDrag, MeasuredItem, completion_indices, intersects_closed,
    normalized_rect, past_threshold, selection_for_hits,
};
use super::types::*;
use super::view::preview::editor::PreviewEditor;

/// State for the file explorer page: the current directory listing, navigation
/// history, sorting and filtering, search, preview, and view layout.
pub struct ExplorerPane {
    /// Absolute path of the currently displayed directory.
    pub cwd: String,
    /// Navigation history of visited directories for back/forward.
    pub history: Vec<String>,
    /// Index of the current position within `history`.
    pub history_index: usize,
    /// All entries read from `cwd`, before filtering.
    pub entries: Vec<FileEntryDto>,
    /// Entries currently shown after applying hidden/search filters and sorting.
    pub filtered_entries: Vec<FileEntryDto>,
    // Whether the current directory has been loaded. Tracked explicitly rather
    // than via `entries.is_empty()` so an empty (or failed-to-read) directory is
    // not reloaded on every render.
    /// Whether `cwd` has been loaded into `entries` at least once.
    pub loaded: bool,
    /// Column the listing is sorted by.
    pub sort_key: SortKey,
    /// Whether the sort is ascending.
    pub sort_asc: bool,
    // Whether dotfiles are listed; driven by `ui.show_hidden` (config.md §5).
    /// Whether dotfiles are listed; driven by `ui.show_hidden`.
    pub show_hidden: bool,
    // Identifier of the active icon pack; driven by `ui.icon_pack`.
    /// Identifier of the active icon pack; driven by `ui.icon_pack`.
    pub icon_pack: String,
    /// Current name-filter query for the in-directory listing.
    pub search_query: String,
    /// Whether the search bar is visible.
    pub search_visible: bool,
    /// Input state backing the search field.
    pub search_input: Entity<InputState>,
    /// State of the resizable split between listing and preview.
    pub resizable: Entity<ResizableState>,
    /// The listing's virtualized list entity, once initialized.
    pub list: Option<Entity<ListState<FileListDelegate>>>,
    /// GPUI subscriptions kept alive for the lifetime of the page.
    pub subs: Vec<gpui::Subscription>,
    /// Path of the file currently shown in the preview pane.
    pub preview_path: Option<String>,
    /// Text content of the previewed file, when it is textual.
    pub preview_text: Option<String>,
    /// Indices (into `filtered_entries`) of all currently selected rows.
    pub selection: std::collections::BTreeSet<usize>,
    /// Anchor row for Shift range selection, or `None` when nothing is anchored.
    pub selection_anchor: Option<usize>,
    /// The active/primary row that drives the preview and keyboard navigation.
    pub active_index: Option<usize>,
    /// Scroll handle for the virtualized listing.
    pub virtual_scroll_handle: VirtualListScrollHandle,
    /// Per-row sizes for the virtualized listing.
    pub item_sizes: Rc<Vec<gpui::Size<gpui::Pixels>>>,
    // Column widths (resizable)
    /// Width of the name column.
    pub col_name_width: f32,
    /// Width of the type column.
    pub col_type_width: f32,
    /// Width of the size column.
    pub col_size_width: f32,
    /// Width of the modified-time column.
    pub col_modified_width: f32,
    /// Properties dialog, if open.
    pub properties_dialog: Option<gpui::Entity<crate::explorer::properties::PropertiesDialog>>,
    /// Batch-rename dialog, if open (T006).
    pub batch_rename: Option<gpui::Entity<crate::explorer::batch_rename::BatchRenameDialog>>,
    /// Right-click context menu, if open.
    pub context_menu: Option<crate::explorer::context_menu::ContextMenuState>,
    /// Width of the action column.
    pub col_action_width: f32,
    // Resize state
    /// The column currently being resized by a drag, if any.
    pub resizing_column: Option<ResizingColumn>,
    /// Focus handle for the explorer page.
    pub focus_handle: FocusHandle,
    /// Whether focus should be requested on the next render.
    pub focus_requested: bool,
    /// Information about the most recent row click, used for double-click detection.
    pub last_click_info: Option<LastClickInfo>,
    /// Whether the listing is shown as a list or a grid.
    pub view_mode: ViewMode,
    /// Whether this pane has an in-process file drop awaiting completion.
    pub(crate) drop_pending: bool,
    /// The active empty-space selection drag, if one has begun.
    pub(crate) marquee: Option<MarqueeDrag>,
    /// Item bounds measured in the current listing layout token.
    pub(crate) measured_items: BTreeMap<usize, Bounds<Pixels>>,
    /// Bounds of the listing viewport measured in window coordinates.
    pub(crate) listing_viewport: Option<Bounds<Pixels>>,
    /// Token identifying the layout that produced the current measurements.
    pub(crate) geometry_token: Option<GeometryToken>,
    /// Bounds that may not start an empty-space marquee (headers and controls),
    /// keyed by stable renderer IDs so repeated prepaint callbacks replace data.
    pub(crate) marquee_exclusions: BTreeMap<&'static str, Bounds<Pixels>>,
    /// Monotonic generation for the current filtered entry order.
    pub(crate) entries_revision: u64,
    /// Monotonic generation for row/tile item sizes.
    pub(crate) item_sizes_revision: u64,
    /// Whether the left quick-access sidebar is shown (toggled with `Cmd/Ctrl+B`).
    pub sidebar_visible: bool,

    // Search
    /// The full-text search service, when available.
    pub search_service: Option<Arc<SearchService>>,
    /// The directory scope for full-text search.
    pub search_scope: SearchScope,
    /// The kind of items full-text search targets.
    pub search_type: SearchType,
    /// Whether full-text search is case-sensitive.
    pub match_case: bool,
    /// Whether full-text search matches whole words only.
    pub match_whole_word: bool,
    /// Whether the full-text search query is treated as a regular expression.
    pub use_regex: bool,
    /// Results of the active full-text search, when one is displayed.
    pub search_results: Option<Vec<SearchFileResult>>,
    /// Whether a full-text search is currently running.
    pub is_performing_search: bool,
    // Monotonic counter incremented on every search request; used to discard
    // results from a stale in-flight search that completes after a newer one.
    /// Monotonic counter used to discard results from stale in-flight searches.
    pub search_generation: u64,
    /// Paths of search-result files whose match snippets are expanded.
    pub expanded_search_files: std::collections::HashSet<String>,
    // Syntax
    /// Service providing syntax highlighting for previews.
    pub syntax_service: Arc<SyntaxService>,
    // Preview State
    /// Path of the image currently shown in the preview pane (filesystem).
    pub preview_image_path: Option<String>,
    /// Decoded image bytes for archive members (or when path load is unavailable).
    pub preview_image_data: Option<std::sync::Arc<gpui::Image>>,
    /// Embedded WebKit (wry) used for HTML preview. Lazy-created; kept across
    /// selections and hidden when not showing HTML.
    pub preview_webview: Option<gpui::Entity<gpui_wry::WebView>>,
    /// True while the preview pane should show the HTML webview.
    pub preview_html_active: bool,
    /// Message shown in the preview pane when a file cannot be previewed.
    pub preview_message: Option<String>,
    /// The editor entity backing a text preview.
    pub preview_editor: Option<Entity<PreviewEditor>>,
    // Transient message shown in the footer status bar.
    /// Transient message shown in the footer status bar.
    pub status_message: Option<StatusMessage>,
    /// The row currently being renamed inline (an index into
    /// `filtered_entries`) and the input state backing its text field, or
    /// `None` when no row is being renamed.
    pub renaming: Option<(usize, Entity<InputState>)>,
    /// Optional virtual FS provider. When `Some`, all filesystem operations
    /// (listing, reading, writing) are dispatched through the provider
    /// instead of the local filesystem (T011).
    pub provider: Option<std::sync::Arc<dyn FileSystemProvider>>,
    /// Cached sidebar shortcuts (Home, Desktop, Downloads, …). Computed once
    /// at construction; the set of existing user directories does not change
    /// at runtime (T015).
    pub shortcuts: Vec<(String, String)>,
}

impl Focusable for ExplorerPane {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PaneEvent> for ExplorerPane {}

impl crate::pane_group::PaneItem for ExplorerPane {
    fn tab_title(&self, _cx: &gpui::App) -> String {
        std::path::Path::new(&self.cwd)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.cwd.clone())
    }
}

impl ExplorerPane {
    /// Builds a self-contained pane, creating the window-bound sub-entities
    /// (listing/preview resizable, search input) and focus handle it owns. Used
    /// by the split-view container, which may hold several independent panes.
    pub fn build(
        search_service: Option<Arc<SearchService>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let resizable = cx.new(|_| ResizableState::default());
        let search_input = cx.new(|cx| InputState::new(window, cx));
        let mut pane = Self::new(resizable, search_input, search_service, cx.focus_handle());
        let focus_handle = pane.focus_handle.clone();
        let focus_out = cx.on_focus_out(&focus_handle, window, |pane, _event, _window, cx| {
            pane.cancel_marquee();
            cx.notify();
        });
        pane.subs.push(focus_out);
        pane
    }

    /// Creates a new explorer pane rooted at the current working directory,
    /// wired to the given resizable, search input, and optional search service.
    pub fn new(
        resizable: Entity<ResizableState>,
        search_input: Entity<InputState>,
        search_service: Option<Arc<SearchService>>,
        focus_handle: FocusHandle,
    ) -> Self {
        Self {
            properties_dialog: None,
            batch_rename: None,
            context_menu: None,
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".into()),
            history: Vec::new(),
            history_index: 0,
            entries: Vec::new(),
            filtered_entries: Vec::new(),
            loaded: false,
            sort_key: SortKey::Name,
            sort_asc: true,
            show_hidden: false,
            icon_pack: "default".to_string(),
            search_query: String::new(),
            search_visible: false,
            search_input,
            resizable,
            list: None,
            subs: Vec::new(),
            preview_path: None,
            preview_text: None,
            selection: std::collections::BTreeSet::new(),
            selection_anchor: None,
            active_index: None,
            virtual_scroll_handle: VirtualListScrollHandle::new(),
            item_sizes: Rc::new(Vec::new()),
            col_name_width: config::COL_NAME_WIDTH,
            col_type_width: config::COL_TYPE_WIDTH,
            col_size_width: config::COL_SIZE_WIDTH,
            col_modified_width: config::COL_MODIFIED_WIDTH,
            col_action_width: config::COL_ACTION_WIDTH,
            resizing_column: None,
            focus_handle,
            focus_requested: false,
            last_click_info: None,
            view_mode: ViewMode::List,
            drop_pending: false,
            marquee: None,
            measured_items: BTreeMap::new(),
            listing_viewport: None,
            geometry_token: None,
            marquee_exclusions: BTreeMap::new(),
            entries_revision: 0,
            item_sizes_revision: 0,
            // Visible by default so the Places sidebar shows on first launch
            // (mockup / Dolphin parity). Split-created panes override this
            // via `ExplorerPage::configure_tab` (issue #164, §2).
            sidebar_visible: true,

            // Search
            search_service,
            search_scope: SearchScope::Home,
            search_type: SearchType::All,
            match_case: false,
            match_whole_word: false,
            use_regex: false,
            search_results: None,
            is_performing_search: false,
            search_generation: 0,
            expanded_search_files: std::collections::HashSet::new(),
            syntax_service: Arc::new(SyntaxService::new()),
            preview_editor: None,
            preview_image_path: None,
            preview_image_data: None,
            preview_webview: None,
            preview_html_active: false,
            preview_message: None,
            status_message: None,
            renaming: None,
            provider: None,
            shortcuts: compute_shortcuts(),
        }
    }

    pub(crate) fn set_status(&mut self, level: StatusLevel, text: impl Into<String>) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            level,
        });
    }

    pub(crate) fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Toggles the left quick-access sidebar (issue #164, §2). Mirrors the
    /// `toggle_search` open/close pattern.
    pub(crate) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    /// Returns the current status text and whether it represents an error, for
    /// rendering in the footer status bar.
    pub fn status_for_footer(&self) -> Option<(String, bool)> {
        self.status_message
            .as_ref()
            .map(|status| (status.text.clone(), status.level == StatusLevel::Error))
    }

    pub(crate) fn set_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        if self.search_scope != scope {
            self.search_scope = scope;
            cx.notify();
        }
    }

    pub(crate) fn toggle_match_case(&mut self, cx: &mut Context<Self>) {
        self.match_case = !self.match_case;
        cx.notify();
    }

    pub(crate) fn toggle_match_whole_word(&mut self, cx: &mut Context<Self>) {
        self.match_whole_word = !self.match_whole_word;
        cx.notify();
    }

    pub(crate) fn toggle_use_regex(&mut self, cx: &mut Context<Self>) {
        self.use_regex = !self.use_regex;
        cx.notify();
    }

    pub(crate) fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        if self.view_mode != mode {
            self.cancel_marquee();
            self.view_mode = mode;
            self.invalidate_marquee_measurements();
            cx.notify();
        }
    }

    pub(crate) fn record_click(&mut self, row: usize, click_count: usize) {
        self.last_click_info = Some(LastClickInfo {
            row,
            timestamp: Instant::now(),
            click_count,
        });
    }

    /// Returns whether the row at `ix` (an index into `filtered_entries`) is
    /// part of the current selection.
    pub fn is_selected(&self, ix: usize) -> bool {
        self.selection.contains(&ix)
    }

    /// Replaces the selection with the single row `ix`, making it both the
    /// anchor and the active row (a plain click or arrow-key move).
    pub(crate) fn select_single(&mut self, ix: usize) {
        self.selection.clear();
        self.selection.insert(ix);
        self.selection_anchor = Some(ix);
        self.active_index = Some(ix);
    }

    /// Toggles row `ix` in the selection (Cmd/Ctrl+click) and re-anchors there.
    pub(crate) fn toggle_select(&mut self, ix: usize) {
        if !self.selection.remove(&ix) {
            self.selection.insert(ix);
        }
        self.selection_anchor = Some(ix);
        self.active_index = Some(ix);
    }

    /// Selects the contiguous range between the current anchor and `ix`
    /// (Shift+click / Shift+arrow). Falls back to a single selection when there
    /// is no anchor yet.
    pub(crate) fn select_range_to(&mut self, ix: usize) {
        let anchor = match self.selection_anchor {
            Some(anchor) => anchor,
            None => {
                self.select_single(ix);
                return;
            }
        };
        let (low, high) = if anchor <= ix {
            (anchor, ix)
        } else {
            (ix, anchor)
        };
        self.selection = (low..=high).collect();
        self.active_index = Some(ix);
    }

    /// Selects every visible row.
    pub(crate) fn select_all(&mut self) {
        let len = self.filtered_entries.len();
        self.selection = (0..len).collect();
        self.selection_anchor = if len > 0 { Some(0) } else { None };
        self.active_index = len.checked_sub(1);
    }

    /// Clears the selection, anchor, and active row.
    pub(crate) fn clear_selection(&mut self) {
        self.selection.clear();
        self.selection_anchor = None;
        self.active_index = None;
    }

    /// Records the listing's measured viewport and returns the token rows/tiles
    /// must attach to measurements made during the same layout pass.
    pub(crate) fn record_listing_viewport(
        &mut self,
        viewport: Bounds<Pixels>,
        scroll_offset: Point<Pixels>,
    ) -> GeometryToken {
        let token = GeometryToken {
            view_mode: self.view_mode,
            entries_revision: self.entries_revision,
            viewport,
            scroll_offset,
            item_sizes_revision: self.item_sizes_revision,
        };

        if self.geometry_token.as_ref() != Some(&token) {
            self.cancel_marquee();
            self.measured_items.clear();
            self.marquee_exclusions.clear();
            self.geometry_token = Some(token.clone());
        }
        self.listing_viewport = Some(viewport);
        token
    }

    /// Records a row or tile bound only when it belongs to the current layout.
    pub(crate) fn record_item_bounds(
        &mut self,
        index: usize,
        bounds: Bounds<Pixels>,
        token: GeometryToken,
    ) {
        if self.geometry_token.as_ref() == Some(&token) {
            self.measured_items.insert(index, bounds);
        }
    }

    /// Records non-item listing chrome that must not be treated as empty space.
    pub(crate) fn record_marquee_exclusion(
        &mut self,
        id: &'static str,
        bounds: Bounds<Pixels>,
        token: GeometryToken,
    ) {
        if self.geometry_token.as_ref() == Some(&token) {
            self.marquee_exclusions.insert(id, bounds);
        }
    }

    /// Starts an empty-space marquee and returns whether the press was accepted.
    pub(crate) fn begin_marquee(&mut self, position: Point<Pixels>, modifiers: Modifiers) -> bool {
        let Some(viewport) = self.listing_viewport else {
            return false;
        };
        let Some(token) = self.geometry_token.clone() else {
            return false;
        };
        if !point_in_bounds(position, viewport)
            || self
                .measured_items
                .values()
                .any(|bounds| point_in_bounds(position, *bounds))
            || self
                .marquee_exclusions
                .values()
                .any(|bounds| point_in_bounds(position, *bounds))
        {
            return false;
        }

        let additive = modifiers.control || modifiers.platform;
        let base_selection = self.selection.clone();
        let prior_anchor = self.selection_anchor;
        let prior_active = self.active_index;
        if !additive {
            self.clear_selection();
        }
        self.marquee = Some(MarqueeDrag {
            start: position,
            current: position,
            token,
            hitboxes: self
                .measured_items
                .iter()
                .map(|(&index, &bounds)| MeasuredItem { index, bounds })
                .collect(),
            base_selection,
            prior_anchor,
            prior_active,
            additive,
            dragging: false,
            hit_indices: Default::default(),
        });
        true
    }

    /// Updates the live selection for the active marquee. A stale layout token
    /// drops the drag while retaining the last selection already shown.
    pub(crate) fn update_marquee(&mut self, position: Point<Pixels>) {
        let Some(drag) = self.marquee.as_mut() else {
            return;
        };
        drag.current = position;
        drag.dragging |= past_threshold(drag.start, drag.current);
        let token_is_current = self.geometry_token.as_ref() == Some(&drag.token);
        let dragging = drag.dragging;
        if !token_is_current {
            self.cancel_marquee();
            return;
        }
        if !dragging {
            return;
        }

        let Some(viewport) = self.listing_viewport else {
            return;
        };
        let (hits, selection) = {
            let drag = self
                .marquee
                .as_ref()
                .expect("the active marquee was validated above");
            let hits = match clip_to_bounds(normalized_rect(drag.start, drag.current), viewport) {
                Some(rect) => drag
                    .hitboxes
                    .iter()
                    .filter(|item| intersects_closed(rect, item.bounds))
                    .map(|item| item.index)
                    .collect::<std::collections::BTreeSet<_>>(),
                None => Default::default(),
            };
            let selection = selection_for_hits(
                &drag.base_selection,
                hits.iter().copied(),
                drag.additive,
            );
            (hits, selection)
        };
        if let Some(drag) = self.marquee.as_mut() {
            drag.hit_indices = hits;
        }
        self.selection = selection;
    }

    /// Returns the visible, clipped marquee rectangle after drag threshold.
    pub(crate) fn marquee_rect(&self) -> Option<Bounds<Pixels>> {
        let drag = self.marquee.as_ref()?;
        if !drag.dragging {
            return None;
        }
        let viewport = self.listing_viewport?;
        clip_to_bounds(normalized_rect(drag.start, drag.current), viewport)
    }

    /// Completes a marquee, applying its stable anchor and active-row rules.
    pub(crate) fn finish_marquee(&mut self) {
        let Some(drag) = self.marquee.take() else {
            return;
        };
        if drag.dragging {
            (self.selection_anchor, self.active_index) = completion_indices(
                &drag.hit_indices,
                drag.additive,
                drag.prior_anchor,
                drag.prior_active,
            );
        }
    }

    /// Cancels only the interaction state, leaving live selection unchanged.
    pub(crate) fn cancel_marquee(&mut self) {
        self.marquee = None;
    }

    fn invalidate_marquee_measurements(&mut self) {
        self.cancel_marquee();
        self.measured_items.clear();
        self.marquee_exclusions.clear();
        self.listing_viewport = None;
        self.geometry_token = None;
    }

    /// Moves the active row by `delta` rows, clamped to the visible range. With
    /// `extend` (Shift held) the selection grows from the anchor; otherwise the
    /// moved-to row becomes the sole selection. Selecting from an empty state
    /// lands on the first row.
    pub(crate) fn move_active(&mut self, delta: isize, extend: bool) {
        let len = self.filtered_entries.len();
        if len == 0 {
            return;
        }
        let next = match self.active_index {
            Some(current) => (current as isize + delta).clamp(0, len as isize - 1) as usize,
            None => 0,
        };
        if extend {
            self.select_range_to(next);
        } else {
            self.select_single(next);
        }
    }

    /// Paths of the currently selected rows, in row order. Used by file
    /// operations and drag-and-drop.
    pub fn selected_paths(&self) -> Vec<String> {
        self.selection
            .iter()
            .filter_map(|&ix| self.filtered_entries.get(ix))
            .map(|entry| entry.path.clone())
            .collect()
    }

    /// The selected entries in row order, for operations that need the full
    /// `FileEntryDto` (Batch Rename's "was" side, T006).
    pub fn filtered_entries_for_selection(&self) -> Vec<FileEntryDto> {
        self.selection
            .iter()
            .filter_map(|&ix| self.filtered_entries.get(ix).cloned())
            .collect()
    }

    pub(crate) fn update_item_sizes(&mut self) {
        let total_width = self.total_table_width();

        let sizes = self
            .filtered_entries
            .iter()
            .map(|entry| {
                // Check if there are match snippets for this file AND it's expanded
                let is_expanded = self.expanded_search_files.contains(&entry.path);
                let snippet_count = if is_expanded {
                    self.search_results
                        .as_ref()
                        .and_then(|results| {
                            results
                                .iter()
                                .find(|r| r.path == entry.path)
                                .map(|r| r.matches.len().min(config::MAX_SNIPPETS))
                        })
                        .unwrap_or(0)
                } else {
                    0
                };
                let total_height =
                    config::BASE_ROW_HEIGHT + (snippet_count as f32 * config::SNIPPET_ROW_HEIGHT);
                size(px(total_width), px(total_height))
            })
            .collect();
        if self.item_sizes.as_ref() != &sizes {
            self.item_sizes = Rc::new(sizes);
            self.item_sizes_revision = self.item_sizes_revision.wrapping_add(1);
            self.invalidate_marquee_measurements();
        }
    }

    pub(crate) fn total_table_width(&self) -> f32 {
        self.col_name_width
            + self.col_type_width
            + self.col_size_width
            + self.col_modified_width
            + self.col_action_width
            + config::TABLE_HORIZONTAL_PADDING
    }

    pub(crate) fn apply_filter(&mut self) {
        // Rebuilding the visible row set (here or via the search path) invalidates
        // the row indices the selection is expressed in, so reset it rather than
        // risk acting on unrelated rows after a sort/filter/reload/search. The
        // inline-rename index is expressed the same way, so it is reset too —
        // committing against a stale row after a listing change would rename the
        // wrong entry.
        self.cancel_marquee();
        self.clear_selection();
        self.renaming = None;

        // When explicit search results are displayed, `filtered_entries` is owned
        // by the search path; only refresh row sizes here.
        if self.search_results.is_some() {
            self.update_item_sizes();
            return;
        }

        let show_hidden = self.show_hidden;
        let visible = self
            .entries
            .iter()
            .filter(move |entry| show_hidden || !is_hidden(&entry.name));

        let filtered_entries: Vec<FileEntryDto> = if self.search_query.is_empty() {
            visible.cloned().collect()
        } else {
            let query = self.search_query.to_lowercase();
            visible
                .filter(|e| e.name.to_lowercase().contains(&query))
                .cloned()
                .collect()
        };

        let mut filtered_entries = filtered_entries;
        entries::sort_entries(&mut filtered_entries, self.sort_key, self.sort_asc);
        self.replace_filtered_entries(filtered_entries);
        self.update_item_sizes();
    }

    /// Replaces index-addressed listing entries and invalidates measurements when
    /// their visible order changes. Async search and reload failures use this
    /// path instead of assigning `filtered_entries` directly.
    pub(crate) fn replace_filtered_entries(&mut self, entries: Vec<FileEntryDto>) {
        let prior_paths = self
            .filtered_entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        self.cancel_marquee();
        let order_changed = filtered_entry_order_changed(&entries, &prior_paths);
        self.filtered_entries = entries;
        if order_changed {
            self.entries_revision = self.entries_revision.wrapping_add(1);
            self.invalidate_marquee_measurements();
        }
    }

    /// Apply the `[ui]` config section to this open view, re-sorting and
    /// re-filtering when anything changed. Used both at startup and on hot
    /// reload (config.md §5). Note: `icon_pack` is stored for the row renderer to
    /// consult; there is no icon cache to invalidate yet.
    pub fn apply_config_ui(&mut self, ui: &config::Ui, cx: &mut Context<Self>) {
        let sort_key = sort_key_from_config(ui.default_sort);
        let mut changed = false;
        if self.sort_key != sort_key {
            self.sort_key = sort_key;
            self.sort_asc = true;
            changed = true;
        }
        if self.show_hidden != ui.show_hidden {
            self.show_hidden = ui.show_hidden;
            changed = true;
        }
        if self.icon_pack != ui.icon_pack {
            self.icon_pack = ui.icon_pack.clone();
            changed = true;
        }
        if changed {
            entries::sort_entries(&mut self.entries, self.sort_key, self.sort_asc);
            self.apply_filter();
            cx.notify();
        }
    }

    pub(crate) fn set_sort_key(&mut self, key: SortKey) {
        if self.sort_key == key {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_key = key;
            self.sort_asc = true;
        }
        entries::sort_entries(&mut self.entries, self.sort_key, self.sort_asc);
        self.apply_filter();
    }
}

fn point_in_bounds(point: Point<Pixels>, bounds: Bounds<Pixels>) -> bool {
    point.x >= bounds.left()
        && point.x <= bounds.right()
        && point.y >= bounds.top()
        && point.y <= bounds.bottom()
}

fn clip_to_bounds(rect: Bounds<Pixels>, viewport: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    let left = rect.left().as_f32().max(viewport.left().as_f32());
    let top = rect.top().as_f32().max(viewport.top().as_f32());
    let right = rect.right().as_f32().min(viewport.right().as_f32());
    let bottom = rect.bottom().as_f32().min(viewport.bottom().as_f32());
    if right < left || bottom < top {
        return None;
    }
    Some(Bounds::new(
        point(px(left), px(top)),
        size(px(right - left), px(bottom - top)),
    ))
}

fn filtered_entry_order_changed(entries: &[FileEntryDto], prior_paths: &[String]) -> bool {
    entries.len() != prior_paths.len()
        || entries
            .iter()
            .zip(prior_paths)
            .any(|(entry, prior_path)| entry.path != *prior_path)
}

fn sort_key_from_config(order: config::SortOrder) -> SortKey {
    match order {
        config::SortOrder::Name => SortKey::Name,
        config::SortOrder::Modified => SortKey::Modified,
        config::SortOrder::Size => SortKey::Size,
        config::SortOrder::Kind => SortKey::Type,
    }
}

/// Build the quick-access folder list (Home + common user directories that
/// exist). Called once at pane construction; the filesystem is only touched
/// at startup, not on every render (T015).
fn compute_shortcuts() -> Vec<(String, String)> {
    let mut v = Vec::new();
    let home = std::env::var("HOME").ok();
    #[cfg(target_os = "windows")]
    let home = home.or_else(|| std::env::var("USERPROFILE").ok());
    if let Some(h) = home {
        let p = |s: &str| {
            std::path::Path::new(&h)
                .join(s)
                .to_string_lossy()
                .to_string()
        };
        v.push(("Home".into(), h.clone()));
        for (label, sub) in [
            ("Desktop", "Desktop"),
            ("Documents", "Documents"),
            ("Downloads", "Downloads"),
            ("Music", "Music"),
            ("Pictures", "Pictures"),
            ("Videos", "Videos"),
        ] {
            let path = p(sub);
            if std::path::Path::new(&path).exists() {
                v.push((label.into(), path));
            }
        }
    }
    v
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_hidden_matches_dotfiles_only() {
        assert!(is_hidden(".gitignore"));
        assert!(is_hidden("..git"));
        assert!(!is_hidden("visible.txt"));
        assert!(!is_hidden(""));
    }

    #[test]
    fn sort_key_from_config_maps_every_order() {
        assert!(sort_key_from_config(config::SortOrder::Name) == SortKey::Name);
        assert!(sort_key_from_config(config::SortOrder::Modified) == SortKey::Modified);
        assert!(sort_key_from_config(config::SortOrder::Size) == SortKey::Size);
        assert!(sort_key_from_config(config::SortOrder::Kind) == SortKey::Type);
    }
}
