use crate::eval::NoObject;

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

    let modules = BTreeMap::from([("test", r#"pub let f() -> Constraint = $V() === 1;"#)]);
    let mut pb_builder = ProblemBuilder::<NoObject, Var>::new(&modules)
        .expect("NoObject and Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Try to call a function that doesn't exist in the module
    let result = pb_builder.add_constraint("test", "nonexistent", vec![]);

    assert!(result.is_err());
    match result {
        Err(ProblemError::UnknownFunction(name)) => {
            assert_eq!(name, "test::nonexistent");
        }
        _ => panic!("Expected UnknownFunction error, got: {:?}", result),
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

    let modules = BTreeMap::from([("bad_type", r#"pub let f() -> Bool = true;"#)]);
    let mut pb_builder = ProblemBuilder::<NoObject, Var>::new(&modules)
        .expect("NoObject and Var should be compatible");

    assert!(
        pb_builder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        pb_builder.get_warnings()
    );

    // Try to use a function that returns Bool instead of Constraint
    let result = pb_builder.add_constraint("bad_type", "f", vec![]);

    assert!(result.is_err());
    match result {
        Err(ProblemError::WrongReturnType {
            func,
            returned,
            expected,
        }) => {
            assert_eq!(func, "bad_type::f");
            assert_eq!(returned, SimpleType::Bool.into());
            assert_eq!(
                expected,
                ExprType::from_variants([
                    SimpleType::Constraint,
                    SimpleType::List(SimpleType::Constraint.into())
                ])
            );
        }
        _ => panic!("Expected WrongReturnType error, got: {:?}", result),
    }
}
