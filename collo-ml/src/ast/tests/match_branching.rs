use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Match Expression AST Tests =============

#[test]
fn parse_simple_match_with_one_type_branch() {
    let input = "let f(x: Int) -> Int = match x { y as Int { 10 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match {
                match_expr: expr,
                branches,
            } => {
                // Check matched expression
                match &expr.node {
                    Expr::Ident(name) => assert_eq!(name.node, "x"),
                    _ => panic!("Expected Ident in match expression"),
                }

                // Check branches
                assert_eq!(branches.len(), 1);

                // First branch: Int { 10 }
                assert!(branches[0].as_typ.is_some());
                assert_eq!(branches[0].as_typ.as_ref().unwrap().node.types.len(), 1);
                assert!(branches[0].into_typ.is_none());
                assert!(branches[0].filter.is_none());
                assert!(matches!(branches[0].body.node, Expr::Number(10)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_else_branch() {
    let input = "let f(x: Int | Bool) -> Int = match x { i as Int { 1 } other { 0 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match {
                match_expr: _,
                branches,
            } => {
                assert_eq!(branches.len(), 2);

                // First branch: Int { 1 }
                assert!(branches[0].as_typ.is_some());
                assert!(matches!(branches[0].body.node, Expr::Number(1)));

                // Second branch: else { 0 } - in new grammar this would be: x { 0 } with no as_typ
                assert!(branches[1].as_typ.is_none());
                assert!(matches!(branches[1].body.node, Expr::Number(0)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_into_conversion() {
    let input = "let f(x: Int) -> Bool = match x { y as Int into Bool { true } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                assert!(branches[0].into_typ.is_some());

                // Check the "into" type is Bool
                let into_typ = branches[0].into_typ.as_ref().unwrap();
                assert_eq!(into_typ.node.types.len(), 1);

                assert!(branches[0].filter.is_none());
                assert!(matches!(branches[0].body.node, Expr::Boolean(true)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_where_filter() {
    let input = "let f(x: Int) -> Int = match x { y as Int where x > 5 { 10 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                assert!(branches[0].into_typ.is_none());
                assert!(branches[0].filter.is_some());

                // Check the filter is a comparison
                let filter = branches[0].filter.as_ref().unwrap();
                assert!(matches!(filter.node, Expr::Gt(_, _)));

                assert!(matches!(branches[0].body.node, Expr::Number(10)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_into_and_where() {
    let input = "let f(x: Int) -> Bool = match x { y as Int into Bool where x > 0 { true } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Check both into and where are present
                assert!(branches[0].into_typ.is_some());
                assert!(branches[0].filter.is_some());

                // Verify the filter is a comparison
                assert!(matches!(
                    branches[0].filter.as_ref().unwrap().node,
                    Expr::Gt(_, _)
                ));

                assert!(matches!(branches[0].body.node, Expr::Boolean(true)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_multiple_type_branches() {
    let input = "let f(x: Int | Bool | None) -> Int = match x { i as Int { 1 } b as Bool { 2 } n as None { 0 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 3);

                // All branches should have as_typ
                assert!(branches[0].as_typ.is_some());
                assert!(branches[1].as_typ.is_some());
                assert!(branches[2].as_typ.is_some());

                // Check bodies
                assert!(matches!(branches[0].body.node, Expr::Number(1)));
                assert!(matches!(branches[1].body.node, Expr::Number(2)));
                assert!(matches!(branches[2].body.node, Expr::Number(0)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_union_type_pattern() {
    let input = "let f(x: Int | Bool) -> Int = match x { y as Int | Bool { 1 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Union type should have 2 types
                assert_eq!(branches[0].as_typ.as_ref().unwrap().node.types.len(), 2);
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_maybe_type() {
    let input = "let f(x: ?Int) -> Int = match x { y as ?Int { 10 } other { 0 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 2);

                // Check it's a maybe type
                assert_eq!(
                    branches[0].as_typ.as_ref().unwrap().node.types[0]
                        .node
                        .maybe_count,
                    1
                );
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_list_type() {
    let input = "let f(x: [Int]) -> Int = match x { lst as [Int] { |x| } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Check it's a list type
                match &branches[0].as_typ.as_ref().unwrap().node.types[0]
                    .node
                    .inner
                {
                    SimpleTypeName::List(_) => {} // Success
                    _ => panic!("Expected list type"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_arithmetic_body() {
    let input = "let f(x: Int) -> Int = match x { y as Int { x * 2 + 1 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be arithmetic expression
                assert!(matches!(branches[0].body.node, Expr::Add(_, _)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_path_in_body() {
    let input = "let f(s: Student) -> Int = match s { st as Student { s.age } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be a path
                match &branches[0].body.node {
                    Expr::Path { segments, .. } => {
                        assert_eq!(segments.len(), 1);
                        assert_eq!(segments[0].node, "age");
                    }
                    _ => panic!("Expected Path in body"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_variable_call_body() {
    let input = "let f(x: Int) -> Constraint = match x { i as Int { $V(x) === 1 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be a constraint
                assert!(matches!(branches[0].body.node, Expr::ConstraintEq(_, _)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_if_in_body() {
    let input = "let f(x: Int) -> Int = match x { i as Int { if x > 0 { x } else { 0 } } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be an if expression
                assert!(matches!(branches[0].body.node, Expr::If { .. }));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_quantifier_in_body() {
    let input =
        "let f(items: [Int]) -> Int = match items { lst as [Int] { sum i in items { i } } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be a sum expression
                match &branches[0].body.node {
                    Expr::Sum { var, .. } => {
                        assert_eq!(var.node, "i");
                    }
                    _ => panic!("Expected Sum in body"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_match_expressions() {
    let input =
        "let f(x: Int) -> Int = match x { i as Int { match x { j as Int { 1 } other { 0 } } } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be another match expression
                match &branches[0].body.node {
                    Expr::Match {
                        branches: inner_branches,
                        ..
                    } => {
                        assert_eq!(inner_branches.len(), 2);
                        assert!(inner_branches[0].as_typ.is_some());
                        assert!(inner_branches[1].as_typ.is_none()); // catch-all
                    }
                    _ => panic!("Expected nested Match in body"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_let_in_body() {
    let input = "let f(x: Int) -> Int = match x { i as Int { let y = x * 2 { y + 1 } } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be a let expression
                match &branches[0].body.node {
                    Expr::Let { var, value, body } => {
                        assert_eq!(var.node, "y");
                        assert!(matches!(value.node, Expr::Mul(_, _)));
                        assert!(matches!(body.node, Expr::Add(_, _)));
                    }
                    _ => panic!("Expected Let in body"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_in_arithmetic_context() {
    let input = "let f(x: Int) -> Int = (match x { i as Int { 10 } other { 0 } }) + 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Body should be an addition
            match &body.node {
                Expr::Add(left, right) => {
                    // Left should be the match expression
                    assert!(matches!(left.node, Expr::Match { .. }));
                    // Right should be 5
                    assert!(matches!(right.node, Expr::Number(5)));
                }
                _ => panic!("Expected Add expression"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_list_comprehension_body() {
    let input =
        "let f(items: [Int]) -> [Int] = match items { lst as [Int] { [x for x in items] } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Body should be a list comprehension
                assert!(matches!(
                    branches[0].body.node,
                    Expr::ListComprehension { .. }
                ));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_cardinality_in_filter() {
    let input =
        "let f(items: [Int]) -> Int = match items { lst as [Int] where |items| > 0 { |items| } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                assert!(branches[0].filter.is_some());

                // Filter should contain a cardinality check
                match &branches[0].filter.as_ref().unwrap().node {
                    Expr::Gt(left, _) => {
                        assert!(matches!(left.node, Expr::Cardinality(_)));
                    }
                    _ => panic!("Expected Gt with Cardinality"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_boolean_filter() {
    let input = "let f(x: Int) -> Int = match x { i as Int where x > 5 and x < 10 { x } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                assert!(branches[0].filter.is_some());

                // Filter should be an And expression
                assert!(matches!(
                    branches[0].filter.as_ref().unwrap().node,
                    Expr::And(_, _)
                ));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_with_path_in_filter() {
    let input = "let f(s: Student) -> Int = match s { st as Student where s.age > 18 { 1 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                assert!(branches[0].filter.is_some());

                // Filter should be a comparison with a path
                match &branches[0].filter.as_ref().unwrap().node {
                    Expr::Gt(left, _) => {
                        assert!(matches!(left.node, Expr::Path { .. }));
                    }
                    _ => panic!("Expected Gt with Path"),
                }
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_only_else_branch() {
    let input = "let f(x: Int) -> Int = match x { y { 42 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 1);

                // Should be a catch-all branch (no as_typ)
                assert!(branches[0].as_typ.is_none());
                assert!(matches!(branches[0].body.node, Expr::Number(42)));
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_match_multiple_conditions_same_type() {
    let input = "let f(x: Int) -> Int = match x { i as Int where x > 0 { 1 } j as Int where x < 0 { -1 } other { 0 } };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 3);

                // First two branches have as_typ and filters
                assert!(branches[0].as_typ.is_some());
                assert!(branches[0].filter.is_some());

                assert!(branches[1].as_typ.is_some());
                assert!(branches[1].filter.is_some());

                // Last branch is catch-all (no as_typ)
                assert!(branches[2].as_typ.is_none());
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_complex_match_real_world() {
    let input = r#"let f(x: Int | Bool) -> Constraint = 
        match x { 
            i as Int into LinExpr where x > 0 { $V(x) === x } 
            b as Bool { if x { $V(1) === 1 } else { $V(0) === 0 } } 
            other { $V(0) === 0 } 
        };"#;
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Match { branches, .. } => {
                assert_eq!(branches.len(), 3);

                // First branch: Int with into and where
                assert!(branches[0].into_typ.is_some());
                assert!(branches[0].filter.is_some());

                // Second branch: Bool
                assert!(branches[1].as_typ.is_some());
                assert!(branches[1].into_typ.is_none());
                assert!(branches[1].filter.is_none());
                assert!(matches!(branches[1].body.node, Expr::If { .. }));

                // Third branch: catch-all
                assert!(branches[2].as_typ.is_none());
            }
            _ => panic!("Expected Match expression"),
        },
        _ => panic!("Expected Let statement"),
    }
}
