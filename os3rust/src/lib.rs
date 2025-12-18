pub mod typed;
pub mod untyped;

pub fn typed_example() {
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
