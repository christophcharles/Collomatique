use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn error_unknown_function() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([(
                Var::V,
                collomatique_ilp::Variable::binary(),
            )]))
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

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([(
                Var::V,
                collomatique_ilp::Variable::binary(),
            )]))
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

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([(
                Var::V,
                collomatique_ilp::Variable::binary(),
            )]))
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

#[test]
fn error_variable_already_defined() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([(
                Var::V,
                collomatique_ilp::Variable::binary(),
            )]))
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

    // Try to reify with the same name as a base variable
    let result = pb_builder.add_reified_variables(
        Script {
            name: "conflict".into(),
            content: r#"pub let f() -> Constraint = 1 === 1;"#.into(),
        },
        vec![("f".to_string(), "V".to_string())],
    );

    assert!(result.is_err());
    match result {
        Err(ProblemError::VariableAlreadyDefined(name)) => {
            assert_eq!(name, "V");
        }
        _ => panic!("Expected VariableAlreadyDefined error, got: {:?}", result),
    }
}

#[test]
fn error_reified_variable_already_defined() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self, _env: &T::Env) -> Option<f64> {
            None
        }
        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            Ok(BTreeMap::from([(
                Var::V,
                collomatique_ilp::Variable::binary(),
            )]))
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

    // First, define a reified variable W
    let result = pb_builder.add_reified_variables(
        Script {
            name: "first_reified".into(),
            content: r#"pub let f() -> Constraint = $V() === 1;"#.into(),
        },
        vec![("f".to_string(), "W".to_string())],
    );
    assert!(result.is_ok(), "First reified variable should succeed");

    // Try to define another reified variable also named W
    let result = pb_builder.add_reified_variables(
        Script {
            name: "second_reified".into(),
            content: r#"pub let g() -> Constraint = $V() === 0;"#.into(),
        },
        vec![("g".to_string(), "W".to_string())],
    );

    assert!(result.is_err());
    match result {
        Err(ProblemError::VariableAlreadyDefined(name)) => {
            assert_eq!(name, "W");
        }
        _ => panic!("Expected VariableAlreadyDefined error, got: {:?}", result),
    }
}
