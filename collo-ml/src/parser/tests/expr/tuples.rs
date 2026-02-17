use super::*;

// =============================================================================
// TUPLE EXPRESSION GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of tuple types and literals.
// Tuples include:
// - Tuple types: (Type1, Type2) or (Type1, Type2, Type3, ...)
// - Tuple literals: (expr1, expr2) or (expr1, expr2, expr3, ...)
// - Tuple access: expr.0, expr.1, etc.
//
// These are grammar tests only - they do NOT validate semantic correctness.

// =============================================================================
// TUPLE TYPE SYNTAX
// =============================================================================

#[test]
fn tuple_type_accepts_two_element_types() {
    let cases = vec![
        "let f() -> (Int, Bool) = (1, true);",
        "let g() -> (Bool, Int) = (false, 42);",
        "let h() -> (String, LinExpr) = get_pair();",
        "let i(x: (Int, Int)) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_three_or_more_elements() {
    let cases = vec![
        "let f() -> (Int, Bool, String) = (1, true, \"x\");",
        "let g() -> (Int, Int, Int, Int) = (1, 2, 3, 4);",
        "let h() -> (Bool, Int, String, LinExpr) = get_quad();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_nested_tuples() {
    let cases = vec![
        "let f() -> ((Int, Bool), String) = nested();",
        "let g() -> (Int, (Bool, String)) = nested();",
        "let h() -> ((Int, Int), (Bool, Bool)) = nested();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_lists_inside() {
    let cases = vec![
        "let f() -> ([Int], Bool) = get_pair();",
        "let g() -> (Int, [Bool]) = get_pair();",
        "let h() -> ([Int], [Bool]) = get_pair();",
        "let i() -> ([[Int]], Bool) = get_pair();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_list_of_tuples() {
    let cases = vec![
        "let f() -> [(Int, Bool)] = get_list();",
        "let g() -> [(Int, Int, Int)] = get_list();",
        "let h() -> [[(Int, Bool)]] = get_nested();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_option_tuples() {
    let cases = vec![
        "let f() -> ?(Int, Bool) = get_maybe();",
        "let g() -> (?Int, Bool) = get_pair();",
        "let h() -> (Int, ?Bool) = get_pair();",
        "let i() -> (?Int, ?Bool) = get_pair();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_union_types_inside() {
    let cases = vec![
        "let f() -> (Int | Bool, String) = get_pair();",
        "let g() -> (Int, Bool | String) = get_pair();",
        "let h() -> (Int | Bool, String | LinExpr) = get_pair();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_accepts_custom_objects() {
    let cases = vec![
        "let f() -> (Student, Week) = get_pair();",
        "let g() -> (Student, Int) = get_pair();",
        "let h() -> ([Student], Week) = get_pair();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_type_rejects_single_element() {
    // (Int) doesn't parse as a tuple type - it would need to be written as (Int,)
    // which we don't support. So (Int) as a type is a syntax error.
    let result = ColloMLParser::parse(Rule::tuple_type_complete, "(Int)");
    assert!(result.is_err(), "Should reject single-element tuple type");
}

#[test]
fn tuple_type_rejects_empty() {
    let result = ColloMLParser::parse(Rule::tuple_type_complete, "()");
    assert!(result.is_err(), "Should reject empty tuple type");
}

// =============================================================================
// TUPLE LITERAL SYNTAX
// =============================================================================

#[test]
fn tuple_literal_accepts_two_elements() {
    let cases = vec![
        "(1, 2)",
        "(true, false)",
        "(1, true)",
        "(x, y)",
        "(1 + 2, 3 * 4)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_literal_accepts_three_or_more_elements() {
    let cases = vec![
        "(1, 2, 3)",
        "(true, false, true)",
        "(1, true, \"hello\")",
        "(a, b, c, d)",
        "(1, 2, 3, 4, 5)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_literal_accepts_nested_tuples() {
    let cases = vec![
        "((1, 2), 3)",
        "(1, (2, 3))",
        "((1, 2), (3, 4))",
        "(((1, 2), 3), 4)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_literal_accepts_complex_expressions() {
    let cases = vec![
        "(f(x), g(y))",
        "(1 + 2, 3 - 4)",
        "(if true { 1 } else { 2 }, 3)",
        "([1, 2, 3], [4, 5])",
        "(x.field, y.other)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_literal_accepts_string_elements() {
    let cases = vec![
        "(\"hello\", \"world\")",
        "(1, \"test\")",
        "(\"a\", \"b\", \"c\")",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_literal_accepts_list_elements() {
    let cases = vec![
        "([1, 2], [3, 4])",
        "([], [1])",
        "([1, 2, 3], true)",
        "(x, [y for y in xs])",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_literal_single_element_is_grouping() {
    // (expr) should parse as grouping, not tuple
    let result = ColloMLParser::parse(Rule::expr_complete, "(42)");
    assert!(result.is_ok(), "Should parse '(42)' as grouped expression");

    let result = ColloMLParser::parse(Rule::expr_complete, "(x + y)");
    assert!(
        result.is_ok(),
        "Should parse '(x + y)' as grouped expression"
    );
}

// =============================================================================
// TUPLE ACCESS SYNTAX
// =============================================================================

#[test]
fn tuple_access_accepts_numeric_indices() {
    let cases = vec!["t.0", "t.1", "t.2", "pair.0", "triple.2"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_accepts_chained_indices() {
    let cases = vec!["t.0.0", "t.0.1", "t.1.0", "nested.0.0.0"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_accepts_mixed_with_field_access() {
    let cases = vec![
        "t.0.field",
        "obj.field.0",
        "t.0.field.1",
        "obj.tuples.0.value",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_accepts_on_literal() {
    let cases = vec!["(1, 2).0", "(1, 2).1", "(a, b, c).2", "((1, 2), 3).0.1"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_accepts_on_function_call() {
    let cases = vec![
        "get_pair().0",
        "get_pair().1",
        "make_tuple(x, y).0",
        "nested_call().0.1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_accepts_large_indices() {
    let cases = vec!["t.10", "t.99", "t.100", "large.999"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_in_arithmetic() {
    let cases = vec![
        "t.0 + t.1",
        "pair.0 * pair.1",
        "(1, 2).0 + (3, 4).1",
        "t.0 + 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_access_in_comparison() {
    let cases = vec![
        "t.0 == t.1",
        "pair.0 < pair.1",
        "(1, 2).0 != (3, 4).0",
        "t.0 >= 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// TUPLES IN CONTEXT
// =============================================================================

#[test]
fn tuple_in_let_statement() {
    let cases = vec![
        "let f() -> (Int, Bool) = (1, true);",
        "let g(t: (Int, Int)) -> Int = t.0 + t.1;",
        "let h() -> Int = (1, 2).0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_in_if_expression() {
    let cases = vec![
        "if true { (1, 2) } else { (3, 4) }",
        "if cond { t.0 } else { t.1 }",
        "(if true { 1 } else { 2 }, 3)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_in_list_comprehension() {
    let cases = vec![
        "[t.0 for t in tuples]",
        "[(x, y) for x in xs for y in ys]",
        "[t.0 + t.1 for t in pairs]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_in_sum_expression() {
    let cases = vec![
        "sum t in tuples { t.0 }",
        "sum t in pairs { t.0 + t.1 }",
        "sum t in [(1, 2), (3, 4)] { t.0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_in_forall_expression() {
    let cases = vec![
        "forall t in tuples { t.0 > 0 }",
        "forall t in pairs { t.0 == t.1 }",
        "forall t in [(1, 1), (2, 2)] { t.0 == t.1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_in_match_expression() {
    let cases = vec![
        "match x { v as Int { (v, 0) } v { (0, 0) } }",
        "match t.0 { v { v + 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn tuple_in_let_in_expression() {
    let cases = vec![
        "let t = (1, 2) { t.0 + t.1 }",
        "let x = pair.0 { x * 2 }",
        "let t = make_tuple() { t.0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn list_of_tuples() {
    let cases = vec![
        "[(1, 2), (3, 4)]",
        "[(a, b) for a in xs for b in ys]", // 'as' is a reserved keyword
        "[(1, true), (2, false), (3, true)]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
