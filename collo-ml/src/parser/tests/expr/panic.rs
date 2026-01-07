use super::*;

// =============================================================================
// PANIC EXPRESSION TESTS
// =============================================================================
// Tests for panic! expressions (divergent control flow)

#[test]
fn panic_accepts_simple_int() {
    let cases = vec!["panic! 42", "panic! 0", "panic! -5"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn panic_accepts_string() {
    let cases = vec![
        r#"panic! "error message""#,
        r#"panic! "something went wrong""#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn panic_accepts_complex_expressions() {
    let cases = vec![
        "panic! (x + y)",
        "panic! func()",
        "panic! student.name",
        "panic! [1, 2, 3]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn panic_in_if_branch() {
    let cases = vec![
        "if x > 5 { panic! 42 } else { 0 }",
        "if condition { 10 } else { panic! 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn panic_rejects_missing_expr() {
    let cases = vec!["panic!"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn panic_rejects_missing_bang() {
    // "panic" without ! should be parsed as an identifier, not a panic expression
    // So "panic 42" would try to parse as "panic" (ident) followed by "42" which fails
    let cases = vec!["panic 42"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}
