use super::*;

// =============================================================================
// ARITHMETIC EXPRESSIONS
// =============================================================================
// Tests for arithmetic operations: +, -, *, /, %

#[test]
fn arithmetic_accepts_addition() {
    let cases = vec![
        "5 + 3",
        "10 + 20",
        "x + y",
        "student.age + 5",
        "$V(x) + $V(y)",
        "$V1(x) + $V2(y) + $V3(z)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_accepts_subtraction() {
    let cases = vec!["10 - 7", "20 - 5", "x - y", "$V(x) - 5", "$V1(x) - $V2(y)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_accepts_multiplication() {
    let cases = vec![
        "3 * 4",
        "2 * 5",
        "x * y",
        "2 * $V(x)",
        "student.weight * $Assigned(student)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_accepts_integer_division() {
    let cases = vec![
        "10 / 3",
        "20 / 4",
        "x / 2",
        "student.age / 10",
        "(10 + 5) / 3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_accepts_modulo() {
    let cases = vec![
        "10 % 3",
        "17 % 5",
        "x % 2",
        "week_number % 2",
        "|@[Week]| % 4",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_accepts_combined_operations() {
    let cases = vec![
        "2 * 3 + 4",
        "10 + 2 * 5",
        "x * y + z",
        "(a + b) * (c - d)",
        "5 * 3 / 2",
        "10 / 2 % 3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_accepts_parenthesized_expressions() {
    let cases = vec![
        "(5 + 3)",
        "(10 - 5)",
        "(2 * 3)",
        "((a + b) * c)",
        "(5 + 3) * 2",
        "2 * (4 + 1)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_respects_operator_precedence() {
    // Tests that multiplication/division happen before addition/subtraction
    let cases = vec![
        "1 + 2 * 3",     // Should be 1 + (2 * 3) = 7, not (1 + 2) * 3 = 9
        "10 - 5 - 2",    // Should be (10 - 5) - 2 = 3 (left associative)
        "2 * 3 + 4 * 5", // Should be (2 * 3) + (4 * 5)
        "10 / 2 + 3",    // Should be (10 / 2) + 3
        "10 % 3 + 2",    // Should be (10 % 3) + 2
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_with_variables() {
    let cases = vec![
        "$V(x) + 10",
        "5 + $Var(x)",
        "$Var(x) - 5",
        "2 * $Var(x)",
        "-1 * $StudentInSlot(s, sl, w)",
        "(a + b) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_complex_linear_combinations() {
    let cases = vec![
        "2 * $V1(x) + 3 * $V2(y)",
        "$V1(x) + 2 * $V2(y) - $V3(z)",
        "10 + 2 * $V1(x) - 3 * $V2(y) + 5",
        "-1 * $V1(x) + -1 * $V2(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_with_function_calls() {
    let cases = vec![
        "compute_value(x) + 5",
        "2 * calculate(student, week)",
        "compute_value(x) * 3",
        "student.weight * get_coefficient(x, y)",
        "(a + b) * calculate(x, y)",
        "$Var(x) + 2 * compute_value(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_with_complex_coefficients() {
    let cases = vec![
        "(2 + 3) * $Var(x)",
        "(10 - 5) * $V(y)",
        "(a * 2) * $V(x)",
        "((a + b) * 2) * $Var(x)",
        "(|@[Student]| / 2) * $Var(x)",
        "(week % 4 + 1) * $V(week)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_with_if_expressions() {
    let cases = vec![
        "(if flag { 2 } else { 3 }) * $Var(x)",
        "if x > 0 { x + 5 } else { 0 }",
        "if condition { a * 2 } else { b * 3 }",
        "(if x { 10 } else { 20 }) + 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_with_cardinality() {
    let cases = vec![
        "|@[Student]| + 1",
        "|@[Student]| * $Var(x)",
        "(|@[Student]|) * $Var(x)",
        "|collection| + $Var(x) + |other_collection|",
        "if x { |collection| + 1 } else { |collection| - 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arithmetic_variable_times_variable() {
    // Non-linear expressions - still parse at grammar level
    let cases = vec![
        "$V1(x) * $V2(y)",
        "$Var(x) * $Var(x)",
        "$V(x) * compute_value(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (non-linear): {:?}",
            case,
            result
        );
    }
}

#[test]
fn arithmetic_accepts_single_slash_division() {
    // Single / is used for integer division
    let cases = vec!["10 / 2", "x / y", "20 / 4"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should accept '{}' (division): {:?}",
            case,
            result
        );
    }
}

#[test]
fn arithmetic_rejects_incomplete_expressions() {
    let cases = vec![
        "5 +",
        "+ 5",
        "* 3",
        "2 * ",
        "10 /",
        "x % ",
        "% 2",
        "$Var(x) +",
        "+ $Var(x)",
    ];
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
fn arithmetic_rejets_double_operators() {
    let cases = vec!["5 + + 3", "x * * y", "x / / y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (double operator): {:?}",
            case,
            result
        );
    }
}

#[test]
fn arithmetic_accepts_minus_after_sub() {
    let case = "10 - - 5";
    let result = ColloMLParser::parse(Rule::expr_complete, case);
    assert!(
        result.is_ok(),
        "Should accept '{}' (minus after sub): {:?}",
        case,
        result
    );
}

#[test]
fn arithmetic_accepts_minus_before_ident() {
    let case = "-x";
    let result = ColloMLParser::parse(Rule::expr_complete, case);
    assert!(
        result.is_ok(),
        "Should accept '{}' (minus before ident): {:?}",
        case,
        result
    );
}

#[test]
fn arithmetic_rejects_invalid_operators() {
    let cases = vec!["x & y", "x | y", "x ^ y", "x ** y"];
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
