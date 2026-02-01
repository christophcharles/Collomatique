use super::types::SimpleType;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("Cannot convert to database type: not representable")]
pub struct DbConversionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbType {
    Int,
    Bool,
    String,
}

impl DbType {
    pub fn as_simple_type(&self) -> SimpleType {
        match self {
            DbType::Int => SimpleType::Int,
            DbType::Bool => SimpleType::Bool,
            DbType::String => SimpleType::String,
        }
    }
}

impl TryFrom<SimpleType> for DbType {
    type Error = DbConversionError;
    fn try_from(value: SimpleType) -> Result<Self, Self::Error> {
        match value {
            SimpleType::Int => Ok(DbType::Int),
            SimpleType::Bool => Ok(DbType::Bool),
            SimpleType::String => Ok(DbType::String),
            _ => Err(DbConversionError),
        }
    }
}

impl TryFrom<&SimpleType> for DbType {
    type Error = DbConversionError;
    fn try_from(value: &SimpleType) -> Result<Self, Self::Error> {
        match value {
            SimpleType::Int => Ok(DbType::Int),
            SimpleType::Bool => Ok(DbType::Bool),
            SimpleType::String => Ok(DbType::String),
            _ => Err(DbConversionError),
        }
    }
}
