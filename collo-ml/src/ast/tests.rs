use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

#[test]
fn visitor_handles_let_statement() {
    let input = "let f(x: Int) -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 1);
    match &file.statements[0].node {
        Statement::Let {
            name,
            params,
            output_type,
            body,
            ..
        } => {
            assert_eq!(name.node, "f");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name.node, "x");
            assert!(matches!(params[0].typ.node, InputType::Int));
            assert_eq!(*output_type, OutputType::LinExpr);
            assert!(matches!(body.node, Expr::LinExpr(LinExpr::Constant(_))));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn visitor_handles_arithmetic() {
    let input = "let f() -> LinExpr = 2 + 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::LinExpr(LinExpr::Add(_, _)) => {
                // Correct!
            }
            _ => panic!("Expected Add, got {:?}", body.node),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_reify() {
    let input = "reify my_constraint as $MyVar;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Reify {
            constraint_name,
            var_name,
            ..
        } => {
            assert_eq!(constraint_name.node, "my_constraint");
            assert_eq!(var_name.node, "MyVar");
        }
        _ => panic!("Expected Reify statement"),
    }
}

#[test]
fn visitor_handles_nested_types() {
    let input = "let f(x: [[Int]]) -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => match &params[0].typ.node {
            InputType::List(inner1) => match &**inner1 {
                InputType::List(inner2) => {
                    assert!(matches!(**inner2, InputType::Int));
                }
                _ => panic!("Expected nested list"),
            },
            _ => panic!("Expected list type"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_comparison_constraint() {
    let input = "let f() -> Constraint = $V(x) <= 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Constraint(Constraint::Comparison { op, .. }) => {
                assert_eq!(*op, ComparisonOp::LessEq);
            }
            _ => panic!("Expected Comparison constraint"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_forall() {
    let input = "let f() -> Constraint = forall x in @[Student]: $V(x) >= 0;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Constraint(Constraint::Forall {
                var,
                collection,
                filter,
                ..
            }) => {
                assert_eq!(var, "x");
                assert!(matches!(collection.node, Collection::Global(_)));
                assert!(filter.is_none());
            }
            _ => panic!("Expected Forall constraint"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_forall_with_filter() {
    let input = "let f() -> Constraint = forall x in @[Student] where x > 5: $V(x) >= 0;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Constraint(Constraint::Forall { filter, .. }) => {
                assert!(filter.is_some());
            }
            _ => panic!("Expected Forall constraint"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_sum() {
    let input = "let f() -> LinExpr = sum x in @[Student]: $V(x);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::LinExpr(LinExpr::Sum {
                var, collection, ..
            }) => {
                assert_eq!(var, "x");
                assert!(matches!(collection.node, Collection::Global(_)));
            }
            _ => panic!("Expected Sum"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_computable_operations() {
    let input = "let f() -> LinExpr = 2 * 3 + 4;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            match &body.node {
                Expr::LinExpr(LinExpr::Add(left, right)) => {
                    // Left should be Mul(Number(2), Constant(Number(3)))
                    match &left.node {
                        LinExpr::Mul { coeff, expr } => {
                            assert!(matches!(coeff.node, Computable::Number(2)));
                            match &expr.node {
                                LinExpr::Constant(comp) => {
                                    assert!(matches!(comp.node, Computable::Number(3)));
                                }
                                _ => panic!("Expected second to be Constant(3)"),
                            }
                        }
                        _ => panic!("Expected left to be Constant(Mul)"),
                    }
                    // Right should be Constant(4)
                    match &right.node {
                        LinExpr::Constant(comp) => {
                            assert!(matches!(comp.node, Computable::Number(4)));
                        }
                        _ => panic!("Expected right to be Constant(4)"),
                    }
                }
                _ => panic!("Expected Add, got {:?}", body.node),
            }
        }
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_path() {
    let input = "let f() -> LinExpr = student.age;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::LinExpr(LinExpr::Constant(comp)) => match &comp.node {
                Computable::Path(path) => {
                    assert_eq!(path.segments, vec!["student", "age"]);
                }
                _ => panic!("Expected Path"),
            },
            _ => panic!(),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_collection_operations() {
    let input = "let f() -> LinExpr = |@[Student] \\ excluded|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::LinExpr(LinExpr::Constant(comp)) => match &comp.node {
                Computable::Cardinality(coll) => {
                    assert!(matches!(coll.node, Collection::Diff(_, _)));
                }
                _ => panic!("Expected Cardinality"),
            },
            _ => panic!(),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_docstrings() {
    let input = "## This is a doc\n## Second line\nlet f() -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { docstring, .. } => {
            assert_eq!(docstring.len(), 2);
            assert_eq!(docstring[0], " This is a doc");
            assert_eq!(docstring[1], " Second line");
        }
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_pub_modifier() {
    let input = "pub let f() -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { public, .. } => {
            assert!(public);
        }
        _ => panic!(),
    }
}
