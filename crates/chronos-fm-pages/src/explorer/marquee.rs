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
    use crate::explorer::types::ViewMode;
    use gpui::{Bounds, point, px, size};
    use std::collections::BTreeSet;

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
