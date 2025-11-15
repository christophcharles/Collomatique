use super::*;
use pest::Parser;

#[test]
fn cond_accepts_comparisons() {
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
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_int_expr_in_comparisons() {
    let cases = vec![
        "x + 5 > 10",
        "student.age * 2 == 36",
        "|@[Student]| > 0",
        "(a + b) <= (c * 2)",
        "x // 2 == 3",
        "week % 2 == 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_in_tests() {
    let cases = vec![
        "subject in pairing",
        "student in @[Student]",
        "x in collection",
        "item in (@[Type] \\ excluded)",
        "slot in (morning_slots union afternoon_slots)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_paths_as_boolean_values() {
    let cases = vec![
        "student.is_active",
        "slot.is_available",
        "flag",
        "x.y.z.enabled",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_logical_and() {
    let cases = vec![
        "x > 0 and y > 0",
        "x > 0 && y > 0",
        "student.is_active and student.age > 18",
        "a == b and c == d and e == f",
        "x in collection and y > 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_logical_or() {
    let cases = vec![
        "x > 0 or y > 0",
        "x > 0 || y > 0",
        "student.is_french or student.is_german",
        "a == b or c == d or e == f",
        "x in set1 or x in set2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_logical_not() {
    let cases = vec![
        "not x",
        "!x",
        "not student.is_active",
        "!(x > 5)",
        "not (a and b)",
        "not not x", // double negation
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_parentheses() {
    let cases = vec![
        "(x > 5)",
        "(student.is_active)",
        "((x > 5))",
        "(x > 5) and (y > 5)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_accepts_complex_combinations() {
    let cases = vec![
        "x > 0 and y > 0 or z > 0",
        "(x > 0 and y > 0) or z > 0",
        "x > 0 and (y > 0 or z > 0)",
        "not (x > 0 and y > 0)",
        "a in set1 and not (b in set2)",
        "x == 5 or (y > 10 and z < 20)",
        "student.is_active and student.age > 18 and student in @[Student]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_rejects_assignment() {
    let cases = vec![
        "x = 5", // single = is not comparison
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_rejects_variables() {
    let cases = vec![
        "$StudentInSlot(s, sl, w)",
        "$HasSubject(subj, stud, week) == 1", // This should be in constraints, not conditions
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_rejects_incomplete_expressions() {
    let cases = vec![
        "x >",
        "> 5",
        "x and",
        "or y",
        "not",
        "x in",
        "in collection",
        "(x > 5",
        "x > 5)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_rejects_invalid_operators() {
    let cases = vec![
        "x & y",  // bitwise and
        "x | y",  // bitwise or
        "x ^ y",  // bitwise xor
        "x <> y", // wrong inequality syntax
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn cond_rejects_arithmetic_without_comparison() {
    let cases = vec![
        "x + y",        // arithmetic, not boolean
        "x * 2",        // arithmetic, not boolean
        "|collection|", // cardinality, not boolean
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::cond_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}
