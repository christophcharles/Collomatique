use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn error_unknown_function() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([(Var::V, collomatique_ilp::Variable::binary())])
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
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    let result = pb_builder.add_constraints(
        Script {
            name: "test".into(),
            content: r#"pub let f() -> Constraint = $V() === 1;"#.into(),
        },
        vec![("nonexistent".to_string(), vec![])],
    );

    assert!(result.is_err());
    match result {
        Err(ProblemError::UnknownFunction(name)) => {
            assert_eq!(name, "nonexistent");
        }
        _ => panic!("Expected UnknownFunction error, got: {:?}", result),
    }
}

#[test]
fn error_wrong_return_type_for_reified() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([(Var::V, collomatique_ilp::Variable::binary())])
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
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    let result = pb_builder.add_reified_variables(
        Script {
            name: "bad_type".into(),
            content: r#"pub let f() -> Int = 42;"#.into(),
        },
        vec![("f".to_string(), "BadVar".to_string())],
    );

    assert!(result.is_err());
    match result {
        Err(ProblemError::WrongReturnType {
            func,
            returned,
            expected,
        }) => {
            assert_eq!(func, "f");
            assert_eq!(returned, ExprType::Int);
            assert_eq!(expected, ExprType::Constraint);
        }
        _ => panic!("Expected WrongReturnType error, got: {:?}", result),
    }
}

#[test]
fn error_wrong_return_type_for_constraint() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([(Var::V, collomatique_ilp::Variable::binary())])
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
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    let result = pb_builder.add_constraints(
        Script {
            name: "bad_type".into(),
            content: r#"pub let f() -> Bool = true;"#.into(),
        },
        vec![("f".to_string(), vec![])],
    );

    assert!(result.is_err());
    match result {
        Err(ProblemError::WrongReturnType {
            func,
            returned,
            expected,
        }) => {
            assert_eq!(func, "f");
            assert_eq!(returned, ExprType::Bool);
            assert_eq!(expected, ExprType::Constraint);
        }
        _ => panic!("Expected WrongReturnType error, got: {:?}", result),
    }
}
