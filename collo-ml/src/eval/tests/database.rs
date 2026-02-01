use std::collections::BTreeMap;

use super::*;
use crate::eval::database::{DatabaseHandle, DbValue, SqlQueryError, SqliteDatabaseConnection};
use crate::eval::values::{CustomValue, NoObject};
use crate::semantics::database::DbConversionError;

// =============================================================================
// HELPER
// =============================================================================

/// Build a CheckedAST (and therefore a GlobalEnv) from DSL source.
fn checked(input: &str) -> CheckedAST {
    CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new()).expect("Should compile")
}

fn empty_ast() -> CheckedAST {
    checked("")
}

// =============================================================================
// 5. to_expr_value — PRIMITIVE CONVERSIONS
// =============================================================================

#[test]
fn to_expr_value_int() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Int);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::Int(42)));
}

#[test]
fn to_expr_value_bool() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Bool);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Bool(true).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::Bool(true)));
}

#[test]
fn to_expr_value_string() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::String);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::String("hi".to_string()).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::String("hi".to_string())));
}

#[test]
fn to_expr_value_null_to_none() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::None);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Null.to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::None));
}

// =============================================================================
// 6. to_expr_value — NULLABLE TARGETS
// =============================================================================

#[test]
fn to_expr_value_null_into_nullable_int() {
    let ast = empty_ast();
    let target = ExprType::from_variants([SimpleType::Int, SimpleType::None]);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Null.to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::None));
}

#[test]
fn to_expr_value_int_into_nullable_int() {
    let ast = empty_ast();
    let target = ExprType::from_variants([SimpleType::Int, SimpleType::None]);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::Int(42)));
}

// =============================================================================
// 7. to_expr_value — TYPE MISMATCH REJECTIONS
// =============================================================================

#[test]
fn to_expr_value_int_into_bool_rejected() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Bool);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Err(DbConversionError));
}

#[test]
fn to_expr_value_int_zero_to_bool() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Bool);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(0).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::Bool(false)));
}

#[test]
fn to_expr_value_int_one_to_bool() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Bool);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(1).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Ok(ExprValue::Bool(true)));
}

#[test]
fn to_expr_value_int_two_into_bool_rejected() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Bool);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(2).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Err(DbConversionError));
}

#[test]
fn to_expr_value_int_into_string_rejected() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::String);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Err(DbConversionError));
}

#[test]
fn to_expr_value_null_into_int_rejected() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Int);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Null.to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Err(DbConversionError));
}

#[test]
fn to_expr_value_bool_into_int_rejected() {
    let ast = empty_ast();
    let target = ExprType::simple(SimpleType::Int);
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Bool(true).to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Err(DbConversionError));
}

// =============================================================================
// 8. to_expr_value — CUSTOM TYPE WRAPPING
// =============================================================================

#[test]
fn to_expr_value_custom_type_wraps_int() {
    let ast = checked("type MyInt = Int;");
    let target = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MyInt".to_string(),
        None,
    ));
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(
        result,
        Ok(ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(42),
        })))
    );
}

#[test]
fn to_expr_value_custom_type_null_into_non_nullable_rejected() {
    let ast = checked("type MyInt = Int;");
    let target = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MyInt".to_string(),
        None,
    ));
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Null.to_expr_value(&ast.global_env, &target);
    assert_eq!(result, Err(DbConversionError));
}

#[test]
fn to_expr_value_custom_nullable_int() {
    let ast = checked("type MaybeInt = Int | None;");
    let target = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MaybeInt".to_string(),
        None,
    ));
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(
        result,
        Ok(ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MaybeInt".to_string(),
            variant: None,
            content: ExprValue::Int(42),
        })))
    );
}

#[test]
fn to_expr_value_custom_nullable_null() {
    let ast = checked("type MaybeInt = Int | None;");
    let target = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MaybeInt".to_string(),
        None,
    ));
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Null.to_expr_value(&ast.global_env, &target);
    assert_eq!(
        result,
        Ok(ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MaybeInt".to_string(),
            variant: None,
            content: ExprValue::None,
        })))
    );
}

// =============================================================================
// 9. to_expr_value — NESTED CUSTOM TYPES
// =============================================================================

#[test]
fn to_expr_value_nested_custom_types() {
    let ast = checked("type MyInt = Int; type Deep = MyInt;");
    let target = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "Deep".to_string(),
        None,
    ));
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(
        result,
        Ok(ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Deep".to_string(),
            variant: None,
            content: ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: ExprValue::Int(42),
            })),
        })))
    );
}

// =============================================================================
// 10. to_expr_value — ENUM VARIANT
// =============================================================================

#[test]
fn to_expr_value_enum_variant() {
    let ast = checked("enum Wrapper = A(Int);");
    let target = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "Wrapper".to_string(),
        Some("A".to_string()),
    ));
    let result: Result<ExprValue<NoObject>, _> =
        DbValue::Int(42).to_expr_value(&ast.global_env, &target);
    assert_eq!(
        result,
        Ok(ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Wrapper".to_string(),
            variant: Some("A".to_string()),
            content: ExprValue::Int(42),
        })))
    );
}

// =============================================================================
// 11. TryFrom<ExprValue> for DbValue — SUCCESS CASES
// =============================================================================

#[test]
fn try_from_expr_value_int() {
    let val: ExprValue<NoObject> = ExprValue::Int(42);
    assert_eq!(DbValue::try_from(val), Ok(DbValue::Int(42)));
}

#[test]
fn try_from_expr_value_bool() {
    let val: ExprValue<NoObject> = ExprValue::Bool(true);
    assert_eq!(DbValue::try_from(val), Ok(DbValue::Bool(true)));
}

#[test]
fn try_from_expr_value_string() {
    let val: ExprValue<NoObject> = ExprValue::String("hi".to_string());
    assert_eq!(
        DbValue::try_from(val),
        Ok(DbValue::String("hi".to_string()))
    );
}

#[test]
fn try_from_expr_value_none() {
    let val: ExprValue<NoObject> = ExprValue::None;
    assert_eq!(DbValue::try_from(val), Ok(DbValue::Null));
}

#[test]
fn try_from_expr_value_custom_unwraps() {
    let val: ExprValue<NoObject> = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "MyInt".to_string(),
        variant: None,
        content: ExprValue::Int(42),
    }));
    assert_eq!(DbValue::try_from(val), Ok(DbValue::Int(42)));
}

#[test]
fn try_from_expr_value_nested_custom_unwraps() {
    let val: ExprValue<NoObject> = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "Deep".to_string(),
        variant: None,
        content: ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(42),
        })),
    }));
    assert_eq!(DbValue::try_from(val), Ok(DbValue::Int(42)));
}

// =============================================================================
// 12. TryFrom<ExprValue> for DbValue — REJECTION CASES
// =============================================================================

#[test]
fn try_from_expr_value_list_rejected() {
    let val: ExprValue<NoObject> = ExprValue::List(vec![ExprValue::Int(1)]);
    assert_eq!(DbValue::try_from(val), Err(DbConversionError));
}

#[test]
fn try_from_expr_value_tuple_rejected() {
    let val: ExprValue<NoObject> = ExprValue::Tuple(vec![ExprValue::Int(1)]);
    assert_eq!(DbValue::try_from(val), Err(DbConversionError));
}

#[test]
fn try_from_expr_value_struct_rejected() {
    let val: ExprValue<NoObject> =
        ExprValue::Struct([("x".to_string(), ExprValue::Int(1))].into_iter().collect());
    assert_eq!(DbValue::try_from(val), Err(DbConversionError));
}

// =============================================================================
// 13. ROUNDTRIP: ExprValue → DbValue → to_expr_value
// =============================================================================

#[test]
fn roundtrip_int() {
    let ast = empty_ast();
    let original: ExprValue<NoObject> = ExprValue::Int(7);
    let target = ExprType::simple(SimpleType::Int);
    let db = DbValue::try_from(original.clone()).expect("Should convert to DbValue");
    let recovered: ExprValue<NoObject> = db
        .to_expr_value(&ast.global_env, &target)
        .expect("Should convert back");
    assert_eq!(recovered, original);
}

#[test]
fn roundtrip_bool() {
    let ast = empty_ast();
    let original: ExprValue<NoObject> = ExprValue::Bool(true);
    let target = ExprType::simple(SimpleType::Bool);
    let db = DbValue::try_from(original.clone()).expect("Should convert to DbValue");
    let recovered: ExprValue<NoObject> = db
        .to_expr_value(&ast.global_env, &target)
        .expect("Should convert back");
    assert_eq!(recovered, original);
}

#[test]
fn roundtrip_string() {
    let ast = empty_ast();
    let original: ExprValue<NoObject> = ExprValue::String("hello".to_string());
    let target = ExprType::simple(SimpleType::String);
    let db = DbValue::try_from(original.clone()).expect("Should convert to DbValue");
    let recovered: ExprValue<NoObject> = db
        .to_expr_value(&ast.global_env, &target)
        .expect("Should convert back");
    assert_eq!(recovered, original);
}

#[test]
fn roundtrip_none() {
    let ast = empty_ast();
    let original: ExprValue<NoObject> = ExprValue::None;
    let target = ExprType::simple(SimpleType::None);
    let db = DbValue::try_from(original.clone()).expect("Should convert to DbValue");
    let recovered: ExprValue<NoObject> = db
        .to_expr_value(&ast.global_env, &target)
        .expect("Should convert back");
    assert_eq!(recovered, original);
}

// =============================================================================
// 14. QUERY EXECUTION — INTEGRATION TESTS
// =============================================================================

async fn test_pool() -> sqlx::SqlitePool {
    sqlx::SqlitePool::connect(":memory:").await.unwrap()
}

#[tokio::test]
async fn query_single_table() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob')")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let (rows, cols) = conn
        .query("SELECT id, name FROM users ORDER BY id", vec![])
        .await
        .unwrap();

    assert_eq!(cols, vec!["id".to_string(), "name".to_string()]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], DbValue::Int(1));
    assert_eq!(rows[0]["name"], DbValue::String("Alice".to_string()));
    assert_eq!(rows[1]["id"], DbValue::Int(2));
    assert_eq!(rows[1]["name"], DbValue::String("Bob".to_string()));
}

#[tokio::test]
async fn query_with_bind_param() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob')")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let (rows, _cols) = conn
        .query("SELECT name FROM users WHERE id = ?", vec![DbValue::Int(1)])
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], DbValue::String("Alice".to_string()));
}

#[tokio::test]
async fn query_join_two_tables() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE orders (id INTEGER, user_id INTEGER, product TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO orders VALUES (1, 1, 'Widget'), (2, 2, 'Gadget')")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let (rows, cols) = conn
        .query(
            "SELECT u.name, o.product FROM users u JOIN orders o ON u.id = o.user_id ORDER BY o.id",
            vec![],
        )
        .await
        .unwrap();

    assert_eq!(cols, vec!["name".to_string(), "product".to_string()]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], DbValue::String("Alice".to_string()));
    assert_eq!(rows[0]["product"], DbValue::String("Widget".to_string()));
    assert_eq!(rows[1]["name"], DbValue::String("Bob".to_string()));
    assert_eq!(rows[1]["product"], DbValue::String("Gadget".to_string()));
}

#[tokio::test]
async fn query_duplicate_column_error() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users VALUES (1, 'Alice')")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let result = conn.query("SELECT id, id FROM users", vec![]).await;

    assert_eq!(
        result,
        Err(SqlQueryError::DuplicateColumnName("id".to_string()))
    );
}

#[tokio::test]
async fn query_empty_result() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let (rows, cols) = conn
        .query("SELECT id, name FROM users WHERE id = 999", vec![])
        .await
        .unwrap();

    assert_eq!(rows.len(), 0);
    assert!(cols.is_empty());
}

#[tokio::test]
async fn query_null_values() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE data (id INTEGER, val TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO data VALUES (1, NULL)")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let (rows, cols) = conn
        .query("SELECT id, val FROM data", vec![])
        .await
        .unwrap();

    assert_eq!(cols, vec!["id".to_string(), "val".to_string()]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], DbValue::Int(1));
    assert_eq!(rows[0]["val"], DbValue::Null);
}

#[tokio::test]
async fn query_write_rejected() {
    let pool = test_pool().await;
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    let conn = SqliteDatabaseConnection::new_sqlite("test", &pool)
        .await
        .unwrap();

    let result = conn
        .query("INSERT INTO users VALUES (1, 'Alice')", vec![])
        .await;

    assert!(
        matches!(result, Err(SqlQueryError::QueryFailed(_))),
        "Expected QueryFailed for write on read-only connection, got: {:?}",
        result
    );
}

// =============================================================================
// 15. TYPED QUERY — DatabaseHandle::query
// =============================================================================

async fn setup_users_table(pool: &sqlx::SqlitePool) {
    sqlx::query("CREATE TABLE users (id INTEGER, name TEXT)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob')")
        .execute(pool)
        .await
        .unwrap();
}

async fn test_handle(pool: &sqlx::SqlitePool) -> DatabaseHandle {
    SqliteDatabaseConnection::new("test", pool).await.unwrap()
}

#[tokio::test]
async fn typed_query_list_of_structs() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // [{id: Int, name: String}]
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Struct(
        [
            ("id".to_string(), ExprType::simple(SimpleType::Int)),
            ("name".to_string(), ExprType::simple(SimpleType::String)),
        ]
        .into_iter()
        .collect(),
    ))));

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users ORDER BY id",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    let expected = ExprValue::List(vec![
        ExprValue::Struct(
            [
                ("id".to_string(), ExprValue::Int(1)),
                ("name".to_string(), ExprValue::String("Alice".to_string())),
            ]
            .into_iter()
            .collect(),
        ),
        ExprValue::Struct(
            [
                ("id".to_string(), ExprValue::Int(2)),
                ("name".to_string(), ExprValue::String("Bob".to_string())),
            ]
            .into_iter()
            .collect(),
        ),
    ]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_optional_struct_found() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // {id: Int, name: String} | None
    let out_type = ExprType::from_variants([
        SimpleType::Struct(
            [
                ("id".to_string(), ExprType::simple(SimpleType::Int)),
                ("name".to_string(), ExprType::simple(SimpleType::String)),
            ]
            .into_iter()
            .collect(),
        ),
        SimpleType::None,
    ]);

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users WHERE id = 1",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    let expected = ExprValue::Struct(
        [
            ("id".to_string(), ExprValue::Int(1)),
            ("name".to_string(), ExprValue::String("Alice".to_string())),
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_optional_struct_not_found() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // {id: Int, name: String} | None
    let out_type = ExprType::from_variants([
        SimpleType::Struct(
            [
                ("id".to_string(), ExprType::simple(SimpleType::Int)),
                ("name".to_string(), ExprType::simple(SimpleType::String)),
            ]
            .into_iter()
            .collect(),
        ),
        SimpleType::None,
    ]);

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users WHERE id = 999",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    assert_eq!(result, ExprValue::None);
}

#[tokio::test]
async fn typed_query_empty_list() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // [{id: Int}]
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Struct(
        [("id".to_string(), ExprType::simple(SimpleType::Int))]
            .into_iter()
            .collect(),
    ))));

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id FROM users WHERE id = 999",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    assert_eq!(result, ExprValue::List(vec![]));
}

#[tokio::test]
async fn typed_query_custom_wrapped_rows() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = checked("type Row = {id: Int, name: String};");

    // [Row] where Row = {id: Int, name: String}
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "Row".to_string(),
        None,
    ))));

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users ORDER BY id",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    let expected = ExprValue::List(vec![
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Row".to_string(),
            variant: None,
            content: ExprValue::Struct(
                [
                    ("id".to_string(), ExprValue::Int(1)),
                    ("name".to_string(), ExprValue::String("Alice".to_string())),
                ]
                .into_iter()
                .collect(),
            ),
        })),
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Row".to_string(),
            variant: None,
            content: ExprValue::Struct(
                [
                    ("id".to_string(), ExprValue::Int(2)),
                    ("name".to_string(), ExprValue::String("Bob".to_string())),
                ]
                .into_iter()
                .collect(),
            ),
        })),
    ]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_custom_wrapped_fields() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = checked("type MyInt = Int;");

    // [{id: MyInt, name: String}]
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Struct(
        [
            (
                "id".to_string(),
                ExprType::simple(SimpleType::Custom(
                    "main".to_string(),
                    "MyInt".to_string(),
                    None,
                )),
            ),
            ("name".to_string(), ExprType::simple(SimpleType::String)),
        ]
        .into_iter()
        .collect(),
    ))));

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users ORDER BY id",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    let expected = ExprValue::List(vec![
        ExprValue::Struct(
            [
                (
                    "id".to_string(),
                    ExprValue::Custom(Box::new(CustomValue {
                        module: "main".to_string(),
                        type_name: "MyInt".to_string(),
                        variant: None,
                        content: ExprValue::Int(1),
                    })),
                ),
                ("name".to_string(), ExprValue::String("Alice".to_string())),
            ]
            .into_iter()
            .collect(),
        ),
        ExprValue::Struct(
            [
                (
                    "id".to_string(),
                    ExprValue::Custom(Box::new(CustomValue {
                        module: "main".to_string(),
                        type_name: "MyInt".to_string(),
                        variant: None,
                        content: ExprValue::Int(2),
                    })),
                ),
                ("name".to_string(), ExprValue::String("Bob".to_string())),
            ]
            .into_iter()
            .collect(),
        ),
    ]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_column_mismatch() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // [{id: Int, wrong_col: String}] — wrong_col doesn't exist in SELECT result
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Struct(
        [
            ("id".to_string(), ExprType::simple(SimpleType::Int)),
            (
                "wrong_col".to_string(),
                ExprType::simple(SimpleType::String),
            ),
        ]
        .into_iter()
        .collect(),
    ))));

    let result: Result<ExprValue<NoObject>, _> = handle
        .query(
            "SELECT id, name FROM users",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await;

    assert!(
        matches!(result, Err(SqlQueryError::ColumnMismatch(_))),
        "Expected ColumnMismatch, got: {:?}",
        result
    );
}

#[tokio::test]
async fn typed_query_column_count_mismatch() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // [{id: Int}] — only 1 field but SELECT returns 2 columns
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Struct(
        [("id".to_string(), ExprType::simple(SimpleType::Int))]
            .into_iter()
            .collect(),
    ))));

    let result: Result<ExprValue<NoObject>, _> = handle
        .query(
            "SELECT id, name FROM users",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await;

    assert!(
        matches!(result, Err(SqlQueryError::ColumnMismatch(_))),
        "Expected ColumnMismatch, got: {:?}",
        result
    );
}

#[tokio::test]
async fn typed_query_param_conversion() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // [{id: Int, name: String}]
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Struct(
        [
            ("id".to_string(), ExprType::simple(SimpleType::Int)),
            ("name".to_string(), ExprType::simple(SimpleType::String)),
        ]
        .into_iter()
        .collect(),
    ))));

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users WHERE id = ?",
            vec![ExprValue::Int(1)],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    let expected = ExprValue::List(vec![ExprValue::Struct(
        [
            ("id".to_string(), ExprValue::Int(1)),
            ("name".to_string(), ExprValue::String("Alice".to_string())),
        ]
        .into_iter()
        .collect(),
    )]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_tuple_output() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // [(Int, String)]
    let out_type = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Tuple(vec![
        ExprType::simple(SimpleType::Int),
        ExprType::simple(SimpleType::String),
    ]))));

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users ORDER BY id",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    let expected = ExprValue::List(vec![
        ExprValue::Tuple(vec![
            ExprValue::Int(1),
            ExprValue::String("Alice".to_string()),
        ]),
        ExprValue::Tuple(vec![
            ExprValue::Int(2),
            ExprValue::String("Bob".to_string()),
        ]),
    ]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_optional_takes_first_row() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // {id: Int, name: String} | None — multiple rows, should take first
    let out_type = ExprType::from_variants([
        SimpleType::Struct(
            [
                ("id".to_string(), ExprType::simple(SimpleType::Int)),
                ("name".to_string(), ExprType::simple(SimpleType::String)),
            ]
            .into_iter()
            .collect(),
        ),
        SimpleType::None,
    ]);

    let result: ExprValue<NoObject> = handle
        .query(
            "SELECT id, name FROM users ORDER BY id",
            vec![],
            out_type,
            &ast.global_env,
        )
        .await
        .unwrap();

    // Should return first row (Alice), ignoring Bob
    let expected = ExprValue::Struct(
        [
            ("id".to_string(), ExprValue::Int(1)),
            ("name".to_string(), ExprValue::String("Alice".to_string())),
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(result, expected);
}

#[tokio::test]
async fn typed_query_invalid_output_type() {
    let pool = test_pool().await;
    setup_users_table(&pool).await;
    let handle = test_handle(&pool).await;
    let ast = empty_ast();

    // Int — not a list or optional
    let out_type = ExprType::simple(SimpleType::Int);

    let result: Result<ExprValue<NoObject>, _> = handle
        .query("SELECT id FROM users", vec![], out_type, &ast.global_env)
        .await;

    assert!(
        matches!(result, Err(SqlQueryError::InvalidOutputType(_))),
        "Expected InvalidOutputType, got: {:?}",
        result
    );
}
