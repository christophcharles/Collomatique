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

impl SqliteDatabaseConnection {
    /// Create a new SQLite database connection, returning the concrete type.
    ///
    /// Opens a read-only transaction on the pool with `PRAGMA query_only = ON`.
    pub async fn new_sqlite(
        name: impl Into<String>,
        pool: &sqlx::SqlitePool,
    ) -> Result<Self, SqlQueryError> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
        sqlx::query("PRAGMA query_only = ON")
            .execute(&mut *tx)
            .await
            .map_err(|e| SqlQueryError::QueryFailed(e.to_string()))?;
        Ok(Self {
            name: name.into(),
            tx: tokio::sync::Mutex::new(tx),
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
}

/// SQLite database driver.
///
/// Builds an in-memory SQLite database, applies the provided schema,
/// then opens a read-only connection via `SqliteDatabaseConnection`.
pub struct SqliteDatabaseDriver;

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
            SqliteDatabaseConnection::new_sqlite(name, &pool).await
        })
    }
}
