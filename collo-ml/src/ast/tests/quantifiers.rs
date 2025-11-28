use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Forall Quantifier =============

#[test]
fn parse_simple_forall() {
    let input = "let f() -> Constraint = forall x in @[Student] { $V(x) >== 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::GlobalList(_)));
                assert!(filter.is_none());
                assert!(matches!(body.node, Expr::ConstraintGe(_, _)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_filter() {
    let input = "let f() -> Constraint = forall x in @[Student] where x.age > 18 { $V(x) >== 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { filter, .. } => {
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::Gt(_, _)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_complex_filter() {
    let input = "let f() -> Constraint = forall s in @[Student] where s.age > 18 and s.grade >= 10 { $V(s) === 1 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { var, filter, .. } => {
                assert_eq!(var.node, "s");
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::And(_, _)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_constraint_body() {
    let input = "let f() -> Constraint = forall x in @[Student] { $V(x) === 1 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { body, .. } => {
                assert!(matches!(body.node, Expr::ConstraintEq(_, _)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_list_literal() {
    let input = "let f() -> Constraint = forall x in [1, 2, 3] { x > 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { collection, .. } => {
                assert!(matches!(collection.node, Expr::ListLiteral { .. }));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_list_comprehension() {
    // The constraint is invalid but this should still parse
    let input =
        "let f() -> Constraint = forall x in [s for s in @[Student] where s.active] { x > 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { collection, .. } => {
                assert!(matches!(collection.node, Expr::ListComprehension { .. }));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_path_collection() {
    let input = "let f() -> Constraint = forall x in my_list { x >== 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { collection, .. } => {
                assert!(matches!(collection.node, Expr::Ident(_)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_forall() {
    let input = "let f() -> Constraint = forall x in @[Student] { forall y in @[Course] { $V(x, y) >== 0 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { body, .. } => {
                assert!(matches!(body.node, Expr::Forall { .. }));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_complex_body() {
    let input = "let f() -> Constraint = forall s in @[Student] { $V(s) + $V(s.teacher) === 10 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { body, .. } => {
                assert!(matches!(body.node, Expr::ConstraintEq(_, _)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_set_operation_collection() {
    let input = "let f() -> Constraint = forall x in @[Student] + @[Teacher] { x.active };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { collection, .. } => {
                assert!(matches!(collection.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Sum Quantifier =============

#[test]
fn parse_simple_sum() {
    let input = "let f() -> LinExpr = sum x in @[Student] { $V(x) };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::GlobalList(_)));
                assert!(filter.is_none());
                assert!(matches!(body.node, Expr::VarCall { .. }));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_filter() {
    let input = "let f() -> LinExpr = sum x in @[Student] where x.age > 18 { $V(x) };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { filter, .. } => {
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::Gt(_, _)));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_complex_body() {
    let input = "let f() -> LinExpr = sum x in @[Student] { $V(x) * 2 + 5 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { body, .. } => {
                // Should be Add(Mul(...), Number(5))
                assert!(matches!(body.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_constant_body() {
    let input = "let f() -> LinExpr = sum x in @[Student] { 1 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { body, .. } => {
                assert!(matches!(body.node, Expr::Number(1)));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_field_access_in_body() {
    let input = "let f() -> LinExpr = sum s in @[Student] { s.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { var, body, .. } => {
                assert_eq!(var.node, "s");
                assert!(matches!(
                    body.node,
                    Expr::Path {
                        object: _,
                        segments: _
                    }
                ));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_list_literal() {
    let input = "let f() -> Int = sum x in [1, 2, 3, 4, 5] { x };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { collection, .. } => {
                assert!(matches!(collection.node, Expr::ListLiteral { .. }));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_sum() {
    let input = "let f() -> LinExpr = sum x in @[Student] { sum y in @[Course] { $V(x, y) } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { body, .. } => {
                assert!(matches!(body.node, Expr::Sum { .. }));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_in_arithmetic_expression() {
    let input = "let f() -> LinExpr = sum x in @[Student] { $V(x) } + 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Add(left, right) => {
                assert!(matches!(left.node, Expr::Sum { .. }));
                assert!(matches!(right.node, Expr::Number(10)));
            }
            _ => panic!("Expected Add"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_list_comprehension() {
    let input = "let f() -> LinExpr = sum x in [s for s in @[Student] where s.active] { $V(x) };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { collection, .. } => {
                assert!(matches!(collection.node, Expr::ListComprehension { .. }));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Combined Quantifiers =============

#[test]
fn parse_sum_and_forall_combined() {
    let input =
        "let f() -> Constraint = forall s in @[Student] { sum c in @[Course] { $V(s, c) } === 5 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { body, .. } => match &body.node {
                Expr::ConstraintEq(left, _) => {
                    assert!(matches!(left.node, Expr::Sum { .. }));
                }
                _ => panic!("Expected ConstraintEq"),
            },
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_multiple_independent_foralls_with_and() {
    let input =
        "let f() -> Constraint = forall x in @[A] { x >== 0 } and forall y in @[B] { y >== 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::And(left, right) => {
                assert!(matches!(left.node, Expr::Forall { .. }));
                assert!(matches!(right.node, Expr::Forall { .. }));
            }
            _ => panic!("Expected And"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_sum_in_filter() {
    let input = "let f() -> Constraint = forall s in @[Student] where sum c in @[Course] { c.count } > 3 { $V(s,c) <== 2 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { filter, .. } => {
                assert!(filter.is_some());
                match &filter.as_ref().unwrap().node {
                    Expr::Gt(left, _) => {
                        assert!(matches!(left.node, Expr::Sum { .. }));
                    }
                    _ => panic!("Expected Gt in filter"),
                }
            }
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}
