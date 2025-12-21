pub mod typed;
pub mod untyped;

pub fn untyped_example() {
    let f = |x: &Vec<u64>, y: &Vec<u64>| return Some((vec![], vec![y.get(0).unwrap() + 1]));
    let a: untyped::SFAtom<Vec<u64>> = untyped::SFAtom {
        the_func: &Box::new(f),
        initial_output: vec![],
        initial_inner: vec![0],
    };
    let b: untyped::SFComplex<Vec<u64>> = untyped::SFComplex {
        variables: vec![untyped::SFComplete::Atom(&a), untyped::SFComplete::Atom(&a)],
        input_configuration: vec![(vec![untyped::VariableIndex::TheInput], 0)],
        output_index: 0,
    };
}

pub fn typed_example() {
    let scan_f = |x: &u64, y: &u64| { x + y + 1 };
    let (x1_1, x1_2, x1_3) = typed::scan(&scan_f, 90);
    let (x2_1, x2_2, x2_3) = typed::scan(&scan_f, 100);
    let (x3_1, x3_2, x3_3) = typed::combine_parallel(
        (&x1_1, &x1_2, x1_3), 
        (&x2_1, &x2_2, x2_3));
}
