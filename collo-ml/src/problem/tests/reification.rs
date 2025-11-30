use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn internal_reification() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
        X,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
            ])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
                (Var::X, collomatique_ilp::Variable::binary()),
            ])
        }
    }

    impl<T: EvalObject> TryFrom<&ExternVar<T>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<T>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "V" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "V".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::V)
                }
                "W" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "W".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::W)
                }
                "X" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "X".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::X)
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Test internal reification: exactly one of V, W, or X must be 1, and we force it to be W
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "reify_test".into(),
                content: r#"
                    let c1() -> Constraint = $V() === 1;
                    let c2() -> Constraint = $W() === 1;
                    let c3() -> Constraint = $X() === 1;
                    reify c1 as $R1;
                    reify c2 as $R2;
                    reify c3 as $R3;
                    pub let exactly_one_and_force_w() -> Constraint = 
                        $R1() + $R2() + $R3() === 1 and $R2() === 1;
                "#
                .into(),
            },
            vec![("exactly_one_and_force_w".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    for (c, _) in problem.get_inner_problem().get_constraints() {
        println!("{:?}", c);
    }

    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // R2 === 1 means W === 1 must hold
    // R1 + R2 + R3 === 1 with R2 === 1 means R1 === 0 and R3 === 0
    // Therefore V === 0 and X === 0
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(0.0),
        "V should be 0"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(1.0),
        "W should be 1"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::X)),
        Some(0.0),
        "X should be 0"
    );
}

#[test]
fn global_reified_variables_basic() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![]), ("W".to_string(), vec![])])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
            ])
        }
    }

    impl<T: EvalObject> TryFrom<&ExternVar<T>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<T>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "V" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "V".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::V)
                }
                "W" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "W".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::W)
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // First, define a reified variable globally
    let warnings = pb_builder
        .add_reified_variables(
            Script {
                name: "reified_defs".into(),
                content: r#"
                    pub let v_is_one() -> Constraint = $V() === 1;
                "#
                .into(),
            },
            vec![("v_is_one".to_string(), "VIsOne".to_string())],
        )
        .expect("Should compile reified variables");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Now use the reified variable in a constraint
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "use_reified".into(),
                content: r#"
                    pub let use_it() -> Constraint = $VIsOne() === 1 and $W() === 0;
                "#
                .into(),
            },
            vec![("use_it".to_string(), vec![])],
        )
        .expect("Should compile constraints");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // VIsOne === 1 means V === 1 must hold
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );
}

#[test]
fn global_reified_used_in_multiple_scripts() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
        X,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
            ])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
                (Var::X, collomatique_ilp::Variable::binary()),
            ])
        }
    }

    impl<T: EvalObject> TryFrom<&ExternVar<T>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<T>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "V" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "V".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::V)
                }
                "W" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "W".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::W)
                }
                "X" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "X".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::X)
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Define a global reified variable
    let warnings = pb_builder
        .add_reified_variables(
            Script {
                name: "reified_def".into(),
                content: r#"
                    pub let v_is_one() -> Constraint = $V() === 1;
                "#
                .into(),
            },
            vec![("v_is_one".to_string(), "VIsOne".to_string())],
        )
        .expect("Should compile reified variables");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Use the reified variable in first script
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "first_script".into(),
                content: r#"
                    pub let constraint1() -> Constraint = $VIsOne() === 1 and $W() === 0;
                "#
                .into(),
            },
            vec![("constraint1".to_string(), vec![])],
        )
        .expect("Should compile first script");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Use the same reified variable in a second script
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "second_script".into(),
                content: r#"
                    pub let constraint2() -> Constraint = $VIsOne() + $X() === 2;
                "#
                .into(),
            },
            vec![("constraint2".to_string(), vec![])],
        )
        .expect("Should compile second script");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // VIsOne === 1 (from first script) means V === 1
    // VIsOne + X === 2 (from second script) with VIsOne === 1 means X === 1
    // W === 0 from first script
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::X)),
        Some(1.0),
        "X should be 1"
    );
}
