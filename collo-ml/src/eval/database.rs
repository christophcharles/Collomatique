//! Database connection types for runtime database values.
//!
//! This module defines:
//! - `DatabaseConnection`: Trait for database backends
//! - `DatabaseHandle`: Type-erased wrapper stored in `ExprValue`
//! - `SqliteDatabaseConnection`: SQLite implementation

use std::fmt;
use std::sync::Arc;

use super::values::ExprValue;
use crate::semantics::database::DbConversionError;
use crate::semantics::{ExprType, GlobalEnv, SimpleType};
use crate::traits::EvalObject;

/// Trait for database connection backends.
///
/// Minimal for now — more methods (schema validation, query execution) will be added later.
pub trait DatabaseConnection: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
}

/// Type-erased wrapper around a `DatabaseConnection`.
///
/// Stored inside `ExprValue::Database`. Uses `Arc` for cheap cloning.
#[derive(Clone)]
pub struct DatabaseHandle {
    inner: Arc<dyn DatabaseConnection>,
}

impl DatabaseHandle {
    pub fn new(inner: impl DatabaseConnection + 'static) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Check if this database handle is compatible with a declared schema.
    /// TODO: Implement actual schema compatibility checks.
    pub fn matches_schema(&self, _declared_schema: &str) -> bool {
        true
    }
}

impl PartialEq for DatabaseHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for DatabaseHandle {}

impl PartialOrd for DatabaseHandle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DatabaseHandle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_ptr = Arc::as_ptr(&self.inner) as *const () as usize;
        let other_ptr = Arc::as_ptr(&other.inner) as *const () as usize;
        self_ptr.cmp(&other_ptr)
    }
}

impl fmt::Debug for DatabaseHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseHandle({})", self.name())
    }
}

impl fmt::Display for DatabaseHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// SQLite database connection implementation.
pub struct SqliteDatabaseConnection {
    name: String,
    #[allow(dead_code)]
    pool: sqlx::SqlitePool,
}

impl SqliteDatabaseConnection {
    /// Create a new SQLite database connection, returning a `DatabaseHandle`.
    pub fn new(name: impl Into<String>, pool: sqlx::SqlitePool) -> DatabaseHandle {
        DatabaseHandle::new(Self {
            name: name.into(),
            pool,
        })
    }
}

impl fmt::Debug for SqliteDatabaseConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SqliteDatabaseConnection({})", self.name)
    }
}

impl DatabaseConnection for SqliteDatabaseConnection {
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbValue {
    Null,
    Int(i32),
    Bool(bool),
    String(String),
}

impl DbValue {
    /// Convert into an [`ExprValue`] guided by the expected `target` type,
    /// deep-resolving `Custom` variants through `env`.
    ///
    /// Iterates over `target`'s variants and returns the first successful
    /// conversion. Primitive variants (Int, Bool, String, None) are matched
    /// directly. `Custom(…)` variants are unwrapped via the `GlobalEnv` and
    /// the conversion recurses into the underlying type, wrapping the result
    /// in `ExprValue::Custom`.
    pub fn to_expr_value<T: EvalObject>(
        self,
        env: &GlobalEnv,
        target: &ExprType,
    ) -> Result<ExprValue<T>, DbConversionError> {
        for variant in target.get_variants() {
            if let Ok(v) = self.clone().try_convert_to(env, variant) {
                return Ok(v);
            }
        }
        Err(DbConversionError)
    }

    /// Try to convert into an [`ExprValue`] matching a single [`SimpleType`].
    ///
    /// For primitives, checks a direct match. For `Custom` types, looks up
    /// the underlying type in `env` and recurses via `to_expr_value`,
    /// wrapping the result in [`ExprValue::Custom`].
    fn try_convert_to<T: EvalObject>(
        self,
        env: &GlobalEnv,
        variant: &SimpleType,
    ) -> Result<ExprValue<T>, DbConversionError> {
        match (self, variant) {
            (DbValue::Null, SimpleType::None) => Ok(ExprValue::None),
            (DbValue::Int(v), SimpleType::Int) => Ok(ExprValue::Int(v)),
            (DbValue::Bool(v), SimpleType::Bool) => Ok(ExprValue::Bool(v)),
            (DbValue::String(v), SimpleType::String) => Ok(ExprValue::String(v)),
            (db_val, SimpleType::Custom(module, root, variant_name)) => {
                let qualified = match variant_name {
                    Some(v) => format!("{}::{}", root, v),
                    None => root.clone(),
                };
                let underlying = env
                    .get_custom_type_underlying(module, &qualified)
                    .ok_or(DbConversionError)?;
                let inner = db_val.to_expr_value(env, underlying)?;
                Ok(ExprValue::Custom(Box::new(super::values::CustomValue {
                    module: module.clone(),
                    type_name: root.clone(),
                    variant: variant_name.clone(),
                    content: inner,
                })))
            }
            _ => Err(DbConversionError),
        }
    }
}

/// Recursively unwraps `ExprValue::Custom` wrappers, then converts the leaf value.
impl<T: EvalObject> TryFrom<ExprValue<T>> for DbValue {
    type Error = DbConversionError;
    fn try_from(value: ExprValue<T>) -> Result<Self, Self::Error> {
        match value {
            ExprValue::None => Ok(DbValue::Null),
            ExprValue::Int(v) => Ok(DbValue::Int(v)),
            ExprValue::Bool(v) => Ok(DbValue::Bool(v)),
            ExprValue::String(v) => Ok(DbValue::String(v)),
            ExprValue::Custom(custom) => DbValue::try_from(custom.content),
            _ => Err(DbConversionError),
        }
    }
}
