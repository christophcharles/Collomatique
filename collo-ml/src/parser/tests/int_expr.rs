use super::*;
use pest::Parser;

#[test]
fn int_expr_accepts_simple_numbers() {
    let cases = vec!["42", "-17", "0"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_paths() {
    let cases = vec!["student.age", "course.duration", "x.y.z"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_arithmetic() {
    let cases = vec![
        "5 + 3",
        "10 - 7",
        "3 * 4",
        "5 + 3 * 2",
        "(5 + 3) * 2",
        "student.age + 5",
        "10 // 3",
        "10 % 3",
        "student.age // 10",
        "week_number % 2",
        "(10 + 5) // 3",
        "|@[Week]| % 4",
        "5 * 3 // 2",
        "10 // 2 % 3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_cardinality() {
    let cases = vec!["|@[Student]|", "|pairing|", "|subject.slots|"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_rejects_division() {
    let result = ColloMLParser::parse(Rule::int_expr_complete, "10 / 2");
    assert!(result.is_err(), "Should not parse division");
}

#[test]
fn int_expr_rejects_variables() {
    let cases = vec!["$StudentInSlot(s, sl, w)", "$HasSubject(subj, stud, week)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_rejects_comparisons() {
    let cases = vec!["5 > 3", "x == y", "a <= b"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_rejects_incomplete() {
    let cases = vec!["5 +", "* 3", "(5 + 3", "10 //", "x // // y", "// 5"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_simple_if() {
    let cases = vec![
        "if x > 5 { 10 } else { 20 }",
        "if student.is_active { 1 } else { 0 }",
        "if flag { 100 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_if_with_paths() {
    let cases = vec![
        "if condition { student.age } else { 0 }",
        "if x { a.value } else { b.value }",
        "if flag { |collection| } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_if_with_arithmetic() {
    let cases = vec![
        "if x > 0 { x + 5 } else { 0 }",
        "if condition { a * 2 } else { b * 3 }",
        "if flag { (x + y) * 2 } else { x - y }",
        "if x { |collection| + 1 } else { |collection| - 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_nested_if() {
    let cases = vec![
        "if x { if y { 1 } else { 2 } } else { 3 }",
        "if a { 10 } else { if b { 20 } else { 30 } }",
        "if x > 0 { if y > 0 { x + y } else { x } } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_if_in_arithmetic() {
    let cases = vec![
        "(if x { 10 } else { 20 }) + 5",
        "2 * (if flag { a } else { b })",
        "if x { 1 } else { 2 } + if y { 3 } else { 4 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_accepts_complex_conditions_in_if() {
    let cases = vec![
        "if x > 5 and y > 10 { 100 } else { 0 }",
        "if a in collection or b in collection { 1 } else { 0 }",
        "if not flag { x } else { y }",
        "if (x > 0 and y > 0) or z > 0 { 1 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_rejects_if_without_else() {
    let cases = vec!["if x > 5 { 10 }", "if condition { value }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing else): {:?}",
            case,
            result
        );
    }
}

#[test]
fn int_expr_rejects_if_with_mismatched_braces() {
    let cases = vec![
        "if x { 10 } else 20 }",
        "if x { 10 else { 20 }",
        "if x 10 } else { 20 }",
        "if x { 10 } else { 20",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_rejects_if_without_condition() {
    let cases = vec!["if { 10 } else { 20 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn int_expr_rejects_if_with_empty_branches() {
    let cases = vec![
        "if x > 5 { } else { 20 }",
        "if x > 5 { 10 } else { }",
        "if x > 5 { } else { }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::int_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (empty branch): {:?}",
            case,
            result
        );
    }
}
