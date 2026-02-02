use super::analyze_with_env;
use crate::database::SqliteDatabaseDriver;
use crate::semantics::database::{DbConversionError, DbType};
use crate::semantics::types::{ExprType, SimpleType};

// =============================================================================
// HELPER
// =============================================================================

/// Build a GlobalEnv from DSL source (module "main"), assert no errors.
async fn env(input: &str) -> crate::semantics::GlobalEnv<SqliteDatabaseDriver> {
    let (env, errors, _warnings) = analyze_with_env(input).await;
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    env
}

/// Shorthand: empty environment (no custom types).
async fn empty_env() -> crate::semantics::GlobalEnv<SqliteDatabaseDriver> {
    env("").await
}

// =============================================================================
// 1. PRIMITIVE NON-NULLABLE TYPES
// =============================================================================

#[tokio::test]
async fn primitive_int() {
    let e = empty_env().await;
    assert_eq!(
        DbType::try_from(&e, &ExprType::simple(SimpleType::Int)),
        Ok(DbType::Int(false))
    );
}

#[tokio::test]
async fn primitive_bool() {
    let e = empty_env().await;
    assert_eq!(
        DbType::try_from(&e, &ExprType::simple(SimpleType::Bool)),
        Ok(DbType::Bool(false))
    );
}

#[tokio::test]
async fn primitive_string() {
    let e = empty_env().await;
    assert_eq!(
        DbType::try_from(&e, &ExprType::simple(SimpleType::String)),
        Ok(DbType::String(false))
    );
}

// =============================================================================
// 2. PRIMITIVE NULLABLE TYPES
// =============================================================================

#[tokio::test]
async fn nullable_int() {
    let e = empty_env().await;
    let typ = ExprType::from_variants([SimpleType::Int, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(true)));
}

#[tokio::test]
async fn nullable_bool() {
    let e = empty_env().await;
    let typ = ExprType::from_variants([SimpleType::Bool, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Bool(true)));
}

#[tokio::test]
async fn nullable_string() {
    let e = empty_env().await;
    let typ = ExprType::from_variants([SimpleType::String, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::String(true)));
}

// =============================================================================
// 3. REJECTION CASES
// =============================================================================

#[tokio::test]
async fn reject_two_non_none_primitives() {
    let e = empty_env().await;
    let typ = ExprType::from_variants([SimpleType::Int, SimpleType::Bool]);
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[tokio::test]
async fn reject_three_variants() {
    let e = empty_env().await;
    let typ = ExprType::from_variants([SimpleType::Int, SimpleType::Bool, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[tokio::test]
async fn reject_list() {
    let e = empty_env().await;
    let typ = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[tokio::test]
async fn reject_struct() {
    let e = empty_env().await;
    let typ = ExprType::simple(SimpleType::Struct(
        [("x".to_string(), ExprType::simple(SimpleType::Int))]
            .into_iter()
            .collect(),
    ));
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[tokio::test]
async fn reject_none_alone() {
    let e = empty_env().await;
    let typ = ExprType::simple(SimpleType::None);
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

// =============================================================================
// 4. CUSTOM TYPE RESOLUTION (DSL-BASED)
// =============================================================================

#[tokio::test]
async fn custom_type_alias_for_int() {
    let e = env("type MyInt = Int;").await;
    let typ = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MyInt".to_string(),
        None,
    ));
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(false)));
}

#[tokio::test]
async fn custom_type_alias_nullable() {
    let e = env("type MaybeInt = Int | None;").await;
    let typ = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MaybeInt".to_string(),
        None,
    ));
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(true)));
}

#[tokio::test]
async fn custom_type_union_with_none() {
    let e = env("type MyInt = Int;").await;
    let typ = ExprType::from_variants([
        SimpleType::Custom("main".to_string(), "MyInt".to_string(), None),
        SimpleType::None,
    ]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(true)));
}

#[tokio::test]
async fn deep_custom_type_chain() {
    let e = env("type MyInt = Int; type Deep = MyInt;").await;
    let typ = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "Deep".to_string(),
        None,
    ));
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(false)));
}
