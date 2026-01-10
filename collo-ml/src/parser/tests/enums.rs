use super::*;

// =============================================================================
// ENUM GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of enum declarations and
// qualified type expressions.
//
// Syntax:
//   enum Result = Ok(Int) | Error(String);
//   enum Option = Some(Int) | None;
//   Result::Ok(42), Option::None
//
// These are grammar tests only - they do NOT validate semantic correctness.

// =============================================================================
// ENUM STATEMENT PARSING
// =============================================================================

#[test]
fn enum_statement_basic() {
    let cases = vec![
        "enum Result = Ok(Int) | Error(String);",
        "enum Option = Some(Int) | None;",
        "enum Color = Red | Green | Blue;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn enum_statement_single_variant() {
    let cases = vec!["enum Wrapper = Value(Int);", "enum Unit = Empty;"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn enum_statement_tuple_variants() {
    let cases = vec![
        "enum Pair = P(Int, Int);",
        "enum Triple = T(Int, Bool, String);",
        "enum Mixed = A(Int) | B(Int, Bool) | C(Int, Bool, String);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn enum_statement_struct_variants() {
    let cases = vec![
        "enum Point = P { x: Int, y: Int };",
        "enum Mixed = Simple(Int) | Complex { x: Int, y: Bool };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn enum_statement_primitive_variant_names() {
    // Primitive type names should be allowed as variant names
    let cases = vec![
        "enum MyType = Int(Int) | Bool(Bool);",
        "enum Wrapper = None | String(String);",
        "enum Primitive = Int | Bool | String | None;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(
            result.is_ok(),
            "Primitive names as variants should parse '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn enum_statement_with_trailing_comma_in_tuple() {
    let cases = vec!["enum E = A(Int,);", "enum E = A(Int, Bool,);"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(
            result.is_ok(),
            "Trailing comma in tuple variant should parse '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn enum_statement_with_complex_types() {
    let cases = vec![
        "enum E = A([Int]) | B(?String);",
        "enum E = List([Int]) | Tuple((Int, Bool));",
        "enum E = Nested({ x: Int, y: [Bool] });",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn enum_statement_with_pub_modifier() {
    // Enum statements can have pub modifier for module visibility
    let cases = vec![
        "pub enum Result = Ok(Int) | Error(String);",
        "pub enum Option = Some(Int) | None;",
        "pub enum Color = Red | Green | Blue;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::enum_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should accept pub modifier '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn enum_statement_rejects_missing_equals() {
    let result = ColloMLParser::parse(Rule::enum_statement_complete, "enum Result Ok | Error;");
    assert!(result.is_err(), "Should reject missing '='");
}

#[test]
fn enum_statement_rejects_missing_semicolon() {
    let result = ColloMLParser::parse(Rule::enum_statement_complete, "enum Result = Ok | Error");
    assert!(result.is_err(), "Should reject missing ';'");
}

#[test]
fn enum_statement_rejects_empty_variants() {
    let result = ColloMLParser::parse(Rule::enum_statement_complete, "enum Result = ;");
    assert!(result.is_err(), "Should reject empty variants");
}

// =============================================================================
// QUALIFIED TYPE CAST EXPRESSIONS
// =============================================================================

#[test]
fn qualified_type_cast_basic() {
    let cases = vec![
        "Result::Ok(42)",
        "Result::Error(\"oops\")",
        "Option::Some(1)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_type_cast_tuple_variant() {
    let cases = vec!["Pair::P(1, 2)", "Triple::T(1, true, \"hello\")"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_type_cast_unit_variant() {
    let cases = vec![
        "Option::None",
        "Option::None()",
        "Option::None(none)",
        "Color::Red",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_struct_cast() {
    let cases = vec!["Point::P { x: 1, y: 2 }", "MyEnum::Struct { field: 42 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_type_cast_with_expressions() {
    let cases = vec![
        "Result::Ok(1 + 2)",
        "Result::Ok(if true { 1 } else { 2 })",
        "Pair::P(x, y + 1)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// QUALIFIED TYPE NAMES IN TYPE ANNOTATIONS
// =============================================================================

#[test]
fn qualified_type_in_type_annotation() {
    // Note: In type annotations, qualified names use regular idents, not variant_name
    // So primitive type names like "None" are not valid here (they're only valid in expressions)
    let cases = vec![
        "Result::Ok",
        "Result::Error",
        "Option::Some",
        "MyEnum::Variant",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::qualified_type_name_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_type_in_maybe_type() {
    // This should parse as a type name
    let result = ColloMLParser::parse(Rule::type_name_complete, "?Result::Ok");
    assert!(
        result.is_ok(),
        "Should parse maybe qualified type: {:?}",
        result
    );
}

#[test]
fn qualified_type_in_list_type() {
    let result = ColloMLParser::parse(Rule::type_name_complete, "[Result::Ok]");
    assert!(
        result.is_ok(),
        "Should parse list of qualified type: {:?}",
        result
    );
}

// =============================================================================
// VARIANT NAME RULE
// =============================================================================

#[test]
fn variant_name_accepts_regular_idents() {
    let cases = vec!["Ok", "Error", "Some", "Value", "MyVariant"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn variant_name_accepts_none() {
    // None is special - it's a keyword but allowed as variant name
    // We test this through enum parsing
    let result = ColloMLParser::parse(Rule::enum_statement_complete, "enum E = None;");
    assert!(
        result.is_ok(),
        "Should parse 'None' as variant name: {:?}",
        result
    );
}
