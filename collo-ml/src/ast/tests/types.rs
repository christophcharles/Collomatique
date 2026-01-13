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
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert!(output_type.node.types[0].node.inner.matches_str("Int"));
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
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert!(output_type.node.types[0].node.inner.matches_str("Bool"));
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
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert!(output_type.node.types[0].node.inner.matches_str("LinExpr"));
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
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert!(output_type.node.types[0]
                .node
                .inner
                .matches_str("Constraint"));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_type() {
    let input = r#"let f() -> String = "hello";"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert!(output_type.node.types[0].node.inner.matches_str("String"));
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
            assert_eq!(output_type.node.types.len(), 1);
            assert_eq!(output_type.node.types[0].node.maybe_count, 0);
            assert!(output_type.node.types[0].node.inner.matches_str("Student"));
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
            assert_eq!(output_type.node.types.len(), 1);
            let outer_type = &output_type.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(inner_typename) => {
                    assert_eq!(inner_typename.node.types.len(), 1);
                    assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                    assert!(inner_typename.node.types[0].node.inner.matches_str("Int"));
                }
                _ => panic!("Expected List type"),
            }
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
            assert_eq!(output_type.node.types.len(), 1);
            let outer_type = &output_type.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(inner_typename) => {
                    assert_eq!(inner_typename.node.types.len(), 1);
                    assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                    assert!(inner_typename.node.types[0].node.inner.matches_str("Bool"));
                }
                _ => panic!("Expected List type"),
            }
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
            assert_eq!(output_type.node.types.len(), 1);
            let outer_type = &output_type.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(inner_typename) => {
                    assert_eq!(inner_typename.node.types.len(), 1);
                    assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                    assert!(inner_typename.node.types[0]
                        .node
                        .inner
                        .matches_str("Student"));
                }
                _ => panic!("Expected List type"),
            }
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
            assert_eq!(params[0].typ.node.types.len(), 1);
            let outer_type = &params[0].typ.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(middle_typename) => {
                    assert_eq!(middle_typename.node.types.len(), 1);
                    match &middle_typename.node.types[0].node.inner {
                        SimpleTypeName::List(inner_typename) => {
                            assert_eq!(inner_typename.node.types.len(), 1);
                            assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                            assert!(inner_typename.node.types[0].node.inner.matches_str("Int"));
                        }
                        _ => panic!("Expected nested List type"),
                    }
                }
                _ => panic!("Expected List type"),
            }
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
            assert_eq!(params[0].typ.node.types.len(), 1);
            let outer_type = &params[0].typ.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(level1_typename) => {
                    assert_eq!(level1_typename.node.types.len(), 1);
                    match &level1_typename.node.types[0].node.inner {
                        SimpleTypeName::List(level2_typename) => {
                            assert_eq!(level2_typename.node.types.len(), 1);
                            match &level2_typename.node.types[0].node.inner {
                                SimpleTypeName::List(level3_typename) => {
                                    assert_eq!(level3_typename.node.types.len(), 1);
                                    assert_eq!(level3_typename.node.types[0].node.maybe_count, 0);
                                    assert!(level3_typename.node.types[0]
                                        .node
                                        .inner
                                        .matches_str("Bool"));
                                }
                                _ => panic!("Expected deeply nested List type"),
                            }
                        }
                        _ => panic!("Expected nested List type"),
                    }
                }
                _ => panic!("Expected List type"),
            }
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
            assert_eq!(output_type.node.types.len(), 1);
            let outer_type = &output_type.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(inner_typename) => {
                    assert_eq!(inner_typename.node.types.len(), 1);
                    assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                    assert!(inner_typename.node.types[0]
                        .node
                        .inner
                        .matches_str("LinExpr"));
                }
                _ => panic!("Expected List type"),
            }
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
            assert_eq!(output_type.node.types.len(), 1);
            let outer_type = &output_type.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(inner_typename) => {
                    assert_eq!(inner_typename.node.types.len(), 1);
                    assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                    assert!(inner_typename.node.types[0]
                        .node
                        .inner
                        .matches_str("Constraint"));
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_string() {
    let input = r#"let f() -> [String] = ["a", "b"];"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            let outer_type = &output_type.node.types[0].node;
            match &outer_type.inner {
                SimpleTypeName::List(inner_typename) => {
                    assert_eq!(inner_typename.node.types.len(), 1);
                    assert_eq!(inner_typename.node.types[0].node.maybe_count, 0);
                    assert!(inner_typename.node.types[0]
                        .node
                        .inner
                        .matches_str("String"));
                }
                _ => panic!("Expected List type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}
