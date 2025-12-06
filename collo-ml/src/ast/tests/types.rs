use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Primitive Types =============

#[test]
fn parse_int_type() {
    let input = "let f() -> Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Int,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_bool_type() {
    let input = "let f() -> Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Bool,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_linexpr_type() {
    let input = "let f() -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::LinExpr,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_constraint_type() {
    let input = "let f() -> Constraint = $V(x) === 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Constraint,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_custom_object_type() {
    let input = "let f() -> Student = x;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Object("Student".into()),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= List Types =============

#[test]
fn parse_simple_list_type() {
    let input = "let f() -> [Int] = [1, 2, 3];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Int,
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_bool() {
    let input = "let f() -> [Bool] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Bool,
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_object() {
    let input = "let f() -> [Student] = @[Student];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Object("Student".into()),
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_list_type() {
    let input = "let f(x: [[Int]]) -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(
                params[0].typ.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::List(TypeName {
                                    types: vec![MaybeTypeName {
                                        maybe_count: 0,
                                        inner: SimpleTypeName::Int,
                                    }]
                                }),
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_deeply_nested_list_type() {
    let input = "let f(x: [[[Bool]]]) -> Int = 1;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(
                params[0].typ.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::List(TypeName {
                                    types: vec![MaybeTypeName {
                                        maybe_count: 0,
                                        inner: SimpleTypeName::List(TypeName {
                                            types: vec![MaybeTypeName {
                                                maybe_count: 0,
                                                inner: SimpleTypeName::Bool,
                                            }]
                                        }),
                                    }]
                                }),
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_linexpr() {
    let input = "let f() -> [LinExpr] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::LinExpr,
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_constraint() {
    let input = "let f() -> [Constraint] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Constraint,
                            }]
                        }),
                    }]
                },
                "Expected List type"
            );
        }
        _ => panic!("Expected Let statement"),
    }
}
