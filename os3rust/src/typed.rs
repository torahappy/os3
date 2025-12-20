/// SF takes some input, then combines them with its own inner variable and updates both the inner
/// variable and the output. It also holds the initial inner variable and the initial output, to
/// make itself well-defined.
/// The lifetime `'a` specifies how long the rust functions in all the SF live.
/// A = input type, B = output type, C = inner state type
pub fn scan_option<A, B: Clone, C, F: Fn(&A, &C) -> Option<(B, C)>>(
    the_func: &F,
    initial_output: B,
    initial_inner: C,
) -> (impl Fn(&A, &C) -> Option<(B, C)>, impl Fn(&A) -> B, C) {
    return (the_func, move |_| initial_output.clone(), initial_inner);
}

/// make a new SF from a function that takes the input and the last output.
pub fn scan<'a, A, B: Clone, C, F: Fn(&A, &B) -> B + 'a>(
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

/// lift some usual functions to SF.
pub fn lift_simple<'a, A, B, F: Fn(&A) -> B + 'a>(
    base_func: &F,
) -> (impl Fn(&A, &()) -> Option<(B, ())>, impl Fn(&A) -> B, ()) {
    return (move |a, _| Some((base_func(a), ())), |x| base_func(x), ());
}

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

/// pass through the first element as-is. process the second element with the given SF.
pub fn process_second<A, B, C, D: Clone, F: Fn(&A, &C) -> Option<(B, C)>, G: Fn(&A) -> B>(
    base_sf: (&F, G, C),
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

///
pub fn combine_parallel<
    A,
    B,
    C,
    D,
    E,
    F,
    S: Fn(&A, &C) -> Option<(B, C)>,
    T: Fn(&D, &F) -> Option<(E, F)>,
>(
    base_sf_1: (impl Fn(&A, &C) -> Option<(B, C)>, impl Fn(&A) -> B, C),
    base_sf_2: (impl Fn(&D, &F) -> Option<(E, F)>, impl Fn(&D) -> E, F),
) -> (
    impl Fn(&(A, D), &(C, F)) -> Option<((B, E), (C, F))>,
    impl Fn(&(A, D)) -> (B, E),
    (C, F),
) {
    let (f_1, init_o_1, init_s_1) = base_sf_1;
    let (f_2, init_o_2, init_s_2) = base_sf_2;
    (
        move |(a, d), (c, f)| {
            let x = f_1(a, c);
            let y = f_2(d, f);
            if x.is_some() && y.is_some() {
                let (b, c) = x.unwrap();
                let (e, f) = y.unwrap();
                Some(((b, e), (c, f)))
            } else if x.is_none() && y.is_some() {
                todo!()
            } else if x.is_some() && y.is_none() {
                todo!()
            } else {
                None
            }
        },
        move |(a, d)| (init_o_1(a), init_o_2(d)),
        (init_s_1, init_s_2),
    )
}
