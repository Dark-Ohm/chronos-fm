use super::ExplorerPane;
use super::types::{PaneEvent, StatusLevel};
use super::undo::{UndoEntry, transfer_entry};
use super::view::listing::{row::icon_path_for, truncate_middle};
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_services::fs::ops::{TransferReport, transfer_paths_resolved};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::{
    App, AppContext, Context, CursorStyle, Entity, EntityId, IntoElement, Modifiers, Render,
    SharedString, WeakEntity, Window, div, px,
};
use gpui_component::Icon;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Immutable local-file payload carried by an in-process file drag.
#[derive(Clone)]
pub(crate) struct FileDrag {
    paths: Arc<[PathBuf]>,
    initiating_name: SharedString,
    initiating_path: PathBuf,
    initiating_kind: SharedString,
    source: Entity<ExplorerPane>,
    source_id: EntityId,
}

impl FileDrag {
    pub(crate) fn new(
        paths: Vec<PathBuf>,
        initiating_name: impl Into<SharedString>,
        initiating_path: PathBuf,
        initiating_kind: impl Into<SharedString>,
        source: Entity<ExplorerPane>,
    ) -> Self {
        let source_id = source.entity_id();
        Self {
            paths: paths.into(),
            initiating_name: initiating_name.into(),
            initiating_path,
            initiating_kind: initiating_kind.into(),
            source,
            source_id,
        }
    }

    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub(crate) fn initiating_path(&self) -> &Path {
        &self.initiating_path
    }

    pub(crate) fn source(&self) -> &Entity<ExplorerPane> {
        &self.source
    }

    pub(crate) fn source_id(&self) -> EntityId {
        self.source_id
    }

    pub(crate) fn preview(&self) -> FileDragPreview {
        FileDragPreview {
            name: self.initiating_name.clone(),
            kind: self.initiating_kind.clone(),
            item_count: self.paths.len(),
        }
    }

    /// Applies the selection and marquee changes that belong to actual drag
    /// initiation rather than the preceding press.
    pub(crate) fn activate(&self, cx: &mut App) {
        let initiating_path = self.initiating_path.to_string_lossy();
        let _ = self.source.update(cx, |pane, cx| {
            let mut changed = false;
            if let Some(index) = pane
                .filtered_entries
                .iter()
                .position(|entry| entry.path == initiating_path)
                && !pane.is_selected(index)
            {
                pane.select_single(index);
                changed = true;
            }
            if pane.marquee.is_some() {
                pane.cancel_marquee();
                changed = true;
            }
            if changed {
                cx.notify();
            }
        });
    }
}

/// Builds the immutable local payload exposed by a production row or tile.
pub(crate) fn file_drag_for_item(
    pane: &ExplorerPane,
    item: &FileEntryDto,
    index: usize,
    source: Entity<ExplorerPane>,
) -> Option<FileDrag> {
    if pane.provider.is_some()
        || pane
            .renaming
            .as_ref()
            .is_some_and(|(renaming_index, _)| *renaming_index == index)
    {
        return None;
    }

    let paths = if pane.is_selected(index) {
        pane.selected_paths()
            .into_iter()
            .map(PathBuf::from)
            .collect()
    } else {
        vec![PathBuf::from(&item.path)]
    };
    Some(FileDrag::new(
        paths,
        item.name.clone(),
        PathBuf::from(&item.path),
        item.kind.clone(),
        source,
    ))
}

/// Concrete local directory and surface kind receiving a file drop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DropTarget {
    pub(crate) directory: PathBuf,
    pub(crate) kind: DropTargetKind,
}

/// Surface that supplied a concrete drop destination.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DropTargetKind {
    FolderItem,
    ListingCwd,
    BreadcrumbCwd,
}

/// Filesystem operation selected from modifiers at drop time.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DropMode {
    Move,
    Copy,
}

impl From<DropMode> for chronos_fm_services::fs::ops::TransferMode {
    fn from(mode: DropMode) -> Self {
        match mode {
            DropMode::Move => Self::Move,
            DropMode::Copy => Self::Copy,
        }
    }
}

/// Predictable reason a complete payload cannot be dropped on a target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DropValidationError {
    EmptyPayload,
    DestinationNotDirectory,
    SourceUnavailable(PathBuf),
    SourceIsDestination(PathBuf),
    DirectoryContainsDestination(PathBuf),
    SameParentMove(PathBuf),
}

impl fmt::Display for DropValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPayload => write!(f, "the drop contains no local paths"),
            Self::DestinationNotDirectory => write!(f, "the drop destination is not a directory"),
            Self::SourceUnavailable(path) => {
                write!(f, "source is unavailable: {}", path.display())
            }
            Self::SourceIsDestination(path) => {
                write!(f, "source is the drop destination: {}", path.display())
            }
            Self::DirectoryContainsDestination(path) => write!(
                f,
                "a directory cannot be dropped into itself: {}",
                path.display()
            ),
            Self::SameParentMove(path) => {
                write!(
                    f,
                    "source is already in the destination: {}",
                    path.display()
                )
            }
        }
    }
}

/// Resolves the operation from the modifiers observed at mouse-up.
pub(crate) fn drop_mode(modifiers: Modifiers) -> DropMode {
    if modifiers.control || modifiers.platform {
        DropMode::Copy
    } else {
        DropMode::Move
    }
}

/// Changes the active file-drag cursor only when the requested state differs.
pub(crate) fn set_file_drag_cursor(cursor: CursorStyle, window: &mut Window, cx: &mut App) {
    if cx.active_drag_cursor_style() != Some(cursor) {
        cx.set_active_drag_cursor_style(cursor, window);
    }
}

/// Validates a complete local payload against one concrete destination.
pub(crate) fn validate_drop(
    paths: &[PathBuf],
    target: &DropTarget,
    mode: DropMode,
) -> Result<(), DropValidationError> {
    if paths.is_empty() {
        return Err(DropValidationError::EmptyPayload);
    }

    let destination = target
        .directory
        .canonicalize()
        .map_err(|_| DropValidationError::DestinationNotDirectory)?;
    if !destination.is_dir() {
        return Err(DropValidationError::DestinationNotDirectory);
    }

    for path in paths {
        let source_parent = path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .ok_or_else(|| DropValidationError::SourceUnavailable(path.clone()))?;
        let source = path
            .canonicalize()
            .map_err(|_| DropValidationError::SourceUnavailable(path.clone()))?;
        if source == destination {
            return Err(DropValidationError::SourceIsDestination(path.clone()));
        }
        if source.is_dir() && destination.starts_with(&source) {
            return Err(DropValidationError::DirectoryContainsDestination(
                path.clone(),
            ));
        }
        if mode == DropMode::Move && source_parent == destination {
            return Err(DropValidationError::SameParentMove(path.clone()));
        }
    }

    Ok(())
}

impl ExplorerPane {
    /// Returns whether this pane and the source pane currently accept a
    /// complete payload for the concrete target.
    pub(crate) fn can_accept_file_drop(
        &self,
        target_id: EntityId,
        drag: &FileDrag,
        target: &DropTarget,
        modifiers: Modifiers,
        cx: &App,
    ) -> bool {
        if self.provider.is_some() || self.drop_pending {
            return false;
        }
        if drag.source_id() != target_id && drag.source().read(cx).provider.is_some() {
            return false;
        }
        validate_drop(drag.paths(), target, drop_mode(modifiers)).is_ok()
    }

    /// Applies the cwd target's measured-item exclusion before normal drop
    /// validation, preventing invalid item targets from falling through.
    pub(crate) fn can_accept_listing_cwd_drop(
        &self,
        target_id: EntityId,
        drag: &FileDrag,
        target: &DropTarget,
        window: &Window,
        cx: &App,
    ) -> bool {
        let position = window.mouse_position();
        self.listing_viewport
            .is_some_and(|viewport| viewport.contains(&position))
            && self
                .measured_items
                .values()
                .all(|bounds| !bounds.contains(&position))
            && self.can_accept_file_drop(target_id, drag, target, window.modifiers(), cx)
    }

    /// Starts one validated local transfer and returns whether it was
    /// accepted. Destination collisions pause the drop behind the conflict
    /// dialog (T053) instead of silently auto-renaming.
    pub(crate) fn begin_file_drop(
        &mut self,
        drag: FileDrag,
        target: DropTarget,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.provider.is_some() || self.drop_pending || self.conflict_dialog.is_some() {
            return false;
        }

        let target_id = cx.entity().entity_id();
        if drag.source_id() != target_id && drag.source().read(cx).provider.is_some() {
            return false;
        }

        let mode = drop_mode(modifiers);
        if let Err(error) = validate_drop(drag.paths(), &target, mode) {
            tracing::debug!(?target.kind, %error, "rejected file drop");
            return false;
        }

        let paths = drag.paths().to_vec();
        let destination = target.directory;
        let source = drag.source().downgrade();
        let source_id = drag.source_id();

        self.drop_pending = true;
        self.cancel_marquee();
        cx.notify();

        let arg_paths = paths.clone();
        let arg_destination = destination.clone();
        self.transfer_with_conflict_dialog(
            arg_paths,
            arg_destination,
            cx,
            move |_pane, cx, resolutions| {
                let background = cx.background_spawn(async move {
                    transfer_paths_resolved(&paths, &destination, mode.into(), &resolutions)
                });
                cx.spawn(async move |target, cx| {
                    let report = background.await;
                    complete_file_drop(target, source, target_id, source_id, mode, report, cx);
                })
                .detach();
            },
        );
        true
    }

    /// Whether an OS-originated (external) drop payload can land on `target`.
    /// External payloads carry no source pane, so the source-pane checks in
    /// [`Self::can_accept_file_drop`] do not apply — validation is purely
    /// against local paths, destination and drop-time modifiers (T052).
    pub(crate) fn can_accept_external_drop(
        &self,
        paths: &[PathBuf],
        target: &DropTarget,
        modifiers: Modifiers,
    ) -> bool {
        if self.provider.is_some() || self.drop_pending {
            return false;
        }
        validate_drop(paths, target, drop_mode(modifiers)).is_ok()
    }

    /// Applies the cwd target's measured-item exclusion to an external
    /// payload, mirroring [`Self::can_accept_listing_cwd_drop`] from T051.
    pub(crate) fn can_accept_listing_cwd_external_drop(
        &self,
        paths: &[PathBuf],
        target: &DropTarget,
        window: &Window,
    ) -> bool {
        let position = window.mouse_position();
        self.listing_viewport
            .is_some_and(|viewport| viewport.contains(&position))
            && self
                .measured_items
                .values()
                .all(|bounds| !bounds.contains(&position))
            && self.can_accept_external_drop(paths, target, window.modifiers())
    }

    /// Starts one validated local transfer for an OS-originated drop. The
    /// payload has no source pane, so completion only refreshes the
    /// destination pane — there is no cross-pane source reload (T052).
    /// Destination collisions pause the drop behind the conflict dialog (T053)
    /// instead of silently auto-renaming.
    pub(crate) fn begin_external_drop(
        &mut self,
        paths: Vec<PathBuf>,
        target: DropTarget,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.provider.is_some() || self.drop_pending || self.conflict_dialog.is_some() {
            return false;
        }

        let mode = drop_mode(modifiers);
        if let Err(error) = validate_drop(&paths, &target, mode) {
            tracing::debug!(?target.kind, %error, "rejected external file drop");
            return false;
        }

        let destination = target.directory;
        self.drop_pending = true;
        self.cancel_marquee();
        cx.notify();

        let arg_paths = paths.clone();
        let arg_destination = destination.clone();
        self.transfer_with_conflict_dialog(
            arg_paths,
            arg_destination,
            cx,
            move |_pane, cx, resolutions| {
                let background = cx.background_spawn(async move {
                    transfer_paths_resolved(&paths, &destination, mode.into(), &resolutions)
                });
                cx.spawn(async move |target, cx| {
                    let report = background.await;
                    complete_external_drop(target, mode, report, cx);
                })
                .detach();
            },
        );
        true
    }
}

/// Human-readable status text for a transfer that had failures, or `None`
/// when everything succeeded. Shared by internal (T051) and external (T052)
/// drop completion so both surfaces report identically.
fn drop_failure_status(report: &TransferReport) -> Option<String> {
    let success_count = report.successes.len();
    let failure_count = report.failures.len();
    if failure_count == 0 {
        return None;
    }
    let failures = report
        .failures
        .iter()
        .map(|failure| {
            let name = failure
                .source
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| failure.source.display().to_string());
            format!("{name}: {}", failure.error)
        })
        .collect::<Vec<_>>();
    if success_count == 0 {
        Some(format!(
            "Drop failed: {failure_count} failed; {}",
            failures.join(", ")
        ))
    } else {
        Some(format!(
            "Drop partially completed: {success_count} succeeded, {failure_count} failed; {}",
            failures.join(", ")
        ))
    }
}

fn complete_file_drop<C: AppContext>(
    target: WeakEntity<ExplorerPane>,
    source: WeakEntity<ExplorerPane>,
    target_id: EntityId,
    source_id: EntityId,
    mode: DropMode,
    report: TransferReport,
    cx: &mut C,
) {
    let success_count = report.successes.len();
    let failure_count = report.failures.len();
    let destinations = report
        .successes
        .iter()
        .map(|success| success.destination.clone())
        .collect::<Vec<_>>();
    let failure_status = drop_failure_status(&report);
    // T054: one window-level undo entry for the whole drop gesture.
    let undo_entry = transfer_entry(&report, mode.into());

    tracing::info!(
        success_count,
        failure_count,
        ?mode,
        "file drop filesystem work completed"
    );

    if target_id == source_id {
        if target
            .update(cx, move |pane, cx| {
                finish_target_drop(pane, &destinations, failure_status, undo_entry, cx);
            })
            .is_err()
        {
            tracing::info!(
                success_count,
                failure_count,
                "file drop target disappeared before same-pane completion"
            );
        }
        return;
    }

    if target
        .update(cx, move |pane, cx| {
            finish_target_drop(pane, &destinations, failure_status, undo_entry, cx);
        })
        .is_err()
    {
        tracing::info!(
            success_count,
            failure_count,
            "file drop target disappeared before completion"
        );
    }

    if mode == DropMode::Move && success_count > 0 {
        if source
            .update(cx, |pane, cx| {
                pane.reload();
                pane.clear_selection();
                cx.notify();
            })
            .is_err()
        {
            tracing::info!(
                success_count,
                failure_count,
                "file drop source disappeared before completion"
            );
        }
    }
}

/// Finishes an OS-originated drop: refreshes the destination pane and reports
/// any failures. There is no source pane to reload, unlike
/// [`complete_file_drop`] (T052).
fn complete_external_drop<C: AppContext>(
    target: WeakEntity<ExplorerPane>,
    mode: DropMode,
    report: TransferReport,
    cx: &mut C,
) {
    let success_count = report.successes.len();
    let failure_count = report.failures.len();
    let destinations = report
        .successes
        .iter()
        .map(|success| success.destination.clone())
        .collect::<Vec<_>>();
    let failure_status = drop_failure_status(&report);
    // T054: one window-level undo entry for the whole external drop gesture.
    let undo_entry = transfer_entry(&report, mode.into());

    tracing::info!(
        success_count,
        failure_count,
        ?mode,
        "external file drop filesystem work completed"
    );

    if target
        .update(cx, move |pane, cx| {
            finish_target_drop(pane, &destinations, failure_status, undo_entry, cx);
        })
        .is_err()
    {
        tracing::info!(
            success_count,
            failure_count,
            "external file drop target disappeared before completion"
        );
    }
}

fn finish_target_drop(
    pane: &mut ExplorerPane,
    destinations: &[PathBuf],
    failure_status: Option<String>,
    undo_entry: Option<UndoEntry>,
    cx: &mut Context<ExplorerPane>,
) {
    if let Some(entry) = undo_entry {
        // T054: window-level undo for the drop gesture (the page subscribed to
        // this pane's `PaneEvent`s pushes it onto the stack).
        cx.emit(PaneEvent::Undoable(entry));
    }
    if !destinations.is_empty() {
        pane.reload();
        select_visible_destinations(pane, destinations);
    }
    pane.drop_pending = false;
    if let Some(status) = failure_status {
        pane.set_status(StatusLevel::Error, status);
    }
    cx.notify();
}

fn select_visible_destinations(pane: &mut ExplorerPane, destinations: &[PathBuf]) {
    pane.clear_selection();
    for (index, entry) in pane.filtered_entries.iter().enumerate() {
        if destinations
            .iter()
            .any(|destination| destination == Path::new(&entry.path))
        {
            pane.selection.insert(index);
            pane.selection_anchor.get_or_insert(index);
            pane.active_index = Some(index);
        }
    }
}

/// Compact cursor-following view for a local file payload.
pub(crate) struct FileDragPreview {
    name: SharedString,
    kind: SharedString,
    item_count: usize,
}

impl Render for FileDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name = truncate_middle(self.name.as_ref(), 28);
        let icon_path = icon_path_for(self.name.as_ref(), self.kind.as_ref());

        div()
            .debug_selector(|| "file-drag-preview".to_string())
            .flex()
            .items_center()
            .gap(px(8.0))
            .h(px(36.0))
            .max_w(px(260.0))
            .max_h(px(44.0))
            .px(px(10.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(theme::border(cx))
            .bg(theme::toolbar_active_bg(cx))
            .text_color(theme::toolbar_active_text(cx))
            .text_sm()
            .child(
                Icon::new(Icon::empty())
                    .path(icon_path)
                    .size_4()
                    .text_color(theme::fg_secondary(cx)),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(name),
            )
            .when(self.item_count > 1, |preview| {
                preview.child(
                    div()
                        .debug_selector(|| "file-drag-preview-count".to_string())
                        .flex_shrink_0()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(theme::accent(cx))
                        .text_color(theme::bg(cx))
                        .text_xs()
                        .child(format!("{} items", self.item_count)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explorer::ExplorerPage;
    use crate::explorer::types::ViewMode;
    use chronos_fm_core::config::SplitDirection;
    use chronos_fm_services::fs::provider::LocalFileSystemProvider;
    use gpui::{
        AppContext, Bounds, CursorStyle, InputEvent, Modifiers, MouseButton, MouseUpEvent, Pixels,
        Point, TestAppContext, VisualTestContext, WeakEntity, WindowHandle, point, px, size,
    };
    use gpui_component::Root;
    use gpui_component::resizable::ResizableState;
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::Duration;

    fn target(path: &Path) -> DropTarget {
        DropTarget {
            directory: path.to_path_buf(),
            kind: DropTargetKind::ListingCwd,
        }
    }

    #[test]
    fn drop_mode_uses_drop_time_copy_modifiers() {
        assert_eq!(drop_mode(Modifiers::default()), DropMode::Move);
        assert_eq!(
            drop_mode(Modifiers {
                control: true,
                ..Modifiers::default()
            }),
            DropMode::Copy
        );
        assert_eq!(
            drop_mode(Modifiers {
                platform: true,
                ..Modifiers::default()
            }),
            DropMode::Copy
        );
    }

    #[test]
    fn validation_rejects_empty_payload_and_non_directory_target() {
        let fixture = tempfile::tempdir().unwrap();
        let destination_file = fixture.path().join("not-a-directory");
        std::fs::write(&destination_file, "x").unwrap();

        assert!(validate_drop(&[], &target(fixture.path()), DropMode::Move).is_err());
        assert!(
            validate_drop(
                &[fixture.path().join("source")],
                &target(&destination_file),
                DropMode::Move,
            )
            .is_err()
        );
    }

    #[test]
    fn validation_rejects_self_descendant_and_same_parent_move() {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let descendant = source_dir.join("nested");
        std::fs::create_dir_all(&descendant).unwrap();
        let source_file = fixture.path().join("file.txt");
        std::fs::write(&source_file, "x").unwrap();

        assert!(
            validate_drop(
                std::slice::from_ref(&source_dir),
                &target(&source_dir),
                DropMode::Move,
            )
            .is_err()
        );
        assert!(
            validate_drop(
                std::slice::from_ref(&source_dir),
                &target(&descendant),
                DropMode::Copy,
            )
            .is_err()
        );
        assert!(
            validate_drop(
                std::slice::from_ref(&source_file),
                &target(fixture.path()),
                DropMode::Move,
            )
            .is_err()
        );
    }

    #[test]
    fn validation_accepts_same_parent_copy_and_sibling_move() {
        let fixture = tempfile::tempdir().unwrap();
        let source_parent = fixture.path().join("from");
        let destination = fixture.path().join("to");
        std::fs::create_dir_all(&source_parent).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source = source_parent.join("file.txt");
        std::fs::write(&source, "x").unwrap();

        assert!(
            validate_drop(
                std::slice::from_ref(&source),
                &target(&source_parent),
                DropMode::Copy,
            )
            .is_ok()
        );
        assert!(
            validate_drop(
                std::slice::from_ref(&source),
                &target(&destination),
                DropMode::Move,
            )
            .is_ok()
        );
    }

    #[test]
    fn validation_rejects_entire_mixed_payload() {
        let fixture = tempfile::tempdir().unwrap();
        let source_parent = fixture.path().join("from");
        let destination = fixture.path().join("to");
        std::fs::create_dir_all(&source_parent).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let valid = source_parent.join("valid.txt");
        std::fs::write(&valid, "x").unwrap();

        assert!(
            validate_drop(
                &[valid, destination.clone()],
                &target(&destination),
                DropMode::Move,
            )
            .is_err()
        );
    }

    #[test]
    fn validation_uses_components_instead_of_string_prefixes() {
        let fixture = tempfile::tempdir().unwrap();
        let source = fixture.path().join("a");
        let prefix_sibling = fixture.path().join("ab");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&prefix_sibling).unwrap();

        assert!(
            validate_drop(
                std::slice::from_ref(&source),
                &target(&prefix_sibling),
                DropMode::Move,
            )
            .is_ok()
        );
    }

    #[cfg(unix)]
    #[test]
    fn validation_rejects_same_parent_move_for_symlink_entry() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::tempdir().unwrap();
        let source_parent = fixture.path().join("source-parent");
        let resolved_parent = fixture.path().join("resolved-parent");
        std::fs::create_dir(&source_parent).unwrap();
        std::fs::create_dir(&resolved_parent).unwrap();
        let resolved_source = resolved_parent.join("real.txt");
        let symlink_source = source_parent.join("link.txt");
        std::fs::write(&resolved_source, "x").unwrap();
        symlink(&resolved_source, &symlink_source).unwrap();

        assert!(
            validate_drop(
                std::slice::from_ref(&symlink_source),
                &target(&source_parent),
                DropMode::Move,
            )
            .is_err(),
            "the symlink entry is already in the destination even though its target is elsewhere"
        );
    }

    fn rooted_pane(
        cx: &mut TestAppContext,
        cwd: &Path,
    ) -> (WindowHandle<Root>, Entity<ExplorerPane>) {
        cx.update(gpui_component::init);
        cx.update(crate::explorer::clipboard::init);
        let cwd = cwd.to_string_lossy().to_string();
        let root = cx.add_window(move |window, cx| {
            let pane = cx.new(|cx| {
                let mut pane = ExplorerPane::build(None, window, cx);
                pane.cwd = cwd;
                pane.sidebar_visible = false;
                pane.reload();
                pane
            });
            Root::new(pane, window, cx)
        });
        let pane = root
            .read_with(cx, |root, _cx| {
                root.view()
                    .clone()
                    .downcast::<ExplorerPane>()
                    .expect("the Root's view is the explorer pane")
            })
            .expect("window is alive");
        (root, pane)
    }

    fn routed_pane(
        cx: &mut TestAppContext,
        cwd: &Path,
        view_mode: ViewMode,
    ) -> (WindowHandle<Root>, Entity<ExplorerPane>) {
        let (root, pane) = rooted_pane(cx, cwd);
        pane.update(cx, |pane, cx| {
            pane.view_mode = view_mode;
            pane.update_item_sizes();
            cx.notify();
        });
        (root, pane)
    }

    fn draw_window(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    fn center(bounds: Bounds<Pixels>) -> Point<Pixels> {
        point(
            bounds.left() + bounds.size.width / 2.,
            bounds.top() + bounds.size.height / 2.,
        )
    }

    fn item_index(pane: &ExplorerPane, path: &Path) -> usize {
        let path = path.to_string_lossy();
        pane.filtered_entries
            .iter()
            .position(|entry| entry.path == path)
            .expect("fixture path is visible")
    }

    fn item_point(
        pane: &Entity<ExplorerPane>,
        path: &Path,
        cx: &VisualTestContext,
    ) -> Point<Pixels> {
        pane.read_with(cx, |pane, _cx| {
            let index = item_index(pane, path);
            center(
                *pane
                    .measured_items
                    .get(&index)
                    .expect("fixture item was measured"),
            )
        })
    }

    fn item_name_point(
        pane: &Entity<ExplorerPane>,
        path: &Path,
        cx: &VisualTestContext,
    ) -> Point<Pixels> {
        pane.read_with(cx, |pane, _cx| {
            let index = item_index(pane, path);
            let bounds = *pane
                .measured_items
                .get(&index)
                .expect("fixture item was measured");
            point(bounds.left() + px(96.0), center(bounds).y)
        })
    }

    fn blank_listing_point(pane: &Entity<ExplorerPane>, cx: &VisualTestContext) -> Point<Pixels> {
        pane.read_with(cx, |pane, _cx| {
            let viewport = pane
                .listing_viewport
                .expect("listing viewport was measured");
            let left = viewport.left().as_f32() + 8.0;
            let right = viewport.right().as_f32() - 8.0;
            let top = viewport.top().as_f32() + 8.0;
            let bottom = viewport.bottom().as_f32() - 8.0;

            for y_step in (0..=10).rev() {
                for x_step in (0..=10).rev() {
                    let candidate = point(
                        px(left + (right - left) * x_step as f32 / 10.0),
                        px(top + (bottom - top) * y_step as f32 / 10.0),
                    );
                    if pane
                        .measured_items
                        .values()
                        .chain(pane.marquee_exclusions.values())
                        .all(|bounds| !bounds.contains(&candidate))
                    {
                        return candidate;
                    }
                }
            }
            panic!("listing viewport must expose empty space")
        })
    }

    fn select_paths(pane: &Entity<ExplorerPane>, paths: &[&Path], cx: &mut TestAppContext) {
        pane.update(cx, |pane, cx| {
            pane.clear_selection();
            for path in paths {
                let index = item_index(pane, path);
                pane.selection.insert(index);
                pane.selection_anchor.get_or_insert(index);
                pane.active_index = Some(index);
            }
            cx.notify();
        });
    }

    fn start_drag(
        cx: &mut VisualTestContext,
        source: Point<Pixels>,
        target: Point<Pixels>,
        target_modifiers: Modifiers,
    ) {
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(
            point(source.x + px(8.0), source.y),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_move(target, Some(MouseButton::Left), target_modifiers);
    }

    fn active_drag(cx: &mut VisualTestContext) -> bool {
        cx.update(|_window, cx| cx.has_active_drag())
    }

    fn active_drag_cursor(cx: &mut VisualTestContext) -> Option<CursorStyle> {
        cx.update(|_window, cx| cx.active_drag_cursor_style())
    }

    fn dispatch_mouse_up(
        cx: &mut VisualTestContext,
        position: Point<Pixels>,
        modifiers: Modifiers,
    ) {
        cx.update(|window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    position,
                    modifiers,
                    button: MouseButton::Left,
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
        });
    }

    fn select_path(pane: &Entity<ExplorerPane>, path: &Path, cx: &mut TestAppContext) {
        let path = path.to_string_lossy();
        pane.update(cx, |pane, _cx| {
            let index = pane
                .filtered_entries
                .iter()
                .position(|entry| entry.path == path)
                .expect("fixture path is visible");
            pane.select_single(index);
        });
    }

    fn file_drag(source: &Entity<ExplorerPane>, paths: Vec<PathBuf>) -> FileDrag {
        let initiating_path = paths.first().expect("non-empty fixture payload").clone();
        let initiating_name = initiating_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 fixture name")
            .to_string();
        let kind = if initiating_path.is_dir() {
            "dir"
        } else {
            "file"
        };
        FileDrag::new(
            paths,
            initiating_name,
            initiating_path,
            kind,
            source.clone(),
        )
    }

    fn unrooted_pane(
        cx: &mut TestAppContext,
        cwd: &Path,
    ) -> (WindowHandle<Root>, Entity<ExplorerPane>) {
        cx.update(gpui_component::init);
        cx.update(crate::explorer::clipboard::init);
        let cwd = cwd.to_string_lossy().to_string();
        let released = Rc::new(RefCell::new(None));
        let released_for_window = released.clone();
        let root = cx.add_window(move |window, cx| {
            let orphan = cx.new(|cx| {
                let mut pane = ExplorerPane::build(None, window, cx);
                pane.cwd = cwd.clone();
                pane.sidebar_visible = false;
                pane.reload();
                pane
            });
            *released_for_window.borrow_mut() = Some(orphan);
            let keeper = cx.new(|cx| ExplorerPane::build(None, window, cx));
            Root::new(keeper, window, cx)
        });
        let pane = released
            .borrow_mut()
            .take()
            .expect("orphan pane was captured");
        (root, pane)
    }

    fn begin_drop(
        target_pane: &Entity<ExplorerPane>,
        drag: FileDrag,
        directory: &Path,
        kind: DropTargetKind,
        modifiers: Modifiers,
        cx: &mut TestAppContext,
    ) -> bool {
        let target = DropTarget {
            directory: directory.to_path_buf(),
            kind,
        };
        target_pane.update(cx, |pane, cx| {
            pane.begin_file_drop(drag, target, modifiers, cx)
        })
    }

    fn begin_external_drop(
        target_pane: &Entity<ExplorerPane>,
        paths: Vec<PathBuf>,
        directory: &Path,
        kind: DropTargetKind,
        modifiers: Modifiers,
        cx: &mut TestAppContext,
    ) -> bool {
        let target = DropTarget {
            directory: directory.to_path_buf(),
            kind,
        };
        target_pane.update(cx, |pane, cx| {
            pane.begin_external_drop(paths, target, modifiers, cx)
        })
    }

    async fn settle_drop(cx: &mut TestAppContext) {
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();
    }

    #[gpui::test]
    async fn external_can_accept_requires_local_payload_and_directory(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let destination_file = fixture.path().join("file.txt");
        std::fs::write(&destination_file, "x").unwrap();
        let source_file = fixture.path().join("source.txt");
        std::fs::write(&source_file, "x").unwrap();
        let (_, pane) = unrooted_pane(cx, fixture.path());

        let rejected = pane.update(cx, |pane, _cx| {
            pane.can_accept_external_drop(&[], &target(fixture.path()), Modifiers::default())
        });
        assert!(!rejected, "an empty external payload is never accepted");

        let rejected = pane.update(cx, |pane, _cx| {
            pane.can_accept_external_drop(
                std::slice::from_ref(&source_file),
                &target(&destination_file),
                Modifiers::default(),
            )
        });
        assert!(
            !rejected,
            "a non-directory destination rejects external payloads",
        );

        let rejected = pane.update(cx, |pane, _cx| {
            pane.can_accept_external_drop(
                std::slice::from_ref(&source_file),
                &target(fixture.path()),
                Modifiers::default(),
            )
        });
        assert!(!rejected, "a same-parent move rejects external payloads");

        let accepted = pane.update(cx, |pane, _cx| {
            pane.can_accept_external_drop(
                std::slice::from_ref(&source_file),
                &target(fixture.path()),
                Modifiers {
                    control: true,
                    ..Modifiers::default()
                },
            )
        });
        assert!(
            accepted,
            "a ctrl-copy of a local file into its parent is allowed",
        );
    }

    #[gpui::test]
    async fn external_drop_moves_files_into_pane_cwd(cx: &mut TestAppContext) {
        let destination = tempfile::tempdir().unwrap();
        let source_dir = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("external.txt");
        std::fs::write(&source, "external").unwrap();
        let (_, pane) = unrooted_pane(cx, destination.path());

        let accepted = begin_external_drop(
            &pane,
            vec![source.clone()],
            destination.path(),
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        );
        assert!(accepted, "a default external drop moves into the cwd");
        settle_drop(cx).await;

        assert!(
            !source.exists(),
            "move removes the external source after a successful transfer",
        );
        assert!(
            destination.path().join("external.txt").exists(),
            "the external payload lands in the pane cwd",
        );
    }

    #[gpui::test]
    async fn external_drop_ctrl_copies_preserving_source(cx: &mut TestAppContext) {
        let destination = tempfile::tempdir().unwrap();
        let source_dir = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("external.txt");
        std::fs::write(&source, "external").unwrap();
        let (_, pane) = unrooted_pane(cx, destination.path());

        let modifiers = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        let accepted = begin_external_drop(
            &pane,
            vec![source.clone()],
            destination.path(),
            DropTargetKind::ListingCwd,
            modifiers,
            cx,
        );
        assert!(accepted, "a ctrl external drop copies into the cwd");
        settle_drop(cx).await;

        assert!(source.exists(), "copy preserves the external source");
        assert!(
            destination.path().join("external.txt").exists(),
            "the copied payload lands in the pane cwd",
        );
    }

    #[gpui::test]
    async fn dnd_routing_selected_list_row_carries_selection_to_folder(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let first = fixture.path().join("a.txt");
        let second = fixture.path().join("b.txt");
        let folder = fixture.path().join("folder");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();
        std::fs::create_dir(&folder).unwrap();
        let (root, pane) = routed_pane(cx, fixture.path(), ViewMode::List);
        select_paths(&pane, &[&first, &second], cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_point(&pane, &first, &cx);
        let target = item_point(&pane, &folder, &cx);
        start_drag(&mut cx, source, target, Modifiers::default());
        assert!(active_drag(&mut cx));
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::ClosedHand),
            "a valid folder hover uses the move cursor",
        );
        assert!(
            cx.debug_bounds("file-drag-preview-count").is_some(),
            "the selected two-item payload renders its count badge",
        );

        cx.simulate_mouse_move(source, Some(MouseButton::Left), Modifiers::default());
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::OperationNotAllowed),
            "leaving a valid folder for a measured file resets the cursor",
        );
        cx.simulate_mouse_move(target, Some(MouseButton::Left), Modifiers::default());
        assert_eq!(active_drag_cursor(&mut cx), Some(CursorStyle::ClosedHand));

        dispatch_mouse_up(&mut cx, target, Modifiers::default());
        assert!(!active_drag(&mut cx));
        assert!(pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        settle_drop(&mut cx).await;
        assert!(folder.join("a.txt").exists());
        assert!(folder.join("b.txt").exists());
    }

    #[gpui::test]
    fn dnd_routing_unselected_list_row_normalizes_and_cancels_marquee(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let first = fixture.path().join("a.txt");
        let second = fixture.path().join("b.txt");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();
        let (root, pane) = routed_pane(cx, fixture.path(), ViewMode::List);
        select_paths(&pane, &[&second], cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.0), px(560.0)));
        draw_window(&mut cx);

        let blank = blank_listing_point(&pane, &cx);
        pane.update(&mut cx, |pane, _cx| {
            assert!(pane.begin_marquee(blank, Modifiers::default()));
        });
        let source = item_point(&pane, &first, &cx);
        start_drag(&mut cx, source, source, Modifiers::default());

        assert!(active_drag(&mut cx));
        pane.read_with(&cx, |pane, _cx| {
            assert_eq!(pane.selected_paths(), vec![first.to_string_lossy()]);
            assert!(pane.marquee.is_none());
        });
        draw_window(&mut cx);
        assert!(cx.debug_bounds("file-drag-preview").is_some());
        assert!(cx.debug_bounds("file-drag-preview-count").is_none());
        cx.simulate_mouse_up(source, MouseButton::Left, Modifiers::default());
    }

    #[gpui::test]
    async fn dnd_routing_grid_tile_uses_selection_preview_and_folder_target(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let first = fixture.path().join("a.txt");
        let second = fixture.path().join("b.txt");
        let folder = fixture.path().join("folder");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();
        std::fs::create_dir(&folder).unwrap();
        let (root, pane) = routed_pane(cx, fixture.path(), ViewMode::Grid);
        select_paths(&pane, &[&first, &second], cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_point(&pane, &first, &cx);
        let target = item_point(&pane, &folder, &cx);
        start_drag(&mut cx, source, target, Modifiers::default());
        assert!(active_drag(&mut cx));
        draw_window(&mut cx);
        assert_eq!(active_drag_cursor(&mut cx), Some(CursorStyle::ClosedHand));
        assert!(cx.debug_bounds("file-drag-preview-count").is_some());

        dispatch_mouse_up(&mut cx, target, Modifiers::default());
        assert!(pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        settle_drop(&mut cx).await;
        assert!(folder.join("a.txt").exists());
        assert!(folder.join("b.txt").exists());
    }

    #[gpui::test]
    async fn dnd_routing_empty_listing_space_accepts_copy_to_cwd(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_path = fixture.path().join("copy.txt");
        let copied_path = fixture.path().join("copy (2).txt");
        std::fs::write(&source_path, "copy").unwrap();
        let (root, pane) = routed_pane(cx, fixture.path(), ViewMode::List);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_point(&pane, &source_path, &cx);
        let target = blank_listing_point(&pane, &cx);
        let copy = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        start_drag(&mut cx, source, target, copy);
        draw_window(&mut cx);
        assert_eq!(active_drag_cursor(&mut cx), Some(CursorStyle::DragCopy));
        dispatch_mouse_up(&mut cx, target, copy);
        assert!(pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        // The same-parent copy collides with the source itself, so the drop
        // pauses behind the conflict dialog (T053); the default Rename
        // decision produces the same unique-name result the old silent
        // auto-rename did.
        assert!(pane.read_with(&cx, |pane, _cx| pane.conflict_dialog.is_some()));
        pane.update(&mut cx, |pane, cx| {
            pane.conflict_decide(crate::explorer::conflict::ConflictChoice::Rename, cx);
        });
        settle_drop(&mut cx).await;
        assert!(source_path.exists());
        assert!(copied_path.exists());
    }

    #[gpui::test]
    async fn dnd_routing_current_breadcrumb_accepts_copy_to_cwd(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_path = fixture.path().join("copy.txt");
        let copied_path = fixture.path().join("copy (2).txt");
        std::fs::write(&source_path, "copy").unwrap();
        let (root, pane) = routed_pane(cx, fixture.path(), ViewMode::Grid);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_point(&pane, &source_path, &cx);
        let target = center(
            cx.debug_bounds("current-directory-breadcrumb")
                .expect("the cwd breadcrumb has a stable target selector"),
        );
        let copy = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        start_drag(&mut cx, source, target, copy);
        draw_window(&mut cx);
        assert_eq!(active_drag_cursor(&mut cx), Some(CursorStyle::DragCopy));
        dispatch_mouse_up(&mut cx, target, copy);
        assert!(pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        // The same-parent copy collides with the source itself, so the drop
        // pauses behind the conflict dialog (T053); the default Rename
        // decision produces the same unique-name result the old silent
        // auto-rename did.
        assert!(pane.read_with(&cx, |pane, _cx| pane.conflict_dialog.is_some()));
        pane.update(&mut cx, |pane, cx| {
            pane.conflict_decide(crate::explorer::conflict::ConflictChoice::Rename, cx);
        });
        settle_drop(&mut cx).await;
        assert!(source_path.exists());
        assert!(copied_path.exists());
    }

    #[gpui::test]
    fn dnd_routing_items_rename_header_and_provider_reject(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let first = fixture.path().join("a.txt");
        let second = fixture.path().join("b.txt");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();
        let (root, pane) = routed_pane(cx, fixture.path(), ViewMode::List);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_point(&pane, &first, &cx);
        let covered_file = item_point(&pane, &second, &cx);
        let copy = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        start_drag(&mut cx, source, covered_file, copy);
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::OperationNotAllowed),
            "a measured file item blocks the otherwise-valid outer cwd copy target",
        );
        cx.simulate_mouse_up(covered_file, MouseButton::Left, copy);
        assert!(!pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        assert!(!fixture.path().join("a (2).txt").exists());

        let rename_index = pane.read_with(&cx, |pane, _cx| item_index(pane, &first));
        cx.update(|window, app| {
            pane.update(app, |pane, cx| pane.begin_rename(rename_index, window, cx));
        });
        draw_window(&mut cx);
        let rename_input = item_name_point(&pane, &first, &cx);
        start_drag(&mut cx, rename_input, rename_input, Modifiers::default());
        assert!(
            !active_drag(&mut cx),
            "inline rename cannot originate a file drag"
        );
        cx.simulate_mouse_up(rename_input, MouseButton::Left, Modifiers::default());
        cx.update(|window, app| {
            pane.update(app, |pane, cx| pane.cancel_rename(window, cx))
        });
        draw_window(&mut cx);

        let resize_control = center(
            cx.debug_bounds("list-column-resize-0")
                .expect("the list header control is rendered"),
        );
        start_drag(
            &mut cx,
            resize_control,
            resize_control,
            Modifiers::default(),
        );
        assert!(
            !active_drag(&mut cx),
            "header controls cannot originate file drags"
        );
        cx.simulate_mouse_up(resize_control, MouseButton::Left, Modifiers::default());

        let provider_blank = blank_listing_point(&pane, &cx);
        start_drag(&mut cx, source, provider_blank, copy);
        assert!(active_drag(&mut cx));
        pane.update(&mut cx, |pane, cx| {
            pane.provider = Some(Arc::new(LocalFileSystemProvider));
            cx.notify();
        });
        draw_window(&mut cx);
        cx.simulate_mouse_move(provider_blank, Some(MouseButton::Left), copy);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::OperationNotAllowed),
            "provider panes cannot accept a local payload",
        );
        dispatch_mouse_up(&mut cx, provider_blank, copy);
        assert!(!pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        assert!(!fixture.path().join("a (2).txt").exists());

        let provider_item = item_point(&pane, &first, &cx);
        start_drag(&mut cx, provider_item, provider_blank, Modifiers::default());
        assert!(
            !active_drag(&mut cx),
            "provider entries cannot originate local drags"
        );
        cx.simulate_mouse_up(provider_blank, MouseButton::Left, Modifiers::default());
        assert!(!pane.read_with(&cx, |pane, _cx| pane.drop_pending));
    }

    #[gpui::test]
    async fn pending_is_synchronous_and_rejects_a_second_drop(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source_path = source_dir.join("file.txt");
        std::fs::write(&source_path, "x").unwrap();
        let (_source_root, source) = rooted_pane(cx, &source_dir);
        let (_target_root, target_pane) = rooted_pane(cx, &destination);
        let drag = file_drag(&source, vec![source_path]);
        assert_eq!(drag.initiating_path(), source_dir.join("file.txt"));
        let _preview = drag.preview();

        assert!(begin_drop(
            &target_pane,
            drag.clone(),
            &destination,
            DropTargetKind::BreadcrumbCwd,
            Modifiers::default(),
            cx,
        ));
        assert!(target_pane.read_with(cx, |pane, _cx| pane.drop_pending));
        assert!(!begin_drop(
            &target_pane,
            drag,
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        ));

        settle_drop(cx).await;
        assert!(!target_pane.read_with(cx, |pane, _cx| pane.drop_pending));
    }

    #[gpui::test]
    async fn cross_pane_move_reloads_both_and_selects_destination(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source_path = source_dir.join("move.txt");
        let destination_path = destination.join("move.txt");
        std::fs::write(&source_path, "move").unwrap();
        let (_source_root, source) = rooted_pane(cx, &source_dir);
        let (_target_root, target_pane) = rooted_pane(cx, &destination);
        select_path(&source, &source_path, cx);

        assert!(begin_drop(
            &target_pane,
            file_drag(&source, vec![source_path.clone()]),
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        ));
        settle_drop(cx).await;

        assert!(!source_path.exists());
        assert!(destination_path.exists());
        assert!(source.read_with(cx, |pane, _cx| pane.selection.is_empty()));
        assert!(source.read_with(cx, |pane, _cx| {
            pane.filtered_entries
                .iter()
                .all(|entry| entry.path != source_path.to_string_lossy())
        }));
        assert_eq!(
            target_pane.read_with(cx, |pane, _cx| pane.selected_paths()),
            vec![destination_path.to_string_lossy().to_string()]
        );
    }

    #[gpui::test]
    async fn cross_pane_copy_retains_source_selection(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source_path = source_dir.join("copy.txt");
        let destination_path = destination.join("copy.txt");
        std::fs::write(&source_path, "copy").unwrap();
        let (_source_root, source) = rooted_pane(cx, &source_dir);
        let (_target_root, target_pane) = rooted_pane(cx, &destination);
        select_path(&source, &source_path, cx);

        assert!(begin_drop(
            &target_pane,
            file_drag(&source, vec![source_path.clone()]),
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
            cx,
        ));
        settle_drop(cx).await;

        assert!(source_path.exists());
        assert!(destination_path.exists());
        assert_eq!(
            source.read_with(cx, |pane, _cx| pane.selected_paths()),
            vec![source_path.to_string_lossy().to_string()]
        );
        assert_eq!(
            target_pane.read_with(cx, |pane, _cx| pane.selected_paths()),
            vec![destination_path.to_string_lossy().to_string()]
        );
    }

    #[gpui::test]
    async fn same_pane_folder_move_reloads_once_and_clears_selection(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let destination = fixture.path().join("folder");
        std::fs::create_dir(&destination).unwrap();
        let source_path = fixture.path().join("move.txt");
        std::fs::write(&source_path, "move").unwrap();
        let (_root, pane) = rooted_pane(cx, fixture.path());
        select_path(&pane, &source_path, cx);
        let reloads_before = pane.read_with(cx, |pane, _cx| pane.reload_count);

        assert!(begin_drop(
            &pane,
            file_drag(&pane, vec![source_path.clone()]),
            &destination,
            DropTargetKind::FolderItem,
            Modifiers::default(),
            cx,
        ));
        settle_drop(cx).await;

        assert!(destination.join("move.txt").exists());
        assert!(pane.read_with(cx, |pane, _cx| pane.selection.is_empty()));
        assert_eq!(
            pane.read_with(cx, |pane, _cx| pane.reload_count),
            reloads_before + 1
        );
    }

    #[gpui::test]
    async fn same_pane_cwd_copy_selects_unique_name(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_path = fixture.path().join("copy.txt");
        let copy_path = fixture.path().join("copy (2).txt");
        std::fs::write(&source_path, "copy").unwrap();
        let (_root, pane) = rooted_pane(cx, fixture.path());
        select_path(&pane, &source_path, cx);

        assert!(begin_drop(
            &pane,
            file_drag(&pane, vec![source_path]),
            fixture.path(),
            DropTargetKind::ListingCwd,
            Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            cx,
        ));
        // The same-parent copy collides with the source itself, so the drop
        // pauses behind the conflict dialog (T053) instead of silently
        // auto-renaming.
        assert!(pane.read_with(cx, |pane, _cx| pane.conflict_dialog.is_some()));
        pane.update(cx, |pane, cx| {
            pane.conflict_decide(crate::explorer::conflict::ConflictChoice::Rename, cx);
        });
        settle_drop(cx).await;

        assert!(copy_path.exists());
        assert_eq!(
            pane.read_with(cx, |pane, _cx| pane.selected_paths()),
            vec![copy_path.to_string_lossy().to_string()]
        );
    }

    #[gpui::test]
    async fn total_failure_sets_target_error_without_success_text(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source_path = source_dir.join("vanishes.txt");
        std::fs::write(&source_path, "x").unwrap();
        let (_source_root, source) = rooted_pane(cx, &source_dir);
        let (_target_root, target_pane) = rooted_pane(cx, &destination);

        assert!(begin_drop(
            &target_pane,
            file_drag(&source, vec![source_path.clone()]),
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        ));
        std::fs::remove_file(&source_path).unwrap();
        settle_drop(cx).await;

        let (status, is_error) = target_pane
            .read_with(cx, |pane, _cx| pane.status_for_footer())
            .expect("failure status");
        assert!(is_error);
        assert!(status.contains("1 failed"));
        assert!(status.contains("vanishes.txt"));
        assert!(!status.contains("succeeded"));
    }

    #[gpui::test]
    async fn partial_failure_reloads_success_and_reports_counts(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let successful = source_dir.join("kept.txt");
        let failing = source_dir.join("vanishes.txt");
        std::fs::write(&successful, "ok").unwrap();
        std::fs::write(&failing, "gone").unwrap();
        let (_source_root, source) = rooted_pane(cx, &source_dir);
        let (_target_root, target_pane) = rooted_pane(cx, &destination);

        assert!(begin_drop(
            &target_pane,
            file_drag(&source, vec![successful.clone(), failing.clone()]),
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        ));
        std::fs::remove_file(&failing).unwrap();
        settle_drop(cx).await;

        let destination_path = destination.join("kept.txt");
        assert!(destination_path.exists());
        assert_eq!(
            target_pane.read_with(cx, |pane, _cx| pane.selected_paths()),
            vec![destination_path.to_string_lossy().to_string()]
        );
        let (status, is_error) = target_pane
            .read_with(cx, |pane, _cx| pane.status_for_footer())
            .expect("partial failure status");
        assert!(is_error);
        assert!(status.contains("1 succeeded, 1 failed"));
        assert!(status.contains("vanishes.txt"));
    }

    #[gpui::test]
    async fn released_source_while_pending_still_completes_target(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source_path = source_dir.join("moved.txt");
        let destination_path = destination.join("moved.txt");
        std::fs::write(&source_path, "moved").unwrap();
        let (_source_host, source) = unrooted_pane(cx, &source_dir);
        let (_target_root, target_pane) = rooted_pane(cx, &destination);
        let released_source = source.downgrade();

        assert!(begin_drop(
            &target_pane,
            file_drag(&source, vec![source_path.clone()]),
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        ));
        assert!(target_pane.read_with(cx, |pane, _cx| pane.drop_pending));
        drop(source);
        assert!(released_source.upgrade().is_none());
        settle_drop(cx).await;

        assert!(!source_path.exists());
        assert!(destination_path.exists());
        assert!(!target_pane.read_with(cx, |pane, _cx| pane.drop_pending));
        assert_eq!(
            target_pane.read_with(cx, |pane, _cx| pane.selected_paths()),
            vec![destination_path.to_string_lossy().to_string()]
        );
    }

    #[gpui::test]
    async fn released_target_while_pending_still_completes_source(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source_dir = fixture.path().join("source");
        let destination = fixture.path().join("destination");
        std::fs::create_dir(&source_dir).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let source_path = source_dir.join("moved.txt");
        let destination_path = destination.join("moved.txt");
        std::fs::write(&source_path, "moved").unwrap();
        let (_source_root, source) = rooted_pane(cx, &source_dir);
        let (_target_host, target_pane) = unrooted_pane(cx, &destination);
        select_path(&source, &source_path, cx);
        let released_target = target_pane.downgrade();

        assert!(begin_drop(
            &target_pane,
            file_drag(&source, vec![source_path.clone()]),
            &destination,
            DropTargetKind::ListingCwd,
            Modifiers::default(),
            cx,
        ));
        assert!(target_pane.read_with(cx, |pane, _cx| pane.drop_pending));
        drop(target_pane);
        assert!(released_target.upgrade().is_none());
        settle_drop(cx).await;

        assert!(!source_path.exists());
        assert!(destination_path.exists());
        assert!(source.read_with(cx, |pane, _cx| pane.selection.is_empty()));
        assert!(source.read_with(cx, |pane, _cx| {
            pane.filtered_entries
                .iter()
                .all(|entry| entry.path != source_path.to_string_lossy())
        }));
    }

    // -- Task 4: production split-pane end-to-end and adversarial coverage --

    /// Roots a production `ExplorerPage` split into two panes, pane 0 at
    /// `left_cwd` and pane 1 at `right_cwd`, both reloaded and unselected.
    fn split_page(
        cx: &mut TestAppContext,
        left_cwd: &Path,
        right_cwd: &Path,
    ) -> (WindowHandle<Root>, Entity<ExplorerPage>) {
        cx.update(gpui_component::init);
        cx.update(crate::explorer::clipboard::init);
        let left_cwd = left_cwd.to_string_lossy().to_string();
        let right_cwd = right_cwd.to_string_lossy().to_string();
        let root = cx.add_window(move |window, cx| {
            let resizable = cx.new(|_| ResizableState::default());
            let page = cx.new(|cx| {
                let mut page = ExplorerPage::new(resizable, None, None, false, window, cx);
                page.split(SplitDirection::Vertical, window, cx);
                let pane0 = page.pane(0);
                let pane1 = page.pane(1);
                pane0.update(cx, |pane, _cx| {
                    pane.cwd = left_cwd.clone();
                    pane.sidebar_visible = false;
                    pane.reload();
                });
                pane1.update(cx, |pane, _cx| {
                    pane.cwd = right_cwd.clone();
                    pane.sidebar_visible = false;
                    pane.reload();
                });
                page.set_active(0, window, cx);
                page
            });
            Root::new(page, window, cx)
        });
        let page = root
            .read_with(cx, |root, _cx| {
                root.view()
                    .clone()
                    .downcast::<ExplorerPage>()
                    .expect("the Root's view is the explorer page")
            })
            .expect("window is alive");
        (root, page)
    }

    #[gpui::test]
    async fn e2e_cross_pane_move_reloads_both_panes_and_selects_destination(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let a = left.join("a.txt");
        let b = left.join("b.txt");
        let c = left.join("c.txt");
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();
        std::fs::write(&c, "c").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        let right_pane = page.read_with(cx, |page, _cx| page.pane(1));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        select_paths(&left_pane, &[&a, &b, &c], cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &a, &cx);
        let target = blank_listing_point(&right_pane, &cx);
        start_drag(&mut cx, source, target, Modifiers::default());
        assert!(active_drag(&mut cx));
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::ClosedHand),
            "the other pane's empty cwd is a valid move target",
        );
        assert!(cx.debug_bounds("file-drag-preview-count").is_some());

        dispatch_mouse_up(&mut cx, target, Modifiers::default());
        assert!(!active_drag(&mut cx));
        assert!(right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        for path in [&a, &b, &c] {
            assert!(!path.exists(), "{path:?} must have moved out of left");
            assert!(
                right.join(path.file_name().unwrap()).exists(),
                "{path:?} must exist in right"
            );
        }
        assert!(right_pane.read_with(&cx, |pane, _cx| !pane.drop_pending));
        assert!(left_pane.read_with(&cx, |pane, _cx| pane.selection.is_empty()));
        let mut expected: Vec<String> = [&a, &b, &c]
            .iter()
            .map(|p| {
                right
                    .join(p.file_name().unwrap())
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        expected.sort();
        let mut selected = right_pane.read_with(&cx, |pane, _cx| pane.selected_paths());
        selected.sort();
        assert_eq!(selected, expected);
    }

    #[gpui::test]
    async fn e2e_ctrl_evaluated_at_drop_time_copies_across_panes(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        let dest_folder = right.join("dest_folder");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        std::fs::create_dir(&dest_folder).unwrap();
        let source_path = left.join("keep.txt");
        std::fs::write(&source_path, "keep").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        let right_pane = page.read_with(cx, |page, _cx| page.pane(1));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        select_path(&left_pane, &source_path, cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &source_path, &cx);
        let target = item_name_point(&right_pane, &dest_folder, &cx);
        // Starts plain (no modifiers), only presses Ctrl once hovering the
        // other-pane folder target — Copy must still be chosen at drop time.
        start_drag(&mut cx, source, target, Modifiers::default());
        assert!(active_drag(&mut cx));
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::ClosedHand),
            "no modifier is held yet, so the target reads as a Move",
        );
        let copy = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        cx.simulate_mouse_move(target, Some(MouseButton::Left), copy);
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::DragCopy),
            "pressing Ctrl mid-drag over the target switches to the Copy cursor",
        );

        dispatch_mouse_up(&mut cx, target, copy);
        assert!(!active_drag(&mut cx));
        assert!(right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        let copied = dest_folder.join("keep.txt");
        assert!(source_path.exists(), "Ctrl-copy must retain the source");
        assert!(copied.exists(), "Ctrl-copy must produce the destination");
        assert_eq!(
            left_pane.read_with(&cx, |pane, _cx| pane.selected_paths()),
            vec![source_path.to_string_lossy().to_string()],
            "cross-pane Copy retains source selection",
        );
    }

    #[gpui::test]
    fn e2e_folder_dragged_onto_self_does_not_fall_through_to_cwd(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let container = left.join("container");
        let inner = container.join("inner");
        std::fs::create_dir_all(&inner).unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &container, &cx);
        // `container` is the only top-level entry, so `inner` is not directly
        // visible/measured; drop the container back onto itself instead, which
        // is the same self/descendant rejection this pane's own item-covered
        // cwd surface must not fall through on.
        start_drag(&mut cx, source, source, Modifiers::default());
        assert!(active_drag(&mut cx));
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::OperationNotAllowed),
            "a directory dropped on itself is invalid and must not fall through",
        );
        cx.simulate_mouse_up(source, MouseButton::Left, Modifiers::default());
        assert!(!active_drag(&mut cx));
        assert!(!left_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        assert!(
            !left.join("container (2)").exists(),
            "the rejected item drop must not have fallen through to the outer cwd target",
        );
    }

    #[gpui::test]
    fn e2e_same_parent_move_via_cwd_surface_performs_no_filesystem_operation(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let source_path = left.join("stay.txt");
        std::fs::write(&source_path, "stay").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        select_path(&left_pane, &source_path, cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &source_path, &cx);
        // The pane's own cwd (its blank listing space) is the source's current
        // parent directory: a same-parent Move must be rejected as a no-op,
        // not silently retargeted.
        let own_cwd = blank_listing_point(&left_pane, &cx);
        start_drag(&mut cx, source, own_cwd, Modifiers::default());
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::OperationNotAllowed),
            "a same-parent Move onto its own cwd is a no-op, not a target",
        );
        cx.simulate_mouse_up(own_cwd, MouseButton::Left, Modifiers::default());
        assert!(!left_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        assert!(source_path.exists());
        assert!(!left.join("stay (2).txt").exists());
    }

    #[gpui::test]
    async fn e2e_same_parent_ctrl_copy_via_cwd_surface_creates_unique_duplicate(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let source_path = left.join("dup.txt");
        std::fs::write(&source_path, "dup").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        select_path(&left_pane, &source_path, cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &source_path, &cx);
        let own_cwd = blank_listing_point(&left_pane, &cx);
        let copy = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        start_drag(&mut cx, source, own_cwd, copy);
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::DragCopy),
            "same-parent Copy is valid and produces a unique-name duplicate",
        );
        dispatch_mouse_up(&mut cx, own_cwd, copy);
        assert!(left_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        // The same-parent copy collides with the source itself, so the drop
        // pauses behind the conflict dialog (T053); Rename keeps the old
        // unique-name outcome.
        assert!(left_pane.read_with(&cx, |pane, _cx| pane.conflict_dialog.is_some()));
        left_pane.update(&mut cx, |pane, cx| {
            pane.conflict_decide(crate::explorer::conflict::ConflictChoice::Rename, cx);
        });
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        let duplicate = left.join("dup (2).txt");
        assert!(source_path.exists());
        assert!(duplicate.exists());
        assert_eq!(
            left_pane.read_with(&cx, |pane, _cx| pane.selected_paths()),
            vec![duplicate.to_string_lossy().to_string()],
        );
    }

    #[gpui::test]
    async fn e2e_cross_pane_partial_failure_reports_visible_error(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let kept = left.join("kept.txt");
        let vanishes = left.join("vanishes.txt");
        std::fs::write(&kept, "ok").unwrap();
        std::fs::write(&vanishes, "gone").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        let right_pane = page.read_with(cx, |page, _cx| page.pane(1));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        select_paths(&left_pane, &[&kept, &vanishes], cx);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &kept, &cx);
        let target = blank_listing_point(&right_pane, &cx);
        start_drag(&mut cx, source, target, Modifiers::default());
        dispatch_mouse_up(&mut cx, target, Modifiers::default());
        assert!(right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        // Race the failing member out from under the in-flight transfer, the
        // same adversarial timing the direct-call `partial_failure_*` test
        // exercises, but driven through the real production hitboxes here.
        std::fs::remove_file(&vanishes).unwrap();
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        assert!(!kept.exists());
        assert!(right.join("kept.txt").exists());
        let (status, is_error) = right_pane
            .read_with(&cx, |pane, _cx| pane.status_for_footer())
            .expect("partial failure status");
        assert!(is_error);
        assert!(status.contains("1 succeeded, 1 failed"));
        assert!(status.contains("vanishes.txt"));
    }

    #[gpui::test]
    async fn e2e_closing_source_pane_before_completion_still_reloads_destination(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let source_path = left.join("moved.txt");
        std::fs::write(&source_path, "moved").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        let right_pane = page.read_with(cx, |page, _cx| page.pane(1));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        select_path(&left_pane, &source_path, cx);
        let source_weak: WeakEntity<ExplorerPane> = left_pane.downgrade();
        drop(left_pane);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let left_pane = page.read_with(&cx, |page, _cx| page.pane(0));
        let source = item_name_point(&left_pane, &source_path, &cx);
        let target = blank_listing_point(&right_pane, &cx);
        start_drag(&mut cx, source, target, Modifiers::default());
        dispatch_mouse_up(&mut cx, target, Modifiers::default());
        assert!(right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));

        // Drop this test's own strong handle, then close the pane through the
        // production action so the source pane's only remaining owner
        // (`PaneGroup`) releases it before the background transfer completes.
        drop(left_pane);
        cx.update(|window, cx| {
            page.update(cx, |page, cx| {
                page.close_pane(0, window, cx);
            });
        });
        assert!(
            source_weak.upgrade().is_none(),
            "the source pane entity must have been released by closing it",
        );

        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        assert!(!source_path.exists());
        assert!(right.join("moved.txt").exists());
        assert!(!right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        assert_eq!(
            right_pane.read_with(&cx, |pane, _cx| pane.selected_paths()),
            vec![right.join("moved.txt").to_string_lossy().to_string()],
        );
    }

    #[gpui::test]
    async fn e2e_pending_target_refuses_a_second_production_drop(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let first = left.join("first.txt");
        let second = left.join("second.txt");
        std::fs::write(&first, "1").unwrap();
        std::fs::write(&second, "2").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        let right_pane = page.read_with(cx, |page, _cx| page.pane(1));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        select_path(&left_pane, &first, &mut cx);
        let source_one = item_name_point(&left_pane, &first, &cx);
        let target = blank_listing_point(&right_pane, &cx);
        start_drag(&mut cx, source_one, target, Modifiers::default());
        dispatch_mouse_up(&mut cx, target, Modifiers::default());
        assert!(!active_drag(&mut cx));
        assert!(right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));

        // A second drop attempts to land on the still-pending destination
        // before the first transfer settles. Exercised through the exact
        // `can_accept_listing_cwd_drop`/`begin_file_drop` methods the
        // production `can_drop`/`on_drop` closures call (`view/listing.rs`),
        // so this is the same shared validation path a second real mouse
        // gesture would hit, without depending on the test harness's support
        // for chaining two full drag gestures through one `dispatch_event`
        // mouse-up.
        let second_target = DropTarget {
            directory: right.clone(),
            kind: DropTargetKind::ListingCwd,
        };
        let second_drag = file_drag(&left_pane, vec![second.clone()]);
        let target_id = right_pane.entity_id();
        let accepted = cx.update(|window, cx| {
            right_pane.read(cx).can_accept_listing_cwd_drop(
                target_id,
                &second_drag,
                &second_target,
                window,
                cx,
            )
        });
        assert!(!accepted, "a pending target must reject a second drop");
        let began = right_pane.update(&mut cx, |pane, cx| {
            pane.begin_file_drop(second_drag, second_target, Modifiers::default(), cx)
        });
        assert!(!began, "begin_file_drop must also refuse while pending");

        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        assert!(right.join("first.txt").exists());
        assert!(
            !right.join("second.txt").exists(),
            "the rejected second drop must not have transferred its payload",
        );
        assert!(second.exists(), "the rejected drop's source is untouched");
        assert!(!right_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
    }

    #[gpui::test]
    fn e2e_mouse_up_on_invalid_target_ends_drag_with_no_stuck_preview_or_cursor(
        cx: &mut TestAppContext,
    ) {
        let fixture = tempfile::tempdir().unwrap();
        let left = fixture.path().join("left");
        let right = fixture.path().join("right");
        std::fs::create_dir(&left).unwrap();
        std::fs::create_dir(&right).unwrap();
        let first = left.join("first.txt");
        let second = left.join("second.txt");
        std::fs::write(&first, "1").unwrap();
        std::fs::write(&second, "2").unwrap();

        let (root, page) = split_page(cx, &left, &right);
        let left_pane = page.read_with(cx, |page, _cx| page.pane(0));
        left_pane.update(cx, |pane, cx| {
            pane.view_mode = ViewMode::List;
            pane.update_item_sizes();
            cx.notify();
        });
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(1200.0), px(560.0)));
        draw_window(&mut cx);

        let source = item_name_point(&left_pane, &first, &cx);
        let invalid_target = item_name_point(&left_pane, &second, &cx);
        start_drag(&mut cx, source, invalid_target, Modifiers::default());
        assert!(active_drag(&mut cx));
        draw_window(&mut cx);
        assert_eq!(
            active_drag_cursor(&mut cx),
            Some(CursorStyle::OperationNotAllowed),
        );
        cx.simulate_mouse_up(invalid_target, MouseButton::Left, Modifiers::default());

        assert!(
            !active_drag(&mut cx),
            "mouse-up always ends the GPUI drag, accepted or not",
        );
        draw_window(&mut cx);
        assert!(
            cx.debug_bounds("file-drag-preview").is_none(),
            "no drag preview may remain painted after mouse-up",
        );
        assert!(!left_pane.read_with(&cx, |pane, _cx| pane.drop_pending));
        assert!(first.exists());
        assert!(second.exists());
        assert!(!left.join("first (2).txt").exists());
    }
}
