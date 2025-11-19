use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Literal Expressions =============

#[test]
fn parse_number_literal() {
    let input = "let f() -> Int = 42;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Number(42)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_negative_number() {
    let input = "let f() -> Int = -10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Number(-10)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_zero() {
    let input = "let f() -> Int = 0;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Number(0)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_boolean_true() {
    let input = "let f() -> Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Boolean(Boolean::True)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_boolean_false() {
    let input = "let f() -> Bool = false;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Boolean(Boolean::False)));
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= Path Expressions =============

#[test]
fn parse_simple_path() {
    let input = "let f() -> Int = x;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Ident(name) => {
                assert_eq!(name.node, "x");
            }
            _ => panic!("Expected Ident"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_path_with_field_access() {
    let input = "let f() -> Int = student.age;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Path { object, segments } => {
                match &object.node {
                    Expr::Ident(ident) => {
                        assert_eq!(ident.node, "student");
                    }
                    _ => panic!("Expected Ident"),
                }
                assert_eq!(segments.len(), 1);
                assert_eq!(segments[0].node, "age");
            }
            _ => panic!("Expected Path"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_deep_path() {
    let input = "let f() -> Int = a.b.c.d;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Path { object, segments } => {
                match &object.node {
                    Expr::Ident(ident) => {
                        assert_eq!(ident.node, "a");
                    }
                    _ => panic!("Expected Ident"),
                }
                assert_eq!(segments.len(), 3);
                assert_eq!(segments[0].node, "b");
                assert_eq!(segments[1].node, "c");
                assert_eq!(segments[2].node, "d");
            }
            _ => panic!("Expected Path"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Function Calls =============

#[test]
fn parse_function_call_no_args() {
    let input = "let f() -> Int = foo();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::FnCall { name, args } => {
                assert_eq!(name.node, "foo");
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected FnCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_function_call_one_arg() {
    let input = "let f() -> Int = foo(42);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::FnCall { name, args } => {
                assert_eq!(name.node, "foo");
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0].node, Expr::Number(42)));
            }
            _ => panic!("Expected FnCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_function_call_multiple_args() {
    let input = "let f() -> Int = foo(1, 2, 3);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::FnCall { name, args } => {
                assert_eq!(name.node, "foo");
                assert_eq!(args.len(), 3);
                assert!(matches!(args[0].node, Expr::Number(1)));
                assert!(matches!(args[1].node, Expr::Number(2)));
                assert!(matches!(args[2].node, Expr::Number(3)));
            }
            _ => panic!("Expected FnCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_function_call_with_complex_args() {
    let input = "let f() -> Int = foo(x.y, bar(), 42);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::FnCall { name, args } => {
                assert_eq!(name.node, "foo");
                assert_eq!(args.len(), 3);
                assert!(matches!(
                    args[0].node,
                    Expr::Path {
                        object: _,
                        segments: _
                    }
                ));
                assert!(matches!(args[1].node, Expr::FnCall { .. }));
                assert!(matches!(args[2].node, Expr::Number(42)));
            }
            _ => panic!("Expected FnCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Variable Calls =============

#[test]
fn parse_var_call_no_args() {
    let input = "let f() -> LinExpr = $V();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::VarCall { name, args } => {
                assert_eq!(name.node, "V");
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected VarCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_var_call_one_arg() {
    let input = "let f() -> LinExpr = $V(x);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::VarCall { name, args } => {
                assert_eq!(name.node, "V");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected VarCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_var_call_multiple_args() {
    let input = "let f() -> LinExpr = $MyVar(s, w, d);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::VarCall { name, args } => {
                assert_eq!(name.node, "MyVar");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected VarCall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= If Expressions =============

#[test]
fn parse_if_expression() {
    let input = "let f() -> Int = if x > 5 { 10 } else { 20 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                assert!(matches!(condition.node, Expr::Gt(_, _)));
                assert!(matches!(then_expr.node, Expr::Number(10)));
                assert!(matches!(else_expr.node, Expr::Number(20)));
            }
            _ => panic!("Expected If expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_if_expression() {
    let input = "let f() -> Int = if x > 5 { if y < 3 { 1 } else { 2 } } else { 3 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::If {
                condition: _,
                then_expr,
                else_expr,
            } => {
                assert!(matches!(then_expr.node, Expr::If { .. }));
                assert!(matches!(else_expr.node, Expr::Number(3)));
            }
            _ => panic!("Expected If expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_if_with_boolean_condition() {
    let input = "let f() -> Int = if true { 1 } else { 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::If { condition, .. } => {
                assert!(matches!(condition.node, Expr::Boolean(Boolean::True)));
            }
            _ => panic!("Expected If expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Parenthesized Expressions =============

#[test]
fn parse_parenthesized_expression() {
    let input = "let f() -> Int = (42);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Number(42)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_parentheses() {
    let input = "let f() -> Int = (((5)));";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Number(5)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_parenthesized_complex_expression() {
    let input = "let f() -> Int = (x + y) * z;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Mul(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= Explicit Type Annotations =============

#[test]
fn parse_explicit_type_annotation() {
    let input = "let f() -> LinExpr = x as LinExpr;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ExplicitType { expr, typ } => {
                assert!(matches!(expr.node, Expr::Ident(_)));
                assert!(matches!(typ.node, TypeName::LinExpr));
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_explicit_type_with_number() {
    let input = "let f() -> Int = 5 as Int;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ExplicitType { expr, typ } => {
                assert!(matches!(expr.node, Expr::Number(5)));
                assert!(matches!(typ.node, TypeName::Int));
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_explicit_type_with_list() {
    let input = "let f() -> [Int] = x as [Int];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ExplicitType { typ, .. } => match &typ.node {
                TypeName::List(inner) => {
                    assert!(matches!(**inner, TypeName::Int));
                }
                _ => panic!("Expected List type"),
            },
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}
