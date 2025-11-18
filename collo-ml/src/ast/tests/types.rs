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
            assert!(matches!(output_type.node, TypeName::Int));
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
            assert!(matches!(output_type.node, TypeName::Bool));
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
            assert!(matches!(output_type.node, TypeName::LinExpr));
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
            assert!(matches!(output_type.node, TypeName::Constraint));
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
            assert!(matches!(output_type.node, TypeName::Object(ref s) if s == "Student"));
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
        Statement::Let { output_type, .. } => match &output_type.node {
            TypeName::List(inner) => {
                assert!(matches!(**inner, TypeName::Int));
            }
            _ => panic!("Expected List type"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_bool() {
    let input = "let f() -> [Bool] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => match &output_type.node {
            TypeName::List(inner) => {
                assert!(matches!(**inner, TypeName::Bool));
            }
            _ => panic!("Expected List type"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_object() {
    let input = "let f() -> [Student] = @[Student];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => match &output_type.node {
            TypeName::List(inner) => {
                assert!(matches!(**inner, TypeName::Object(ref s) if s == "Student"));
            }
            _ => panic!("Expected List type"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_list_type() {
    let input = "let f(x: [[Int]]) -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => match &params[0].typ.node {
            TypeName::List(inner1) => match &**inner1 {
                TypeName::List(inner2) => {
                    assert!(matches!(**inner2, TypeName::Int));
                }
                _ => panic!("Expected nested list"),
            },
            _ => panic!("Expected list type"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_deeply_nested_list_type() {
    let input = "let f(x: [[[Bool]]]) -> Int = 1;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => match &params[0].typ.node {
            TypeName::List(inner1) => match &**inner1 {
                TypeName::List(inner2) => match &**inner2 {
                    TypeName::List(inner3) => {
                        assert!(matches!(**inner3, TypeName::Bool));
                    }
                    _ => panic!("Expected third level list"),
                },
                _ => panic!("Expected second level list"),
            },
            _ => panic!("Expected first level list"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_linexpr() {
    let input = "let f() -> [LinExpr] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => match &output_type.node {
            TypeName::List(inner) => {
                assert!(matches!(**inner, TypeName::LinExpr));
            }
            _ => panic!("Expected List type"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_of_constraint() {
    let input = "let f() -> [Constraint] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => match &output_type.node {
            TypeName::List(inner) => {
                assert!(matches!(**inner, TypeName::Constraint));
            }
            _ => panic!("Expected List type"),
        },
        _ => panic!("Expected Let statement"),
    }
}
