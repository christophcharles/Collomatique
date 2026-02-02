use super::*;

// =============================================================================
// DATABASE SCHEMA TYPE SEMANTIC TESTS
// =============================================================================
// These tests validate the SEMANTIC behavior of database schema types.
// Database schema types: #{ "schema string" }

// =============================================================================
// BASIC TYPE DECLARATIONS
// =============================================================================

#[tokio::test]
async fn database_schema_type_declaration() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[tokio::test]
async fn database_schema_public_type_declaration() {
    let input = r#"
        pub type MyDB = #{ "CREATE TABLE test (id INTEGER PRIMARY KEY)" };
        pub let f() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[tokio::test]
async fn database_schema_multiple_tables() {
    let input = r#"
        type MultiTableDB = #{ "CREATE TABLE t1 (id INTEGER); CREATE TABLE t2 (id INTEGER)" };
        pub let f() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[tokio::test]
async fn database_schema_multiple_string_literals() {
    let input = r#"
        type MultiStringDB = #{
            "CREATE TABLE users (id INTEGER PRIMARY KEY);"
            "CREATE TABLE posts (id INTEGER, user_id INTEGER)"
        };
        pub let f() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// IDENTICAL SCHEMAS ARE EQUAL
// =============================================================================

#[tokio::test]
async fn database_schema_identical_are_equal() {
    // Note: type aliases create distinct Custom types, so DB1 and DB2 are not the same
    // This test verifies that the same type alias works
    let input = r#"
        type DB1 = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: DB1) -> DB1 = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Same type alias should be assignable: {:?}",
        errors
    );
}

#[tokio::test]
async fn database_schema_identical_inline() {
    let input = r#"
        pub let f(x: #{ "CREATE TABLE t (id INTEGER)" }) -> #{ "CREATE TABLE t (id INTEGER)" } = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Identical inline schemas should be equal: {:?}",
        errors
    );
}

// =============================================================================
// DIFFERENT SCHEMAS ARE NOT EQUAL
// =============================================================================

#[tokio::test]
async fn database_schema_different_not_equal() {
    let input = r#"
        type DB1 = #{ "CREATE TABLE test (id INTEGER)" };
        type DB2 = #{ "CREATE TABLE other (id INTEGER)" };
        pub let f(x: DB1) -> DB2 = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "Different schemas should not be equal");
}

#[tokio::test]
async fn database_schema_whitespace_matters() {
    // Even whitespace differences make schemas different
    let input = r#"
        type DB1 = #{ "CREATE TABLE test(id INTEGER)" };
        type DB2 = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: DB1) -> DB2 = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Whitespace differences should make schemas different"
    );
}

// =============================================================================
// DATABASE SCHEMA IN SUM TYPES
// =============================================================================

#[tokio::test]
async fn database_schema_in_sum_type() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        type MaybeDB = MyDB | None;
        pub let f() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Sum type should work: {:?}", errors);
}

#[tokio::test]
async fn database_schema_inline_in_sum_type() {
    let input = r#"
        type MaybeDB = #{ "CREATE TABLE test (id INTEGER)" } | None;
        pub let f() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Inline sum type should work: {:?}",
        errors
    );
}

// =============================================================================
// DATABASE SCHEMA AS FUNCTION PARAMETERS
// =============================================================================

#[tokio::test]
async fn database_schema_as_parameter() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let use_db(db: MyDB) -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Schema as parameter should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn database_schema_inline_as_parameter() {
    let input = r#"
        pub let use_db(db: #{ "CREATE TABLE test (id INTEGER)" }) -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Inline schema as parameter should work: {:?}",
        errors
    );
}

// =============================================================================
// DATABASE SCHEMA NOT SUBTYPE OF OTHER TYPES
// =============================================================================

#[tokio::test]
async fn database_schema_not_subtype_of_int() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: MyDB) -> Int = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Database schema should not be subtype of Int"
    );
}

#[tokio::test]
async fn database_schema_not_subtype_of_string() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: MyDB) -> String = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    // Database schema types are type-level constructs, not runtime values,
    // so they don't convert to String like other types
    assert!(
        !errors.is_empty(),
        "Database schema should not convert to String"
    );
}

#[tokio::test]
async fn database_schema_not_subtype_of_bool() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: MyDB) -> Bool = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Database schema should not be subtype of Bool"
    );
}

#[tokio::test]
async fn int_not_subtype_of_database_schema() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: Int) -> MyDB = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Int should not be subtype of database schema"
    );
}

#[tokio::test]
async fn string_not_subtype_of_database_schema() {
    let input = r#"
        type MyDB = #{ "CREATE TABLE test (id INTEGER)" };
        pub let f(x: String) -> MyDB = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "String should not be subtype of database schema"
    );
}

// =============================================================================
// MULTIPLE DATABASE SCHEMA TYPES
// =============================================================================

#[tokio::test]
async fn multiple_database_schema_types() {
    let input = r#"
        type UsersDB = #{ "CREATE TABLE users (id INTEGER, name TEXT)" };
        type OrdersDB = #{ "CREATE TABLE orders (id INTEGER, user_id INTEGER)" };
        pub let f(u: UsersDB, o: OrdersDB) -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Multiple schema types should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn cannot_mix_different_schemas() {
    let input = r#"
        type UsersDB = #{ "CREATE TABLE users (id INTEGER)" };
        type OrdersDB = #{ "CREATE TABLE orders (id INTEGER)" };
        pub let f(u: UsersDB) -> OrdersDB = u;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Different schemas should not be assignable"
    );
}
