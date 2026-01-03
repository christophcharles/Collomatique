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
            assert!(matches!(body.node, Expr::Boolean(true)));
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
            assert!(matches!(body.node, Expr::Boolean(false)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_literal_basic() {
    let input = r#"let f() -> String = "hello";"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StringLiteral(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected StringLiteral, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_literal_empty() {
    let input = r#"let f() -> String = "";"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StringLiteral(s) => assert_eq!(s, ""),
            _ => panic!("Expected StringLiteral, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_literal_with_spaces() {
    let input = r#"let f() -> String = "hello world";"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StringLiteral(s) => assert_eq!(s, "hello world"),
            _ => panic!("Expected StringLiteral, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_literal_with_tildes() {
    let input = r#"let f() -> String = ~"He said "hello""~;"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StringLiteral(s) => assert_eq!(s, r#"He said "hello""#),
            _ => panic!("Expected StringLiteral, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_literal_with_newline() {
    let input = "let f() -> String = \"line1\nline2\";";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StringLiteral(s) => assert_eq!(s, "line1\nline2"),
            _ => panic!("Expected StringLiteral, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_string_literal_with_unicode() {
    let input = r#"let f() -> String = "Hello 世界";"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StringLiteral(s) => assert_eq!(s, "Hello 世界"),
            _ => panic!("Expected StringLiteral, got {:?}", body.node),
        },
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
                assert!(matches!(condition.node, Expr::Boolean(true)));
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
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(
                    typ.node.types[0].node,
                    MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::LinExpr,
                    }
                );
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
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(
                    typ.node.types[0].node,
                    MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Int,
                    }
                );
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
            Expr::ExplicitType { typ, .. } => {
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(typ.node.types[0].node.maybe_count, 0);
                match &typ.node.types[0].node.inner {
                    SimpleTypeName::List(typ_name) => {
                        assert_eq!(typ_name.node.types.len(), 1);
                        assert_eq!(
                            typ_name.node.types[0].node,
                            MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Int,
                            }
                        );
                    }
                    _ => panic!("Expected List type"),
                }
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_explicit_type_for_empty_typed_list() {
    let input = "let f() -> [Int] = [<Int>];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ExplicitType { typ, .. } => {
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(typ.node.types[0].node.maybe_count, 0);
                match &typ.node.types[0].node.inner {
                    SimpleTypeName::List(typ_name) => {
                        assert_eq!(typ_name.node.types.len(), 1);
                        assert_eq!(
                            typ_name.node.types[0].node,
                            MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Int,
                            }
                        );
                    }
                    _ => panic!("Expected List type"),
                }
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Type Conversions =============

#[test]
fn parse_type_conversion_annotation() {
    let input = "let f() -> LinExpr = x into LinExpr;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::TypeConversion { expr, typ } => {
                assert!(matches!(expr.node, Expr::Ident(_)));
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(
                    typ.node.types[0].node,
                    MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::LinExpr,
                    }
                );
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_type_conversion_with_number() {
    let input = "let f() -> Int = 5 into Int;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::TypeConversion { expr, typ } => {
                assert!(matches!(expr.node, Expr::Number(5)));
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(
                    typ.node.types[0].node,
                    MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Int,
                    }
                );
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_type_conversion_with_list() {
    let input = "let f() -> [Int] = x into [Int];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::TypeConversion { typ, .. } => {
                assert_eq!(typ.node.types.len(), 1);
                assert_eq!(typ.node.types[0].node.maybe_count, 0);
                match &typ.node.types[0].node.inner {
                    SimpleTypeName::List(typ_name) => {
                        assert_eq!(typ_name.node.types.len(), 1);
                        assert_eq!(
                            typ_name.node.types[0].node,
                            MaybeTypeName {
                                maybe_count: 0,
                                inner: SimpleTypeName::Int,
                            }
                        );
                    }
                    _ => panic!("Expected List type"),
                }
            }
            _ => panic!("Expected ExplicitType"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Let Expressions =============

#[test]
fn parse_simple_let_in_expression() {
    let input = "let f() -> Int = let x = 5 { x + 1 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "x");
                assert!(matches!(value.node, Expr::Number(5)));
                assert!(matches!(body.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_variable_value() {
    let input = "let f(n: Int) -> Int = let doubled = n * 2 { doubled };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "doubled");
                assert!(matches!(value.node, Expr::Mul(_, _)));
                match &body.node {
                    Expr::Ident(name) => assert_eq!(name.node, "doubled"),
                    _ => panic!("Expected Ident in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_let_in_expressions() {
    let input = "let f() -> Int = let x = 1 { let y = 2 { x + y } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                // Outer let
                assert_eq!(var.node, "x");
                assert!(matches!(value.node, Expr::Number(1)));

                // Inner let
                match &body.node {
                    Expr::Let { var, value, body } => {
                        assert_eq!(var.node, "y");
                        assert!(matches!(value.node, Expr::Number(2)));
                        assert!(matches!(body.node, Expr::Add(_, _)));
                    }
                    _ => panic!("Expected nested LetIn expression"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_if_body() {
    let input = "let f(x: Int) -> Int = let bound = 10 { if x > bound { 1 } else { 0 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "bound");
                assert!(matches!(value.node, Expr::Number(10)));
                match &body.node {
                    Expr::If {
                        condition,
                        then_expr,
                        else_expr,
                    } => {
                        assert!(matches!(condition.node, Expr::Gt(_, _)));
                        assert!(matches!(then_expr.node, Expr::Number(1)));
                        assert!(matches!(else_expr.node, Expr::Number(0)));
                    }
                    _ => panic!("Expected If expression in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_forall_body() {
    let input = "let f(n: Int) -> Constraint = let bound = n * 2 { forall i in [0..bound] { $V(i) === 1 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "bound");
                assert!(matches!(value.node, Expr::Mul(_, _)));
                match &body.node {
                    Expr::Forall {
                        var,
                        collection,
                        filter,
                        body: forall_body,
                    } => {
                        assert_eq!(var.node, "i");
                        assert!(matches!(collection.node, Expr::ListRange { .. }));
                        assert!(filter.is_none());
                        assert!(matches!(forall_body.node, Expr::ConstraintEq(_, _)));
                    }
                    _ => panic!("Expected Forall expression in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_membership_test() {
    let input = "let f(x: Int, list: [Int]) -> Bool = let is_member = x in list { is_member };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "is_member");
                match &value.node {
                    Expr::In { item, collection } => {
                        match &item.node {
                            Expr::Ident(name) => assert_eq!(name.node, "x"),
                            _ => panic!("Expected Ident for item"),
                        }
                        match &collection.node {
                            Expr::Ident(name) => assert_eq!(name.node, "list"),
                            _ => panic!("Expected Ident for collection"),
                        }
                    }
                    _ => panic!("Expected In expression"),
                }
                match &body.node {
                    Expr::Ident(name) => assert_eq!(name.node, "is_member"),
                    _ => panic!("Expected Ident in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_function_call_value() {
    let input = "let f(x: Int) -> Int = let result = helper(x) { result + 1 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "result");
                match &value.node {
                    Expr::FnCall { name, args } => {
                        assert_eq!(name.node, "helper");
                        assert_eq!(args.len(), 1);
                    }
                    _ => panic!("Expected FnCall"),
                }
                assert!(matches!(body.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_list_literal() {
    let input = "let f() -> [Int] = let items = [1, 2, 3] { items };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "items");
                match &value.node {
                    Expr::ListLiteral { elements } => {
                        assert_eq!(elements.len(), 3);
                    }
                    _ => panic!("Expected ListLiteral"),
                }
                match &body.node {
                    Expr::Ident(name) => assert_eq!(name.node, "items"),
                    _ => panic!("Expected Ident in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_list_range() {
    let input = "let f(n: Int) -> [Int] = let range = [0..n] { range };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "range");
                assert!(matches!(value.node, Expr::ListRange { .. }));
                match &body.node {
                    Expr::Ident(name) => assert_eq!(name.node, "range"),
                    _ => panic!("Expected Ident in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_arithmetic_value() {
    let input =
        "let f(a: Int, b: Int) -> Int = let sum_ = a + b { let prod = a * b { sum_ + prod } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "sum_");
                assert!(matches!(value.node, Expr::Add(_, _)));

                match &body.node {
                    Expr::Let { var, value, body } => {
                        assert_eq!(var.node, "prod");
                        assert!(matches!(value.node, Expr::Mul(_, _)));
                        assert!(matches!(body.node, Expr::Add(_, _)));
                    }
                    _ => panic!("Expected nested LetIn"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_constraint_value() {
    let input = "let f(x: Int) -> Constraint = let c = $V(x) === 1 { c };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "c");
                assert!(matches!(value.node, Expr::ConstraintEq(_, _)));
                match &body.node {
                    Expr::Ident(name) => assert_eq!(name.node, "c"),
                    _ => panic!("Expected Ident in body"),
                }
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_in_with_path_value() {
    let input = "let f(student: Student) -> Int = let age = student.age { age + 1 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Let { var, value, body } => {
                assert_eq!(var.node, "age");
                match &value.node {
                    Expr::Path { object, segments } => {
                        matches!(object.node, Expr::Ident(_));
                        assert_eq!(segments.len(), 1);
                        assert_eq!(segments[0].node, "age");
                    }
                    _ => panic!("Expected Path"),
                }
                assert!(matches!(body.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected LetIn expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_deeply_nested_let_in() {
    let input = "let f() -> Int = let a = 1 { let b = 2 { let c = 3 { a + b + c } } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // First level
            match &body.node {
                Expr::Let { var, value, body } => {
                    assert_eq!(var.node, "a");
                    assert!(matches!(value.node, Expr::Number(1)));

                    // Second level
                    match &body.node {
                        Expr::Let { var, value, body } => {
                            assert_eq!(var.node, "b");
                            assert!(matches!(value.node, Expr::Number(2)));

                            // Third level
                            match &body.node {
                                Expr::Let { var, value, body } => {
                                    assert_eq!(var.node, "c");
                                    assert!(matches!(value.node, Expr::Number(3)));
                                    // Body is a + b + c (nested Add expressions)
                                    assert!(matches!(body.node, Expr::Add(_, _)));
                                }
                                _ => panic!("Expected third LetIn"),
                            }
                        }
                        _ => panic!("Expected second LetIn"),
                    }
                }
                _ => panic!("Expected first LetIn"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}
