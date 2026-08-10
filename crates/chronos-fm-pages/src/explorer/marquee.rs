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
    use gpui::{
        AppContext, Bounds, Entity, Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent,
        TestAppContext, VisualTestContext, WindowHandle, point, px, size,
    };
    use gpui_component::Root;
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

    fn rooted_pane(
        cx: &mut TestAppContext,
        view_mode: ViewMode,
    ) -> (WindowHandle<Root>, Entity<ExplorerPane>) {
        rooted_pane_with_entries(cx, view_mode, 6)
    }

    fn rooted_pane_with_entries(
        cx: &mut TestAppContext,
        view_mode: ViewMode,
        entry_count: usize,
    ) -> (WindowHandle<Root>, Entity<ExplorerPane>) {
        cx.update(gpui_component::init);
        cx.update(crate::explorer::clipboard::init);
        let entries = (0..entry_count)
            .map(|ix| file(&format!("marquee-{ix}.txt")))
            .collect::<Vec<_>>();
        let root = cx.add_window(move |window, cx| {
            let pane = cx.new(|cx| {
                let mut pane = ExplorerPane::build(None, window, cx);
                pane.loaded = true;
                pane.sidebar_visible = false;
                pane.view_mode = view_mode;
                pane.entries = entries.clone();
                pane.replace_filtered_entries(entries);
                pane.update_item_sizes();
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

    fn draw_window(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    fn point_inside(bounds: Bounds<gpui::Pixels>, position: Point<gpui::Pixels>) -> bool {
        position.x.as_f32() >= bounds.left().as_f32()
            && position.x.as_f32() <= bounds.right().as_f32()
            && position.y.as_f32() >= bounds.top().as_f32()
            && position.y.as_f32() <= bounds.bottom().as_f32()
    }

    fn center(bounds: Bounds<gpui::Pixels>) -> Point<gpui::Pixels> {
        point(
            bounds.left() + bounds.size.width / 2.,
            bounds.top() + bounds.size.height / 2.,
        )
    }

    fn routing_points(
        pane: &ExplorerPane,
    ) -> (Point<gpui::Pixels>, Point<gpui::Pixels>, BTreeSet<usize>) {
        let viewport = pane
            .listing_viewport
            .expect("marquee routing must measure the listing viewport");
        assert!(
            pane.measured_items.len() >= 3,
            "marquee routing must measure at least three visible items"
        );

        let left = viewport.left().as_f32() + 8.;
        let right = viewport.right().as_f32() - 8.;
        let top = viewport.top().as_f32() + 8.;
        let bottom = viewport.bottom().as_f32() - 8.;
        let mut starts = Vec::new();
        for x_step in 0..=10 {
            for y_step in 0..=10 {
                let x = left + (right - left) * x_step as f32 / 10.;
                let y = top + (bottom - top) * y_step as f32 / 10.;
                let candidate = point(px(x), px(y));
                if pane
                    .measured_items
                    .values()
                    .chain(pane.marquee_exclusions.values())
                    .all(|bounds| !point_inside(*bounds, candidate))
                {
                    starts.push(candidate);
                }
            }
        }

        let mut ends = Vec::new();
        for bounds in pane.measured_items.values() {
            let item_center = center(*bounds);
            ends.push(point(
                px(item_center.x.as_f32().max(left).min(right)),
                px(item_center.y.as_f32().max(top).min(bottom)),
            ));
            ends.push(point(
                px(bounds.left().as_f32().max(left).min(right)),
                px(bounds.top().as_f32().max(top).min(bottom)),
            ));
            ends.push(point(
                px(bounds.right().as_f32().max(left).min(right)),
                px(bounds.bottom().as_f32().max(top).min(bottom)),
            ));
        }

        let mut best = None;
        for start in starts {
            for &end in &ends {
                if !past_threshold(start, end) {
                    continue;
                }
                let rect = normalized_rect(start, end);
                let hits = pane
                    .measured_items
                    .iter()
                    .filter(|(_, bounds)| intersects_closed(rect, **bounds))
                    .map(|(&ix, _)| ix)
                    .collect::<BTreeSet<_>>();
                if hits.len() < 3 {
                    continue;
                }
                let distance = (start.x - end.x).abs().as_f32() + (start.y - end.y).abs().as_f32();
                if best.as_ref().is_none_or(
                    |(best_hits, best_distance, _, _): &(BTreeSet<usize>, f32, _, _)| {
                        hits.len() > best_hits.len()
                            || (hits.len() == best_hits.len() && distance > *best_distance)
                    },
                ) {
                    best = Some((hits, distance, start, end));
                }
            }
        }

        let (hits, _, start, end) =
            best.expect("measured listing must expose an empty drag spanning three items");
        (start, end, hits)
    }

    fn assert_rect_inside(inner: Bounds<gpui::Pixels>, outer: Bounds<gpui::Pixels>) {
        assert!(inner.left().as_f32() >= outer.left().as_f32());
        assert!(inner.top().as_f32() >= outer.top().as_f32());
        assert!(inner.right().as_f32() <= outer.right().as_f32());
        assert!(inner.bottom().as_f32() <= outer.bottom().as_f32());
    }

    #[gpui::test]
    fn marquee_routing_list_measures_rejects_chrome_and_finishes(cx: &mut TestAppContext) {
        let (root, pane) = rooted_pane(cx, ViewMode::List);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.), px(560.)));
        draw_window(&mut cx);

        let (row_points, header_bounds, header_point) = pane.read_with(&cx, |pane, _cx| {
            let rows = [0, 2, 4].map(|ix| {
                center(
                    *pane
                        .measured_items
                        .get(&ix)
                        .expect("list must measure the click-test row"),
                )
            });
            let header = *pane
                .marquee_exclusions
                .get("list-header")
                .expect("list must measure its header exclusion");
            let blank_action_column = point(
                header.left()
                    + px(16.0
                        + pane.col_name_width
                        + pane.col_type_width
                        + pane.col_size_width
                        + pane.col_modified_width
                        + pane.col_action_width / 2.0),
                center(header).y,
            );
            (rows, header, blank_action_column)
        });
        let resize_bounds = [
            "list-column-resize-0",
            "list-column-resize-1",
            "list-column-resize-2",
            "list-column-resize-3",
        ]
        .map(|selector| {
            cx.debug_bounds(selector)
                .expect("list must render every nested column resize handle")
        });
        assert!(point_inside(header_bounds, header_point));
        assert!(
            resize_bounds
                .iter()
                .all(|bounds| !point_inside(*bounds, header_point)),
            "blank header point must be outside nested resize controls"
        );
        let resize_point = center(resize_bounds[0]);

        cx.simulate_mouse_down(row_points[0], MouseButton::Left, Modifiers::default());
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_none()));
        cx.simulate_mouse_up(row_points[0], MouseButton::Left, Modifiers::default());
        assert_eq!(
            pane.read_with(&cx, |pane, _cx| pane.selection.clone()),
            BTreeSet::from([0])
        );
        cx.simulate_click(
            row_points[1],
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(
            pane.read_with(&cx, |pane, _cx| pane.selection.clone()),
            BTreeSet::from([0, 2])
        );
        cx.simulate_click(
            row_points[2],
            Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
        );
        pane.read_with(&cx, |pane, _cx| {
            assert_eq!(pane.selection, BTreeSet::from([2, 3, 4]));
            assert_eq!(pane.selection_anchor, Some(2));
        });
        cx.simulate_mouse_down(header_point, MouseButton::Left, Modifiers::default());
        pane.read_with(&cx, |pane, _cx| {
            assert!(pane.marquee.is_none());
            assert!(
                pane.resizing_column.is_none(),
                "blank header press must not be owned by a resize control"
            );
        });
        cx.simulate_mouse_up(header_point, MouseButton::Left, Modifiers::default());

        cx.simulate_mouse_down(resize_point, MouseButton::Left, Modifiers::default());
        pane.read_with(&cx, |pane, _cx| {
            assert!(pane.marquee.is_none());
            assert!(
                pane.resizing_column.is_some(),
                "the nested name-column resize handle must own the press"
            );
        });
        cx.simulate_mouse_up(resize_point, MouseButton::Left, Modifiers::default());
        draw_window(&mut cx);

        let (start, end, expected) = pane.read_with(&cx, |pane, _cx| routing_points(pane));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_some()));
        cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
        pane.read_with(&cx, |pane, _cx| {
            assert_eq!(pane.selection, expected);
            assert!(pane.selection.len() >= 3);
            assert_rect_inside(
                pane.marquee_rect()
                    .expect("drag paints a marquee rectangle"),
                pane.listing_viewport.expect("viewport remains measured"),
            );
        });
        draw_window(&mut cx);
        let overlay = cx
            .debug_bounds("marquee-overlay")
            .expect("active drag paints the marquee overlay");
        assert_eq!(
            overlay,
            pane.read_with(&cx, |pane, _cx| pane.marquee_rect().unwrap()),
            "the local absolute overlay must land on the window-coordinate marquee",
        );
        assert_rect_inside(
            overlay,
            pane.read_with(&cx, |pane, _cx| pane.listing_viewport.unwrap()),
        );

        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_none()));
    }

    #[gpui::test]
    fn marquee_routing_grid_is_additive_and_cleans_up_outside_or_on_scroll(
        cx: &mut TestAppContext,
    ) {
        let (root, pane) = rooted_pane(cx, ViewMode::Grid);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(900.), px(560.)));
        draw_window(&mut cx);

        let tile_points = pane.read_with(&cx, |pane, _cx| {
            [0, 2, 4].map(|ix| {
                center(
                    *pane
                        .measured_items
                        .get(&ix)
                        .expect("grid must measure the click-test tile"),
                )
            })
        });
        cx.simulate_mouse_down(tile_points[0], MouseButton::Left, Modifiers::default());
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_none()));
        cx.simulate_mouse_up(tile_points[0], MouseButton::Left, Modifiers::default());
        assert_eq!(
            pane.read_with(&cx, |pane, _cx| pane.selection.clone()),
            BTreeSet::from([0])
        );
        cx.simulate_click(
            tile_points[1],
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(
            pane.read_with(&cx, |pane, _cx| pane.selection.clone()),
            BTreeSet::from([0, 2])
        );
        cx.simulate_click(
            tile_points[2],
            Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
        );
        pane.read_with(&cx, |pane, _cx| {
            assert_eq!(pane.selection, BTreeSet::from([2, 3, 4]));
            assert_eq!(pane.selection_anchor, Some(2));
        });

        pane.update(&mut cx, |pane, cx| {
            pane.selection = BTreeSet::from([5]);
            pane.selection_anchor = Some(5);
            pane.active_index = Some(5);
            cx.notify();
        });
        draw_window(&mut cx);
        let (start, end, hits) = pane.read_with(&cx, |pane, _cx| routing_points(pane));
        let modifiers = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        cx.simulate_mouse_down(start, MouseButton::Left, modifiers);
        cx.simulate_mouse_move(end, Some(MouseButton::Left), modifiers);
        let mut expected = hits;
        expected.insert(5);
        assert_eq!(
            pane.read_with(&cx, |pane, _cx| pane.selection.clone()),
            expected
        );
        assert!(pane.read_with(&cx, |pane, _cx| pane.selection.len() >= 3));

        let viewport = pane.read_with(&cx, |pane, _cx| pane.listing_viewport.unwrap());
        let outside = point(viewport.left() - px(4.), viewport.top() - px(4.));
        cx.simulate_mouse_up(outside, MouseButton::Left, modifiers);
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_none()));

        let (start, end, _) = pane.read_with(&cx, |pane, _cx| routing_points(pane));
        cx.simulate_mouse_down(start, MouseButton::Left, modifiers);
        cx.simulate_mouse_move(end, Some(MouseButton::Left), modifiers);
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_some()));
        cx.simulate_event(ScrollWheelEvent {
            position: start,
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });
        assert!(pane.read_with(&cx, |pane, _cx| pane.marquee.is_none()));
    }

    #[gpui::test]
    fn marquee_routing_grid_retains_only_tiles_intersecting_viewport(cx: &mut TestAppContext) {
        let (root, pane) = rooted_pane_with_entries(cx, ViewMode::Grid, 48);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(720.), px(360.)));
        draw_window(&mut cx);

        pane.read_with(&cx, |pane, _cx| {
            let viewport = pane
                .listing_viewport
                .expect("overflowing grid must measure its viewport");
            let offscreen = pane
                .measured_items
                .iter()
                .filter_map(|(&ix, &bounds)| {
                    (!intersects_closed(bounds, viewport)).then_some((ix, bounds))
                })
                .collect::<Vec<_>>();
            assert!(
                offscreen.is_empty(),
                "grid retained off-screen prepaint bounds: {offscreen:?}"
            );
            assert!(
                pane.measured_items.len() < pane.filtered_entries.len(),
                "overflowing grid must exclude non-visible tiles"
            );
            assert!(
                !pane
                    .measured_items
                    .contains_key(&(pane.filtered_entries.len() - 1)),
                "the final overflowing tile must not remain measured"
            );
        });
    }

    #[gpui::test]
    fn marquee_routing_list_measures_only_virtual_visible_rows(cx: &mut TestAppContext) {
        let (root, pane) = rooted_pane_with_entries(cx, ViewMode::List, 48);
        let mut cx = VisualTestContext::from_window(root.into(), cx);
        cx.simulate_resize(size(px(720.), px(360.)));
        draw_window(&mut cx);

        pane.read_with(&cx, |pane, _cx| {
            assert!(!pane.measured_items.is_empty());
            assert!(
                pane.measured_items.len() < pane.filtered_entries.len(),
                "virtual list must not measure every overflowing row"
            );
            assert!(
                !pane
                    .measured_items
                    .contains_key(&(pane.filtered_entries.len() - 1)),
                "the final overflowing row must not remain measured"
            );
        });
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
    async fn pane_token_mismatch_cancels_and_keeps_last_live_selection(cx: &mut TestAppContext) {
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
                pane.open_context_menu("/tmp/selected".to_string(), 0, point(px(0.), px(0.)), cx);
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
