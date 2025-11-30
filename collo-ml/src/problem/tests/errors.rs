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
