#[derive(Clone)]
struct NoObjectEnv;
use collomatique_ilp::ObjectiveSense;

use super::*;

#[tokio::test]
async fn constraints_and_objectives_same_call() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum Var {
        V,
        W,
    }

    impl DescribeVar for Var {
        type Env = NoObjectEnv;
        fn enumerate(
            _env: &NoObjectEnv,
        ) -> std::collections::HashMap<Self, collomatique_ilp::Variable> {
            HashMap::from([
                (Var::V, collomatique_ilp::Variable::binary()),
                (Var::W, collomatique_ilp::Variable::binary()),
            ])
        }
        fn check_fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([("V".to_string(), vec![]), ("W".to_string(), vec![])])
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
        "combined",
        r#"
            pub let constraint() -> Constraint = $V() + $W() === 1;
            pub let objective() -> LinExpr = $V();
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

    // Add constraint from the combined module
    pb_builder
        .add_constraint("combined", "constraint", vec![])
        .expect("Should add constraint");

    // Add objective from the combined module
    pb_builder
        .add_objective(
            "combined",
            "objective",
            vec![],
            1.0,
            ObjectiveSense::Maximize,
        )
        .expect("Should add objective");

    let problem = pb_builder
        .build(&env, None)
        .await
        .expect("Build should succeed");

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // Constraint: V + W === 1
    // Objective: Maximize V
    // Should select V=1, W=0
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
}
