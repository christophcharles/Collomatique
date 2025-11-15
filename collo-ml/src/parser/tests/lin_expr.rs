use super::*;

#[test]
fn lin_expr_accepts_simple_variables() {
    let cases = vec![
        "$StudentInSlot(student, slot, week)",
        "$HasSubject(subject, student, week)",
        "$Var(x)",
        "$V(a, b, c)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_constants() {
    let cases = vec!["0", "1", "42", "-5", "-100"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_coefficient_times_variable() {
    let cases = vec![
        "2 * $Var(x)",
        "-1 * $StudentInSlot(s, sl, w)",
        "5 * $HasSubject(subj, stud, week)",
        "student.weight * $Assigned(student)",
        "(a + b) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_addition_and_subtraction() {
    let cases = vec![
        "$V1(x) + $V2(y)",
        "$V1(x) - $V2(y)",
        "$V1(x) + $V2(y) + $V3(z)",
        "$V1(x) - $V2(y) + $V3(z)",
        "5 + $Var(x)",
        "$Var(x) + 10",
        "$Var(x) - 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_complex_linear_combinations() {
    let cases = vec![
        "2 * $V1(x) + 3 * $V2(y)",
        "$V1(x) + 2 * $V2(y) - $V3(z)",
        "10 + 2 * $V1(x) - 3 * $V2(y) + 5",
        "-1 * $V1(x) + -1 * $V2(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_sum() {
    let cases = vec![
        "sum x in @[Student]: $Var(x)",
        "sum slot in subject.slots: $StudentInSlot(student, slot, week)",
        "sum s in collection: 2 * $V(s)",
        "sum x in set: $V1(x) + $V2(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_sum_with_where() {
    let cases = vec![
        "sum x in @[Student] where x.is_active: $Var(x)",
        "sum slot in slots where slot.hour > 8: $StudentInSlot(s, slot, w)",
        "sum x in collection where x.value > 0 and x.flag: $V(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_nested_sums() {
    let cases = vec![
        "sum x in @[X]: sum y in @[Y]: $Var(x, y)",
        "sum student in @[Student]: sum week in @[Week]: $HasSubject(math, student, week)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_sum_in_larger_expression() {
    let cases = vec![
        "(sum x in @[X]: $V(x)) + 10",
        "2 * (sum x in @[X]: $V(x))",
        "(sum x in @[X]: $V(x)) - (sum y in @[Y]: $V(y))",
        "sum x in @[X]: $V(x) + 5", // 5 is outside the sum
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_if_expressions() {
    let cases = vec![
        "if x > 5 { $V1(x) } else { $V2(x) }",
        "if condition { 10 } else { 20 }",
        "if flag { sum x in @[X]: $V(x) } else { 0 }",
        "2 * (if x { $V(x) } else { $W(x) })",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_function_calls() {
    let cases = vec![
        "compute_value(student, week)",
        "get_coefficient(x, y)",
        "calculate(a, b, c)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_parentheses() {
    let cases = vec![
        "($Var(x))",
        "(2 * $Var(x))",
        "($V1(x) + $V2(y))",
        "((($Var(x))))",
        "(sum x in @[X]: $V(x))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_not_on_variables() {
    let cases = vec![
        "1 - $Var(x)", // This is how 'not' is expressed
        "(1 - $HasSubject(subject, student, week))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_rejects_variable_times_variable() {
    let cases = vec![
        "$V1(x) * $V2(y)",   // Non-linear!
        "$Var(x) * $Var(x)", // Non-linear!
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (non-linear): {:?}",
            case,
            result
        );
    }
}

#[test]
fn lin_expr_rejects_division_and_modulo() {
    let cases = vec!["$Var(x) / 2", "$Var(x) // 2", "$Var(x) % 2", "10 / $Var(x)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_rejects_comparisons() {
    let cases = vec!["$Var(x) > 5", "$V1(x) == $V2(y)", "$Var(x) <= 10"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (comparison, not expression): {:?}",
            case,
            result
        );
    }
}

#[test]
fn lin_expr_rejects_logical_operators() {
    let cases = vec![
        "$V1(x) and $V2(y)",
        "$V1(x) or $V2(y)",
        "not $Var(x)", // 'not' is a keyword, not subtraction
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_rejects_incomplete_expressions() {
    let cases = vec![
        "$Var(x) +",
        "+ $Var(x)",
        "2 * ",
        " * $Var(x)",
        "sum x in @[X]:",
        "sum x in @[X] $Var(x)", // missing colon
        "($Var(x)",
        "$Var(x))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_function_calls_rightmost_in_product() {
    let cases = vec![
        // Function call alone (no product)
        "compute_value(student, week)",
        // Function call on the right of multiplication
        "2 * compute_value(student, week)",
        "student.weight * get_coefficient(x, y)",
        "(a + b) * calculate(x, y)",
        "-1 * compute(x)",
        // In larger expressions
        "$Var(x) + 2 * compute_value(y)",
        "5 - student.count * calculate(student)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_rejects_function_calls_leftmost_in_product() {
    let cases = vec![
        // Function call on the left of multiplication (invalid coefficient)
        "compute_value(student, week) * 2",
        "get_coefficient(x, y) * $Var(x)",
        "calculate(x) * student.weight",
        "compute(x) * (a + b)",
        // In larger expressions
        "$Var(x) + compute_value(y) * 2",
        "5 - calculate(student) * student.count",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (function call as coefficient): {:?}",
            case,
            result
        );
    }
}

#[test]
fn lin_expr_accepts_complex_int_expr_as_coefficients() {
    let cases = vec![
        // Simple parenthesized arithmetic
        "(2 + 3) * $Var(x)",
        "(10 - 5) * $V(y)",
        "(a * 2) * $V(x)",
        // If expressions as coefficients
        "(if flag { 2 } else { 3 }) * $Var(x)",
        "(if x > 5 { 10 } else { 20 }) * $V(y)",
        "(if student.is_active { 1 } else { 0 }) * $V(student)",
        // Cardinality as coefficients
        "(|@[Student]|) * $Var(x)",
        "(|collection| + 1) * $V(y)",
        "(|@[Week]| - 1) * $V(w)",
        // Paths as coefficients
        "(student.weight) * $Assigned(student)",
        "(course.credits * 2) * $Selected(course)",
        // Complex nested expressions
        "((a + b) * 2) * $Var(x)",
        "(if x { |collection| } else { 0 }) * $V(y)",
        "(|@[Student]| // 2) * $Var(x)",
        "(week % 4 + 1) * $V(week)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_accepts_complex_int_expr_as_standalone_terms() {
    let cases = vec![
        // Parenthesized expressions as terms
        "$Var(x) + (a + b)",
        "$Var(x) - (10 - 5)",
        "(2 + 3) + $Var(x)",
        // If expressions as terms
        "$Var(x) + if flag { 10 } else { 20 }",
        "if condition { a } else { b } + $Var(x)",
        "$V1(x) - if x > 0 { x } else { 0 }",
        // Cardinality as terms
        "$Var(x) + |@[Student]|",
        "|collection| + $Var(x) + |other_collection|",
        // Mixed complex expressions
        "$Var(x) + if flag { |@[X]| } else { 0 } + $Var(y)",
        "(a + b) + 2 * $Var(x) - (c * d)",
        // Nested if with arithmetic
        "if x > 0 { x + 5 } else { x - 5 } + $Var(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn lin_expr_complex_nested_scenarios() {
    let cases = vec![
        // Complex coefficient with complex term
        "(if x { 2 } else { 3 }) * $V(x) + (if y { 10 } else { 20 })",
        // Multiple terms with complex coefficients
        "(a + b) * $V1(x) + (c - d) * $V2(y) + (|@[Z]|) * $V3(z)",
        // Inside sum with complex expressions
        "sum x in @[X]: (if x.valid { x.weight } else { 0 }) * $V(x)",
        "sum x in @[X]: $V(x) + (|x.items|)",
        // Inside if with complex expressions
        "if flag { (a + b) * $V(x) } else { (c * d) * $V(y) }",
        "if cond { $V(x) + (|@[Y]|) } else { 0 }",
        // Deeply nested
        "((if x { |@[A]| } else { |@[B]| }) + 1) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::lin_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
