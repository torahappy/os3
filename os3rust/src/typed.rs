/// SF takes some input, then combines them with its own inner variable and updates both the inner
/// variable and the output. It also holds the initial inner variable and the initial output, to
/// make itself well-defined.
/// The lifetime `'a` specifies how long the rust functions in all the SF live.
/// A = input type, B = output type, C = inner state type
pub fn scan_option<A: Clone, B: Clone, C: Clone, F: Fn(&A, &C) -> Option<(B, C)>>(
    the_func: F,
    initial_output: B,
    initial_inner: C,
) -> (impl Fn(&A, &C) -> Option<(B, C)>, B, C) {
    return (
        the_func,
        initial_output,
        initial_inner,
    );
}

pub fn scan<'a, A: Clone, B: Clone, C: Clone, F: Fn(&A, &B) -> B + 'a>(
    base_func: F,
    first_input: B,
) -> (impl Fn(&A, &B) -> Option<(B, B)>, B, B) {
    return (move |input, last_output|{
        Some((base_func(&input, &last_output), base_func(&input, &last_output)))
    }, first_input.clone(), first_input)
}
