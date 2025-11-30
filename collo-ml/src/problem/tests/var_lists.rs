use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn list_constraint_reification() {
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

    // Reify a list of constraints: V===1, W===1, X===1
    // Then enforce at least one must be true
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "list_reify".into(),
                content: r#"
                    let constraints() -> [Constraint] = [$V() === 1, $W() === 1, $X() === 1];
                    reify constraints as $[R];
                    pub let exactly_one() -> Constraint = sum r in $[R]() { r } === 1;
                "#
                .into(),
            },
            vec![("exactly_one".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // At least one of V, W, X should be 1
    let v_val = sol.get(ProblemVar::Base(Var::V)).unwrap_or(0.0);
    let w_val = sol.get(ProblemVar::Base(Var::W)).unwrap_or(0.0);
    let x_val = sol.get(ProblemVar::Base(Var::X)).unwrap_or(0.0);

    let count = (v_val >= 0.99) as i32 + (w_val >= 0.99) as i32 + (x_val >= 0.99) as i32;

    assert!(
        count == 1,
        "Exactly one of V, W, or X should be 1, got V={}, W={}, X={}",
        v_val,
        w_val,
        x_val
    );
}
