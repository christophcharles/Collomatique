use crate::eval::{NoObject, NoObjectEnv};
use collomatique_ilp::ObjectiveSense;

use super::*;

#[test]
fn simple_objective_selects_solution() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![]), ("W".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
            ]))
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

    // Constraint: V + W === 1 (exactly one must be 1)
    // This has two valid solutions: (V=1, W=0) or (V=0, W=1)
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "constraints".into(),
                content: r#"
                    pub let exactly_one() -> Constraint = $V() + $W() === 1;
                "#
                .into(),
            },
            vec![("exactly_one".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Objective: maximize V
    // This should select the solution V=1, W=0
    let warnings = pb_builder
        .add_to_objective(
            Script {
                name: "objective".into(),
                content: r#"
                    pub let maximize_v() -> LinExpr = $V();
                "#
                .into(),
            },
            vec![(
                "maximize_v".to_string(),
                vec![],
                1.0,
                ObjectiveSense::Maximize,
            )],
        )
        .expect("Should compile objective");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // The objective should select V=1, W=0
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
}

#[test]
fn objective_direction_changes_solution() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![]), ("W".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
            ]))
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

    // Same constraint as before: V + W === 1 (exactly one must be 1)
    // This has two valid solutions: (V=1, W=0) or (V=0, W=1)
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "constraints".into(),
                content: r#"
                    pub let exactly_one() -> Constraint = $V() + $W() === 1;
                "#
                .into(),
            },
            vec![("exactly_one".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Objective: MINIMIZE V (opposite of the previous test)
    // This should select the solution V=0, W=1
    let warnings = pb_builder
        .add_to_objective(
            Script {
                name: "objective".into(),
                content: r#"
                    pub let minimize_v() -> LinExpr = $V();
                "#
                .into(),
            },
            vec![(
                "minimize_v".to_string(),
                vec![],
                1.0,
                ObjectiveSense::Minimize,
            )],
        )
        .expect("Should compile objective");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // The objective should select V=0, W=1 (opposite of maximize test)
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(0.0),
        "V should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(1.0),
        "W should be 1"
    );
}
