use super::*;

// =============================================================================
// ENUM SEMANTICS TESTS
// =============================================================================
// These tests verify semantic analysis of enum declarations, qualified type
// expressions, and type checking involving enum types.

// =============================================================================
// ENUM DECLARATION SEMANTICS
// =============================================================================

#[test]
fn enum_decl_creates_types() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let f() -> Result = Result::Ok(42);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
}

#[test]
fn enum_variant_is_subtype_of_root() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let identity(x: Result) -> Result = x;
        pub let test() -> Result = identity(Result::Ok(42));
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Variant should be accepted where root type expected: {:?}",
        errors
    );
}

#[test]
fn enum_variant_return_type() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_ok(x: Int) -> Result::Ok = Result::Ok(x);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow variant return type: {:?}",
        errors
    );
}

#[test]
fn enum_unit_variant() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_none() -> Option = Option::None;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Should allow unit variant: {:?}", errors);
}

#[test]
fn enum_tuple_variant() {
    let input = r#"
        enum Pair = P(Int, Bool);
        pub let make_pair() -> Pair = Pair::P(42, true);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow tuple variant: {:?}",
        errors
    );
}

#[test]
fn enum_struct_variant() {
    let input = r#"
        enum Point = P { x: Int, y: Int };
        pub let make_point() -> Point = Point::P { x: 1, y: 2 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow struct variant: {:?}",
        errors
    );
}

#[test]
fn enum_primitive_variant_names() {
    let input = r#"
        enum MyType = Int(Int) | Bool(Bool) | None;
        pub let test_int() -> MyType = MyType::Int(42);
        pub let test_bool() -> MyType = MyType::Bool(true);
        pub let test_none() -> MyType = MyType::None;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow primitive names as variants: {:?}",
        errors
    );
}

// =============================================================================
// ENUM TYPE CHECKING ERRORS
// =============================================================================

#[test]
fn enum_wrong_argument_type() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let bad() -> Result = Result::Ok("not an int");
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(!errors.is_empty(), "Should reject wrong argument type");
}

#[test]
fn enum_wrong_argument_count() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let bad() -> Result = Result::Ok(1, 2);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(!errors.is_empty(), "Should reject wrong argument count");
}

#[test]
fn enum_unknown_variant() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let bad() -> Result = Result::Unknown(42);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(!errors.is_empty(), "Should reject unknown variant");
}

#[test]
fn enum_root_shadows_primitive_allowed() {
    // Note: Primitive type names (Int, Bool, etc.) are no longer reserved keywords.
    // They are resolved to built-in types in semantics, but can be shadowed by
    // user-defined types. This test verifies parsing succeeds.
    use crate::parser::{ColloMLParser, Rule};
    use pest::Parser;

    let input = "enum Int = A | B;";
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(
        result.is_ok(),
        "Should allow enum name that shadows built-in type"
    );
}

// =============================================================================
// MATCH EXPRESSION WITH ENUMS
// =============================================================================

#[test]
fn enum_match_exhaustive() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let handle(r: Result) -> Int = match r {
            x as Result::Ok { Int(x) }
            _ as Result::Error { 0 }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should accept exhaustive match on enum: {:?}",
        errors
    );
}

#[test]
fn enum_match_non_exhaustive() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let handle(r: Result) -> Int = match r {
            x as Result::Ok { Int(x) }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should reject non-exhaustive match on enum"
    );
}

#[test]
fn enum_match_three_variants() {
    let input = r#"
        enum Color = Red | Green | Blue;
        pub let name(c: Color) -> String = match c {
            _ as Color::Red { "red" }
            _ as Color::Green { "green" }
            _ as Color::Blue { "blue" }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should accept exhaustive match on three variants: {:?}",
        errors
    );
}

// =============================================================================
// QUALIFIED TYPES IN VARIOUS CONTEXTS
// =============================================================================

#[test]
fn qualified_type_in_list() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let oks() -> [Result::Ok] = [Result::Ok(1), Result::Ok(2)];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow qualified type in list: {:?}",
        errors
    );
}

#[test]
fn qualified_type_in_maybe() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let maybe_ok(b: Bool) -> ?Result::Ok = if b { Result::Ok(42) } else { none };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow qualified type in maybe: {:?}",
        errors
    );
}

#[test]
fn qualified_type_in_param() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let extract_ok(x: Result::Ok) -> Int = Int(x);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Should allow qualified type in param: {:?}",
        errors
    );
}

// =============================================================================
// IF EXPRESSION TYPE UNIFICATION
// =============================================================================

#[test]
fn enum_if_expression_unifies_to_root() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let decide(b: Bool) -> Result = if b { Result::Ok(1) } else { Result::Error("no") };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "If branches with different variants should unify to root: {:?}",
        errors
    );
}
