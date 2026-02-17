use super::global_env::GlobalEnv;
use super::types::{ExprType, SimpleType};
use crate::database::{DatabaseDriver, DbType};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("Cannot convert to database type: not representable")]
pub struct DbConversionError;

impl DbType {
    /// Convert an `ExprType` to a `DbType` by deep-resolving through `env`.
    ///
    /// Valid resolved patterns:
    /// - `[Int]` → `DbType::Int(false)`, `[Bool]` → …, `[String]` → …
    /// - `[None, Int]` → `DbType::Int(true)`, etc.
    pub fn try_from<D: DatabaseDriver>(
        env: &GlobalEnv<D>,
        typ: &ExprType,
    ) -> Result<Self, DbConversionError> {
        let resolved = env
            .resolve_type_until_several_or_not_custom(typ)
            .ok_or(DbConversionError)?;
        match resolved.len() {
            1 => Self::from_primitive(&resolved[0]),
            2 => {
                // Resolve each individually — each must resolve to exactly 1 type
                let a = env
                    .resolve_type_until_several_or_not_custom(&ExprType::from(resolved[0].clone()))
                    .ok_or(DbConversionError)?;
                let b = env
                    .resolve_type_until_several_or_not_custom(&ExprType::from(resolved[1].clone()))
                    .ok_or(DbConversionError)?;

                if a.len() != 1 || b.len() != 1 {
                    return Err(DbConversionError);
                }

                if a[0].is_none() {
                    Self::from_primitive(&b[0]).map(|db| db.as_nullable())
                } else if b[0].is_none() {
                    Self::from_primitive(&a[0]).map(|db| db.as_nullable())
                } else {
                    Err(DbConversionError)
                }
            }
            4 => {
                // Check for {None, Int, String, Bool} in any order → DbType::Any
                // This matches SQLite's unknown column type (e.g. computed columns in CTEs).
                let mut has_none = false;
                let mut has_int = false;
                let mut has_string = false;
                let mut has_bool = false;
                for st in &resolved {
                    let inner = env
                        .resolve_type_until_several_or_not_custom(&ExprType::from(st.clone()))
                        .ok_or(DbConversionError)?;
                    if inner.len() != 1 {
                        return Err(DbConversionError);
                    }
                    match &inner[0] {
                        SimpleType::None => has_none = true,
                        SimpleType::Int => has_int = true,
                        SimpleType::String => has_string = true,
                        SimpleType::Bool => has_bool = true,
                        _ => return Err(DbConversionError),
                    }
                }
                if has_none && has_int && has_string && has_bool {
                    Ok(DbType::Any)
                } else {
                    Err(DbConversionError)
                }
            }
            _ => Err(DbConversionError),
        }
    }

    /// Map a single primitive SimpleType to a non-nullable DbType.
    fn from_primitive(value: &SimpleType) -> Result<Self, DbConversionError> {
        match value {
            SimpleType::Int => Ok(DbType::Int(false)),
            SimpleType::Bool => Ok(DbType::Bool(false)),
            SimpleType::String => Ok(DbType::String(false)),
            _ => Err(DbConversionError),
        }
    }
}
