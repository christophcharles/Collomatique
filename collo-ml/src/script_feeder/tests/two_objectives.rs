#[derive(Clone)]
struct NoObjectEnv;
use collomatique_ilp::ObjectiveSense;

use super::*;

#[tokio::test]
async fn two_objectives_same_script() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum Var {
        V,
        W,
        X,
        Y,
    }

    impl DescribeVar for Var {
        type Env = NoObjectEnv;
        fn enumerate(
            _env: &NoObjectEnv,
        ) -> std::collections::HashMap<Self, collomatique_ilp::Variable> {
            HashMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
                (Var::X, collomatique_ilp::Variable::binary()),
                (Var::Y, collomatique_ilp::Variable::binary()),
            ])
        }
        fn check_fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
                ("Y".to_string(), vec![]),
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
    let modules = BTreeMap::from([(
        "main",
        r#"
            pub let c1() -> Constraint = $V() + $W() === 1;
            pub let c2() -> Constraint = $X() + $Y() === 1;
            pub let obj_v() -> LinExpr = $V();
            pub let obj_x() -> LinExpr = $X();
        "#,
    )]);
    let mut feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        feeder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        feeder.get_warnings()
    );

    feeder
        .add_constraint("main", "c1", vec![])
        .expect("Should add constraint");
    feeder
        .add_constraint("main", "c2", vec![])
        .expect("Should add constraint");

    feeder
        .add_objective("main", "obj_v", vec![], 1.0, ObjectiveSense::Maximize)
        .expect("Should add objective");
    feeder
        .add_objective("main", "obj_x", vec![], 1.0, ObjectiveSense::Minimize)
        .expect("Should add objective");

    let model = build_model(feeder, &env).await;

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(model.problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(InternalVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (maximized)"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );

    assert_eq!(
        sol.get(InternalVar::Base(Var::X)),
        Some(0.0),
        "X should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::Y)),
        Some(1.0),
        "Y should be 1"
    );
}

#[tokio::test]
async fn two_objectives_different_scripts() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum Var {
        V,
        W,
        X,
        Y,
    }

    impl DescribeVar for Var {
        type Env = NoObjectEnv;
        fn enumerate(
            _env: &NoObjectEnv,
        ) -> std::collections::HashMap<Self, collomatique_ilp::Variable> {
            HashMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
                (Var::X, collomatique_ilp::Variable::binary()),
                (Var::Y, collomatique_ilp::Variable::binary()),
            ])
        }
        fn check_fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
                ("Y".to_string(), vec![]),
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
    let modules = BTreeMap::from([
        (
            "constraints",
            r#"
                pub let c1() -> Constraint = $V() + $W() === 1;
                pub let c2() -> Constraint = $X() + $Y() === 1;
            "#,
        ),
        (
            "objective1",
            r#"
                pub let obj_v() -> LinExpr = $V();
            "#,
        ),
        (
            "objective2",
            r#"
                pub let obj_x() -> LinExpr = $X();
            "#,
        ),
    ]);
    let mut feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        feeder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        feeder.get_warnings()
    );

    feeder
        .add_constraint("constraints", "c1", vec![])
        .expect("Should add constraint");
    feeder
        .add_constraint("constraints", "c2", vec![])
        .expect("Should add constraint");

    feeder
        .add_objective("objective1", "obj_v", vec![], 1.0, ObjectiveSense::Maximize)
        .expect("Should add first objective");

    feeder
        .add_objective("objective2", "obj_x", vec![], 1.0, ObjectiveSense::Minimize)
        .expect("Should add second objective");

    let model = build_model(feeder, &env).await;

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(model.problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(InternalVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (maximized)"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );

    assert_eq!(
        sol.get(InternalVar::Base(Var::X)),
        Some(0.0),
        "X should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::Y)),
        Some(1.0),
        "Y should be 1"
    );
}

#[tokio::test]
async fn objectives_with_different_senses() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum Var {
        V,
        W,
        X,
        Y,
    }

    impl DescribeVar for Var {
        type Env = NoObjectEnv;
        fn enumerate(
            _env: &NoObjectEnv,
        ) -> std::collections::HashMap<Self, collomatique_ilp::Variable> {
            HashMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
                (Var::X, collomatique_ilp::Variable::binary()),
                (Var::Y, collomatique_ilp::Variable::binary()),
            ])
        }
        fn check_fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([
                ("V".to_string(), vec![]),
                ("W".to_string(), vec![]),
                ("X".to_string(), vec![]),
                ("Y".to_string(), vec![]),
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
    let modules = BTreeMap::from([(
        "main",
        r#"
            pub let c1() -> Constraint = $V() + $W() === 1;
            pub let c2() -> Constraint = $X() + $Y() === 1;
            pub let obj_v() -> LinExpr = $V();
            pub let obj_x() -> LinExpr = $X();
        "#,
    )]);
    let mut feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        feeder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        feeder.get_warnings()
    );

    feeder
        .add_constraint("main", "c1", vec![])
        .expect("Should add constraint");
    feeder
        .add_constraint("main", "c2", vec![])
        .expect("Should add constraint");

    feeder
        .add_objective("main", "obj_v", vec![], 1.0, ObjectiveSense::Maximize)
        .expect("Should add objective");
    feeder
        .add_objective("main", "obj_x", vec![], 1.0, ObjectiveSense::Minimize)
        .expect("Should add objective");

    let model = build_model(feeder, &env).await;

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(model.problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(InternalVar::Base(Var::V)),
        Some(1.0),
        "V should be 1 (maximized)"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::W)),
        Some(0.0),
        "W should be 0"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::X)),
        Some(0.0),
        "X should be 0 (minimized)"
    );
    assert_eq!(
        sol.get(InternalVar::Base(Var::Y)),
        Some(1.0),
        "Y should be 1"
    );
}
