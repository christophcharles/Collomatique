use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Arithmetic Operators =============

#[test]
fn parse_addition() {
    let input = "let f() -> Int = 2 + 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Add(left, right) => {
                assert!(matches!(left.node, Expr::Number(2)));
                assert!(matches!(right.node, Expr::Number(3)));
            }
            _ => panic!("Expected Add, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_subtraction() {
    let input = "let f() -> Int = 10 - 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sub(left, right) => {
                assert!(matches!(left.node, Expr::Number(10)));
                assert!(matches!(right.node, Expr::Number(3)));
            }
            _ => panic!("Expected Sub"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_multiplication() {
    let input = "let f() -> Int = 4 * 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Mul(left, right) => {
                assert!(matches!(left.node, Expr::Number(4)));
                assert!(matches!(right.node, Expr::Number(5)));
            }
            _ => panic!("Expected Mul"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_division() {
    let input = "let f() -> Int = 20 // 4;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Div(left, right) => {
                assert!(matches!(left.node, Expr::Number(20)));
                assert!(matches!(right.node, Expr::Number(4)));
            }
            _ => panic!("Expected Div"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_modulo() {
    let input = "let f() -> Int = 10 % 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Mod(left, right) => {
                assert!(matches!(left.node, Expr::Number(10)));
                assert!(matches!(right.node, Expr::Number(3)));
            }
            _ => panic!("Expected Mod"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_complex_arithmetic() {
    let input = "let f() -> Int = 2 * 3 + 4;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (2 * 3) + 4 due to precedence
            match &body.node {
                Expr::Add(left, right) => {
                    // Left should be Mul(2, 3)
                    match &left.node {
                        Expr::Mul(expr1, expr2) => {
                            assert!(matches!(expr1.node, Expr::Number(2)));
                            assert!(matches!(expr2.node, Expr::Number(3)));
                        }
                        _ => panic!("Expected left to be Mul"),
                    }
                    // Right should be 4
                    assert!(matches!(right.node, Expr::Number(4)));
                }
                _ => panic!("Expected Add, got {:?}", body.node),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_arithmetic_with_parentheses() {
    let input = "let f() -> Int = 2 * (3 + 4);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as 2 * (3 + 4)
            match &body.node {
                Expr::Mul(left, right) => {
                    assert!(matches!(left.node, Expr::Number(2)));
                    assert!(matches!(right.node, Expr::Add(_, _)));
                }
                _ => panic!("Expected Mul"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_chained_multiplication() {
    let input = "let f() -> Int = 2 * 3 * 4;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (2 * 3) * 4 (left-associative)
            assert!(matches!(body.node, Expr::Mul(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_mixed_add_sub() {
    let input = "let f() -> Int = 10 + 5 - 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (10 + 5) - 3 (left-associative)
            assert!(matches!(body.node, Expr::Sub(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= Comparison Operators =============

#[test]
fn parse_equality() {
    let input = "let f() -> Bool = x == y;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Eq(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_inequality() {
    let input = "let f() -> Bool = x != y;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Ne(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_less_than() {
    let input = "let f() -> Bool = x < y;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Lt(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_less_than_or_equal() {
    let input = "let f() -> Bool = x <= y;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Le(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_greater_than() {
    let input = "let f() -> Bool = x > y;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Gt(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_greater_than_or_equal() {
    let input = "let f() -> Bool = x >= y;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Ge(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_comparison_with_arithmetic() {
    let input = "let f() -> Bool = x + 5 > y * 2;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Gt(left, right) => {
                assert!(matches!(left.node, Expr::Add(_, _)));
                assert!(matches!(right.node, Expr::Mul(_, _)));
            }
            _ => panic!("Expected Gt"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Constraint Operators =============

#[test]
fn parse_constraint_equality() {
    let input = "let f() -> Constraint = $V(x) === 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::ConstraintEq(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_constraint_less_equal() {
    let input = "let f() -> Constraint = $V(x) <== 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::ConstraintLe(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_constraint_greater_equal() {
    let input = "let f() -> Constraint = $V(x) >== 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::ConstraintGe(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_constraint_with_linexpr() {
    let input = "let f() -> Constraint = $V(x) + $V(y) === 100;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ConstraintEq(left, _) => {
                assert!(matches!(left.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected ConstraintEq"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// Note: Regular comparisons (<=, >=) are different from constraint operators (<==, >==)
#[test]
fn parse_regular_vs_constraint_comparison() {
    let input = "let f() -> Constraint = $V(x) <= 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // This should parse as regular <= (Le), not constraint <== (ConstraintLe)
            assert!(matches!(body.node, Expr::Le(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= Logical Operators =============

#[test]
fn parse_logical_and_with_ampersands() {
    let input = "let f() -> Bool = x > 5 && y < 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::And(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_logical_and_with_keyword() {
    let input = "let f() -> Bool = x > 5 and y < 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::And(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_logical_or_with_pipes() {
    let input = "let f() -> Bool = x > 5 || y < 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Or(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_logical_or_with_keyword() {
    let input = "let f() -> Bool = x > 5 or y < 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Or(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_logical_not_with_exclamation() {
    let input = "let f() -> Bool = !x;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Not(_)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_logical_not_with_keyword() {
    let input = "let f() -> Bool = not x;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Not(_)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_double_negation() {
    let input = "let f() -> Bool = not not x;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Not(inner) => {
                assert!(matches!(inner.node, Expr::Not(_)));
            }
            _ => panic!("Expected Not"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_double_negation_with_exclamation() {
    let input = "let f() -> Bool = !!x;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Not(inner) => {
                assert!(matches!(inner.node, Expr::Not(_)));
            }
            _ => panic!("Expected Not"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_complex_boolean_expression() {
    let input = "let f() -> Bool = (x > 5 and y < 3) or z == 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (x > 5 and y < 3) or (z == 10)
            assert!(matches!(body.node, Expr::Or(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_or_has_higher_precedence_than_and() {
    let input = "let f() -> Bool = a or b and c;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (a or b) and c since or has higher precedence
            match &body.node {
                Expr::And(left, right) => {
                    assert!(matches!(left.node, Expr::Or(_, _)));
                    assert!(matches!(right.node, Expr::Path(_)));
                }
                _ => panic!("Expected Or with And on right"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}
