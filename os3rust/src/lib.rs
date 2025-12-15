/// The most primitive SF.
/// SF takes some input, the combines them with its own inner variable and updates both the inner
/// variable and the output. It also holds the initial inner variable and the initial output, to
/// make itself well-defined.
#[derive(Clone)]
pub struct SFAtom<'a, A: Clone> {
    /// (input, inner state) -> (output, next inner state)
    pub the_func: &'a dyn Fn(&A, &A) -> Option<(A, A)>,
    pub initial_output: A,
    pub initial_inner: A,
}

/// A (supposedly) pure calculation function which aggregates multiple inputs to a single output.
/// Essential for creating variables flow between SFAtom, SFComplex, and AggregateFunc;
#[derive(Clone)]
pub struct AggregateFunc<'a, A: Clone> {
    pub the_func: &'a dyn Fn(&Vec<A>) -> A,
}

/// The all types of SF and Aggragate Functions.
#[derive(Clone)]
pub enum SFComplete<'a, A: Clone> {
    Atom(&'a SFAtom<'a, A>),
    Complex(&'a SFComplex<'a, A>),
    Aggragate(&'a AggregateFunc<'a, A>),
}

/// When making SFComplex, there are 2 types of variables that can be fed into SFAtom, AggregateFunc or SFComplex; the
/// outer input and the inner variables. Obviously, there are only one input. Each inner variable is ony-by-one corresponding to the each SF or Aggragate Function within SFComplex.
#[derive(Clone)]
pub enum VariableIndex {
    InnerVariableIndex(usize),
    TheInput,
}

/// The output of each SFs will be stored to the place corresponding to the index of the SF in
/// `variables`; `input_configuration` configures which outputs should be fed into which SF or
/// Aggragate Function. `output_index` configures which output is the final output that will be
/// exposed to outer SF.
#[derive(Clone)]
pub struct SFComplex<'a, A: Clone> {
    pub variables: Vec<SFComplete<'a, A>>,
    /// For feeding to an Aggragate Function, multiple inputs are allowed; For SFComplex or SFAtom, only one
    /// input is allowed.
    pub input_configuration: Vec<(Vec<VariableIndex>, usize)>,
    pub output_index: usize,
}

#[derive(Clone)]
pub enum SFDataUnit {
    /// output, inner state
    ComplexData((usize, RelationTable)),
    /// output, inner state
    UnitData((usize, usize)),
}

#[derive(Clone)]
pub struct RelationTable {
    pub variables: Vec<SFDataUnit>,
}

pub fn make_relation_table<'a, A: Clone>(
    sf: &SFComplex<'a, A>,
    base_data: &mut Vec<A>,
) -> RelationTable {
    RelationTable { variables: vec![] }
}

pub fn get_out_idx(x: &SFDataUnit) -> usize {
    match x {
        SFDataUnit::ComplexData((a, _)) => *a,
        SFDataUnit::UnitData((a, _)) => *a,
    }
}

pub fn run_sf<'a, A: Clone>(
    sf: &SFComplex<'a, A>,
    relation_table: &RelationTable,
    input: A,
    base_data: &mut Vec<A>,
) -> A {
    sf.input_configuration
        .iter()
        .for_each(|(current_inputs, current_input_target)| {
            match relation_table.variables.get(*current_input_target).unwrap() {
                SFDataUnit::ComplexData((output_, inner_state)) => {
                    match sf.variables.get(*current_input_target).unwrap() {
                        SFComplete::Atom(sfatom) => panic!("something wrong"),
                        SFComplete::Complex(sfcomplex) => {
                            let input_idx = current_inputs.get(0).unwrap();
                            match input_idx {
                                VariableIndex::InnerVariableIndex(other_var) => {
                                    let contents =
                                        relation_table.variables.get(*other_var).unwrap();
                                    let output_idx = get_out_idx(contents);
                                    let input_ = base_data.get(output_idx).unwrap().clone();
                                    let r =
                                        run_sf(&sfcomplex, inner_state, input_.clone(), base_data);
                                    let output_mut = base_data.get_mut(*output_).unwrap();
                                    *output_mut = r;
                                }
                                VariableIndex::TheInput => {
                                    let r =
                                        run_sf(&sfcomplex, inner_state, input.clone(), base_data);

                                    let output_mut = base_data.get_mut(*output_).unwrap();
                                    *output_mut = r;
                                }
                            }
                        }
                        SFComplete::Aggragate(aggregate_func) => panic!("something wrong"),
                    }
                    //sf.variables.get(complex_idx)
                    //let output_mut = base_data.get_mut(*output_).unwrap();
                }
                SFDataUnit::UnitData((output_, inner_state)) => {
                    let inner_non_mut = base_data.get(*inner_state).unwrap().clone();
                    match sf.variables.get(*current_input_target).unwrap() {
                        SFComplete::Atom(sfatom) => {
                            let input_idx = current_inputs.get(0).unwrap();
                            match input_idx {
                                VariableIndex::InnerVariableIndex(other_var) => {
                                    let contents =
                                        relation_table.variables.get(*other_var).unwrap();
                                    let output_idx = get_out_idx(contents);
                                    let input_ = base_data.get(output_idx).unwrap().clone();
                                    let result = (sfatom.the_func)(&input_, &inner_non_mut).clone();
                                    match result {
                                        Some((output, next_inner)) => {
                                            let output_mut = base_data.get_mut(*output_).unwrap();
                                            *output_mut = output;

                                            let inner_mut =
                                                base_data.get_mut(*inner_state).unwrap();
                                            *inner_mut = next_inner;
                                        }
                                        None => {}
                                    }
                                }
                                VariableIndex::TheInput => {
                                    match (sfatom.the_func)(&input, &inner_non_mut) {
                                        Some((output, next_inner)) => {
                                            let output_mut = base_data.get_mut(*output_).unwrap();
                                            *output_mut = output;

                                            let inner_mut =
                                                base_data.get_mut(*inner_state).unwrap();
                                            *inner_mut = next_inner;
                                        }
                                        None => {}
                                    }
                                }
                            }
                        }
                        SFComplete::Complex(sfcomplex) => {
                            panic!("something wrong");
                        }
                        SFComplete::Aggragate(aggregate_func) => {
                            let arr: Vec<A> = (*current_inputs)
                                .iter()
                                .map(|x| match x {
                                    VariableIndex::InnerVariableIndex(y) => base_data
                                        .get(get_out_idx(relation_table.variables.get(*y).unwrap()))
                                        .unwrap()
                                        .clone(),
                                    VariableIndex::TheInput => input.clone(),
                                })
                                .collect::<Vec<A>>();
                            let out = (aggregate_func.the_func)(&arr);
                            let output_mut = base_data.get_mut(*output_).unwrap();
                            *output_mut = out;
                        }
                    }
                }
            }
        });

    let final_i = get_out_idx(relation_table.variables.get(sf.output_index).unwrap());

    return base_data.get(final_i).unwrap().clone();
}

pub fn example() {
    let f = |x: &Vec<u64>, y: &Vec<u64>| return Some((vec![], vec![y.get(0).unwrap() + 1]));
    let a: SFAtom<Vec<u64>> = SFAtom {
        the_func: &Box::new(f),
        initial_output: vec![],
        initial_inner: vec![0],
    };
    let b: SFComplex<Vec<u64>> = SFComplex {
        variables: vec![SFComplete::Atom(&a), SFComplete::Atom(&a)],
        input_configuration: vec![(vec![VariableIndex::TheInput], 0)],
        output_index: 0,
    };
}
