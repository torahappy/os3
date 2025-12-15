pub struct SF<A, B, C> {
    pub the_func: Box<dyn Fn(&C, &A) -> Option<(C, B)>>,
    pub initial_output: B,
    pub initial_inner: C,
}

pub fn scan<A: Clone + 'static, B: Clone + 'static> (f: &'static Box<dyn Fn(&B, &A) -> B>, b: &B) -> SF<A, B, B> {
    return SF { the_func: Box::new(|bb, aa|{
        Some((bb.clone(), f(bb, aa)))
    }), initial_output: b.clone(), initial_inner: b.clone() };
}

pub fn scan_option<A, B, C> (f: Box<dyn Fn(&C, &A) -> Option<(C, B)>>, c: C, b: B) -> SF<A, B, C> {
    return SF { the_func: f, initial_output: b, initial_inner: c };
}
