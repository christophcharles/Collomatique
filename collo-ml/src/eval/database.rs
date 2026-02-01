//! Database connection types for runtime database values.
//!
//! This module defines:
//! - `DatabaseConnection`: Trait for database backends
//! - `DatabaseHandle`: Type-erased wrapper stored in `ExprValue`
//! - `SqliteDatabaseConnection`: SQLite implementation

use std::fmt;
use std::sync::Arc;

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
