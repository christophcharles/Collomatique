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

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
            ])
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
                (Var::X, collomatique_ilp::Variable::binary()),
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
    let modules = BTreeMap::from([(
        "reify_test",
        r#"
            let c1() -> Constraint = $V() === 1;
            let c2() -> Constraint = $W() === 1;
            let c3() -> Constraint = $X() === 1;
            reify c1 as $R1;
            reify c2 as $R2;
            reify c3 as $R3;
            pub let exactly_one_and_force_w() -> Constraint =
                $R1() + $R2() + $R3() === 1 and $R2() === 1;
        "#,
    )]);
    let mut pb_builder = ProblemBuilder::<NoObject, Var>::new(&env, &modules)
        .expect("NoObject and Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Test internal reification: exactly one of V, W, or X must be 1, and we force it to be W
    pb_builder
        .add_constraint("reify_test", "exactly_one_and_force_w", vec![])
        .expect("Should add constraint");

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
fn private_reification_does_not_leak() {
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
    // Define both modules upfront
    // First module: private reification R means V === 1
    // Second module: private reification R means W === 1 (opposite constraint)
    let modules = BTreeMap::from([
        (
            "first_module",
            r#"
                let v_constraint() -> Constraint = $V() === 1;
                reify v_constraint as $R;
                pub let use_r() -> Constraint = $R() === 1;
            "#,
        ),
        (
            "second_module",
            r#"
                let w_constraint() -> Constraint = $W() === 1;
                reify w_constraint as $R;
                pub let use_r_again() -> Constraint = $R() === 0;
            "#,
        ),
    ]);
    let mut pb_builder = ProblemBuilder::<NoObject, Var>::new(&env, &modules)
        .expect("NoObject and Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Add constraint from first module
    pb_builder
        .add_constraint("first_module", "use_r", vec![])
        .expect("Should add constraint from first_module");

    // Add constraint from second module
    pb_builder
        .add_constraint("second_module", "use_r_again", vec![])
        .expect("Should add constraint from second_module");

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // First module: R === 1 means V === 1 must hold
    // Second module: R === 0 means W === 1 must NOT hold, so W === 0
    // If private reifications leaked, these would conflict
    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (from first module's private R)"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(0.0),
        "W should be 0 (from second module's private R)"
    );
}
