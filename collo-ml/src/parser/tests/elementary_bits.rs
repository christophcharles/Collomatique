use super::*;
use pest::Parser;

#[test]
fn fn_call_accepts_valid_calls() {
    let cases = vec![
        "compute(x)",
        "calculate(student, week)",
        "get_value(a, b, c)",
        "my_function(param)",
        "func123(x, y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::fn_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_accepts_no_arguments() {
    let cases = vec!["compute()", "get_value()"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::fn_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_accepts_paths_as_arguments() {
    let cases = vec![
        "compute(student.age)",
        "calculate(course.teacher.name, week.number)",
        "func(a.b.c)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::fn_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_rejects_invalid_syntax() {
    let cases = vec![
        "compute",      // missing parentheses
        "compute[x]",   // wrong brackets
        "compute(x, )", // trailing comma
        "compute(, x)", // leading comma
        "compute(x y)", // missing comma
        "compute x)",   // missing opening paren
        "compute(x",    // missing closing paren
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::fn_call_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_accepts_valid_calls() {
    let cases = vec![
        "$Var(x)",
        "$StudentInSlot(student, slot, week)",
        "$HasSubject(subject, student, week)",
        "$V(a, b, c)",
        "$MyVar123(param)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::var_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_accepts_no_arguments() {
    let cases = vec!["$Var()", "$Flag()"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::var_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_accepts_paths_as_arguments() {
    let cases = vec![
        "$Var(student.age)",
        "$Assigned(course.teacher.name, week.number)",
        "$Value(a.b.c)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::var_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_rejects_missing_dollar_sign() {
    let cases = vec![
        "Var(x)", // missing $
        "StudentInSlot(s, sl, w)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::var_call_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_rejects_invalid_syntax() {
    let cases = vec![
        "$Var",       // missing parentheses
        "$Var[x]",    // wrong brackets
        "$Var(x, )",  // trailing comma
        "$(x)",       // missing identifier after $
        "$123Var(x)", // identifier can't start with digit
        "$Var(x y)",  // missing comma
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::var_call_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_accepts_simple_identifiers() {
    let cases = vec!["x", "student", "week", "my_variable", "var123"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_accepts_field_access() {
    let cases = vec![
        "student.age",
        "course.teacher.name",
        "a.b.c.d.e",
        "obj.field",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_rejects_invalid_syntax() {
    let cases = vec![
        "student.",     // trailing dot
        ".student",     // leading dot
        "student..age", // double dot
        "123student",   // starts with digit
        "student age",  // space in identifier
        "student-age",  // hyphen not allowed
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn input_type_accepts_simple_types() {
    let cases = vec!["Int", "Bool", "Student", "Week", "MyType"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::input_type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn input_type_accepts_list_types() {
    let cases = vec!["[Student]", "[Int]", "[Week]", "[MyType]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::input_type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn input_type_accepts_nested_list_types() {
    let cases = vec!["[[Student]]", "[[[Int]]]", "[[Week]]", "[[MyType]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::input_type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn input_type_rejects_invalid_syntax() {
    let cases = vec![
        "[Student",        // missing closing bracket
        "Student]",        // missing opening bracket
        "[]",              // empty brackets
        "[Student][Week]", // multiple bracket groups
        "Student Week",    // space
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::input_type_name_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}
