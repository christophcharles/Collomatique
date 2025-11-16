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
            assert!(matches!(params[0].typ.node, TypeName::Int));
            assert_eq!(output_type.node, TypeName::LinExpr);
            assert!(matches!(body.node, Expr::Number(_)));
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
            Expr::Add(_, _) => {
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
            TypeName::List(inner1) => match &**inner1 {
                TypeName::List(inner2) => {
                    assert!(matches!(**inner2, TypeName::Int));
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
            Expr::Le(_, _) => {
                // OK
            }
            _ => panic!("Expected Comparison constraint"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_forall() {
    let input = "let f() -> Constraint = forall x in @[Student] { $V(x) >= 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall {
                var,
                collection,
                filter,
                ..
            } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::Global(_)));
                assert!(filter.is_none());
            }
            _ => panic!("Expected Forall constraint"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_forall_with_filter() {
    let input = "let f() -> Constraint = forall x in @[Student] where x > 5 { $V(x) >= 0 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Forall { filter, .. } => {
                assert!(filter.is_some());
            }
            _ => panic!("Expected Forall constraint"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_sum() {
    let input = "let f() -> LinExpr = sum x in @[Student] { $V(x) };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Sum {
                var, collection, ..
            } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::Global(_)));
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
                Expr::Add(left, right) => {
                    // Left should be Mul(Number(2), Number(3))
                    match &left.node {
                        Expr::Mul(expr1, expr2) => {
                            assert!(matches!(expr1.node, Expr::Number(2)));
                            match &expr2.node {
                                Expr::Number(3) => {
                                    // OK
                                }
                                _ => panic!("Expected second to be Constant(3)"),
                            }
                        }
                        _ => panic!("Expected left to be (Mul)"),
                    }
                    // Right should be Number(4)
                    match &right.node {
                        Expr::Number(4) => {
                            // OK
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
            Expr::Path(path) => {
                assert_eq!(
                    path.segments
                        .iter()
                        .map(|x| x.node.as_str())
                        .collect::<Vec<_>>(),
                    vec!["student", "age"]
                );
            }
            _ => panic!("Expected Path"),
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
            Expr::Cardinality(coll) => {
                assert!(matches!(coll.node, Expr::Diff(_, _)));
            }
            _ => panic!("Expected Cardinality"),
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

#[test]
fn visitor_handles_boolean_literals() {
    let input = "let f() -> Bool = true;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Boolean(Boolean::True)));
        }
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_list_literal() {
    let input = "let f() -> [Int] = [1, 2, 3];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListLiteral { elements } => {
                assert_eq!(elements.len(), 3);
            }
            _ => panic!("Expected ListLiteral"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_empty_list() {
    let input = "let f() -> [Int] = [];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListLiteral { elements } => {
                assert_eq!(elements.len(), 0);
            }
            _ => panic!("Expected ListLiteral"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_list_comprehension() {
    let input = "let f() -> [Int] = [x for x in @[Student]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { var, collection, filter, .. } => {
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::Global(_)));
                assert!(filter.is_none());
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_list_comprehension_with_filter() {
    let input = "let f() -> [Int] = [s.age for s in @[Student] where s.age > 18];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { filter, .. } => {
                assert!(filter.is_some());
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_union_and_inter() {
    let input = "let f() -> [Student] = @[Student] union @[Teacher];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Union(_, _)));
        }
        _ => panic!(),
    }
}

#[test]
fn visitor_handles_logical_operators() {
    let input = "let f() -> Bool = x > 5 or y < 3;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Or(_, _)));
        }
        _ => panic!(),
    }
}