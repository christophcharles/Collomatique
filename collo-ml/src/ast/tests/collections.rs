use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= List Literals =============

#[test]
fn parse_empty_list() {
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
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_with_single_element() {
    let input = "let f() -> [Int] = [42];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListLiteral { elements } => {
                assert_eq!(elements.len(), 1);
                assert!(matches!(elements[0].node, Expr::Number(42)));
            }
            _ => panic!("Expected ListLiteral"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_with_multiple_elements() {
    let input = "let f() -> [Int] = [1, 2, 3];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListLiteral { elements } => {
                assert_eq!(elements.len(), 3);
                assert!(matches!(elements[0].node, Expr::Number(1)));
                assert!(matches!(elements[1].node, Expr::Number(2)));
                assert!(matches!(elements[2].node, Expr::Number(3)));
            }
            _ => panic!("Expected ListLiteral"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_with_complex_expressions() {
    let input = "let f() -> [Int] = [x + 1, y * 2, foo()];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListLiteral { elements } => {
                assert_eq!(elements.len(), 3);
                assert!(matches!(elements[0].node, Expr::Add(_, _)));
                assert!(matches!(elements[1].node, Expr::Mul(_, _)));
                assert!(matches!(elements[2].node, Expr::FnCall { .. }));
            }
            _ => panic!("Expected ListLiteral"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= List Ranges =============

#[test]
fn collection_accepts_lists_range_with_numbers() {
    let input = "let f() -> [Int] = [0..2];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListRange { start, end } => {
                matches!(start.node, Expr::Number(0));
                matches!(end.node, Expr::Number(2));
            }
            _ => panic!("Expected List Range"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn collection_accepts_lists_range_with_expr() {
    let input = "let f() -> [Int] = [f(x)..|@[Student]|];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListRange { start, end } => {
                matches!(start.node, Expr::FnCall { name: _, args: _ });
                matches!(end.node, Expr::Cardinality(_));
            }
            _ => panic!("Expected List Range"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= List Comprehensions =============

#[test]
fn parse_simple_list_comprehension() {
    let input = "let f() -> [Int] = [x for x in @[Student]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension {
                body: expr,
                vars_and_collections,
                filter,
            } => {
                assert!(matches!(expr.node, Expr::Ident(_)));
                assert_eq!(vars_and_collections.len(), 1);
                let (var, collection) = &vars_and_collections[0];
                assert_eq!(var.node, "x");
                assert!(matches!(collection.node, Expr::GlobalList(_)));
                assert!(filter.is_none());
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_expression() {
    let input = "let f() -> [Int] = [x * 2 for x in @[Student]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { body: expr, .. } => {
                assert!(matches!(expr.node, Expr::Mul(_, _)));
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_field_access() {
    let input = "let f() -> [Int] = [s.age for s in @[Student]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension {
                body: expr,
                vars_and_collections,
                ..
            } => {
                assert!(matches!(
                    expr.node,
                    Expr::Path {
                        object: _,
                        segments: _
                    }
                ));
                assert_eq!(vars_and_collections.len(), 1);
                let (var, _collection) = &vars_and_collections[0];
                assert_eq!(var.node, "s");
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_filter() {
    let input = "let f() -> [Int] = [s.age for s in @[Student] where s.age > 18];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { filter, .. } => {
                assert!(filter.is_some());
                match &filter.as_ref().unwrap().node {
                    Expr::Gt(_, _) => {} // Expected
                    _ => panic!("Expected Gt in filter"),
                }
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_complex_filter() {
    let input = "let f() -> [Student] = [s for s in @[Student] where s.age > 18 and s.grade > 10];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { filter, .. } => {
                assert!(filter.is_some());
                assert!(matches!(filter.as_ref().unwrap().node, Expr::And(_, _)));
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_nested_list_comprehension() {
    let input = "let f() -> [[Int]] = [[y for y in x] for x in @[Class]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { body: expr, .. } => {
                assert!(matches!(expr.node, Expr::ListComprehension { .. }));
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_multiple_for() {
    let input = "let f() -> [Int] = [x.age + y.num for x in @[Student] for y in @[Class]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { body: expr, .. } => {
                assert!(matches!(expr.node, Expr::Add { .. }));
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_multiple_dependant_for() {
    let input = "let f() -> [Int] = [x.num for x in y.room for y in @[Class]];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension { body: expr, .. } => {
                assert!(matches!(expr.node, Expr::Path { .. }));
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_list_comprehension_with_multiple_for_and_where_clause() {
    let input = "let f() -> [Int] = [x.age + y.num for x in @[Student] for y in @[Class] where x in y.students];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::ListComprehension {
                body: expr, filter, ..
            } => {
                assert!(matches!(expr.node, Expr::Add { .. }));
                assert!(filter.is_some());
            }
            _ => panic!("Expected ListComprehension"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Global Collections =============

#[test]
fn parse_global_collection() {
    let input = "let f() -> [Student] = @[Student];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::GlobalList(name) => {
                assert_eq!(name.node, TypeName::Object("Student".into()));
            }
            _ => panic!("Expected GlobalList"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_global_collection_with_builtin_type() {
    let input = "let f() -> [Int] = @[Int];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::GlobalList(name) => {
                assert_eq!(name.node, TypeName::Int);
            }
            _ => panic!("Expected GlobalList"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_global_collection_in_expression() {
    let input = "let f() -> Int = |@[Student]|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Cardinality(coll) => {
                assert!(matches!(coll.node, Expr::GlobalList(_)));
            }
            _ => panic!("Expected Cardinality"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Set Operations =============

#[test]
fn parse_union_operation() {
    let input = "let f() -> [Student] = @[Student] union @[Teacher];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Union(left, right) => {
                assert!(matches!(left.node, Expr::GlobalList(_)));
                assert!(matches!(right.node, Expr::GlobalList(_)));
            }
            _ => panic!("Expected Union"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_intersection_operation() {
    let input = "let f() -> [Student] = @[Student] inter @[Athlete];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            assert!(matches!(body.node, Expr::Inter(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_difference_operation() {
    let input = "let f() -> [Student] = @[Student] \\ excluded;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Diff(left, right) => {
                assert!(matches!(left.node, Expr::GlobalList(_)));
                assert!(matches!(right.node, Expr::Ident(_)));
            }
            _ => panic!("Expected Diff"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_chained_union() {
    let input = "let f() -> [Int] = a union b union c;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (a union b) union c (left-associative)
            assert!(matches!(body.node, Expr::Union(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_chained_intersection() {
    let input = "let f() -> [Int] = a inter b inter c;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // Should parse as (a inter b) inter c (left-associative)
            match &body.node {
                Expr::Inter(left, right) => {
                    assert!(matches!(left.node, Expr::Inter(_, _)));
                    assert!(matches!(right.node, Expr::Ident(_)));
                }
                _ => panic!("Expected Inter expr"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_mixed_set_operations() {
    let input = "let f() -> [Int] = a union b inter c;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => {
            // union has precedence over inter
            assert!(matches!(body.node, Expr::Union(_, _)));
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= Membership =============

#[test]
fn parse_in_operator() {
    let input = "let f() -> Bool = x in @[Student];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::In { item, collection } => {
                assert!(matches!(item.node, Expr::Ident(_)));
                assert!(matches!(collection.node, Expr::GlobalList(_)));
            }
            _ => panic!("Expected In"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_in_with_list_literal() {
    let input = "let f() -> Bool = 5 in [1, 2, 3, 4, 5];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::In { item, collection } => {
                assert!(matches!(item.node, Expr::Number(5)));
                assert!(matches!(collection.node, Expr::ListLiteral { .. }));
            }
            _ => panic!("Expected In"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_in_with_complex_collection() {
    let input = "let f() -> Bool = x in @[Student] union @[Teacher];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::In { collection, .. } => {
                assert!(matches!(collection.node, Expr::Union(_, _)));
            }
            _ => panic!("Expected In"),
        },
        _ => panic!("Expected Let statement"),
    }
}

// ============= Cardinality =============

#[test]
fn parse_cardinality_of_global_list() {
    let input = "let f() -> Int = |@[Student]|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Cardinality(coll) => {
                assert!(matches!(coll.node, Expr::GlobalList(_)));
            }
            _ => panic!("Expected Cardinality"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_cardinality_of_list_literal() {
    let input = "let f() -> Int = |[1, 2, 3]|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Cardinality(coll) => {
                assert!(matches!(coll.node, Expr::ListLiteral { .. }));
            }
            _ => panic!("Expected Cardinality"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_cardinality_of_difference() {
    let input = "let f() -> Int = |@[Student] \\ excluded|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Cardinality(coll) => {
                assert!(matches!(coll.node, Expr::Diff(_, _)));
            }
            _ => panic!("Expected Cardinality"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_cardinality_of_list_comprehension() {
    let input = "let f() -> Int = |[s for s in @[Student] where s.age > 18]|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Cardinality(coll) => {
                assert!(matches!(coll.node, Expr::ListComprehension { .. }));
            }
            _ => panic!("Expected Cardinality"),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_cardinality_in_arithmetic() {
    let input = "let f() -> Int = |@[Student]| + |@[Teacher]|;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::Add(left, right) => {
                assert!(matches!(left.node, Expr::Cardinality(_)));
                assert!(matches!(right.node, Expr::Cardinality(_)));
            }
            _ => panic!("Expected Add"),
        },
        _ => panic!("Expected Let statement"),
    }
}
