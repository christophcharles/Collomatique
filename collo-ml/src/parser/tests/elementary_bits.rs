use super::*;

// =============================================================================
// ELEMENTARY GRAMMAR COMPONENT TESTS
// =============================================================================
// These tests validate basic building blocks of the language:
// - Function calls: func(args)
// - Variable calls: $Var(args)
// - Paths: x.y.z
// - Type names: Int, [Int], [[Int]]
//
// These are tested in isolation using Rule::expr_complete, Rule::path_complete,
// and Rule::type_name_complete.

// =============================================================================
// FUNCTION CALLS
// =============================================================================

#[test]
fn fn_call_basic_syntax() {
    let cases = vec![
        "compute(x)",
        "calculate(a, b)",
        "get_value(x, y, z)",
        "my_function(param)",
        "func123(x, y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_no_arguments() {
    let cases = vec!["compute()", "get_value()", "func()"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_with_path_arguments() {
    let cases = vec![
        "compute(student.age)",
        "calculate(course.teacher.name, week.number)",
        "func(a.b.c)",
        "process(obj.field.nested)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_with_complex_arguments() {
    // Function calls can have any expression as arguments
    let cases = vec![
        "compute(5 + 3)",
        "func([1, 2, 3])",
        "process(x, y + 1, z * 2)",
        "calculate(if flag { 1 } else { 0 })",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fn_call_rejects_wrong_brackets() {
    let cases = vec![
        "compute[x]", // square brackets
        "compute{x}", // curly braces
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (wrong brackets): {:?}",
            case,
            result
        );
    }
}

#[test]
fn fn_call_rejects_trailing_comma() {
    let cases = vec!["compute(x, )", "func(a, b, )"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (trailing comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn fn_call_rejects_leading_comma() {
    let cases = vec!["compute(, x)", "func(, a, b)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (leading comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn fn_call_rejects_missing_comma() {
    let cases = vec!["compute(x y)", "func(a b c)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn fn_call_rejects_mismatched_parens() {
    let cases = vec![
        "compute x)",  // missing opening paren
        "compute(x",   // missing closing paren
        "compute((x)", // mismatched
        "compute(x))", // extra closing
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (mismatched parens): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// VARIABLE CALLS (REIFIED VARIABLES)
// =============================================================================

#[test]
fn var_call_basic_syntax() {
    let cases = vec![
        "$Var(x)",
        "$StudentInSlot(student, slot, week)",
        "$HasSubject(subject, student, week)",
        "$V(a, b, c)",
        "$MyVar123(param)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_no_arguments() {
    let cases = vec!["$Var()", "$Flag()", "$V()"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_with_path_arguments() {
    let cases = vec![
        "$Var(student.age)",
        "$Assigned(course.teacher.name, week.number)",
        "$Value(a.b.c)",
        "$Check(obj.field.nested)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_with_complex_arguments() {
    // Variable calls can have any expression as arguments
    let cases = vec![
        "$Var(5 + 3)",
        "$V([1, 2, 3])",
        "$Assigned(x, y + 1, z * 2)",
        "$Check(if flag { 1 } else { 0 })",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_with_underscore_in_name() {
    let cases = vec!["$My_Var(x)", "$Student_In_Slot(s, sl)", "$_Private(x)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn var_call_rejects_missing_parentheses() {
    let cases = vec![
        "$Var", // no parens at all
        "$Flag", "$V",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing parentheses): {:?}",
            case,
            result
        );
    }
}

#[test]
fn var_call_rejects_wrong_brackets() {
    let cases = vec![
        "$Var[x]", // square brackets
        "$Var{x}", // curly braces
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (wrong brackets): {:?}",
            case,
            result
        );
    }
}

#[test]
fn var_call_rejects_trailing_comma() {
    let cases = vec!["$Var(x, )", "$V(a, b, )"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (trailing comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn var_call_rejects_missing_identifier() {
    let cases = vec![
        "$(x)",       // missing identifier after $
        "$123Var(x)", // identifier can't start with digit
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (invalid identifier): {:?}",
            case,
            result
        );
    }
}

#[test]
fn var_call_rejects_missing_comma() {
    let cases = vec!["$Var(x y)", "$V(a b c)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing comma): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// PATHS (FIELD ACCESS)
// =============================================================================

#[test]
fn path_simple_identifiers() {
    let cases = vec!["x", "student", "week", "my_variable", "var123", "_private"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_single_field_access() {
    let cases = vec!["student.age", "room.capacity", "obj.field", "x.value"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_multiple_field_access() {
    let cases = vec![
        "student.group.name",
        "course.teacher.name",
        "a.b.c",
        "a.b.c.d.e",
        "x.field.nested.deep",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_with_numbers_and_underscores() {
    let cases = vec!["var123.field", "obj.value_1", "x.y_2.z_3", "_private.field"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn path_rejects_trailing_dot() {
    let cases = vec!["student.", "x.y.", "a.b.c."];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (trailing dot): {:?}",
            case,
            result
        );
    }
}

#[test]
fn path_rejects_leading_dot() {
    let cases = vec![".student", ".x", ".a.b.c"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (leading dot): {:?}",
            case,
            result
        );
    }
}

#[test]
fn path_rejects_double_dot() {
    let cases = vec!["student..age", "x..y", "a.b..c"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (double dot): {:?}",
            case,
            result
        );
    }
}

#[test]
fn path_rejects_starting_with_digit() {
    let cases = vec!["123student", "1x", "0value"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (starts with digit): {:?}",
            case,
            result
        );
    }
}

#[test]
fn path_rejects_spaces() {
    let cases = vec!["student age", "x y", "my variable"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains space): {:?}",
            case,
            result
        );
    }
}

#[test]
fn path_rejects_hyphens() {
    let cases = vec!["student-age", "x-y", "my-variable"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::path_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains hyphen): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// TYPE NAMES
// =============================================================================

#[test]
fn type_primitive_types() {
    let cases = vec!["Int", "Bool", "LinExpr", "Constraint"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_custom_types() {
    let cases = vec!["Student", "Week", "Room", "MyType", "CustomType"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_simple_list_types() {
    let cases = vec!["[Int]", "[Bool]", "[Student]", "[Week]", "[MyType]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_nested_list_types() {
    let cases = vec![
        "[[Int]]",
        "[[Student]]",
        "[[[Int]]]",
        "[[Week]]",
        "[[MyType]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_deeply_nested_list_types() {
    let cases = vec!["[[[[Int]]]]", "[[[[[Student]]]]]", "[[[[[[Bool]]]]]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_rejects_missing_closing_bracket() {
    let cases = vec!["[Student", "[[Int]", "[[[Bool]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing closing bracket): {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_rejects_missing_opening_bracket() {
    let cases = vec!["Student]", "Int]]", "Bool]]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing opening bracket): {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_rejects_empty_brackets() {
    let cases = vec!["[]", "[[]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (empty brackets): {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_rejects_multiple_bracket_groups() {
    let cases = vec!["[Student][Week]", "[Int][Bool]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (multiple bracket groups): {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_rejects_spaces() {
    let cases = vec!["Student Week", "[Student Week]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains space): {:?}",
            case,
            result
        );
    }
}
