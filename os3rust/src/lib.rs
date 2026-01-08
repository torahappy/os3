use crate::untyped::{
    make_primitive_arrow_functions, make_primitive_arrow_functions_box, PrimitiveArrowFunctions,
    PrimitiveArrowFunctionsBox,
};

pub mod typed;
pub mod untyped;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn untyped_example() {
        // hold all the base functions for making arrows here
        let paf_box: PrimitiveArrowFunctionsBox<Vec<u64>> = make_primitive_arrow_functions_box(
            |x: &Vec<u64>| vec![x.get(0).unwrap().clone()],
            |x: &Vec<u64>| vec![x.get(1).unwrap().clone()],
            |x: &Vec<u64>, y: &Vec<u64>| vec![x.get(0).unwrap().clone(), y.get(0).unwrap().clone()],
            Vec::new(),
        );

        // create a reference for the above functions
        let paf: PrimitiveArrowFunctions<Vec<u64>> = make_primitive_arrow_functions(&paf_box);

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

    #[test]
    pub fn typed_add_2() {
        let scan_f = |x: &u64, y: &u64| x + y + 1;
        let (x1_1, x1_2, x1_3) = typed::scan(&scan_f, 90);
        let (x2_1, x2_2, x2_3) = typed::scan(&scan_f, 100);
        let (x3_1, x3_2, x3_3) =
            typed::combine_parallel((&x1_1, &x1_2, x1_3), (&x2_1, &x2_2, x2_3));
        let init_input = (5, 2);
        let init_output = x3_2(&init_input);
        assert!(init_output == (90, 100));
        let init_state = x3_3;
        assert!(x3_1(&init_input, &init_state) == Some(((96, 103), (96, 103, Some(96), Some(103)))));
    }
}
