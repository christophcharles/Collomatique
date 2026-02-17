//! Database connection types.
//!
//! This module defines:
//! - `DatabaseConnection`: Trait for database backends
//! - `SqliteDatabaseConnection`: SQLite implementation
//! - `DbValue`: Primitive value type exchanged with databases
//! - `SqlQueryError`: Error type for SQL operations

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use sqlx::{Row, ValueRef};
use thiserror::Error;

/// A database-level type. Each variant carries a `bool` indicating nullability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbType {
    Int(bool),
    Bool(bool),
    String(bool),
    Any,
}

impl DbType {
    pub fn is_nullable(&self) -> bool {
        match self {
            DbType::Int(n) | DbType::Bool(n) | DbType::String(n) => *n,
            DbType::Any => true,
        }
    }

    pub fn as_nullable(self) -> Self {
        match self {
            DbType::Int(_) => DbType::Int(true),
            DbType::Bool(_) => DbType::Bool(true),
            DbType::String(_) => DbType::String(true),
            DbType::Any => DbType::Any,
        }
    }

    /// Check whether a SQL column type (`self`) is assignable to a declared ColloML type.
    /// Base types must match; if the SQL column is nullable, the declared type must also be nullable.
    /// `Any` (unknown SQL type) only assigns to `Any`; everything assigns to `Any`.
    pub fn is_assignable_to(&self, declared: &DbType) -> bool {
        match (self, declared) {
            // Everything assigns to Any
            (_, DbType::Any) => true,
            // Any only assigns to Any (handled above)
            (DbType::Any, _) => false,
            (DbType::Int(sql_null), DbType::Int(decl_null))
            | (DbType::Bool(sql_null), DbType::Bool(decl_null))
            | (DbType::String(sql_null), DbType::String(decl_null)) => {
                // If SQL column is nullable, declared must also be nullable
                !sql_null || *decl_null
            }
            _ => false,
        }
    }
}

impl fmt::Display for DbType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbType::Int(true) => write!(f, "?Int"),
            DbType::Int(false) => write!(f, "Int"),
            DbType::Bool(true) => write!(f, "?Bool"),
            DbType::Bool(false) => write!(f, "Bool"),
            DbType::String(true) => write!(f, "?String"),
            DbType::String(false) => write!(f, "String"),
            DbType::Any => write!(f, "Any"),
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SqlQueryError {
    #[error("SQL query failed: {0}")]
    QueryFailed(String),
    #[error("Duplicate column name in result: {0}")]
    DuplicateColumnName(String),
    #[error("Unsupported column type in row {row}, column \"{column}\": {type_name}")]
    UnsupportedColumnType {
        row: usize,
        column: String,
        type_name: String,
    },
    #[error("Invalid output type for query: {0}")]
    InvalidOutputType(String),
    #[error("Column mismatch: {0}")]
    ColumnMismatch(String),
    #[error("Parameter conversion failed at index {index}")]
    ParamConversionError { index: usize },
    #[error("Result conversion failed at row {row}, column \"{column}\"")]
    ResultConversionError { row: usize, column: String },
    #[error("Unsupported column type in describe for column \"{column}\": {type_name}")]
    UnsupportedDescribeType { column: String, type_name: String },
}

/// Trait for database drivers that can build a fresh in-memory database from a schema.
pub trait DatabaseDriver {
    type Connection: DatabaseConnection;

    fn build_with_schema<'a>(
        name: &'a str,
        schema: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Connection, SqlQueryError>> + Send + 'a>>;
}

/// Trait for database connection backends.
pub trait DatabaseConnection: fmt::Debug + Send + Sync + 'static {
    fn name(&self) -> &str;

    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: Vec<DbValue>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<(Vec<BTreeMap<String, DbValue>>, Vec<String>), SqlQueryError>,
                > + Send
                + 'a,
        >,
    >;

    /// Validate SQL and get column metadata WITHOUT executing the query.
    /// Returns (column_name, DbType) for each column.
    fn describe_query<'a>(
        &'a self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(String, DbType)>, SqlQueryError>> + Send + 'a>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbValue {
    Null,
    Int(i32),
    Bool(bool),
    String(String),
}

/// SQLite database connection implementation.
///
/// Holds an open read-only SQLite transaction for snapshot isolation.
/// `PRAGMA query_only = ON` prevents accidental writes.
pub struct SqliteDatabaseConnection {
    name: String,
    tx: tokio::sync::Mutex<sqlx::Transaction<'static, sqlx::Sqlite>>,
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

    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: Vec<DbValue>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<(Vec<BTreeMap<String, DbValue>>, Vec<String>), SqlQueryError>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            use sqlx::Column;
            use sqlx::TypeInfo;

            let mut tx = self.tx.lock().await;

            let mut query = sqlx::query(sql);
            for param in params {
                query = match param {
                    DbValue::Null => query.bind(None::<i32>),
                    DbValue::Int(v) => query.bind(v),
                    DbValue::Bool(v) => query.bind(v),
                    DbValue::String(v) => query.bind(v),
                };
            }

            let rows = query
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;

            if rows.is_empty() {
                return Ok((vec![], vec![]));
            }

            // Extract column names from the first row
            let columns: Vec<String> = rows[0]
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect();

            // Check for duplicate column names
            {
                let mut seen = std::collections::HashSet::new();
                for name in &columns {
                    if !seen.insert(name) {
                        return Err(SqlQueryError::DuplicateColumnName(name.clone()));
                    }
                }
            }

            // Decode rows
            let mut result = Vec::with_capacity(rows.len());
            for (row_idx, row) in rows.iter().enumerate() {
                let mut map = BTreeMap::new();
                for (col_idx, col_name) in columns.iter().enumerate() {
                    let raw = row
                        .try_get_raw(col_idx)
                        .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
                    let type_name = raw.type_info().name().to_uppercase();
                    let value = if raw.is_null() {
                        DbValue::Null
                    } else {
                        match type_name.as_str() {
                            "INTEGER" | "INT" => {
                                let v: i32 = row
                                    .try_get(col_idx)
                                    .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
                                DbValue::Int(v)
                            }
                            "TEXT" => {
                                let v: String = row
                                    .try_get(col_idx)
                                    .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
                                DbValue::String(v)
                            }
                            "BOOLEAN" | "BOOL" => {
                                let v: bool = row
                                    .try_get(col_idx)
                                    .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
                                DbValue::Bool(v)
                            }
                            "NULL" => DbValue::Null,
                            _ => {
                                return Err(SqlQueryError::UnsupportedColumnType {
                                    row: row_idx,
                                    column: col_name.clone(),
                                    type_name,
                                });
                            }
                        }
                    };
                    map.insert(col_name.clone(), value);
                }
                result.push(map);
            }

            Ok((result, columns))
        })
    }

    fn describe_query<'a>(
        &'a self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(String, DbType)>, SqlQueryError>> + Send + 'a>>
    {
        Box::pin(async move {
            use sqlx::Column;
            use sqlx::Executor;
            use sqlx::TypeInfo;

            let mut tx = self.tx.lock().await;

            let describe = (&mut **tx)
                .describe(sql)
                .await
                .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;

            let mut columns = Vec::with_capacity(describe.columns().len());
            for (idx, col) in describe.columns().iter().enumerate() {
                let name = col.name().to_string();
                let type_name = col.type_info().name().to_uppercase();
                // nullable() returns Option<bool>, default to true if unknown
                let is_nullable = describe.nullable(idx).unwrap_or(true);

                let base_db_type = match type_name.as_str() {
                    "INTEGER" | "INT" => DbType::Int(false),
                    "TEXT" => DbType::String(false),
                    "BOOLEAN" | "BOOL" => DbType::Bool(false),
                    "NULL" => DbType::Any,
                    _ => {
                        return Err(SqlQueryError::UnsupportedDescribeType {
                            column: name,
                            type_name,
                        });
                    }
                };

                let db_type = if is_nullable {
                    base_db_type.as_nullable()
                } else {
                    base_db_type
                };

                columns.push((name, db_type));
            }

            Ok(columns)
        })
    }
}

/// SQLite database driver.
///
/// Builds an in-memory SQLite database, applies the provided schema,
/// then opens a read-only connection via `SqliteDatabaseConnection`.
pub struct SqliteDatabaseDriver;

impl SqliteDatabaseDriver {
    /// Create a new SQLite database connection from an existing pool.
    ///
    /// Opens a read-only transaction on the pool with `PRAGMA query_only = ON`.
    pub async fn new_connection(
        name: impl Into<String>,
        pool: &sqlx::SqlitePool,
    ) -> Result<SqliteDatabaseConnection, SqlQueryError> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
        sqlx::query("PRAGMA query_only = ON")
            .execute(&mut *tx)
            .await
            .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
        Ok(SqliteDatabaseConnection {
            name: name.into(),
            tx: tokio::sync::Mutex::new(tx),
        })
    }
}

impl DatabaseDriver for SqliteDatabaseDriver {
    type Connection = SqliteDatabaseConnection;

    fn build_with_schema<'a>(
        name: &'a str,
        schema: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Connection, SqlQueryError>> + Send + 'a>> {
        Box::pin(async move {
            let pool = sqlx::SqlitePool::connect(":memory:")
                .await
                .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
            sqlx::raw_sql(schema)
                .execute(&pool)
                .await
                .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
            Self::new_connection(name, &pool).await
        })
    }
}
