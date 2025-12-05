use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Fold Aggregation =============

#[test]
fn parse_simple_fold() {
    let input = "let f() -> Int = fold x in @[Student] with acc = 0 { acc + x.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold {
                var,
                collection,
                accumulator,
                init_value,
                filter,
                body,
                reversed,
            } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::GlobalList(_)));
                assert_eq!(accumulator.node, "acc");
                assert!(matches!(init_value.node, Expr::Number(0)));
                assert!(filter.is_none());
                assert!(matches!(body.node, Expr::Add(_, _)));
                assert_eq!(*reversed, false);
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_different_accumulator_names() {
    let inputs = vec![
        (
            "let f() -> Int = fold x in [1, 2, 3] with acc = 0 { acc + x };",
            "acc",
        ),
        (
            "let f() -> Int = fold x in [1, 2, 3] with total = 0 { total + x };",
            "total",
        ),
        (
            "let f() -> Int = fold x in [1, 2, 3] with result = 0 { result + x };",
            "result",
        ),
        (
            "let f() -> Int = fold x in [1, 2, 3] with counter = 0 { counter + x };",
            "counter",
        ),
    ];

    for (input, expected_acc) in inputs {
        let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
        let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

        match &file.statements[0].node {
            Statement::Let { body, .. } => match &body.node {
                Expr::Fold { accumulator, .. } => {
                    assert_eq!(accumulator.node, expected_acc);
                }
                _ => panic!("Expected Fold for input: {}", input),
            },
            _ => panic!("Expected Let statement for input: {}", input),
        }
    }
}

#[test]
fn parse_fold_with_complex_init_value() {
    let input = "let f() -> Int = fold x in list with acc = 2 * 5 { acc + x };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { init_value, .. } => {
                assert!(matches!(init_value.node, Expr::Mul(_, _)));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_function_call_init_value() {
    let input = "let f() -> Int = fold x in list with acc = get_initial() { acc + x };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { init_value, .. } => {
                assert!(matches!(init_value.node, Expr::FnCall { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_filter() {
    let input =
        "let f() -> Int = fold x in @[Student] with acc = 0 where x.age > 18 { acc + x.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { filter, .. } => {
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::Gt(_, _)));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_complex_filter() {
    let input = "let f() -> Int = fold s in @[Student] with acc = 0 where s.age > 18 and s.grade >= 10 { acc + s.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { var, filter, .. } => {
                assert_eq!(var.node, "s");
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::And(_, _)));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_list_literal() {
    let input = "let f() -> Int = fold x in [1, 2, 3, 4, 5] with acc = 0 { acc + x };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { collection, .. } => {
                assert!(matches!(collection.node, Expr::ListLiteral { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_list_comprehension() {
    let input = "let f() -> Int = fold x in [s for s in @[Student] where s.active] with acc = 0 { acc + x.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { collection, .. } => {
                assert!(matches!(collection.node, Expr::ListComprehension { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_path_collection() {
    let input = "let f() -> Int = fold x in my_list with acc = 0 { acc + x };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { collection, .. } => {
                assert!(matches!(collection.node, Expr::Ident(_)));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_fold() {
    let input = "let f() -> Int = fold x in @[Student] with acc1 = 0 { fold y in @[Course] with acc2 = 0 { acc2 + x.grade + y.credits } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { body, .. } => {
                assert!(matches!(body.node, Expr::Fold { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_complex_body() {
    let input = "let f() -> Int = fold x in @[Student] with acc = 0 { acc + x.grade * 2 + 5 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { body, .. } => {
                // Should be Add(Add(...), Number(5))
                assert!(matches!(body.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_if_in_body() {
    let input =
        "let f() -> Int = fold x in list with acc = 0 { if x > 0 { acc + x } else { acc } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { body, .. } => {
                assert!(matches!(body.node, Expr::If { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_set_operation_collection() {
    let input = "let f() -> Int = fold x in @[Student] + @[Teacher] with acc = 0 { acc + x.age };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { collection, .. } => {
                assert!(matches!(collection.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_in_arithmetic_expression() {
    let input = "let f() -> Int = fold x in @[Student] with acc = 0 { acc + x.grade } + 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Add(left, right) => {
                assert!(matches!(left.node, Expr::Fold { .. }));
                assert!(matches!(right.node, Expr::Number(10)));
            }
            _ => panic!("Expected Add"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= RFold Aggregation =============

#[test]
fn parse_simple_rfold() {
    let input = "let f() -> Int = rfold x in @[Student] with acc = 0 { acc + x.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold {
                var,
                collection,
                accumulator,
                init_value,
                filter,
                body,
                reversed,
            } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::GlobalList(_)));
                assert_eq!(accumulator.node, "acc");
                assert!(matches!(init_value.node, Expr::Number(0)));
                assert!(filter.is_none());
                assert!(matches!(body.node, Expr::Add(_, _)));
                assert_eq!(*reversed, true); // This is the key difference!
            }
            _ => panic!("Expected Fold (rfold)"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_rfold_with_filter() {
    let input =
        "let f() -> Int = rfold x in @[Student] with acc = 0 where x.age > 18 { acc + x.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold {
                filter, reversed, ..
            } => {
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::Gt(_, _)));
                assert_eq!(*reversed, true);
            }
            _ => panic!("Expected Fold (rfold)"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_rfold_with_complex_body() {
    let input = "let f() -> Int = rfold x in @[Student] with acc = 0 { acc + x.grade * 2 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { body, reversed, .. } => {
                assert!(matches!(body.node, Expr::Add(_, _)));
                assert_eq!(*reversed, true);
            }
            _ => panic!("Expected Fold (rfold)"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_rfold() {
    let input = "let f() -> Int = rfold x in @[Student] with acc1 = 0 { rfold y in @[Course] with acc2 = 0 { acc2 + x.grade } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { body, reversed, .. } => {
                assert!(matches!(body.node, Expr::Fold { .. }));
                assert_eq!(*reversed, true);
            }
            _ => panic!("Expected Fold (rfold)"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_rfold_with_list_literal() {
    let input = "let f() -> Int = rfold x in [1, 2, 3, 4, 5] with acc = 0 { acc + x };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold {
                collection,
                reversed,
                ..
            } => {
                assert!(matches!(collection.node, Expr::ListLiteral { .. }));
                assert_eq!(*reversed, true);
            }
            _ => panic!("Expected Fold (rfold)"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Combined Fold with Other Quantifiers =============

#[test]
fn parse_fold_with_sum_in_body() {
    let input = "let f() -> Int = fold s in @[Student] with acc = 0 { acc + sum c in @[Course] { c.credits } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { body, .. } => match &body.node {
                Expr::Add(_, right) => {
                    assert!(matches!(right.node, Expr::Sum { .. }));
                }
                _ => panic!("Expected Add in fold body"),
            },
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_sum_with_fold_in_body() {
    let input = "let f() -> Int = sum x in @[Student] { fold y in x.courses with acc = 0 { acc + y.credits } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum { body, .. } => {
                assert!(matches!(body.node, Expr::Fold { .. }));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_forall_with_fold_in_body() {
    let input = "let f() -> Constraint = forall s in @[Student] { fold c in s.courses with acc = 0 { acc + c.credits } >= 30 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { body, .. } => match &body.node {
                Expr::Ge(left, _) => {
                    assert!(matches!(left.node, Expr::Fold { .. }));
                }
                _ => panic!("Expected Ge in forall body"),
            },
            _ => panic!("Expected Forall"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_forall_in_filter() {
    let input = "let f() -> Int = fold s in @[Student] with acc = 0 where forall c in s.courses { c.passed } { acc + s.grade };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { filter, .. } => {
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::Forall { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_mixing_fold_and_rfold() {
    let input = "let f() -> Int = fold x in list1 with acc1 = 0 { rfold y in list2 with acc2 = acc1 { acc2 + y } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold {
                body,
                reversed: reversed_outer,
                ..
            } => {
                assert_eq!(*reversed_outer, false); // outer is fold
                match &body.node {
                    Expr::Fold {
                        reversed: reversed_inner,
                        ..
                    } => {
                        assert_eq!(*reversed_inner, true); // inner is rfold
                    }
                    _ => panic!("Expected inner Fold (rfold)"),
                }
            }
            _ => panic!("Expected outer Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_fold_with_list_init_value() {
    let input = "let f() -> [Int] = fold x in [1, 2, 3] with acc = [] { acc + [x] };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Fold { init_value, .. } => {
                assert!(matches!(init_value.node, Expr::ListLiteral { .. }));
            }
            _ => panic!("Expected Fold"),
        },
        _ => panic!("Expected Let statement"),
    }
}
