use super::*;

// ========== Numbers and Paths ==========

#[test]
fn computable_accepts_simple_numbers() {
    let cases = vec!["42", "-17", "0"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_paths() {
    let cases = vec![
        "student.age",
        "course.duration",
        "x.y.z",
        "student.is_active",
        "flag",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Arithmetic Operations ==========

#[test]
fn computable_accepts_arithmetic() {
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
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_cardinality() {
    let cases = vec![
        "|@[Student]|",
        "|pairing|",
        "|subject.slots|",
        "|@[Student]| + 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Comparisons ==========

#[test]
fn computable_accepts_comparisons() {
    let cases = vec![
        "x == y",
        "student.age > 18",
        "count < 10",
        "a >= b",
        "x <= y",
        "name != other_name",
        "5 == 5",
        "student.group.size > 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_arithmetic_in_comparisons() {
    let cases = vec![
        "x + 5 > 10",
        "student.age * 2 == 36",
        "|@[Student]| > 0",
        "(a + b) <= (c * 2)",
        "x // 2 == 3",
        "week % 2 == 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_in_tests() {
    let cases = vec![
        "subject in pairing",
        "student in @[Student]",
        "x in collection",
        "item in (@[Type] \\ excluded)",
        "slot in (morning_slots union afternoon_slots)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Boolean Operations ==========

#[test]
fn computable_accepts_logical_and() {
    let cases = vec![
        "x > 0 and y > 0",
        "x > 0 && y > 0",
        "student.is_active and student.age > 18",
        "a == b and c == d and e == f",
        "x in collection and y > 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_logical_or() {
    let cases = vec![
        "x > 0 or y > 0",
        "x > 0 || y > 0",
        "student.is_french or student.is_german",
        "a == b or c == d or e == f",
        "x in set1 or x in set2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_logical_not() {
    let cases = vec![
        "not x",
        "!x",
        "not student.is_active",
        "!(x > 5)",
        "not (a and b)",
        "not not x",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== If Expressions ==========

#[test]
fn computable_accepts_simple_if() {
    let cases = vec![
        "if x > 5 { 10 } else { 20 }",
        "if student.is_active { 1 } else { 0 }",
        "if flag { 100 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_if_with_paths() {
    let cases = vec![
        "if condition { student.age } else { 0 }",
        "if x { a.value } else { b.value }",
        "if flag { |collection| } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_if_with_arithmetic() {
    let cases = vec![
        "if x > 0 { x + 5 } else { 0 }",
        "if condition { a * 2 } else { b * 3 }",
        "if flag { (x + y) * 2 } else { x - y }",
        "if x { |collection| + 1 } else { |collection| - 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_if_with_boolean_branches() {
    let cases = vec![
        "if flag { x > 5 } else { y > 10 }",
        "if condition { a and b } else { c or d }",
        "if x { student.is_active } else { false_flag }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_nested_if() {
    let cases = vec![
        "if x { if y { 1 } else { 2 } } else { 3 }",
        "if a { 10 } else { if b { 20 } else { 30 } }",
        "if x > 0 { if y > 0 { x + y } else { x } } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_if_in_arithmetic() {
    let cases = vec![
        "(if x { 10 } else { 20 }) + 5",
        "2 * (if flag { a } else { b })",
        "if x { 1 } else { 2 } + if y { 3 } else { 4 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_accepts_complex_conditions_in_if() {
    let cases = vec![
        "if x > 5 and y > 10 { 100 } else { 0 }",
        "if a in collection or b in collection { 1 } else { 0 }",
        "if not flag { x } else { y }",
        "if (x > 0 and y > 0) or z > 0 { 1 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Parentheses ==========

#[test]
fn computable_accepts_parentheses() {
    let cases = vec![
        "(x > 5)",
        "(student.is_active)",
        "((x > 5))",
        "(x > 5) and (y > 5)",
        "(5 + 3)",
        "((a + b) * c)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Complex Combinations ==========

#[test]
fn computable_accepts_complex_combinations() {
    let cases = vec![
        "x > 0 and y > 0 or z > 0",
        "(x > 0 and y > 0) or z > 0",
        "x > 0 and (y > 0 or z > 0)",
        "not (x > 0 and y > 0)",
        "a in set1 and not (b in set2)",
        "x == 5 or (y > 10 and z < 20)",
        "student.is_active and student.age > 18 and student in @[Student]",
        "(a + b) * 2 > 10 and x or y",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Rejections ==========

#[test]
fn computable_rejects_single_slash_division() {
    let result = ColloMLParser::parse(Rule::computable_complete, "10 / 2");
    assert!(result.is_err(), "Should not parse single slash division");
}

#[test]
fn computable_rejects_variables() {
    let cases = vec!["$StudentInSlot(s, sl, w)", "$HasSubject(subj, stud, week)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_rejects_assignment() {
    let cases = vec!["x = 5"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_rejects_incomplete_expressions() {
    let cases = vec![
        "5 +",
        "* 3",
        "(5 + 3",
        "10 //",
        "x // // y",
        "// 5",
        "x >",
        "> 5",
        "x and",
        "or y",
        "not",
        "x in",
        "in collection",
        "x > 5)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_rejects_invalid_operators() {
    let cases = vec!["x & y", "x | y", "x ^ y", "x <> y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_rejects_if_without_else() {
    let cases = vec!["if x > 5 { 10 }", "if condition { value }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing else): {:?}",
            case,
            result
        );
    }
}

#[test]
fn computable_rejects_if_with_mismatched_braces() {
    let cases = vec![
        "if x { 10 } else 20 }",
        "if x { 10 else { 20 }",
        "if x 10 } else { 20 }",
        "if x { 10 } else { 20",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_rejects_if_without_condition() {
    let cases = vec!["if { 10 } else { 20 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn computable_rejects_if_with_empty_branches() {
    let cases = vec![
        "if x > 5 { } else { 20 }",
        "if x > 5 { 10 } else { }",
        "if x > 5 { } else { }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (empty branch): {:?}",
            case,
            result
        );
    }
}

#[test]
fn computable_rejects_function_calls() {
    let cases = vec![
        "compute_value(x)",
        "calculate(student, week)",
        "get_coefficient(a, b, c)",
        "my_function(x, y)",
        "func()",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (function calls not allowed in computable): {:?}",
            case,
            result
        );
    }
}

#[test]
fn computable_rejects_function_calls_in_expressions() {
    let cases = vec![
        "compute_value(x) + 5",
        "2 * calculate(student, week)",
        "if flag { get_value(x) } else { 0 }",
        "|collection| + compute(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::computable_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (function calls not allowed): {:?}",
            case,
            result
        );
    }
}
