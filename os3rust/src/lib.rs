/// The most primitive SF.
/// SF takes some input, then combines them with its own inner variable and updates both the inner
/// variable and the output. It also holds the initial inner variable and the initial output, to
/// make itself well-defined.
/// The lifetime `'a` specifies how long the rust functions in all the SFAtom live.
#[derive(Clone)]
pub struct SFAtom<'a, A: Clone> {
    /// (input, inner state) -> (output, next inner state)
    pub the_func: &'a dyn Fn(&A, &A) -> Option<(A, A)>,
    pub initial_output: A,
    pub initial_inner: A,
}

/// A (supposedly) pure calculation function which aggregates multiple inputs to a single output.
/// Essential for creating variables flow between SFAtom, SFComplex, and AggregateFunc;
/// The lifetime `'a` specifies how long the rust functions in all the SFAtom live.
#[derive(Clone)]
pub struct AggregateFunc<'a, A: Clone> {
    pub the_func: &'a dyn Fn(Vec<&A>) -> A,
}

/// The all types of SF and Aggragate Functions.
/// - Atom: the minimal unit of SF. Can be created from a pointer to a rust function.
/// - Complex: Combines Atom, Complex and AggregateFunc into some kind of graph structure, which
///   dictates the calculation procedure.
/// - Aggragate: A pure function to combine multiple data into one data.
/// The lifetime `'a` specifies how long the rust functions in all the SFAtom live.
#[derive(Clone)]
pub enum SFComplete<'a, A: Clone> {
    Atom(&'a SFAtom<'a, A>),
    Complex(&'a SFComplex<'a, A>),
    Aggragate(&'a AggregateFunc<'a, A>),
}

/// When making SFComplex, there are 2 types of variables that can be fed into SFAtom, AggregateFunc or SFComplex; the
/// outer input and the inner variables (= individual outputs). Obviously, there are only one input. Each inner variable is ony-by-one corresponding to the each SF or Aggragate Function within SFComplex.
/// The lifetime `'a` specifies how long the rust functions in all the SFAtom live.
#[derive(Clone)]
pub enum VariableIndex {
    InnerVariableIndex(usize),
    TheInput,
}

/// The output of each SFs will be stored to the place corresponding to the index of the SF in
/// `variables`; `input_configuration` configures which outputs should be fed into which SF or
/// Aggragate Function. `output_index` configures which output is the final output that will be
/// exposed to outer SF.
/// The lifetime `'a` specifies how long the rust functions in all the SFAtom live.
#[derive(Clone)]
pub struct SFComplex<'a, A: Clone> {
    pub variables: Vec<SFComplete<'a, A>>,
    /// For feeding to an Aggragate Function, multiple inputs are allowed; For SFComplex or SFAtom, only one
    /// input is allowed.
    pub input_configuration: Vec<(Vec<VariableIndex>, usize)>,
    pub output_index: usize,
}

/// The indices in `base_data` to store each inner state and (inertim or final) output.
/// It is made of a tree structure, and each SFComplex will make another level of the tree.
/// Otherwise, the tree will not be deepen anymore, and it just stores the output and the inner state.
#[derive(Clone)]
pub enum SFDataUnit {
    /// output, inner state
    ComplexData((usize, RelationTable)),
    /// output, inner state
    UnitData((usize, usize)),
}

/// Represents data indices of a SFComplex. The length of `variables` is the same as that of
/// `variables` in SFComplex, and each element is corresponding one-by-one with the same index.
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

/// A function to run SF in the "main loop"-like manner.
pub fn run_sf<'a, A: Clone>(
    sf: &SFComplex<'a, A>,
    relation_table: &RelationTable,
    root_input: A,
    base_data: &mut Vec<A>,
) -> A {
    sf.input_configuration
        .iter()
        .for_each(|(current_inputs, current_target)| {
            match relation_table.variables.get(*current_target).unwrap() {
                SFDataUnit::ComplexData((target_output_ref, target_inner_state)) => {
                    match sf.variables.get(*current_target).unwrap() {
                        SFComplete::Atom(_) => panic!("something wrong"),
                        SFComplete::Complex(sfcomplex) => {
                            let current_input_idx = current_inputs.get(0).unwrap();
                            match current_input_idx {
                                VariableIndex::InnerVariableIndex(other_var) => {
                                    let contents =
                                        relation_table.variables.get(*other_var).unwrap();
                                    let output_idx = get_out_idx(contents);
                                    let input_ = base_data.get(output_idx).unwrap().clone();
                                    let r =
                                        run_sf(&sfcomplex, target_inner_state, input_, base_data);
                                    let output_mut = base_data.get_mut(*target_output_ref).unwrap();
                                    *output_mut = r;
                                }
                                VariableIndex::TheInput => {
                                    let r = run_sf(
                                        &sfcomplex,
                                        target_inner_state,
                                        root_input.clone(),
                                        base_data,
                                    );

                                    let output_mut = base_data.get_mut(*target_output_ref).unwrap();
                                    *output_mut = r;
                                }
                            }
                        }
                        SFComplete::Aggragate(_) => panic!("something wrong"),
                    }
                }
                SFDataUnit::UnitData((target_output_ref, target_inner_state)) => {
                    match sf.variables.get(*current_target).unwrap() {
                        SFComplete::Atom(sfatom) => {
                            let target_inner_current = base_data.get(*target_inner_state).unwrap();
                            let input_idx = current_inputs.get(0).unwrap();
                            match input_idx {
                                VariableIndex::InnerVariableIndex(other_var) => {
                                    let contents =
                                        relation_table.variables.get(*other_var).unwrap();
                                    let var_base_data_idx = get_out_idx(contents);
                                    let input_data = base_data.get(var_base_data_idx).unwrap();
                                    let result =
                                        (sfatom.the_func)(input_data, &target_inner_current);
                                    match result {
                                        Some((output, next_inner)) => {
                                            let output_mut =
                                                base_data.get_mut(*target_output_ref).unwrap();
                                            *output_mut = output;

                                            let inner_mut =
                                                base_data.get_mut(*target_inner_state).unwrap();
                                            *inner_mut = next_inner;
                                        }
                                        None => {}
                                    }
                                }
                                VariableIndex::TheInput => {
                                    match (sfatom.the_func)(&root_input, &target_inner_current) {
                                        Some((output, next_inner)) => {
                                            let output_mut =
                                                base_data.get_mut(*target_output_ref).unwrap();
                                            *output_mut = output;

                                            let inner_mut =
                                                base_data.get_mut(*target_inner_state).unwrap();
                                            *inner_mut = next_inner;
                                        }
                                        None => {}
                                    }
                                }
                            }
                        }
                        SFComplete::Complex(_) => {
                            panic!("something wrong");
                        }
                        SFComplete::Aggragate(aggregate_func) => {
                            let arr: Vec<&A> = (*current_inputs)
                                .iter()
                                .map(|x| match x {
                                    VariableIndex::InnerVariableIndex(y) => base_data
                                        .get(get_out_idx(relation_table.variables.get(*y).unwrap()))
                                        .unwrap(),
                                    VariableIndex::TheInput => &root_input,
                                })
                                .collect::<Vec<&A>>();
                            let out = (aggregate_func.the_func)(arr);
                            let output_mut = base_data.get_mut(*target_output_ref).unwrap();
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
