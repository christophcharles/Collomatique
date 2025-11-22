use super::*;

// =============================================================================
// COMPARISON EXPRESSIONS
// =============================================================================
// Tests for comparison operators:
// - Regular: ==, !=, <, >, <=, >=
// - Constraint-specific: ===, <==, >==

#[test]
fn comparison_accepts_equality() {
    let cases = vec![
        "x == y",
        "5 == 5",
        "student.age == 18",
        "$V(x) == 1",
        "$V1(x) == $V2(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_inequality() {
    let cases = vec!["x != y", "5 != 10", "name != other_name", "$V(x) != 0"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_less_than() {
    let cases = vec![
        "x < y",
        "5 < 10",
        "student.age < 18",
        "count < 10",
        "$V(x) < 10",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_greater_than() {
    let cases = vec![
        "x > y",
        "10 > 5",
        "student.age > 18",
        "$Var(x) > 5",
        "student.group.size > 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_less_than_or_equal() {
    let cases = vec![
        "x <= y",
        "5 <= 10",
        "a >= b",
        "$Var(x) <= 10",
        "$V1(x) + $V2(y) <= 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_greater_than_or_equal() {
    let cases = vec![
        "x >= y",
        "10 >= 5",
        "$Var(x) >= 0",
        "sum x in @[X] { $V(x) } >= 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_with_arithmetic() {
    let cases = vec![
        "x + 5 > 10",
        "student.age * 2 == 36",
        "|@[Student]| > 0",
        "(a + b) <= (c * 2)",
        "x // 2 == 3",
        "week % 2 == 0",
        "2 * $V1(x) + 3 * $V2(y) <= 10",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// CONSTRAINT-SPECIFIC OPERATORS
// =============================================================================

#[test]
fn comparison_accepts_constraint_equality() {
    let cases = vec![
        "$Var(x) === 10",
        "$V1(x) === $V2(y)",
        "2 * $V(x) === 5",
        "$V(x) + $V(y) === 10",
        "sum x in @[X] { $V(x) } === |@[X]|",
        "5 === 10", // Int coerces to LinExpr
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_constraint_less_equal() {
    let cases = vec![
        "$Var(x) <== 10",
        "$V1(x) + $V2(y) <== 5",
        "2 * $V(x) <== |@[Student]|",
        "sum x in @[X] { $V(x) } <== 100",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_accepts_constraint_greater_equal() {
    let cases = vec![
        "$Var(x) >== 0",
        "$V1(x) - $V2(y) >== -5",
        "sum x in @[X] { $V(x) } >== 1",
        "$V(x) * 2 >== student.min_value",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_distinguishes_constraint_from_regular() {
    // All should parse - semantics determines validity
    let cases = vec![
        "$V(x) === 10", // constraint equality
        "$V(x) == 10",  // regular equality (returns Bool)
        "$V(x) <== 10", // constraint le
        "$V(x) <= 10",  // regular le (returns Bool)
        "$V(x) >== 10", // constraint ge
        "$V(x) >= 10",  // regular ge (returns Bool)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// COMPARISONS IN CONTEXT
// =============================================================================

#[test]
fn comparison_in_aggregations() {
    let cases = vec![
        "forall x in @[X] { $V(x) === 1 }",
        "forall x in @[X] { $V(x) <== 10 }",
        "forall x in @[X] { $V(x) >== 0 }",
        "forall x in @[X] { $V(x) <== 10 and $V(x) >== 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_with_function_calls() {
    let cases = vec![
        "compute_value(x) > 5",
        "get_coefficient(x, y) == 10",
        "$V(x) + compute_value(y) >= 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn comparison_with_if_expressions() {
    let cases = vec![
        "(if flag { $V1(x) } else { $V2(x) }) <= 10",
        "if x > 5 { $V(x) === 1 } else { $V(x) === 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// NEGATIVE TESTS
// =============================================================================

#[test]
fn comparison_rejects_incomplete_expressions() {
    let cases = vec!["x ==", "== y", "x <", "> 5", "$V(x) ===", "<== 10"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (incomplete): {:?}",
            case,
            result
        );
    }
}

#[test]
fn comparison_rejects_invalid_operators() {
    let cases = vec!["x <> y", "x => y", "x =< y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (invalid operator): {:?}",
            case,
            result
        );
    }
}
