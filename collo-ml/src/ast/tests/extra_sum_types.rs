use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// =============================================================================
// OPTION TYPES - ?Type
// =============================================================================

#[test]
fn parse_option_int_type() {
    let input = "let f() -> ?Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_bool_type() {
    let input = "let f() -> ?Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_linexpr_type() {
    let input = "let f() -> ?LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("LinExpr".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_constraint_type() {
    let input = "let f() -> ?Constraint = $V() === 0;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Constraint".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_none_type() {
    let input = "let f() -> ?None = none;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("None".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_custom_object_type() {
    let input = "let f() -> ?Student = get();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Student".into())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// OPTION LIST TYPES
// =============================================================================

#[test]
fn parse_option_list_type() {
    let input = "let f() -> ?[Int] = [1];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);

            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(inner_type) => {
                    assert_eq!(inner_type.node.types.len(), 1);
                    assert_eq!(inner_type.node.types[0].node.maybe_count, 0);
                    assert_eq!(
                        inner_type.node.types[0].node.inner,
                        SimpleTypeName::Other("Int".to_string())
                    );
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_option_type() {
    let input = "let f() -> [?Int] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);

            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(inner_type) => {
                    assert_eq!(inner_type.node.types.len(), 1);
                    assert_eq!(inner_type.node.types[0].node.maybe_count, 1);
                    assert_eq!(
                        inner_type.node.types[0].node.inner,
                        SimpleTypeName::Other("Int".to_string())
                    );
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_nested_list_type() {
    let input = "let f() -> ?[[Int]] = [[1]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);

            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(outer_list) => {
                    assert_eq!(outer_list.node.types.len(), 1);
                    assert_eq!(outer_list.node.types[0].node.maybe_count, 0);

                    match &outer_list.node.types[0].node.inner {
                        SimpleTypeName::List(inner_list) => {
                            assert_eq!(inner_list.node.types.len(), 1);
                            assert_eq!(inner_list.node.types[0].node.maybe_count, 0);
                            assert_eq!(
                                inner_list.node.types[0].node.inner,
                                SimpleTypeName::Other("Int".to_string())
                            );
                        }
                        _ => panic!("Expected inner List type"),
                    }
                }
                _ => panic!("Expected outer List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// SUM TYPES - Type1 | Type2
// =============================================================================

#[test]
fn parse_sum_type_int_bool() {
    let input = "let f() -> Int | Bool = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 2);

            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );

            assert_eq!(output_type.node.types[1].node.maybe_count, 0);
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_type_three_variants() {
    let input = "let f() -> Int | Bool | LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 3);

            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
            assert_eq!(
                output_type.node.types[2].node.inner,
                SimpleTypeName::Other("LinExpr".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_type_with_none() {
    let input = "let f() -> None | Int | Bool = none;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 3);

            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("None".to_string())
            );
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );
            assert_eq!(
                output_type.node.types[2].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_type_custom_objects() {
    let input = "let f() -> Student | Teacher | Admin = get();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 3);

            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Student".into())
            );
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Teacher".into())
            );
            assert_eq!(
                output_type.node.types[2].node.inner,
                SimpleTypeName::Other("Admin".into())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// SUM TYPES WITH LISTS
// =============================================================================

#[test]
fn parse_list_of_sum_type() {
    let input = "let f() -> [Int | Bool] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);

            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(inner_type) => {
                    assert_eq!(inner_type.node.types.len(), 2);
                    assert_eq!(
                        inner_type.node.types[0].node.inner,
                        SimpleTypeName::Other("Int".to_string())
                    );
                    assert_eq!(
                        inner_type.node.types[1].node.inner,
                        SimpleTypeName::Other("Bool".to_string())
                    );
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_of_list_types() {
    let input = "let f() -> [Int] | [Bool] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 2);

            // First variant: [Int]
            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(inner_type) => {
                    assert_eq!(inner_type.node.types.len(), 1);
                    assert_eq!(
                        inner_type.node.types[0].node.inner,
                        SimpleTypeName::Other("Int".to_string())
                    );
                }
                _ => panic!("Expected List type"),
            }

            // Second variant: [Bool]
            match &output_type.node.types[1].node.inner {
                SimpleTypeName::List(inner_type) => {
                    assert_eq!(inner_type.node.types.len(), 1);
                    assert_eq!(
                        inner_type.node.types[0].node.inner,
                        SimpleTypeName::Other("Bool".to_string())
                    );
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_of_nested_list_types() {
    let input = "let f() -> [[Int]] | [[Bool]] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 2);

            // Both should be nested lists
            for variant in &output_type.node.types {
                match &variant.node.inner {
                    SimpleTypeName::List(_) => {
                        // OK
                    }
                    _ => panic!("Expected List type"),
                }
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// COMBINING OPTIONS AND SUMS
// =============================================================================

#[test]
fn parse_option_in_sum_type() {
    // ?Int | Bool is syntactically valid, semantically should flatten to None | Int | Bool
    let input = "let f() -> ?Int | Bool = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 2);

            // First variant: ?Int (maybe_count = 1)
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );

            // Second variant: Bool (maybe_count = 0)
            assert_eq!(output_type.node.types[1].node.maybe_count, 0);
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_multiple_options_in_sum_type() {
    let input = "let f() -> ?Int | ?Bool = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 2);

            assert_eq!(output_type.node.types[0].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );

            assert_eq!(output_type.node.types[1].node.maybe_count, 1);
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_option_of_list_of_sum() {
    let input = "let f() -> ?[Int | Bool] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1);

            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(inner_type) => {
                    assert_eq!(inner_type.node.types.len(), 2);
                    assert_eq!(
                        inner_type.node.types[0].node.inner,
                        SimpleTypeName::Other("Int".to_string())
                    );
                    assert_eq!(
                        inner_type.node.types[1].node.inner,
                        SimpleTypeName::Other("Bool".to_string())
                    );
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// MULTIPLE QUESTION MARKS (SEMANTICALLY INVALID)
// =============================================================================

#[test]
fn parse_double_question_mark() {
    let input = "let f() -> ??Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 2); // Should capture count
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_triple_question_mark() {
    let input = "let f() -> ???Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 3);
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Bool".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// DUPLICATE TYPES IN SUM (SEMANTICALLY INVALID)
// =============================================================================

#[test]
fn parse_duplicate_types_in_sum() {
    let input = "let f() -> Int | Int = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 2);
            // Both are Int - should be caught in semantic analysis
            assert_eq!(
                output_type.node.types[0].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );
            assert_eq!(
                output_type.node.types[1].node.inner,
                SimpleTypeName::Other("Int".to_string())
            );
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_triplicate_types_in_sum() {
    let input = "let f() -> Student | Student | Student = get();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 3);
            // All three are Student
            for variant in &output_type.node.types {
                assert_eq!(variant.node.inner, SimpleTypeName::Other("Student".into()));
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// COMPLEX NESTED STRUCTURES
// =============================================================================

#[test]
fn parse_deeply_nested_option_and_sum() {
    let input = "let f() -> ?[?[Int | Bool]] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 1); // Outer ?

            match &output_type.node.types[0].node.inner {
                SimpleTypeName::List(outer_list) => {
                    assert_eq!(outer_list.node.types.len(), 1);
                    assert_eq!(outer_list.node.types[0].node.maybe_count, 1); // Inner ?

                    match &outer_list.node.types[0].node.inner {
                        SimpleTypeName::List(inner_list) => {
                            assert_eq!(inner_list.node.types.len(), 2); // Int | Bool
                            assert_eq!(
                                inner_list.node.types[0].node.inner,
                                SimpleTypeName::Other("Int".to_string())
                            );
                            assert_eq!(
                                inner_list.node.types[1].node.inner,
                                SimpleTypeName::Other("Bool".to_string())
                            );
                        }
                        _ => panic!("Expected inner List type"),
                    }
                }
                _ => panic!("Expected outer List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}
