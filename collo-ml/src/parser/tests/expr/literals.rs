use super::*;

// =============================================================================
// LITERAL EXPRESSIONS
// =============================================================================
// Tests for basic literal values: numbers, booleans, paths

#[test]
fn literal_accepts_positive_integers() {
    let cases = vec!["0", "1", "42", "100", "999"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_negative_integers() {
    let cases = vec!["-1", "-5", "-42", "-100"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_booleans() {
    let cases = vec!["true", "false"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_simple_paths() {
    let cases = vec!["x", "student", "week", "flag", "value", "_private"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_field_access_paths() {
    let cases = vec![
        "student.age",
        "course.duration",
        "room.capacity",
        "student.is_active",
        "obj.field",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_nested_field_access() {
    let cases = vec![
        "x.y.z",
        "student.group.name",
        "course.teacher.name",
        "a.b.c.d.e",
        "student.group.leader.age",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_paths_with_numbers() {
    let cases = vec!["var123", "student1.age", "x.value_1", "obj_2.field_3"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_accepts_paths_with_underscores() {
    let cases = vec![
        "my_variable",
        "student_name",
        "x_y_z",
        "_private.field",
        "obj._internal",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn literal_rejects_paths_starting_with_digit() {
    let cases = vec!["123variable", "1x", "0value"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (starts with digit): {:?}",
            case,
            result
        );
    }
}

#[test]
fn literal_rejects_paths_with_trailing_dot() {
    let cases = vec!["student.", "x.y.", "object.field."];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (trailing dot): {:?}",
            case,
            result
        );
    }
}

#[test]
fn literal_rejects_paths_with_leading_dot() {
    let cases = vec![".student", ".x", ".field"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (leading dot): {:?}",
            case,
            result
        );
    }
}
