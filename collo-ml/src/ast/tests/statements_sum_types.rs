use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// =============================================================================
// OPTION TYPE AST GENERATION - ?Type
// =============================================================================

#[test]
fn parse_let_with_option_primitive_types() {
    let input = "let f() -> ?Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::Int,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_option_custom_type() {
    let input = "let f() -> ?Student = get_student();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::Object("Student".into()),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_option_list_type() {
    let input = "let f() -> ?[Int] = [1, 2];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Int,
                            }]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_list_of_option_type() {
    let input = "let f() -> [?Int] = [1, none];";
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
                                maybe_count: 1,
                                inner: SimpleTypeName::Int,
                            }]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_option_parameter() {
    let input = "let f(x: ?Int) -> Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(
                params[0].typ.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::Int,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_nested_option_list() {
    let input = "let f() -> ?[?[Int]] = [[1]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 1,
                                inner: SimpleTypeName::List(TypeName {
                                    types: vec![MaybeTypeName {
                                        maybe_count: 0,
                                        inner: SimpleTypeName::Int,
                                    }]
                                }),
                            }]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// SUM TYPE AST GENERATION - Type1 | Type2
// =============================================================================

#[test]
fn parse_let_with_simple_sum_type() {
    let input = "let f() -> Int | Bool = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Bool,
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_multi_variant_sum_type() {
    let input = "let f() -> Int | Bool | LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Bool,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::LinExpr,
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_sum_type_including_none() {
    let input = "let f() -> None | Int = none;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::None,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_sum_of_custom_types() {
    let input = "let f() -> Student | Teacher = get_person();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Object("Student".into()),
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Object("Teacher".into()),
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_sum_type_parameter() {
    let input = "let f(x: Int | Bool) -> Int = 0;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(
                params[0].typ.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Bool,
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_list_of_sum_type() {
    let input = "let f() -> [Int | Bool] = [1, true];";
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
                            types: vec![
                                MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::Int,
                                },
                                MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::Bool,
                                }
                            ]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_sum_of_list_types() {
    let input = "let f() -> [Int] | [Bool] = [1];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::List(TypeName {
                                types: vec![MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::Int,
                                }]
                            }),
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::List(TypeName {
                                types: vec![MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::Bool,
                                }]
                            }),
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// COMBINING OPTION AND SUM TYPES
// =============================================================================

#[test]
fn parse_let_with_option_in_sum_type() {
    // Syntactically valid but semantically should be flattened to None | Int | Bool
    let input = "let f() -> ?Int | Bool = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 1, // ?Int
                            inner: SimpleTypeName::Int,
                        },
                        MaybeTypeName {
                            maybe_count: 0, // Bool
                            inner: SimpleTypeName::Bool,
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_option_of_list_of_sum() {
    let input = "let f() -> ?[Int | Bool] = [1, true];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![
                                MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::Int,
                                },
                                MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::Bool,
                                }
                            ]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// SYNTACTICALLY VALID BUT SEMANTICALLY INVALID
// =============================================================================

#[test]
fn parse_let_with_multiple_question_marks() {
    // ??Int should parse but be rejected in semantic analysis
    let input = "let f() -> ??Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 2, // Two question marks!
                        inner: SimpleTypeName::Int,
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_many_question_marks() {
    let input = "let f() -> ????Student = get();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 4, // Four question marks!
                        inner: SimpleTypeName::Object("Student".into()),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_duplicate_types_in_sum() {
    // Int | Int should parse but be deduplicated in semantic analysis
    let input = "let f() -> Int | Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        }
                    ]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// COMPLEX NESTED EXAMPLES
// =============================================================================

#[test]
fn parse_let_with_complex_nested_sum_and_option() {
    let input = "let f() -> [[Int | Bool] | [LinExpr]] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            // Outer list contains: [Int | Bool] or [LinExpr]
            assert_eq!(
                output_type.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![
                                // First variant: [Int | Bool]
                                MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::List(TypeName {
                                        types: vec![
                                            MaybeTypeName {
                                                maybe_count: 0,
                                                inner: SimpleTypeName::Int,
                                            },
                                            MaybeTypeName {
                                                maybe_count: 0,
                                                inner: SimpleTypeName::Bool,
                                            }
                                        ]
                                    }),
                                },
                                // Second variant: [LinExpr]
                                MaybeTypeName {
                                    maybe_count: 0,
                                    inner: SimpleTypeName::List(TypeName {
                                        types: vec![MaybeTypeName {
                                            maybe_count: 0,
                                            inner: SimpleTypeName::LinExpr,
                                        }]
                                    }),
                                }
                            ]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_multiple_option_and_sum_params() {
    let input = "let f(x: ?Int, y: Int | Bool, z: ?[Student]) -> Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(params.len(), 3);

            // x: ?Int
            assert_eq!(
                params[0].typ.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::Int,
                    }]
                }
            );

            // y: Int | Bool
            assert_eq!(
                params[1].typ.node,
                TypeName {
                    types: vec![
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Int,
                        },
                        MaybeTypeName {
                            maybe_count: 0,
                            inner: SimpleTypeName::Bool,
                        }
                    ]
                }
            );

            // z: ?[Student]
            assert_eq!(
                params[2].typ.node,
                TypeName {
                    types: vec![MaybeTypeName {
                        maybe_count: 1,
                        inner: SimpleTypeName::List(TypeName {
                            types: vec![MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Object("Student".into()),
                            }]
                        }),
                    }]
                }
            );
        }
        _ => panic!("Expected Let statement"),
    }
}
