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

    impl<T: EvalObject> EvalVar<T> for Var {
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

#[test]
fn list_constraint_reification_exact_count_with_param() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        X(i32), // Parameter from 0 to 99
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("X".to_string(), vec![crate::traits::FieldType::Int])])
        }

        fn fix(&self) -> Option<f64> {
            match self {
                Var::X(i) => {
                    // Fix to 0 (false) if out of valid range [0, 100)
                    if *i < 0 || *i >= 100 {
                        Some(0.0)
                    } else {
                        None
                    }
                }
            }
        }

        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            let mut vars = BTreeMap::new();
            // Create a binary variable for each valid index
            for i in 0..100 {
                vars.insert(Var::X(i), collomatique_ilp::Variable::binary());
            }
            Ok(vars)
        }
    }

    impl<T: EvalObject> TryFrom<&ExternVar<T>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<T>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "X" => {
                    if value.params.len() != 1 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "X".into(),
                            expected: 1,
                            found: value.params.len(),
                        });
                    }
                    let param = match &value.params[0] {
                        crate::eval::ExprValue::Int(i) => *i,
                        _ => {
                            return Err(VarConversionError::WrongParameterType {
                                name: "X".into(),
                                param: 0,
                                expected: crate::traits::FieldType::Int,
                            })
                        }
                    };
                    Ok(Var::X(param))
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Reify a list of constraints: X(i) === 1 for i in 0..100
    // Then enforce exactly 5 must be true
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "exact_count".into(),
                content: r#"
                    let constraints() -> [Constraint] = [$X(i) === 1 for i in [0..100]];
                    reify constraints as $[R];
                    pub let exactly_five() -> Constraint = sum r in $[R]() { r } === 5;
                "#
                .into(),
            },
            vec![("exactly_five".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // Count how many X(i) are set to 1
    let mut count = 0;
    for i in 0..100 {
        if let Some(val) = sol.get(ProblemVar::Base(Var::X(i))) {
            if val >= 0.99 {
                count += 1;
            }
        }
    }

    assert_eq!(
        count, 5,
        "Exactly 5 variables should be set to 1, but got {}",
        count
    );
}
