#[derive(Clone)]
struct NoObjectEnv;

use super::*;

#[tokio::test]
async fn error_unknown_function() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum Var {
        V,
    }

    impl DescribeVar for Var {
        type Env = NoObjectEnv;
        fn enumerate(
            _env: &NoObjectEnv,
        ) -> std::collections::HashMap<Self, collomatique_ilp::Variable> {
            HashMap::from([(Var::V, collomatique_ilp::Variable::binary())])
        }
        fn check_fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([("V".to_string(), vec![])])
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

    let modules = BTreeMap::from([("test", r#"pub let f() -> Constraint = $V() === 1;"#)]);
    let mut feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        feeder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        feeder.get_warnings()
    );

    let result = feeder.add_constraint("test", "nonexistent", vec![]);

    assert!(result.is_err());
    match result {
        Err(ScriptError::UnknownFunction(name)) => {
            assert_eq!(name, "test::nonexistent");
        }
        _ => panic!("Expected UnknownFunction error, got: {:?}", result),
    }
}

#[tokio::test]
async fn error_wrong_return_type_for_constraint() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum Var {
        V,
    }

    impl DescribeVar for Var {
        type Env = NoObjectEnv;
        fn enumerate(
            _env: &NoObjectEnv,
        ) -> std::collections::HashMap<Self, collomatique_ilp::Variable> {
            HashMap::from([(Var::V, collomatique_ilp::Variable::binary())])
        }
        fn check_fix(&self, _env: &NoObjectEnv) -> Option<f64> {
            None
        }
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<ExprType>> {
            HashMap::from([("V".to_string(), vec![])])
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

    let modules = BTreeMap::from([("bad_type", r#"pub let f() -> Bool = true;"#)]);
    let mut feeder = ScriptFeeder::<SqliteDatabaseDriver, Var, E, C>::new(&modules)
        .await
        .expect("Var should be compatible");

    assert!(
        feeder.get_warnings().is_empty(),
        "Unexpected warnings: {:?}",
        feeder.get_warnings()
    );

    let result = feeder.add_constraint("bad_type", "f", vec![]);

    assert!(result.is_err());
    match result {
        Err(ScriptError::WrongReturnType {
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
