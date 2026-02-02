use super::analyze_with_env;
use crate::database::SqliteDatabaseDriver;
use crate::semantics::database::{DbConversionError, DbType};
use crate::semantics::types::{ExprType, SimpleType};

// =============================================================================
// HELPER
// =============================================================================

/// Build a GlobalEnv from DSL source (module "main"), assert no errors.
fn env(input: &str) -> crate::semantics::GlobalEnv<SqliteDatabaseDriver> {
    let (env, errors, _warnings) = analyze_with_env(input);
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    env
}

/// Shorthand: empty environment (no custom types).
fn empty_env() -> crate::semantics::GlobalEnv<SqliteDatabaseDriver> {
    env("")
}

// =============================================================================
// 1. PRIMITIVE NON-NULLABLE TYPES
// =============================================================================

#[test]
fn primitive_int() {
    let e = empty_env();
    assert_eq!(
        DbType::try_from(&e, &ExprType::simple(SimpleType::Int)),
        Ok(DbType::Int(false))
    );
}

#[test]
fn primitive_bool() {
    let e = empty_env();
    assert_eq!(
        DbType::try_from(&e, &ExprType::simple(SimpleType::Bool)),
        Ok(DbType::Bool(false))
    );
}

#[test]
fn primitive_string() {
    let e = empty_env();
    assert_eq!(
        DbType::try_from(&e, &ExprType::simple(SimpleType::String)),
        Ok(DbType::String(false))
    );
}

// =============================================================================
// 2. PRIMITIVE NULLABLE TYPES
// =============================================================================

#[test]
fn nullable_int() {
    let e = empty_env();
    let typ = ExprType::from_variants([SimpleType::Int, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(true)));
}

#[test]
fn nullable_bool() {
    let e = empty_env();
    let typ = ExprType::from_variants([SimpleType::Bool, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Bool(true)));
}

#[test]
fn nullable_string() {
    let e = empty_env();
    let typ = ExprType::from_variants([SimpleType::String, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::String(true)));
}

// =============================================================================
// 3. REJECTION CASES
// =============================================================================

#[test]
fn reject_two_non_none_primitives() {
    let e = empty_env();
    let typ = ExprType::from_variants([SimpleType::Int, SimpleType::Bool]);
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[test]
fn reject_three_variants() {
    let e = empty_env();
    let typ = ExprType::from_variants([SimpleType::Int, SimpleType::Bool, SimpleType::None]);
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[test]
fn reject_list() {
    let e = empty_env();
    let typ = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[test]
fn reject_struct() {
    let e = empty_env();
    let typ = ExprType::simple(SimpleType::Struct(
        [("x".to_string(), ExprType::simple(SimpleType::Int))]
            .into_iter()
            .collect(),
    ));
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

#[test]
fn reject_none_alone() {
    let e = empty_env();
    let typ = ExprType::simple(SimpleType::None);
    assert_eq!(DbType::try_from(&e, &typ), Err(DbConversionError));
}

// =============================================================================
// 4. CUSTOM TYPE RESOLUTION (DSL-BASED)
// =============================================================================

#[test]
fn custom_type_alias_for_int() {
    let e = env("type MyInt = Int;");
    let typ = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MyInt".to_string(),
        None,
    ));
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(false)));
}

#[test]
fn custom_type_alias_nullable() {
    let e = env("type MaybeInt = Int | None;");
    let typ = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "MaybeInt".to_string(),
        None,
    ));
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(true)));
}

#[test]
fn custom_type_union_with_none() {
    let e = env("type MyInt = Int;");
    let typ = ExprType::from_variants([
        SimpleType::Custom("main".to_string(), "MyInt".to_string(), None),
        SimpleType::None,
    ]);
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(true)));
}

#[test]
fn deep_custom_type_chain() {
    let e = env("type MyInt = Int; type Deep = MyInt;");
    let typ = ExprType::simple(SimpleType::Custom(
        "main".to_string(),
        "Deep".to_string(),
        None,
    ));
    assert_eq!(DbType::try_from(&e, &typ), Ok(DbType::Int(false)));
}
