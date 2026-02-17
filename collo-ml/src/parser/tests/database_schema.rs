use super::*;

// =============================================================================
// DATABASE SCHEMA TYPE GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of database schema types.
// Database schema types: #{ "schema string" }
//
// These are grammar tests only - they do NOT validate semantic correctness.

// =============================================================================
// BASIC DATABASE SCHEMA TYPE SYNTAX
// =============================================================================

#[test]
fn database_schema_type_basic() {
    let input = r#"#{ "CREATE TABLE test (id INTEGER PRIMARY KEY)" }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_in_type_statement() {
    let input = r#"type MyDB = #{ "CREATE TABLE test (id INTEGER)" };"#;
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_multi_table() {
    let input = r#"#{ "CREATE TABLE t1 (id INTEGER); CREATE TABLE t2 (id INTEGER)" }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_multiple_strings() {
    // Multiple string literals should be allowed (concatenated)
    let input = r#"#{
        "CREATE TABLE t1 (id INTEGER);"
        "CREATE TABLE t2 (id INTEGER)"
    }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_with_comments_between_strings() {
    // Comments are allowed between string literals
    let input = r#"#{
        "CREATE TABLE users (id INTEGER PRIMARY KEY)"
        // Users table above, orders table below
        "CREATE TABLE orders (id INTEGER, user_id INTEGER)"
    }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_complex_schema() {
    let input = r#"#{
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE
        );"
        "CREATE TABLE posts (
            id INTEGER PRIMARY KEY,
            user_id INTEGER REFERENCES users(id),
            title TEXT NOT NULL,
            content TEXT
        );"
        "CREATE INDEX idx_posts_user ON posts(user_id);"
    }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

// =============================================================================
// DATABASE SCHEMA IN TYPE DECLARATIONS
// =============================================================================

#[test]
fn database_schema_type_declaration_public() {
    let input = r#"pub type MyDB = #{ "CREATE TABLE test (id INTEGER)" };"#;
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_multiple_type_declarations() {
    let input = r#"
        type DB1 = #{ "CREATE TABLE t1 (id INTEGER)" };
        type DB2 = #{ "CREATE TABLE t2 (id INTEGER)" };
    "#;
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

// =============================================================================
// DATABASE SCHEMA IN FUNCTION SIGNATURES
// =============================================================================

#[test]
fn database_schema_as_parameter_type() {
    let input = r#"pub let f(db: #{ "CREATE TABLE t (id INTEGER)" }) -> Bool = true;"#;
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn database_schema_as_return_type() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE t (id INTEGER)" };
        pub let get_schema() -> MyDB = panic! "not implemented";
    "#;
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

// =============================================================================
// DATABASE SCHEMA IN SUM TYPES
// =============================================================================

#[test]
fn database_schema_in_sum_type() {
    let input = r#"type MaybeDB = #{ "CREATE TABLE t (id INTEGER)" } | None;"#;
    let result = ColloMLParser::parse(Rule::file, input);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn database_schema_empty_string() {
    let input = r#"#{ "" }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(
        result.is_ok(),
        "Empty string should be allowed: {:?}",
        result
    );
}

#[test]
fn database_schema_with_raw_string() {
    // Using raw string to include quotes in schema
    let input = r#"#{ ~"CREATE TABLE t (name TEXT DEFAULT "unnamed")"~ }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_ok(), "Raw strings should work: {:?}", result);
}

#[test]
fn database_schema_no_space_between_hash_and_brace() {
    // # { should NOT work (space between)
    let input = r#"# { "schema" }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_err(), "Space between # and {{ should fail");
}

#[test]
fn database_schema_rejects_missing_string() {
    let input = r#"#{ }"#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(
        result.is_err(),
        "Empty schema should be rejected: {:?}",
        result
    );
}

#[test]
fn database_schema_rejects_unclosed_brace() {
    let input = r#"#{ "schema" "#;
    let result = ColloMLParser::parse(Rule::type_name_complete, input);
    assert!(result.is_err(), "Unclosed brace should be rejected");
}
