struct NoObjectEnv;

use super::*;

#[tokio::test]
async fn single_constraint_problem() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl EvalVar for Var {
        type Env = NoObjectEnv;
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
        fn vars(
            _env: &NoObjectEnv,
        ) -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([(Var::V, collomatique_ilp::Variable::binary())])
        }
    }

    impl<D: DatabaseConnection> TryFrom<&ExternVar<D>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<D>) -> Result<Self, Self::Error> {
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
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let modules = BTreeMap::from([("main", "pub let f() -> Constraint = $V() === 1;")]);
    let mut pb_builder = ProblemBuilder::<SqliteDatabaseDriver, Var>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    pb_builder
        .add_constraint("main", "f", vec![])
        .expect("Should add constraint");

    let problem = pb_builder
        .build(&env, None)
        .await
        .expect("Build should succeed");

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "Wrong value for solution!"
    );
}

#[tokio::test]
async fn multiple_constraints_in_script() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
        X,
    }

    impl EvalVar for Var {
        type Env = NoObjectEnv;
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
            ])
        }
        fn fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
        fn vars(
            _env: &NoObjectEnv,
        ) -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
                (Var::X, collomatique_ilp::Variable::binary()),
            ])
        }
    }

    impl<D: DatabaseConnection> TryFrom<&ExternVar<D>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<D>) -> Result<Self, Self::Error> {
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
        "main",
        r#"
            pub let constraints() -> Constraint =
                $V() === 1 and $W() === 0 and $X() === 1;
        "#,
    )]);
    let mut pb_builder = ProblemBuilder::<SqliteDatabaseDriver, Var>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    pb_builder
        .add_constraint("main", "constraints", vec![])
        .expect("Should add constraint");

    let problem = pb_builder
        .build(&env, None)
        .await
        .expect("Build should succeed");

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

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

#[tokio::test]
async fn multiple_function_calls() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
    }

    impl EvalVar for Var {
        type Env = NoObjectEnv;
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([("V".to_string(), vec![]), ("W".to_string(), vec![])])
        }
        fn fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
        fn vars(
            _env: &NoObjectEnv,
        ) -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
            ])
        }
    }

    impl<D: DatabaseConnection> TryFrom<&ExternVar<D>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<D>) -> Result<Self, Self::Error> {
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
            pub let c1() -> Constraint = $V() === 1;
            pub let c2() -> Constraint = $W() === 1;
        "#,
    )]);
    let mut pb_builder = ProblemBuilder::<SqliteDatabaseDriver, Var>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Add two different constraints from the same module
    pb_builder
        .add_constraint("main", "c1", vec![])
        .expect("Should add constraint");
    pb_builder
        .add_constraint("main", "c2", vec![])
        .expect("Should add constraint");

    let problem = pb_builder
        .build(&env, None)
        .await
        .expect("Build should succeed");

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(1.0),
        "W should be 1"
    );
}

#[tokio::test]
async fn constraints_from_different_modules() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
        W,
    }

    impl EvalVar for Var {
        type Env = NoObjectEnv;
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([("V".to_string(), vec![]), ("W".to_string(), vec![])])
        }
        fn fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
        fn vars(
            _env: &NoObjectEnv,
        ) -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
            ])
        }
    }

    impl<D: DatabaseConnection> TryFrom<&ExternVar<D>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<D>) -> Result<Self, Self::Error> {
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
    let modules = BTreeMap::from([
        (
            "module1",
            r#"
                pub let c1() -> Constraint = $V() === 1;
            "#,
        ),
        (
            "module2",
            r#"
                pub let c2() -> Constraint = $W() === 1;
            "#,
        ),
    ]);
    let mut pb_builder = ProblemBuilder::<SqliteDatabaseDriver, Var>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Add constraint from first module
    pb_builder
        .add_constraint("module1", "c1", vec![])
        .expect("Should add constraint from module1");

    // Add constraint from second module
    pb_builder
        .add_constraint("module2", "c2", vec![])
        .expect("Should add constraint from module2");

    let problem = pb_builder
        .build(&env, None)
        .await
        .expect("Build should succeed");

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "V should be 1"
    );
    assert_eq!(
        sol.get(ProblemVar::Base(Var::W)),
        Some(1.0),
        "W should be 1"
    );
}
