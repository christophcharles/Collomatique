use super::*;

// =============================================================================
// CONTROL FLOW EXPRESSIONS
// =============================================================================
// Tests for if-else expressions

#[test]
fn control_flow_accepts_simple_if() {
    let cases = vec![
        "if x > 5 { 10 } else { 20 }",
        "if student.is_active { 1 } else { 0 }",
        "if flag { 100 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_if_with_paths() {
    let cases = vec![
        "if condition { student.age } else { 0 }",
        "if x { a.value } else { b.value }",
        "if flag { |collection| } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_if_with_arithmetic() {
    let cases = vec![
        "if x > 0 { x + 5 } else { 0 }",
        "if condition { a * 2 } else { b * 3 }",
        "if flag { (x + y) * 2 } else { x - y }",
        "if x { |collection| + 1 } else { |collection| - 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_if_with_variables() {
    let cases = vec![
        "if x > 5 { $V1(x) } else { $V2(x) }",
        "if condition { 10 } else { 20 }",
        "if flag { sum x in @[X] { $V(x) } } else { 0 }",
        "2 * (if x { $V(x) } else { $W(x) })",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_if_with_boolean_branches() {
    let cases = vec![
        "if flag { x > 5 } else { y > 10 }",
        "if condition { a and b } else { c or d }",
        "if x { student.is_active } else { false_flag }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_if_with_constraints() {
    let cases = vec![
        "if flag { $V1(x) <= 10 } else { $V2(x) <= 10 }",
        "if x > 5 { $V(x) === 1 } else { $V(x) === 0 }",
        "if condition { forall y in @[Y] { $V(y) >== 0 } } else { $V(z) === 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_nested_if() {
    let cases = vec![
        "if x { if y { 1 } else { 2 } } else { 3 }",
        "if a { 10 } else { if b { 20 } else { 30 } }",
        "if x > 0 { if y > 0 { x + y } else { x } } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_if_in_arithmetic() {
    let cases = vec![
        "(if x { 10 } else { 20 }) + 5",
        "2 * (if flag { a } else { b })",
        "if x { 1 } else { 2 } + if y { 3 } else { 4 }",
        "(if flag { 2 } else { 3 }) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_complex_conditions() {
    let cases = vec![
        "if x > 5 and y > 10 { 100 } else { 0 }",
        "if a in collection or b in collection { 1 } else { 0 }",
        "if not flag { x } else { y }",
        "if (x > 0 and y > 0) or z > 0 { 1 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_rejects_if_without_else() {
    let cases = vec!["if x > 5 { 10 }", "if condition { value }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_rejects_mismatched_braces() {
    let cases = vec![
        "if x { 10 } else 20 }",
        "if x { 10 else { 20 }",
        "if x 10 } else { 20 }",
        "if x { 10 } else { 20",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_rejects_empty_branches() {
    let cases = vec![
        "if x > 5 { } else { 20 }",
        "if x > 5 { 10 } else { }",
        "if x > 5 { } else { }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn control_flow_rejects_missing_condition() {
    let cases = vec!["if { 10 } else { 20 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}
