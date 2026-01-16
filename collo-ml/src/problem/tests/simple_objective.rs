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
    let modules = BTreeMap::from([(
        "main",
        r#"
            pub let exactly_one() -> Constraint = $V() + $W() === 1;
            pub let maximize_v() -> LinExpr = $V();
        "#,
    )]);
    let mut pb_builder = ProblemBuilder::<NoObject, Var>::new(&modules)
        .expect("NoObject and Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Constraint: V + W === 1 (exactly one must be 1)
    // This has two valid solutions: (V=1, W=0) or (V=0, W=1)
    pb_builder
        .add_constraint("main", "exactly_one", vec![])
        .expect("Should add constraint");

    // Objective: maximize V
    // This should select the solution V=1, W=0
    pb_builder
        .add_objective("main", "maximize_v", vec![], 1.0, ObjectiveSense::Maximize)
        .expect("Should add objective");

    let problem = pb_builder.build(&env).expect("Build should succeed");

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
    let modules = BTreeMap::from([(
        "main",
        r#"
            pub let exactly_one() -> Constraint = $V() + $W() === 1;
            pub let minimize_v() -> LinExpr = $V();
        "#,
    )]);
    let mut pb_builder = ProblemBuilder::<NoObject, Var>::new(&modules)
        .expect("NoObject and Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Same constraint as before: V + W === 1 (exactly one must be 1)
    // This has two valid solutions: (V=1, W=0) or (V=0, W=1)
    pb_builder
        .add_constraint("main", "exactly_one", vec![])
        .expect("Should add constraint");

    // Objective: MINIMIZE V (opposite of the previous test)
    // This should select the solution V=0, W=1
    pb_builder
        .add_objective("main", "minimize_v", vec![], 1.0, ObjectiveSense::Minimize)
        .expect("Should add objective");

    let problem = pb_builder.build(&env).expect("Build should succeed");

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
