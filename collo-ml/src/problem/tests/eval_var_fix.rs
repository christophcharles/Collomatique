use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn test_fix_forces_variable_values() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V(i32), // Parameter from 0 to 9
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![crate::traits::FieldType::Int])])
        }

        fn fix(&self) -> Option<f64> {
            match self {
                Var::V(i) => {
                    // Fix all variables to 0 except V(7)
                    if *i != 7 {
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
            // Only include variables that are not fixed
            // In this case, only V(7) is not fixed
            vars.insert(Var::V(7), collomatique_ilp::Variable::binary());
            Ok(vars)
        }
    }

    impl<T: EvalObject> TryFrom<&ExternVar<T>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<T>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "V" => {
                    if value.params.len() != 1 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "V".into(),
                            expected: 1,
                            found: value.params.len(),
                        });
                    }
                    let param = match &value.params[0] {
                        crate::eval::ExprValue::Int(i) => *i,
                        _ => {
                            return Err(VarConversionError::WrongParameterType {
                                name: "V".into(),
                                param: 0,
                                expected: crate::traits::FieldType::Int,
                            })
                        }
                    };
                    Ok(Var::V(param))
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    // Enforce exactly one V(i) must be 1
    // Since all are fixed to 0 except V(7), only V(7) can be 1
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "test_fix".into(),
                content: r#"
                    pub let exactly_one() -> Constraint = sum i in [0..10] { $V(i) } === 1;
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

    // V(7) should be 1, all others should be 0
    for i in 0..10 {
        let val = sol.get(ProblemVar::Base(Var::V(i))).unwrap_or(0.0);
        if i == 7 {
            assert_eq!(
                val, 1.0,
                "V(7) should be 1 (it's the only unfixed variable)"
            );
        } else {
            assert_eq!(val, 0.0, "V({}) should be 0 (fixed by fix())", i);
        }
    }
}
