use comrak::{Options, markdown_to_html, options::Render};
use ordered_float::OrderedFloat;
use rand::distr::Distribution;
use rand::distr::Uniform;
use rand::rng;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound::Included;
use web_sys::window;
// use rustybuzz::{UnicodeBuffer, shape};
use std::collections::HashMap;
use yew::{Html, prelude::*, virtual_dom::VNode};

pub fn nl2br(text: &str) -> Html {
    let mut nodes = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            nodes.push(html! { <br/> });
        }
        nodes.push(html! { {line} });
    }
    html! { {for nodes} }
}

pub fn dangerous_raw_html(raw_html_string: String) -> VNode {
    return Html::from_html_unchecked(AttrValue::from(raw_html_string));
}

pub fn md(md_str: String) -> VNode {
    dangerous_raw_html(markdown_to_html(
        &md_str,
        &Options {
            render: Render {
                r#unsafe: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ))
}

pub fn make_data_table(str_in: String) -> HashMap<String, String> {
    let key_re = Regex::new(r"^\s*\\?\[([^\]]+?)\\?\]\s*$").unwrap();

    let mut table: HashMap<String, String> = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut buffer: Vec<String> = Vec::new();

    for raw_line in str_in.lines() {
        let line = raw_line;

        if let Some(caps) = key_re.captures(line) {
            if let Some(k) = current_key.take() {
                let value = buffer.join("\n").trim().to_string();
                table.insert(k, value);
                buffer.clear();
            }

            current_key = Some(caps[1].to_string());
        } else if current_key.is_some() {
            buffer.push(line.to_string());
        }
    }

    if let Some(k) = current_key {
        let value = buffer.join("\n").trim().to_string();
        table.insert(k, value);
    }

    table
}

#[derive(Clone, PartialEq, Debug)]
pub struct RectMask {
    pub w: f64,
    pub h: f64,
    pub x: f64,
    pub y: f64,
}

impl RectMask {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
    /// Does this rectangle intersect `other` ?
    pub fn intersects(&self, other: &RectMask) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Does this rectangle fully contain `other` ?
    pub fn contains(&self, other: &RectMask) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.w <= self.x + self.w
            && other.y + other.h <= self.y + self.h
    }
}

/// One node of a quadtree.  `T` is the *index* type – for your
/// application it will be `(usize, usize)`, but any type that can be
/// cloned works.
#[derive(Clone, Debug)]
pub struct QuadNode<T: Clone> {
    /// Bounding rectangle that encloses everything stored in this node
    pub bounds: RectMask,
    /// Items that actually live in this node (index + rectangle)
    pub items: Vec<(T, RectMask)>,
    /// Children – 4 quadrants (None until we split)
    pub children: Option<Box<[QuadNode<T>; 4]>>,
}

impl<T: Clone> QuadNode<T> {
    /// Create a node that covers the supplied rectangle
    pub fn new(bounds: RectMask) -> Self {
        Self {
            bounds,
            items: Vec::new(),
            children: None,
        }
    }

    /// Split the node into 4 children.  The current node becomes
    /// an internal node – all items are moved to the appropriate child.
    pub fn subdivide(&mut self, max_items: usize) {
        // Only split if we actually have to
        if self.items.len() <= max_items {
            return;
        }

        let mid_x = self.bounds.x + self.bounds.w / 2.0;
        let mid_y = self.bounds.y + self.bounds.h / 2.0;

        let mut children = [
            // NW
            QuadNode::new(RectMask {
                x: self.bounds.x,
                y: self.bounds.y,
                w: mid_x - self.bounds.x,
                h: mid_y - self.bounds.y,
            }),
            // NE
            QuadNode::new(RectMask {
                x: mid_x,
                y: self.bounds.y,
                w: self.bounds.x + self.bounds.w - mid_x,
                h: mid_y - self.bounds.y,
            }),
            // SW
            QuadNode::new(RectMask {
                x: self.bounds.x,
                y: mid_y,
                w: mid_x - self.bounds.x,
                h: self.bounds.y + self.bounds.h - mid_y,
            }),
            // SE
            QuadNode::new(RectMask {
                x: mid_x,
                y: mid_y,
                w: self.bounds.x + self.bounds.w - mid_x,
                h: self.bounds.y + self.bounds.h - mid_y,
            }),
        ];

        // Move all current items into the children
        for (idx, rect) in self.items.drain(..) {
            // Find the child that can fully contain this rectangle
            let child_idx = children
                .iter_mut()
                .position(|c| c.bounds.intersects(&rect))
                .expect("Rect cannot be contained by any child – should never happen");
            children[child_idx].items.push((idx, rect));
        }

        self.children = Some(Box::new(children));
    }

    /// Insert a rectangle into the node (or one of its children)
    pub fn insert(&mut self, idx: T, rect: RectMask, max_items: usize) {
        if let Some(children) = &mut self.children {
            // node already split – forward to the child that can hold the rect
            let child_idx = children
                .iter_mut()
                .position(|c| c.bounds.intersects(&rect))
                .expect("Rect cannot be contained by any child – should never happen");
            children[child_idx].insert(idx, rect, max_items);
            return;
        }

        self.items.push((idx, rect));
        // split when we exceed the threshold
        if self.items.len() > max_items {
            self.subdivide(max_items);
        }
    }

    /// Return all items that intersect `rect` (including the ones stored
    /// in child nodes).  The result is appended to `out`.
    pub fn query(&self, rect: &RectMask, out: &mut Vec<(T, RectMask)>) {
        // if node's own bounds do not intersect – no point to look further
        if !self.bounds.intersects(rect) {
            return;
        }

        // check items stored directly in this node
        for (idx, item) in &self.items {
            if item.intersects(rect) {
                out.push((idx.clone(), item.clone()));
            }
        }

        // descend into children if we have any
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.query(rect, out);
            }
        }
    }
}

pub struct ParsedDomRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// get bounding rect from elem id
pub fn get_bounding_from_id(elem_id: &str) -> Option<ParsedDomRect> {
    let target = window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(elem_id);

    target.map(|x| {
        let bounding = x.get_bounding_client_rect();
        ParsedDomRect {
            x: bounding.x(),
            y: bounding.y(),
            width: bounding.width(),
            height: bounding.height(),
            left: bounding.left(),
            top: bounding.top(),
            bottom: bounding.bottom(),
            right: bounding.right(),
        }
    })
}

/// Generate `count` random masks that all fit inside
/// a bounding box of size `whole_width × whole_height`.
///
/// # Panics
/// The function will `panic!` if the requested window size is larger than the
/// whole area – in that situation there is simply no valid placement.
pub fn gen_random_masks(
    whole_width: f64,
    whole_height: f64,
    count: usize,
    window_width: f64,
    window_height: f64,
) -> Vec<RectMask> {
    // The rectangle must be able to fit – otherwise nothing can be placed.
    assert!(
        window_width <= whole_width && window_height <= whole_height,
        "Window size must be <= whole size"
    );

    // Uniform distributions for x and y.  They are *inclusive* so that 0 and
    // the maximum allowed coordinates are possible.
    let x_rng = Uniform::new_inclusive(0.0, whole_width - window_width).unwrap();
    let y_rng = Uniform::new_inclusive(0.0, whole_height - window_height).unwrap();

    let mut rng = rng();
    let mut masks = Vec::with_capacity(count);

    for _ in 0..count {
        let x = x_rng.sample(&mut rng);
        let y = y_rng.sample(&mut rng);

        masks.push(RectMask {
            w: window_width,
            h: window_height,
            x,
            y,
        });
    }

    masks
}

/// Return all indices of the masks that intersect the point `(x, y)`.
/// The search is limited to the indices listed in `search_indices`.
///
/// The point is considered inside a mask when  
/// `x` ∈ `[mask.x, mask.x + mask.w)` **and**  
/// `y` ∈ `[mask.y, mask.y + mask.h)`.
///
/// # Arguments
/// * `masks` – a 2‑D vector: `masks[row][col]` is a reference to an
///   optional rectangle mask.
/// * `x`, `y` – query point.
/// * `search_indices` – a slice of grid indices that should be examined.
///
/// # Returns
/// A `Vec<(usize, usize)>` containing the indices of every rectangle that
/// really contains the point.
///
/// # Complexity
/// `O(|search_indices|)` – the function just scans the slice once.
pub fn search_intersects_limit(
    masks: &Vec<&Vec<Option<RectMask>>>,
    x: f64,
    y: f64,
    search_indices: &Vec<(usize, usize)>,
) -> Vec<(usize, usize)> {
    let mut hits = Vec::new();

    for &(row, col) in search_indices {
        // Safety: the caller guarantees that the indices are in bounds.
        if let Some(ref mask) = masks[row][col] {
            // Check if the point lies inside the rectangle.
            // Left/top edges are inclusive, right/bottom are exclusive.
            if mask.contains(&RectMask {
                w: 0.0,
                h: 0.0,
                x,
                y,
            }) {
                hits.push((row, col));
            }
        }
    }

    hits
}

/// Return all indices of the masks that contain the point `(x, y)`.
///
/// * `masks` – a 2‑d vector: `masks[row][col]` is a reference to an
///   optional rectangle mask.
/// * `xlist` – indices sorted by the left side (`mask.x`) in ascending order.
/// * `max_width` – maximum width any mask can have.
///
/// The caller builds `xlist` once (it is a `Vec<(usize, usize)>`).
pub fn search_intersects_b_2d(
    masks: &Vec<&Vec<Option<RectMask>>>,
    xlist: &Vec<(usize, usize)>,
    x: f64,
    y: f64,
    max_width: f64,
) -> Vec<(usize, usize)> {
    let mut res = Vec::new();

    if xlist.is_empty() {
        return res;
    }

    /* ---------- 1. find insertion point of the first mask with left > x ---------- */
    let pos = match xlist.binary_search_by(|&(r, c)| {
        // comparison returns Ordering::Less when the mask’s left side is <= x
        // (so the insertion point is after all such masks).
        let mask_opt = &masks[r][c];
        match mask_opt {
            Some(mask) => {
                if mask.x <= x {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            None => Ordering::Greater, // treat `None` as > x
        }
    }) {
        Ok(i) => i + 1, // exact match → go to the right
        Err(i) => i,    // insertion point
    };

    // nothing has left side <= x → nothing can intersect
    if pos == 0 {
        return res;
    }

    /* ---------- 2. walk leftwards and test each candidate ---------- */
    let start_idx = pos - 1; // last index whose left side ≤ x

    for idx in (0..=start_idx).rev() {
        let (r, c) = xlist[idx];
        let mask_opt = &masks[r][c];

        // stop as soon as we are too far left
        if let Some(mask) = mask_opt {
            if mask.x < x - max_width {
                break;
            }

            // point must be inside the rectangle
            if mask.contains(&RectMask {
                w: 0.0,
                h: 0.0,
                x,
                y,
            }) {
                res.push((r, c));
            }
        }
    }

    res
}

/// Build a BTreeMap that maps an x–coordinate → a vector of (row,col) indices
/// where a rectangle exists.  The map is sorted by x so that we can fetch
/// a contiguous range in O(log N) time.
pub fn gen_btreemap(
    masks: &Vec<&Vec<Option<RectMask>>>,
) -> BTreeMap<OrderedFloat<f64>, Vec<(usize, usize)>> {
    let mut btree: BTreeMap<OrderedFloat<f64>, Vec<(usize, usize)>> = BTreeMap::new();

    for (i, a) in masks.iter().enumerate() {
        for (j, b) in a.iter().enumerate() {
            if let Some(b) = b {
                if btree.contains_key(&b.x.into()) {
                    let a = btree.get_mut(&b.x.into()).unwrap();
                    a.push((i, j));
                } else {
                    btree.insert(b.x.into(), vec![(i, j)]);
                }
            }
        }
    }
    return btree;
}

/// Find all rectangles that intersect a query rectangle defined by
/// (x, y, max_width, max_width).  The search is limited to the x‑range
/// `[x, x + max_width]` using the BTreeMap, and an additional quick
/// y‑filter is applied before the expensive `intersects` call.
pub fn search_intersects_btreemap(
    masks: &Vec<&Vec<Option<RectMask>>>,
    btree: &BTreeMap<OrderedFloat<f64>, Vec<(usize, usize)>>,
    point: (f64, f64),
    max_width: f64,
) -> Vec<(usize, usize)> {
    let (qx, qy) = point;
    let query = RectMask::new(qx, qy, 0.0, 0.0);

    // Range of x‑values that could overlap the query rectangle.
    let lower = OrderedFloat(qx - max_width);
    let upper = OrderedFloat(qx);

    let mut result = Vec::new();

    for (_x, positions) in btree.range((Included(&lower), Included(&upper))) {
        for &(row, col) in positions {
            // `masks[row][col]` is &Option<RectMask>
            let rect_opt = &masks[row][col];
            if let Some(rect) = rect_opt {
                if rect.contains(&query) {
                    result.push((row, col));
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests_rectmask {
    use ordered_float::OrderedFloat;

    use crate::util::{search_intersects_b_2d, search_intersects_limit};

    use super::{QuadNode, RectMask};

    // ------------------------------------------------------------------
    // Helper to create a rectangle – keeps the tests readable
    fn r(x: f64, y: f64, w: f64, h: f64) -> RectMask {
        RectMask::new(x, y, w, h)
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_intersects_overlapping() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(0.5, 0.5, 1.0, 1.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_intersects_disjoint() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(2.0, 2.0, 1.0, 1.0);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_intersects_touching_edges() {
        // Touching on the right side – should *not* intersect
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(1.0, 0.0, 1.0, 1.0);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));

        // Touching on the top side – should *not* intersect
        let c = r(0.0, 1.0, 1.0, 1.0);
        assert!(!a.intersects(&c));
        assert!(!c.intersects(&a));

        // Corner touch – also false
        let d = r(1.0, 1.0, 1.0, 1.0);
        assert!(!a.intersects(&d));
        assert!(!d.intersects(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_intersects_zero_area() {
        // A rectangle of zero width/height should *not* intersect a non‑empty one
        let a = r(0.0, 0.0, 0.0, 0.0);
        let b = r(0.5, 0.5, 1.0, 1.0);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));

        // A zero‑area rectangle inside a larger one *does* intersect
        let c = r(0.5, 0.5, 0.0, 0.0);
        let d = r(0.0, 0.0, 1.0, 1.0);
        assert!(d.intersects(&c));
        assert!(c.intersects(&d)); // symmetry still holds
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_contains_full() {
        let a = r(0.0, 0.0, 2.0, 2.0);
        let b = r(0.5, 0.5, 1.0, 1.0);
        assert!(a.contains(&b));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_contains_equal() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(0.0, 0.0, 1.0, 1.0);
        assert!(a.contains(&b));
        assert!(b.contains(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_contains_zero_area_inside() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(0.5, 0.5, 0.0, 0.0);
        assert!(a.contains(&b)); // zero‑area rectangle is “inside” any rectangle
        assert!(!b.contains(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_contains_touching_edges() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(1.0, 0.0, 1.0, 1.0);
        assert!(!a.contains(&b)); // touching on the right side
        assert!(!b.contains(&a)); // touching on the left side
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_contains_not_full() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(0.5, 0.5, 0.6, 0.6);
        assert!(!a.contains(&b)); // partially overlaps but not fully inside
        assert!(!b.contains(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_intersects_symmetry() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(0.2, 0.2, 0.5, 0.5);
        assert_eq!(a.intersects(&b), b.intersects(&a));
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_contains_asymmetry() {
        let a = r(0.0, 0.0, 1.0, 1.0);
        let b = r(0.2, 0.2, 0.5, 0.5);
        assert!(a.contains(&b));
        assert!(!b.contains(&a)); // contains is not symmetric
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_negative_coordinates() {
        // Negative coordinates are handled correctly
        let a = r(-1.0, -1.0, 2.0, 2.0);
        let b = r(0.0, 0.0, 1.0, 1.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(a.contains(&b));
        assert!(!b.contains(&a));
    }
}

#[cfg(test)]
mod tests_quadnode {
    use ordered_float::OrderedFloat;

    use crate::util::{search_intersects_b_2d, search_intersects_limit};

    use super::{QuadNode, RectMask};
    /// Helper to build a node covering the square `0..10` in both axes
    fn root_node() -> QuadNode<usize> {
        QuadNode::new(RectMask::new(0.0, 0.0, 10.0, 10.0))
    }

    /// Helper to create a rectangle that fits entirely inside a child
    /// quadrant. The returned rectangle is anchored at the centre of the child.
    fn child_rect(idx: usize, child: usize) -> (usize, RectMask) {
        // child 0 = NW, 1 = NE, 2 = SW, 3 = SE
        let (x, y) = match child {
            0 => (1.0, 1.0), // NW
            1 => (6.0, 1.0), // NE
            2 => (1.0, 6.0), // SW
            3 => (6.0, 6.0), // SE
            _ => unreachable!(),
        };
        (idx, RectMask::new(x, y, 2.0, 2.0))
    }

    #[test]
    fn test_new() {
        let bounds = RectMask::new(0.0, 0.0, 5.0, 5.0);
        let node = QuadNode::<usize>::new(bounds);

        assert_eq!(node.bounds.x, 0.0);
        assert_eq!(node.bounds.y, 0.0);
        assert_eq!(node.bounds.w, 5.0);
        assert_eq!(node.bounds.h, 5.0);
        assert!(node.items.is_empty());
        assert!(node.children.is_none());
    }

    #[test]
    fn test_subdivide_and_insert() {
        let mut root = root_node();
        // Use a very small max_items so that the root subdivides early
        const MAX_ITEMS: usize = 1;

        // Insert 4 items that fall into the 4 different quadrants
        for (idx, rect) in (0..4).map(|i| child_rect(i, i)) {
            root.insert(idx, rect, MAX_ITEMS);
        }

        // After the second insert the root must have split
        assert!(root.children.is_some());

        // After the split the root should have no items of its own
        assert!(root.items.is_empty());

        // Each child should contain exactly one item
        let children = root.children.unwrap();
        for (i, child) in children.iter().enumerate() {
            assert_eq!(child.items.len(), 1);
            let (idx, rect) = &child.items[0];
            assert_eq!(*idx, i);
            // The rectangle should be the one we inserted
            assert_eq!(*rect, child_rect(*idx, *idx).1);
        }
    }

    #[test]
    fn test_subdivide_bounds() {
        let mut root = root_node();
        const MAX_ITEMS: usize = 1;
        // Insert two items to trigger the subdivision
        root.insert(0, RectMask::new(1.0, 1.0, 1.0, 1.0), MAX_ITEMS);
        root.insert(1, RectMask::new(7.0, 1.0, 1.0, 1.0), MAX_ITEMS);

        // Grab the children after subdivision
        let children = root.children.unwrap();

        // Expected bounds for each child
        let expected = [
            // NW
            RectMask::new(0.0, 0.0, 5.0, 5.0),
            // NE
            RectMask::new(5.0, 0.0, 5.0, 5.0),
            // SW
            RectMask::new(0.0, 5.0, 5.0, 5.0),
            // SE
            RectMask::new(5.0, 5.0, 5.0, 5.0),
        ];

        for (child, exp) in children.iter().zip(expected.iter()) {
            assert_eq!(child.bounds.x, exp.x);
            assert_eq!(child.bounds.y, exp.y);
            assert_eq!(child.bounds.w, exp.w);
            assert_eq!(child.bounds.h, exp.h);
        }
    }

    #[test]
    fn test_query() {
        let mut root = root_node();
        const MAX_ITEMS: usize = 1;

        // Insert four items, one per quadrant
        for (idx, rect) in (0..4).map(|i| child_rect(i, i)) {
            root.insert(idx, rect, MAX_ITEMS);
        }

        // Query that covers the whole root – should return all 4 items
        let mut out: Vec<(usize, RectMask)> = Vec::new();
        root.query(&RectMask::new(0.0, 0.0, 10.0, 10.0), &mut out);
        assert_eq!(out.len(), 4);
        out.sort_by_key(|(idx, _)| *idx);
        for (i, (idx, _)) in out.iter().enumerate() {
            assert_eq!(*idx, i);
        }

        // Query that covers only the NW quadrant – should return one item
        let mut out: Vec<(usize, RectMask)> = Vec::new();
        root.query(&RectMask::new(0.0, 0.0, 5.0, 5.0), &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 0);

        // Query that does not intersect anything – should return empty
        let mut out: Vec<(usize, RectMask)> = Vec::new();
        root.query(&RectMask::new(20.0, 20.0, 5.0, 5.0), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_insert_into_children_after_subdivide() {
        let mut root = root_node();
        const MAX_ITEMS: usize = 1;

        // Insert two items – triggers subdivision
        root.insert(0, RectMask::new(1.0, 5.5, 1.0, 1.0), MAX_ITEMS);
        root.insert(1, RectMask::new(7.0, 1.0, 1.0, 1.0), MAX_ITEMS);

        // Now insert a new item that belongs to the SW quadrant (be careful not to trigger
        // further subdivision)
        let new_item = (2, RectMask::new(1.0, 7.0, 1.0, 1.0));
        root.insert(new_item.0, new_item.1, 100000);

        // The root should still have no items of its own
        assert!(root.items.is_empty());

        println!("{:?}", root);

        // The child corresponding to SW should now contain two items
        let children = root.children.unwrap();
        let sw_child = &children[2]; // index 2 == SW
        assert_eq!(sw_child.items.len(), 2);
        // The first item was the one inserted earlier (idx=0)
        assert_eq!(sw_child.items[0].0, 0);
        // The second item is the new one
        assert_eq!(sw_child.items[1].0, 2);
    }
}

#[cfg(test)]
mod tests_simple_range {
    use std::collections::{BTreeMap, HashSet};

    use ordered_float::OrderedFloat;

    use crate::util::{
        gen_btreemap, search_intersects_b_2d, search_intersects_btreemap, search_intersects_limit,
    };

    use super::{QuadNode, RectMask};
    // Helper that builds a grid and returns the two structures needed by the tests
    fn build_grid<'a>() -> (
        Vec<Vec<Option<RectMask>>>, // masks
        Vec<(usize, usize)>,        // xlist sorted by mask.x
    ) {
        // 3 × 3 grid with a few rectangles
        let mut rows: Vec<Vec<Option<RectMask>>> = Vec::new();

        rows.push(vec![
            Some(RectMask {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            }), // (0,0)
            Some(RectMask {
                x: 15.0,
                y: 5.0,
                w: 5.0,
                h: 5.0,
            }), // (0,1)
            None, // (0,2)
        ]);

        rows.push(vec![
            None, // (1,0)
            Some(RectMask {
                x: 5.0,
                y: 5.0,
                w: 10.0,
                h: 10.0,
            }), // (1,1)
            None, // (1,2)
        ]);

        rows.push(vec![
            Some(RectMask {
                x: 0.0,
                y: 15.0,
                w: 10.0,
                h: 10.0,
            }), // (2,0)
            None, // (2,1)
            Some(RectMask {
                x: 20.0,
                y: 20.0,
                w: 5.0,
                h: 5.0,
            }), // (2,2)
        ]);

        // Build the xlist – sorted by mask.x
        let mut xlist: Vec<(usize, usize)> = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            for (c, opt) in row.iter().enumerate() {
                if let Some(mask) = opt {
                    xlist.push((r, c));
                }
            }
        }
        // sort by mask.x
        xlist.sort_by_key(|&(r, c)| OrderedFloat::from(rows[r][c].as_ref().unwrap().x));

        (rows, xlist)
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_search_limit_basic() {
        let (masks_orig, xlist) = build_grid();
        let masks = masks_orig.iter().collect::<Vec<_>>();

        // 1. point inside (0,0) only
        let hits = search_intersects_limit(&masks, 2.0, 2.0, &vec![(0, 0), (1, 1)]);
        assert_eq!(hits, vec![(0, 0)]);

        // 2. point inside (1,1) and (0,0)
        let hits = search_intersects_limit(&masks, 6.0, 6.0, &vec![(1, 1), (0, 0)]);
        assert_eq!(hits, vec![(1, 1), (0, 0)]);

        // 3. point outside all masks
        let hits = search_intersects_limit(&masks, 120.0, 120.0, &vec![(0, 0), (1, 1), (2, 0)]);
        assert_eq!(hits, Vec::<(usize, usize)>::new());

        // 4. point on edges of (0,0) and (1.1) – should hit
        let hits = search_intersects_limit(&masks, 10.0, 5.0, &vec![(0, 0), (1, 1)]);
        assert_eq!(hits, vec![(0, 0), (1, 1)]);

        // 5. point on right edge of (1,1) – should hit
        let hits = search_intersects_limit(&masks, 6.0, 15.0, &vec![(1, 1)]);
        assert_eq!(hits, vec![(1, 1)]);
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_search_b_2d_basic() {
        let (masks_orig, xlist) = build_grid();
        let masks = masks_orig.iter().collect();

        // 1. point inside (0,0) only
        let hits = search_intersects_b_2d(&masks, &xlist, 2.0, 2.0, 20.0);
        assert_eq!(hits, vec![(0, 0)]);

        // 2. point inside (1,1) and (2,0)
        let hits = search_intersects_b_2d(&masks, &xlist, 6.0, 6.0, 20.0);
        assert_eq!(hits, vec![(1, 1), (0, 0)]);

        // 3. point inside (0,1) only
        let hits = search_intersects_b_2d(&masks, &xlist, 16.0, 7.0, 20.0);
        assert_eq!(hits, vec![(0, 1)]);

        // 4. point inside (2,2) only
        let hits = search_intersects_b_2d(&masks, &xlist, 21.0, 21.0, 20.0);
        assert_eq!(hits, vec![(2, 2)]);

        // 5. point outside all masks
        let hits = search_intersects_b_2d(&masks, &xlist, 120.0, 120.0, 20.0);
        assert_eq!(hits, Vec::<(usize, usize)>::new());

        // 6. point on edge of (0,0) (1,1) – should hit
        let hits = search_intersects_b_2d(&masks, &xlist, 5.0, 5.0, 20.0);
        assert_eq!(hits, vec![(1, 1), (0, 0)]);

        // 7. point on edge of (1,1) – should hit
        let hits = search_intersects_b_2d(&masks, &xlist, 6.0, 15.0, 20.0);
        assert_eq!(hits, vec![(1, 1), (2, 0)]);
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_search_b_2d_with_max_width() {
        let (masks_orig, xlist) = build_grid();
        let masks = masks_orig.iter().collect();

        // `max_width` small enough to stop before (1,1)
        let hits = search_intersects_b_2d(&masks, &xlist, 13.0, 7.0, 5.0);
        assert_eq!(hits, vec![]);

        // Same query with a large max_width – all possible masks are examined
        let hits = search_intersects_b_2d(&masks, &xlist, 13.0, 7.0, 20.0);
        assert_eq!(hits, vec![(1, 1)]);

        // Query point for both (0,0) (1,1)
        let hits = search_intersects_b_2d(&masks, &xlist, 5.0, 5.0, 5.0);
        assert_eq!(hits, vec![(1, 1), (0, 0)]);

        // Query point for (1,1) but width is too small to catch (0, 0)
        let hits = search_intersects_b_2d(&masks, &xlist, 5.0, 5.0, 2.0);
        assert_eq!(hits, vec![(1, 1)]);

        // 5.0 + 2.0 == 7.0 so we can catch (1, 1) barely
        let hits = search_intersects_b_2d(&masks, &xlist, 7.0, 7.0, 2.0);
        assert_eq!(hits, vec![(1, 1)]);

        // a bit further to right we can't catch (1, 1)
        let hits = search_intersects_b_2d(&masks, &xlist, 7.1, 7.0, 2.0);
        assert_eq!(hits, vec![]);

        // Using a very small max_width that excludes the far‑right mask (2,2)
        let hits = search_intersects_b_2d(&masks, &xlist, 25.0, 25.0, 3.0);
        assert_eq!(hits, vec![]);
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_search_b_2d_with_empty_xlist() {
        let (masks_orig, _) = build_grid();
        let masks = masks_orig.iter().collect();
        let xlist: Vec<(usize, usize)> = Vec::new(); // no masks at all

        let hits = search_intersects_b_2d(&masks, &xlist, 2.0, 2.0, 20.0);
        assert_eq!(hits, Vec::<(usize, usize)>::new());
    }

    // ------------------------------------------------------------------
    #[test]
    fn test_functions_agree() {
        let (masks_orig, xlist) = build_grid();
        let masks: Vec<&Vec<Option<RectMask>>> = masks_orig.iter().collect();

        // List of points to query
        let points: Vec<(f64, f64, Vec<(usize, usize)>)> = vec![
            (2.0, 2.0, vec![(0, 0)]),
            (6.0, 6.0, vec![(0, 0), (1, 1)]),
            (16.0, 7.0, vec![(0, 1)]),
            (15.0, 6.0, vec![(1, 1), (0, 1)]),
            (21.0, 21.0, vec![(2, 2)]),
            (20.0, 0.0, vec![]),
            (20.0, 1.0, vec![]),
            (0.0, 0.0, vec![(0, 0)]),
            (1200.0, 1200.0, Vec::new()),
        ];

        let btree = gen_btreemap(&masks);

        for (x, y, v) in points {
            let limit_hits = search_intersects_limit(
                &masks,
                x,
                y,
                // search all indices – the limit function does not need a
                // particular order
                &vec![(0, 0), (0, 1), (1, 1), (2, 0), (2, 2)],
            );
            let b2d_hits = search_intersects_b_2d(&masks, &xlist, x, y, 20.0);

            // The two results must contain exactly the same set of indices
            let h1 = limit_hits
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            let h2 = b2d_hits
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            let h3 = v.iter().copied().collect::<HashSet<_>>();
            let h4 = search_intersects_btreemap(&masks, &btree, (x, y), 10.0)
                .iter()
                .copied()
                .collect();
            assert_eq!(h1, h2);
            assert_eq!(h1, h3);
            assert_eq!(h1, h4, "{:?}", &(x, y));
        }
    }
}
