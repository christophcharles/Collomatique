use super::*;

// =============================================================================
// LOGICAL OPERATIONS
// =============================================================================
// Tests for: and (&&), or (||), not (!)

#[test]
fn logical_accepts_and() {
    let cases = vec![
        "x > 0 and y > 0",
        "x > 0 && y > 0",
        "student.is_active and student.age > 18",
        "a == b and c == d and e == f",
        "x in collection and y > 5",
        "$V1(x) <== 10 and $V2(y) >== 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn logical_accepts_or() {
    let cases = vec![
        "x > 0 or y > 0",
        "x > 0 || y > 0",
        "student.is_french or student.is_german",
        "a == b or c == d or e == f",
        "x in set1 or x in set2",
        "$V1(x) <== 10 or $V2(y) >== 0", // Not valid semantically but should parse
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn logical_accepts_not() {
    let cases = vec![
        "not x",
        "!x",
        "not student.is_active",
        "!(x > 5)",
        "not (a and b)",
        "not not x",
        "not (x == y)",
        "not ($V(x) <== 10)", // Not valid semantically but should parse
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn logical_respects_operator_precedence() {
    let cases = vec![
        "x and y or z",    // (x and y) or z
        "not x and y",     // (not x) and y
        "a < b and c > d", // (a < b) and (c > d)
        "x or y and z",    // x or (y and z)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn logical_accepts_complex_combinations() {
    let cases = vec![
        "x > 0 and y > 0 or z > 0",
        "(x > 0 and y > 0) or z > 0",
        "x > 0 and (y > 0 or z > 0)",
        "not (x > 0 and y > 0)",
        "a in set1 and not (b in set2)",
        "x == 5 or (y > 10 and z < 20)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn logical_with_constraints() {
    let cases = vec![
        "($V(x) === 1) and ($V(y) === 1)",
        "x === y and $V(z) === 1",
        "$V1(x) === 0 and $V2(y) >== 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn logical_rejects_incomplete_expressions() {
    let cases = vec!["x and", "and y", "or y", "not"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}
