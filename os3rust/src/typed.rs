//! The implementation of Yampa's primitive functions and Haskell's `Control::Arrow` definitions in
//! Rust.
//!
//! ## Basic Functions
//!
//! Arrow definitions:
//! - `arr`: lift a primitive function without any inner variables
//! - `first`: pass through second element, process first element
//! - `second`: pass through first element, process second element
//! - `***`: combine two parallel SF
//! - `&&&`: combine forked SF
//!
//! Category definitions:
//! - `id`: (from Category) identity morphism; if you already have `arr`, we can define it trivially
//!   with `id := arr (|x| {x})`
//! - `.`: (from Category) morphism composition; should have associative law
//!
//! Yampa primitive functions:
//! - `sscanPrim`: create a new SF from a primitive function; with an inner variable, and with an option
//!   whether or not to update the output and inner variable;
//! - `sscan`: create a new SF from a primitive function, using the last input as the inner
//!   variable.
//!
//! ## SF type specification:
//!
//! In this program, the type definition of SF is as follows:
//!
//! `(impl Fn(&A, &C) -> Option<(B, C)>, impl Fn(&A) -> B, C)`
//!
//! where `A` denotes the input type, `B` denotes the output type, and `C` denotes the inner state
//! type. Obviously, this is the same as the return type of `scan_option`, which is the most primitive
//! but sufficient way of describing a SF.
//!
//! The first tuple element specifies the calculation from the input and current inner state,
//! to the output and next inner state. 
//!
//! The second tuple element specifies the calculation of initial output from the initial input.
//! To support the lifting operation (i.e. `arr`), the initial output cannot be a constant data,
//! instead, it have to be dynamically generated from the initial input. It does not involve any
//! calculation about the inner state, thus it can be seen as the calculation at `frame 0`.
//!
//! The third tuple element specifies the type of whole inner variables under the calculation tree.
//!

// ======================================================================
//  START: all the basic functions to construct the Arrowized FRP system
// ======================================================================

/// from `sscanPrim` in Yampa, Haskell.
/// SF takes some input, then combines them with its own inner variable and updates both the inner
/// variable and the output. It also holds the initial inner variable and the initial output, to
/// make itself well-defined.
/// A = input type, B = output type, C = inner state type
pub fn scan_option<A, B: Clone, C, F: Fn(&A, &C) -> Option<(B, C)>>(
    the_func: &F,
    initial_output: B,
    initial_inner: C,
) -> (impl Fn(&A, &C) -> Option<(B, C)>, impl Fn(&A) -> B, C) {
    return (the_func, move |_| initial_output.clone(), initial_inner);
}

/// from `sscan` in Yampa, Haskell.
/// make a new SF from a function that takes the input and the last output.
pub fn scan<'a, A, B: Clone, F: Fn(&A, &B) -> B + 'a>(
    base_func: &F,
    first_output: B,
) -> (impl Fn(&A, &B) -> Option<(B, B)>, impl Fn(&A) -> B, B) {
    let cloned = first_output.clone();
    return (
        move |input, last_output| {
            Some((
                base_func(&input, &last_output),
                base_func(&input, &last_output),
            ))
        },
        move |_| first_output.clone(),
        cloned,
    );
}

/// from `arr` in Control::Arrow, Haskell.
/// lift some usual functions to SF.
pub fn lift_simple<'a, A, B, F: Fn(&A) -> B + 'a>(
    base_func: &F,
) -> (impl Fn(&A, &()) -> Option<(B, ())>, impl Fn(&A) -> B, ()) {
    return (|a, _| Some((base_func(a), ())), |x| base_func(x), ());
}

/// from `first` in Control::Arrow, Haskell.
/// pass through the second element as-is. process the first element with the given SF.
pub fn process_first<A, B, C, D: Clone, F: Fn(&A, &C) -> Option<(B, C)>, G: Fn(&A) -> B>(
    base_sf: (&F, &G, C),
) -> (
    impl Fn(&(A, D), &C) -> Option<((B, D), C)>,
    impl Fn(&(A, D)) -> (B, D),
    C,
) {
    let (base_func, init_out, init_state) = base_sf;
    return (
        move |(a, d), c| match base_func(&a, &c) {
            Some((b_new, c_new)) => Some(((b_new, d.clone()), c_new)),
            None => None,
        },
        move |(a, d)| (init_out(a), d.clone()),
        init_state,
    );
}

/// from `second` in Control::Arrow, Haskell.
/// pass through the first element as-is. process the second element with the given SF.
pub fn process_second<A, B, C, D: Clone, F: Fn(&A, &C) -> Option<(B, C)>, G: Fn(&A) -> B>(
    base_sf: (&F, &G, C),
) -> (
    impl Fn(&(D, A), &C) -> Option<((D, B), C)>,
    impl Fn(&(D, A)) -> (D, B),
    C,
) {
    let (base_func, init_out, init_state) = base_sf;
    return (
        move |(d, a), c| match base_func(&a, &c) {
            Some((b_new, c_new)) => Some(((d.clone(), b_new), c_new)),
            None => None,
        },
        move |(d, a)| (d.clone(), init_out(a)),
        init_state,
    );
}

/// from `***` in Control::Arrow, Haskell.
/// combine two SF in a parallel-shaped manner
pub fn combine_parallel<
    A,
    B: Clone,
    C: Clone,
    D,
    E: Clone,
    F: Clone,
    S: Fn(&A, &C) -> Option<(B, C)>,
    T: Fn(&D, &F) -> Option<(E, F)>,
    W: Fn(&A) -> B,
    X: Fn(&D) -> E,
>(
    base_sf_1: (&S, &W, C),
    base_sf_2: (&T, &X, F),
) -> (
    impl Fn(&(A, D), &(C, F, Option<B>, Option<E>)) -> Option<((B, E), (C, F, Option<B>, Option<E>))>,
    impl Fn(&(A, D)) -> (B, E),
    (C, F, Option<B>, Option<E>),
) {
    let (f_1, init_o_1, init_s_1) = base_sf_1;
    let (f_2, init_o_2, init_s_2) = base_sf_2;
    (
        move |(a, d), (c_pre, f_pre, last_1, last_2)| {
            let x = f_1(a, c_pre);
            let y = f_2(d, f_pre);
            if x.is_some() && y.is_some() {
                let (b, c) = x.unwrap();
                let (e, f) = y.unwrap();
                Some(((b.clone(), e.clone()), (c, f, Some(b), Some(e))))
            } else if x.is_none() && y.is_some() {
                let (e, f) = y.unwrap();
                let b = last_1.clone().unwrap();
                let c = c_pre;
                Some(((b.clone(), e.clone()), (c.clone(), f, Some(b), Some(e))))
            } else if x.is_some() && y.is_none() {
                let (b, c) = x.unwrap();
                let e = last_2.clone().unwrap();
                let f = f_pre;
                Some(((b.clone(), e.clone()), (c, f.clone(), Some(b), Some(e))))
            } else {
                None
            }
        },
        move |(a, d)| (init_o_1(a), init_o_2(d)),
        (init_s_1, init_s_2, None, None),
    )
}

/// from `&&&` in Control::Arrow, Haskell.
/// combine two SF in fork-shaped manner
/// TODO
pub fn combine_fork() {}

/// from `.` in Control::Arrow, Haskell.
/// combine two SF in sequence
/// TODO
pub fn combine_sequence() {}

// ======================================================================
//  END: all the basic functions to construct the Arrowized FRP system
// ======================================================================

// ==========================
//  START: utility functions
// ==========================
