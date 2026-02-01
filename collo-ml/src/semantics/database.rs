use super::global_env::GlobalEnv;
use super::types::{ExprType, SimpleType};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("Cannot convert to database type: not representable")]
pub struct DbConversionError;

/// A database-level type. Each variant carries a `bool` indicating nullability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbType {
    Int(bool),
    Bool(bool),
    String(bool),
}

impl DbType {
    pub fn is_nullable(&self) -> bool {
        match self {
            DbType::Int(n) | DbType::Bool(n) | DbType::String(n) => *n,
        }
    }

    /// Convert an `ExprType` to a `DbType` by deep-resolving through `env`.
    ///
    /// Valid resolved patterns:
    /// - `[Int]` → `DbType::Int(false)`, `[Bool]` → …, `[String]` → …
    /// - `[None, Int]` → `DbType::Int(true)`, etc.
    pub fn try_from(env: &GlobalEnv, typ: &ExprType) -> Result<Self, DbConversionError> {
        let resolved = env.resolve_type_deep(typ).ok_or(DbConversionError)?;
        match resolved.len() {
            1 => Self::from_primitive(&resolved[0]),
            2 => {
                let nones = resolved.iter().filter(|v| v.is_none()).count();
                if nones != 1 {
                    return Err(DbConversionError);
                }
                let non_none = resolved.iter().find(|v| !v.is_none()).unwrap();
                Self::from_primitive(non_none).map(|db| db.as_nullable())
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

    fn as_nullable(self) -> Self {
        match self {
            DbType::Int(_) => DbType::Int(true),
            DbType::Bool(_) => DbType::Bool(true),
            DbType::String(_) => DbType::String(true),
        }
    }
}
