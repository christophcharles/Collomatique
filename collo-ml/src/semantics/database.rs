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
