use crate::data::*;
use rand::{
    distr::{Distribution, Uniform},
    rng,
};
// use rustybuzz::{UnicodeBuffer, shape};
use std::
    cmp::Ordering
;

/// Return all indices of the masks that intersect the point `(x, y)`.
///
/// `masks` – a 2‑D vector: `masks[row][col]` is a reference to an
/// optional rectangle mask.
///
/// The point is considered inside a mask when  
/// `x` is in `[mask.x, mask.x + mask.w]` **and**  
/// `y` is in `[mask.y, mask.y + mask.h]`.
///
pub fn search_intersects_2d(
    masks: &Vec<&Vec<Option<RectMask>>>,
    x: f64,
    y: f64,
) -> Vec<(usize, usize)> {
    let mut result = Vec::new();

    // iterate over rows and columns with their indices
    for (row_idx, row) in masks.iter().enumerate() {
        for (col_idx, mask_ref) in row.iter().enumerate() {
            // `mask_ref` has the type `&&Option<RectMask>`
            // we pattern‑match on a reference to the `Option`
            if let &Some(ref rect) = mask_ref {
                // `rect` is now a `&RectMask`
                if x >= rect.x && x <= rect.x + rect.w &&
                   y >= rect.y && y <= rect.y + rect.h {
                    result.push((row_idx, col_idx));
                }
            }
        }
    }

    result
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
        Ok(i) => i + 1,   // exact match → go to the right
        Err(i) => i,       // insertion point
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
            if x <= mask.x + mask.w && y >= mask.y && y <= mask.y + mask.h {
                res.push((r, c));
            }
        }
    }

    res
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
