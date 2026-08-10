use super::types::ViewMode;
use gpui::{Bounds, Pixels, Point, point, px, size};
use std::collections::BTreeSet;

/// The minimum movement on either axis that turns an empty-space press into a drag.
pub(crate) const DRAG_THRESHOLD: Pixels = px(4.0);

/// Geometry-relevant state used to reject measurements from an older layout.
#[derive(Clone, PartialEq)]
pub(crate) struct GeometryToken {
    pub view_mode: ViewMode,
    pub entries_revision: u64,
    pub viewport: Bounds<Pixels>,
    pub scroll_offset: Point<Pixels>,
    pub item_sizes_revision: u64,
}

impl std::fmt::Debug for GeometryToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let view_mode = match self.view_mode {
            ViewMode::List => "List",
            ViewMode::Grid => "Grid",
        };

        formatter
            .debug_struct("GeometryToken")
            .field("view_mode", &view_mode)
            .field("entries_revision", &self.entries_revision)
            .field("viewport", &self.viewport)
            .field("scroll_offset", &self.scroll_offset)
            .field("item_sizes_revision", &self.item_sizes_revision)
            .finish()
    }
}

/// A listing item measured in GPUI's window coordinate space.
#[derive(Clone, Debug)]
pub(crate) struct MeasuredItem {
    pub index: usize,
    pub bounds: Bounds<Pixels>,
}

/// State captured when an empty-space marquee press begins.
#[derive(Clone, Debug)]
pub(crate) struct MarqueeDrag {
    pub start: Point<Pixels>,
    pub current: Point<Pixels>,
    pub token: GeometryToken,
    pub hitboxes: Vec<MeasuredItem>,
    pub base_selection: BTreeSet<usize>,
    pub prior_anchor: Option<usize>,
    pub prior_active: Option<usize>,
    pub additive: bool,
    /// Whether this press has crossed the drag threshold at least once.
    pub dragging: bool,
    pub hit_indices: BTreeSet<usize>,
}

/// Returns the rectangle spanning `start` and `current`, regardless of drag direction.
pub(crate) fn normalized_rect(start: Point<Pixels>, current: Point<Pixels>) -> Bounds<Pixels> {
    let left = start.x.as_f32().min(current.x.as_f32());
    let top = start.y.as_f32().min(current.y.as_f32());
    let right = start.x.as_f32().max(current.x.as_f32());
    let bottom = start.y.as_f32().max(current.y.as_f32());

    Bounds::new(
        point(px(left), px(top)),
        size(px(right - left), px(bottom - top)),
    )
}

/// Returns whether two rectangles overlap, including touching edges and corners.
pub(crate) fn intersects_closed(first: Bounds<Pixels>, second: Bounds<Pixels>) -> bool {
    first.left().as_f32() <= second.right().as_f32()
        && second.left().as_f32() <= first.right().as_f32()
        && first.top().as_f32() <= second.bottom().as_f32()
        && second.top().as_f32() <= first.bottom().as_f32()
}

/// Returns whether movement has reached the marquee threshold on either axis.
pub(crate) fn past_threshold(start: Point<Pixels>, current: Point<Pixels>) -> bool {
    (current.x - start.x).abs().as_f32() >= DRAG_THRESHOLD.as_f32()
        || (current.y - start.y).abs().as_f32() >= DRAG_THRESHOLD.as_f32()
}

/// Computes the live selection from the press snapshot and the current hit indices.
pub(crate) fn selection_for_hits(
    base_selection: &BTreeSet<usize>,
    hits: impl IntoIterator<Item = usize>,
    additive: bool,
) -> BTreeSet<usize> {
    let mut selection = if additive {
        base_selection.clone()
    } else {
        BTreeSet::new()
    };
    selection.extend(hits);
    selection
}

/// Computes the anchor and active indices to apply when a marquee is released.
pub(crate) fn completion_indices(
    hits: &BTreeSet<usize>,
    additive: bool,
    prior_anchor: Option<usize>,
    prior_active: Option<usize>,
) -> (Option<usize>, Option<usize>) {
    if !additive {
        return (
            hits.iter().next().copied(),
            hits.iter().next_back().copied(),
        );
    }

    let anchor = prior_anchor.or_else(|| hits.iter().next().copied());
    let active = hits.iter().next_back().copied().or(prior_active);
    (anchor, active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explorer::ExplorerPane;
    use crate::explorer::types::ViewMode;
    use chronos_fm_services::fs::listing::FileEntryDto;
    use gpui::{AppContext, Bounds, Modifiers, TestAppContext, WindowHandle, point, px, size};
    use gpui_component::input::InputState;
    use gpui_component::resizable::ResizableState;
    use std::collections::BTreeSet;

    fn new_pane(cx: &mut TestAppContext) -> WindowHandle<ExplorerPane> {
        cx.update(gpui_component::init);
        cx.update(crate::explorer::clipboard::init);
        cx.add_window(|window, cx| {
            let resizable = cx.new(|_| ResizableState::default());
            let search_input = cx.new(|cx| InputState::new(window, cx));
            ExplorerPane::new(resizable, search_input, None, cx.focus_handle())
        })
    }

    fn measure_pane(page: &mut ExplorerPane) {
        let viewport = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.)));
        let token = page.record_listing_viewport(viewport, point(px(0.), px(0.)));
        for (index, y) in [0., 15., 30., 60.].into_iter().enumerate() {
            page.record_item_bounds(
                index,
                Bounds::new(point(px(10.), px(y)), size(px(20.), px(10.))),
                token.clone(),
            );
        }
    }

    fn file(name: &str) -> FileEntryDto {
        FileEntryDto {
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            kind: "file".to_string(),
            size: 1,
            modified: 0,
        }
    }

    #[gpui::test]
    async fn pane_plain_drag_replaces_selection_and_sets_completion_indices(
        cx: &mut TestAppContext,
    ) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                measure_pane(pane);

                assert!(pane.begin_marquee(point(px(0.), px(0.)), Modifiers::default()));
                pane.update_marquee(point(px(40.), px(40.)));
                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));
                assert!(pane.marquee_rect().is_some());

                pane.finish_marquee();
                assert_eq!(pane.selection_anchor, Some(0));
                assert_eq!(pane.active_index, Some(2));
                assert!(pane.marquee.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_ctrl_drag_unions_selection_and_preserves_anchor(cx: &mut TestAppContext) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                measure_pane(pane);
                pane.selection = BTreeSet::from([3]);
                pane.selection_anchor = Some(3);
                pane.active_index = Some(3);

                assert!(pane.begin_marquee(
                    point(px(0.), px(0.)),
                    Modifiers {
                        control: true,
                        ..Modifiers::default()
                    },
                ));
                pane.update_marquee(point(px(40.), px(40.)));
                pane.finish_marquee();

                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2, 3]));
                assert_eq!(pane.selection_anchor, Some(3));
                assert_eq!(pane.active_index, Some(2));
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_subthreshold_empty_click_keeps_plain_clear_and_ctrl_selection(
        cx: &mut TestAppContext,
    ) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                measure_pane(pane);
                pane.selection = BTreeSet::from([2]);
                pane.selection_anchor = Some(2);
                pane.active_index = Some(2);

                assert!(pane.begin_marquee(point(px(0.), px(80.)), Modifiers::default()));
                pane.update_marquee(point(px(3.99), px(80.)));
                assert!(pane.selection.is_empty());
                assert!(pane.marquee_rect().is_none());
                pane.finish_marquee();
                assert_eq!(pane.selection_anchor, None);
                assert_eq!(pane.active_index, None);

                pane.selection = BTreeSet::from([2]);
                pane.selection_anchor = Some(2);
                pane.active_index = Some(2);
                assert!(pane.begin_marquee(
                    point(px(0.), px(80.)),
                    Modifiers {
                        control: true,
                        ..Modifiers::default()
                    },
                ));
                pane.update_marquee(point(px(3.99), px(80.)));
                pane.finish_marquee();
                assert_eq!(pane.selection, BTreeSet::from([2]));
                assert_eq!(pane.selection_anchor, Some(2));
                assert_eq!(pane.active_index, Some(2));
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_token_mismatch_cancels_and_keeps_last_live_selection(
        cx: &mut TestAppContext,
    ) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                measure_pane(pane);
                assert!(pane.begin_marquee(point(px(0.), px(0.)), Modifiers::default()));
                pane.update_marquee(point(px(40.), px(40.)));
                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));

                pane.record_listing_viewport(
                    Bounds::new(point(px(1.), px(0.)), size(px(100.), px(100.))),
                    point(px(0.), px(0.)),
                );
                pane.update_marquee(point(px(50.), px(50.)));

                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));
                assert!(pane.marquee.is_none());
                assert!(pane.marquee_rect().is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_viewport_token_change_cancels_before_mouse_up(cx: &mut TestAppContext) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                measure_pane(pane);
                assert!(pane.begin_marquee(point(px(0.), px(0.)), Modifiers::default()));
                pane.update_marquee(point(px(40.), px(40.)));
                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));

                pane.record_listing_viewport(
                    Bounds::new(point(px(1.), px(0.)), size(px(100.), px(100.))),
                    point(px(0.), px(0.)),
                );

                assert!(pane.marquee.is_none());
                assert!(pane.marquee_rect().is_none());
                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));
                pane.finish_marquee();
                assert_eq!(pane.selection_anchor, None);
                assert_eq!(pane.active_index, None);
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_drag_threshold_latches_after_returning_below_four_pixels(
        cx: &mut TestAppContext,
    ) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                measure_pane(pane);
                assert!(pane.begin_marquee(point(px(0.), px(0.)), Modifiers::default()));
                pane.update_marquee(point(px(40.), px(40.)));
                assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));

                pane.update_marquee(point(px(3.99), px(0.)));
                assert!(pane.marquee_rect().is_some());
                assert!(pane.selection.is_empty());
                pane.finish_marquee();
                assert_eq!(pane.selection_anchor, None);
                assert_eq!(pane.active_index, None);
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_replacing_filtered_entries_cancels_and_revisions_measurements(
        cx: &mut TestAppContext,
    ) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                pane.filtered_entries = vec![file("before")];
                pane.update_item_sizes();
                measure_pane(pane);
                let revision = pane.entries_revision;
                let item_sizes_revision = pane.item_sizes_revision;
                assert!(pane.begin_marquee(point(px(0.), px(0.)), Modifiers::default()));

                pane.replace_filtered_entries(vec![file("search-result")]);
                pane.update_item_sizes();

                assert!(pane.marquee.is_none());
                assert!(pane.measured_items.is_empty());
                assert!(pane.geometry_token.is_none());
                assert_eq!(pane.entries_revision, revision + 1);
                assert_eq!(pane.item_sizes_revision, item_sizes_revision);
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_opening_listing_overlays_cancels_marquee(cx: &mut TestAppContext) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, cx| {
                pane.filtered_entries = vec![file("selected")];
                pane.active_index = Some(0);
                measure_pane(pane);
                let additive = Modifiers {
                    control: true,
                    ..Modifiers::default()
                };
                assert!(pane.begin_marquee(point(px(0.), px(0.)), additive));
                pane.show_properties(cx);
                assert!(pane.marquee.is_none());

                measure_pane(pane);
                assert!(pane.begin_marquee(point(px(0.), px(0.)), additive));
                pane.open_context_menu_for_directory(point(px(0.), px(0.)), cx);
                assert!(pane.marquee.is_none());

                measure_pane(pane);
                assert!(pane.begin_marquee(point(px(0.), px(0.)), additive));
                pane.open_context_menu(
                    "/tmp/selected".to_string(),
                    0,
                    point(px(0.), px(0.)),
                    cx,
                );
                assert!(pane.marquee.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    async fn pane_deduplicates_same_token_exclusions(cx: &mut TestAppContext) {
        let window = new_pane(cx);
        window
            .update(cx, |pane, _window, _cx| {
                let viewport = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.)));
                let token = pane.record_listing_viewport(viewport, point(px(0.), px(0.)));
                let exclusion = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(10.)));
                let replacement = Bounds::new(point(px(0.), px(2.)), size(px(100.), px(12.)));
                pane.record_marquee_exclusion("header", exclusion, token.clone());
                pane.record_marquee_exclusion("header", exclusion, token.clone());
                pane.record_marquee_exclusion("header", replacement, token);

                assert_eq!(pane.marquee_exclusions.len(), 1);
                assert_eq!(pane.marquee_exclusions["header"], replacement);
            })
            .unwrap();
    }

    #[test]
    fn normalizes_all_drag_directions() {
        let expected = Bounds::new(point(px(10.), px(20.)), size(px(30.), px(40.)));

        assert_eq!(
            normalized_rect(point(px(10.), px(20.)), point(px(40.), px(60.))),
            expected
        );
        assert_eq!(
            normalized_rect(point(px(40.), px(20.)), point(px(10.), px(60.))),
            expected
        );
        assert_eq!(
            normalized_rect(point(px(10.), px(60.)), point(px(40.), px(20.))),
            expected
        );
        assert_eq!(
            normalized_rect(point(px(40.), px(60.)), point(px(10.), px(20.))),
            expected
        );
    }

    #[test]
    fn touching_edges_intersect() {
        let marquee = Bounds::new(point(px(0.), px(0.)), size(px(10.), px(10.)));
        let item = Bounds::new(point(px(10.), px(10.)), size(px(5.), px(5.)));
        assert!(intersects_closed(marquee, item));
    }

    #[test]
    fn separated_rectangles_do_not_intersect() {
        let marquee = Bounds::new(point(px(0.), px(0.)), size(px(10.), px(10.)));
        let item = Bounds::new(point(px(10.1), px(10.1)), size(px(5.), px(5.)));
        assert!(!intersects_closed(marquee, item));
    }

    #[test]
    fn threshold_is_reached_on_either_axis_at_four_pixels() {
        let start = point(px(10.), px(10.));
        assert!(!past_threshold(start, point(px(13.99), px(10.))));
        assert!(past_threshold(start, point(px(14.), px(10.))));
        assert!(past_threshold(start, point(px(10.), px(6.))));
    }

    #[test]
    fn plain_hits_replace_press_selection() {
        let base = BTreeSet::from([1, 4]);
        assert_eq!(
            selection_for_hits(&base, [2, 3], false),
            BTreeSet::from([2, 3])
        );
    }

    #[test]
    fn ctrl_hits_union_with_press_selection() {
        let base = BTreeSet::from([1, 4]);
        assert_eq!(
            selection_for_hits(&base, [2, 4], true),
            BTreeSet::from([1, 2, 4])
        );
    }

    #[test]
    fn completion_uses_stable_minimum_and_maximum_indices() {
        let hits = BTreeSet::from([5, 2, 9]);
        assert_eq!(
            completion_indices(&hits, false, None, None),
            (Some(2), Some(9))
        );
        assert_eq!(
            completion_indices(&hits, true, Some(7), Some(1)),
            (Some(7), Some(9))
        );
    }

    #[test]
    fn additive_completion_keeps_anchor_and_active_without_new_hits() {
        let hits = BTreeSet::new();
        assert_eq!(
            completion_indices(&hits, true, Some(7), Some(1)),
            (Some(7), Some(1))
        );
    }

    #[test]
    fn geometry_token_equality_distinguishes_stale_measurements() {
        let viewport = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.)));
        let current = GeometryToken {
            view_mode: ViewMode::List,
            entries_revision: 1,
            viewport,
            scroll_offset: point(px(0.), px(0.)),
            item_sizes_revision: 1,
        };
        let stale = GeometryToken {
            entries_revision: 2,
            ..current.clone()
        };

        assert_ne!(current, stale);
    }
}
