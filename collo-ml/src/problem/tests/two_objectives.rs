use crate::eval::{NoObject, NoObjectEnv};
use collomatique_ilp::ObjectiveSense;

use super::*;

#[test]
fn two_objectives_same_script() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
        X,
        Y,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
                ("Y".to_string(), vec![]),
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
                (Var::Y, collomatique_ilp::Variable::binary()),
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
                "Y" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "Y".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::Y)
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Two independent constraints:
    // V + W === 1 (solutions: V=1,W=0 or V=0,W=1)
    // X + Y === 1 (solutions: X=1,Y=0 or X=0,Y=1)
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "constraints".into(),
                content: r#"
                    pub let c1() -> Constraint = $V() + $W() === 1;
                    pub let c2() -> Constraint = $X() + $Y() === 1;
                "#
                .into(),
            },
            vec![("c1".to_string(), vec![]), ("c2".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Two objectives from the same script:
    // - Maximize V (coefficient 1.0) -> should select V=1, W=0
    // - Minimize X (coefficient 1.0) -> should select X=0, Y=1
    let warnings = pb_builder
        .add_to_objective(
            Script {
                name: "objectives".into(),
                content: r#"
                    pub let obj_v() -> LinExpr = $V();
                    pub let obj_x() -> LinExpr = $X();
                "#
                .into(),
            },
            vec![
                ("obj_v".to_string(), vec![], 1.0, ObjectiveSense::Maximize),
                ("obj_x".to_string(), vec![], 1.0, ObjectiveSense::Minimize),
            ],
        )
        .expect("Should compile objectives");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // First objective maximizes V -> V=1, W=0
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (maximized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );

    // Second objective minimizes X -> X=0, Y=1
    assert_eq!(
        sol.get(ProblemVar::Base(Var::X)),
        Some(0.0),
        "X should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::Y)),
        Some(1.0),
        "Y should be 1"
    );
}

#[test]
fn two_objectives_different_scripts() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
        X,
        Y,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
                ("Y".to_string(), vec![]),
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
                (Var::Y, collomatique_ilp::Variable::binary()),
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
                "Y" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "Y".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::Y)
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Two independent constraints:
    // V + W === 1 (solutions: V=1,W=0 or V=0,W=1)
    // X + Y === 1 (solutions: X=1,Y=0 or X=0,Y=1)
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "constraints".into(),
                content: r#"
                    pub let c1() -> Constraint = $V() + $W() === 1;
                    pub let c2() -> Constraint = $X() + $Y() === 1;
                "#
                .into(),
            },
            vec![("c1".to_string(), vec![]), ("c2".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // First objective from first script: Maximize V -> should select V=1, W=0
    let warnings = pb_builder
        .add_to_objective(
            Script {
                name: "objective1".into(),
                content: r#"
                    pub let obj_v() -> LinExpr = $V();
                "#
                .into(),
            },
            vec![("obj_v".to_string(), vec![], 1.0, ObjectiveSense::Maximize)],
        )
        .expect("Should compile first objective");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Second objective from different script: Minimize X -> should select X=0, Y=1
    let warnings = pb_builder
        .add_to_objective(
            Script {
                name: "objective2".into(),
                content: r#"
                    pub let obj_x() -> LinExpr = $X();
                "#
                .into(),
            },
            vec![("obj_x".to_string(), vec![], 1.0, ObjectiveSense::Minimize)],
        )
        .expect("Should compile second objective");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // First objective maximizes V -> V=1, W=0
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (maximized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );

    // Second objective minimizes X -> X=0, Y=1
    assert_eq!(
        sol.get(ProblemVar::Base(Var::X)),
        Some(0.0),
        "X should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::Y)),
        Some(1.0),
        "Y should be 1"
    );
}

#[test]
fn objectives_with_different_senses() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
        X,
        Y,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
                ("Y".to_string(), vec![]),
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
                (Var::Y, collomatique_ilp::Variable::binary()),
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
                "Y" => {
                    if value.params.len() != 0 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "Y".into(),
                            expected: 0,
                            found: value.params.len(),
                        });
                    }
                    Ok(Var::Y)
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Two independent constraints:
    // V + W === 1 (solutions: V=1,W=0 or V=0,W=1)
    // X + Y === 1 (solutions: X=1,Y=0 or X=0,Y=1)
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "constraints".into(),
                content: r#"
                    pub let c1() -> Constraint = $V() + $W() === 1;
                    pub let c2() -> Constraint = $X() + $Y() === 1;
                "#
                .into(),
            },
            vec![("c1".to_string(), vec![]), ("c2".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Two objectives with different senses:
    // - Maximize V (coefficient 1.0)
    // - Minimize X (coefficient 1.0)
    // Combined: Maximize (V - X)
    //
    // With V+W=1 and X+Y=1, we have 4 solutions:
    // (V=1,W=0,X=1,Y=0): objective = 1 - 1 = 0
    // (V=1,W=0,X=0,Y=1): objective = 1 - 0 = 1  <- best
    // (V=0,W=1,X=1,Y=0): objective = 0 - 1 = -1
    // (V=0,W=1,X=0,Y=1): objective = 0 - 0 = 0
    let warnings = pb_builder
        .add_to_objective(
            Script {
                name: "objectives".into(),
                content: r#"
                    pub let obj_v() -> LinExpr = $V();
                    pub let obj_x() -> LinExpr = $X();
                "#
                .into(),
            },
            vec![
                ("obj_v".to_string(), vec![], 1.0, ObjectiveSense::Maximize),
                ("obj_x".to_string(), vec![], 1.0, ObjectiveSense::Minimize),
            ],
        )
        .expect("Should compile objectives");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // The combined objective Maximize(V - X) is maximized when V=1, X=0
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (maximized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::X)),
        Some(0.0),
        "X should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::Y)),
        Some(1.0),
        "Y should be 1"
    );
}
