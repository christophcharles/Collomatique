use super::*;

// =============================================================================
// STRUCT GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of anonymous structs.
// Struct types: {field1: Type1, field2: Type2}
// Struct literals: {field1: expr1, field2: expr2}
// Field access: struct.field
//
// These are grammar tests only - they do NOT validate semantic correctness.

// =============================================================================
// STRUCT TYPE SYNTAX
// =============================================================================

#[test]
fn struct_type_empty() {
    let result = ColloMLParser::parse(Rule::type_name_complete, "{}");
    assert!(
        result.is_ok(),
        "Empty struct type should parse: {:?}",
        result
    );
}

#[test]
fn struct_type_single_field() {
    let cases = vec![
        "{x: Int}",
        "{name: String}",
        "{flag: Bool}",
        "{value: LinExpr}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_multiple_fields() {
    let cases = vec![
        "{x: Int, y: Int}",
        "{name: String, age: Int}",
        "{x: Int, y: Int, z: Int}",
        "{a: Bool, b: String, c: Int, d: LinExpr}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_with_list_fields() {
    let cases = vec![
        "{items: [Int]}",
        "{names: [String], ages: [Int]}",
        "{matrix: [[Int]]}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_with_tuple_fields() {
    let cases = vec!["{point: (Int, Int)}", "{coords: (Int, Int), name: String}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_nested() {
    let cases = vec![
        "{inner: {x: Int}}",
        "{a: {b: {c: Int}}}",
        "{point: {x: Int, y: Int}, name: String}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_with_union_fields() {
    let cases = vec!["{value: Int | Bool}", "{x: Int | String, y: Bool}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_with_maybe_fields() {
    let cases = vec!["{value: ?Int}", "{name: ?String, age: Int}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_with_trailing_comma() {
    let cases = vec![
        "{x: Int,}",
        "{x: Int, y: Bool,}",
        "{a: Int, b: String, c: Bool,}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_ok(),
            "Should accept '{}' (trailing comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn struct_type_with_whitespace() {
    let cases = vec![
        "{ x: Int }",
        "{  x: Int,  y: Bool  }",
        "{ x : Int , y : Bool }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_type_rejects_missing_colon() {
    let cases = vec!["{x Int}", "{x Int, y Bool}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing colon): {:?}",
            case,
            result
        );
    }
}

#[test]
fn struct_type_rejects_missing_type() {
    let cases = vec!["{x:}", "{x: Int, y:}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing type): {:?}",
            case,
            result
        );
    }
}

#[test]
fn struct_type_rejects_missing_comma() {
    let cases = vec!["{x: Int y: Bool}", "{a: Int b: Bool c: String}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing comma): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// STRUCT LITERAL SYNTAX
// =============================================================================

#[test]
fn struct_literal_empty() {
    let result = ColloMLParser::parse(Rule::expr_complete, "{}");
    assert!(
        result.is_ok(),
        "Empty struct literal should parse: {:?}",
        result
    );
}

#[test]
fn struct_literal_single_field() {
    let cases = vec!["{x: 1}", "{name: \"hello\"}", "{flag: true}", "{value: 42}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_multiple_fields() {
    let cases = vec![
        "{x: 1, y: 2}",
        "{name: \"test\", age: 25}",
        "{a: true, b: false, c: true}",
        "{x: 1, y: 2, z: 3, w: 4}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_with_expressions() {
    let cases = vec![
        "{x: 1 + 2}",
        "{total: 10 + 20, difference: 30 - 5}",
        "{flag: true and false}",
        "{result: if true { 1 } else { 2 }}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_with_list_fields() {
    let cases = vec![
        "{items: [1, 2, 3]}",
        "{names: [\"a\", \"b\"], counts: [1, 2]}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_with_tuple_fields() {
    let cases = vec!["{point: (1, 2)}", "{coord: (10, 20), label: \"origin\"}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_nested() {
    let cases = vec![
        "{inner: {x: 1}}",
        "{a: {b: {c: 42}}}",
        "{point: {x: 10, y: 20}, name: \"p\"}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_with_trailing_comma() {
    let cases = vec!["{x: 1,}", "{x: 1, y: 2,}", "{a: 1, b: 2, c: 3,}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should accept '{}' (trailing comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn struct_literal_with_path_values() {
    let cases = vec!["{x: foo}", "{a: obj.field}", "{x: a.b.c, y: d.e}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_with_function_calls() {
    let cases = vec![
        "{result: compute(1)}",
        "{total: add(1, 2), prod: mul(3, 4)}",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_literal_rejects_missing_colon() {
    let cases = vec!["{x 1}", "{x 1, y 2}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing colon): {:?}",
            case,
            result
        );
    }
}

#[test]
fn struct_literal_rejects_missing_value() {
    let cases = vec!["{x:}", "{x: 1, y:}"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing value): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// STRUCT FIELD ACCESS (via path)
// =============================================================================

#[test]
fn struct_field_access_basic() {
    // This is tested via expressions that include path access
    let cases = vec!["{x: 1}.x", "{a: 1, b: 2}.a", "{name: \"test\"}.name"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn struct_field_access_nested() {
    let cases = vec!["{inner: {x: 1}}.inner.x", "{a: {b: {c: 42}}}.a.b.c"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// TRAILING COMMA TESTS (Additional grammar features)
// =============================================================================

#[test]
fn tuple_type_with_trailing_comma() {
    let cases = vec!["(Int, Bool,)", "(Int, Bool, String,)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::type_name_complete, case);
        assert!(
            result.is_ok(),
            "Should accept '{}' (trailing comma): {:?}",
            case,
            result
        );
    }
}

#[test]
fn tuple_literal_with_trailing_comma() {
    let cases = vec!["(1, 2,)", "(true, false, 42,)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should accept '{}' (trailing comma): {:?}",
            case,
            result
        );
    }
}
