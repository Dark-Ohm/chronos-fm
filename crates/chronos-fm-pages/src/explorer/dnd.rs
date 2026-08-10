use super::ExplorerPane;
use super::types::StatusLevel;
use super::view::listing::{row::icon_path_for, truncate_middle};
use chronos_fm_services::fs::ops::{TransferReport, transfer_paths};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::{
    AppContext, Context, Entity, EntityId, IntoElement, Modifiers, Render, SharedString,
    WeakEntity, Window, div, px,
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
    /// Starts one validated local transfer and returns whether it was accepted.
    pub(crate) fn begin_file_drop(
        &mut self,
        drag: FileDrag,
        target: DropTarget,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.provider.is_some() || self.drop_pending {
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

        let background =
            cx.background_spawn(async move { transfer_paths(&paths, &destination, mode.into()) });
        cx.spawn(async move |target, cx| {
            let report = background.await;
            complete_file_drop(target, source, target_id, source_id, mode, report, cx);
        })
        .detach();
        true
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
    let failure_status = if failure_count == 0 {
        None
    } else if success_count == 0 {
        Some(format!(
            "Drop failed: {failure_count} failed; {}",
            failures.join(", ")
        ))
    } else {
        Some(format!(
            "Drop partially completed: {success_count} succeeded, {failure_count} failed; {}",
            failures.join(", ")
        ))
    };

    tracing::info!(
        success_count,
        failure_count,
        ?mode,
        "file drop filesystem work completed"
    );

    if target_id == source_id {
        if target
            .update(cx, move |pane, cx| {
                finish_target_drop(pane, &destinations, failure_status, cx);
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
            finish_target_drop(pane, &destinations, failure_status, cx);
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

fn finish_target_drop(
    pane: &mut ExplorerPane,
    destinations: &[PathBuf],
    failure_status: Option<String>,
    cx: &mut Context<ExplorerPane>,
) {
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
    use gpui::{AppContext, Modifiers, TestAppContext, WindowHandle};
    use gpui_component::Root;
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;
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

    async fn settle_drop(cx: &mut TestAppContext) {
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();
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
}
