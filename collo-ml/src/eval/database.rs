//! Database connection types for runtime database values.
//!
//! This module defines:
//! - `DatabaseConnection`: Trait for database backends
//! - `DatabaseHandle`: Generic wrapper stored in `ExprValue`
//! - `SqliteDatabaseConnection`: SQLite implementation

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sqlx::{Row, ValueRef};
use thiserror::Error;

use super::values::ExprValue;
use crate::semantics::database::DbConversionError;
use crate::semantics::{ExprType, GlobalEnv, SimpleType};
use crate::traits::EvalObject;

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

/// Generic wrapper around a `DatabaseConnection`.
///
/// Stored inside `ExprValue::Database`. Uses `Arc` for cheap cloning.
pub struct DatabaseHandle<D: DatabaseConnection> {
    inner: Arc<D>,
}

impl<D: DatabaseConnection> DatabaseHandle<D> {
    pub fn new(inner: D) -> Self {
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

impl<D: DatabaseConnection> Clone for DatabaseHandle<D> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<D: DatabaseConnection> PartialEq for DatabaseHandle<D> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<D: DatabaseConnection> Eq for DatabaseHandle<D> {}

impl<D: DatabaseConnection> PartialOrd for DatabaseHandle<D> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<D: DatabaseConnection> Ord for DatabaseHandle<D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_ptr = Arc::as_ptr(&self.inner) as *const () as usize;
        let other_ptr = Arc::as_ptr(&other.inner) as *const () as usize;
        self_ptr.cmp(&other_ptr)
    }
}

impl<D: DatabaseConnection> fmt::Debug for DatabaseHandle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseHandle({})", self.name())
    }
}

impl<D: DatabaseConnection> fmt::Display for DatabaseHandle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
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
    /// Create a new SQLite database connection, returning a `DatabaseHandle`.
    pub async fn new(
        name: impl Into<String>,
        pool: &sqlx::SqlitePool,
    ) -> Result<DatabaseHandle<SqliteDatabaseConnection>, SqlQueryError> {
        let conn = Self::new_sqlite(name, pool).await?;
        Ok(DatabaseHandle::new(conn))
    }

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
    pub fn to_expr_value<T: EvalObject, D: DatabaseConnection>(
        self,
        env: &GlobalEnv,
        target: &ExprType,
    ) -> Result<ExprValue<T, D>, DbConversionError> {
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
    fn try_convert_to<T: EvalObject, D: DatabaseConnection>(
        self,
        env: &GlobalEnv,
        variant: &SimpleType,
    ) -> Result<ExprValue<T, D>, DbConversionError> {
        match (self, variant) {
            (DbValue::Null, SimpleType::None) => Ok(ExprValue::None),
            (DbValue::Int(v), SimpleType::Int) => Ok(ExprValue::Int(v)),
            (DbValue::Int(0), SimpleType::Bool) => Ok(ExprValue::Bool(false)),
            (DbValue::Int(1), SimpleType::Bool) => Ok(ExprValue::Bool(true)),
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
impl<T: EvalObject, D: DatabaseConnection> TryFrom<ExprValue<T, D>> for DbValue {
    type Error = DbConversionError;
    fn try_from(value: ExprValue<T, D>) -> Result<Self, Self::Error> {
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

// =============================================================================
// Typed query helpers
// =============================================================================

/// Wrap an `ExprValue` to match a target `ExprType`, adding `Custom` wrappers as needed.
///
/// Iterates over the target's variants and returns the first successful wrapping.
fn wrap_expr_value<T: EvalObject, D: DatabaseConnection>(
    value: ExprValue<T, D>,
    target: &ExprType,
    env: &GlobalEnv,
) -> Result<ExprValue<T, D>, DbConversionError> {
    for variant in target.get_variants() {
        if let Ok(v) = try_wrap_to(value.clone(), variant, env) {
            return Ok(v);
        }
    }
    Err(DbConversionError)
}

/// Try to wrap an `ExprValue` to match a single `SimpleType` variant.
fn try_wrap_to<T: EvalObject, D: DatabaseConnection>(
    value: ExprValue<T, D>,
    variant: &SimpleType,
    env: &GlobalEnv,
) -> Result<ExprValue<T, D>, DbConversionError> {
    match variant {
        SimpleType::None => {
            if matches!(value, ExprValue::None) {
                Ok(value)
            } else {
                Err(DbConversionError)
            }
        }
        SimpleType::Struct(_) => {
            if matches!(value, ExprValue::Struct(_)) {
                Ok(value)
            } else {
                Err(DbConversionError)
            }
        }
        SimpleType::Tuple(_) => {
            if matches!(value, ExprValue::Tuple(_)) {
                Ok(value)
            } else {
                Err(DbConversionError)
            }
        }
        SimpleType::List(_) => {
            if matches!(value, ExprValue::List(_)) {
                Ok(value)
            } else {
                Err(DbConversionError)
            }
        }
        SimpleType::Custom(module, root, variant_name) => {
            let qualified = match variant_name {
                Some(v) => format!("{}::{}", root, v),
                None => root.clone(),
            };
            let underlying = env
                .get_custom_type_underlying(module, &qualified)
                .ok_or(DbConversionError)?;
            let inner = wrap_expr_value(value, underlying, env)?;
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

enum OutputMode {
    List,
    Optional,
}

enum ResolvedRowShape {
    Struct(BTreeMap<String, ExprType>),
    Tuple(Vec<ExprType>),
}

fn convert_params<T: EvalObject, D: DatabaseConnection>(
    params: Vec<ExprValue<T, D>>,
) -> Result<Vec<DbValue>, SqlQueryError> {
    params
        .into_iter()
        .enumerate()
        .map(|(index, p)| {
            DbValue::try_from(p).map_err(|_| SqlQueryError::ParamConversionError { index })
        })
        .collect()
}

fn classify_output_type(
    out_type: &ExprType,
    env: &GlobalEnv,
) -> Result<(OutputMode, ResolvedRowShape, ExprType), SqlQueryError> {
    let resolved = env
        .resolve_type_deep(out_type)
        .ok_or_else(|| SqlQueryError::InvalidOutputType("cyclic type".to_string()))?;

    match resolved.len() {
        // 1 element — must be List(inner)
        1 => {
            let variant = &resolved[0];
            let inner_type = match variant {
                SimpleType::List(inner) => inner,
                _ => {
                    return Err(SqlQueryError::InvalidOutputType(format!(
                        "expected list or optional type, got single non-list variant"
                    )));
                }
            };
            // Resolve the inner type to get the row shape
            let inner_resolved = env.resolve_type_deep(inner_type).ok_or_else(|| {
                SqlQueryError::InvalidOutputType("cyclic type in list element".to_string())
            })?;
            if inner_resolved.len() != 1 {
                return Err(SqlQueryError::InvalidOutputType(format!(
                    "list element type must resolve to a single struct or tuple, got {} variants",
                    inner_resolved.len()
                )));
            }
            let shape = match &inner_resolved[0] {
                SimpleType::Struct(fields) => ResolvedRowShape::Struct(fields.clone()),
                SimpleType::Tuple(elements) => ResolvedRowShape::Tuple(elements.clone()),
                _ => {
                    return Err(SqlQueryError::InvalidOutputType(
                        "list element must be a struct or tuple".to_string(),
                    ));
                }
            };
            Ok((OutputMode::List, shape, inner_type.clone()))
        }
        // 2 elements — one None, one other (optional)
        2 => {
            let non_none: Vec<_> = resolved.iter().filter(|v| !v.is_none()).collect();
            let none_count = resolved.iter().filter(|v| v.is_none()).count();
            if none_count != 1 || non_none.len() != 1 {
                return Err(SqlQueryError::InvalidOutputType(
                    "expected exactly one None and one non-None variant for optional type"
                        .to_string(),
                ));
            }
            let shape = match non_none[0] {
                SimpleType::Struct(fields) => ResolvedRowShape::Struct(fields.clone()),
                SimpleType::Tuple(elements) => ResolvedRowShape::Tuple(elements.clone()),
                _ => {
                    return Err(SqlQueryError::InvalidOutputType(
                        "optional variant must be a struct or tuple".to_string(),
                    ));
                }
            };
            // Build the row target type from the original out_type's non-None variants
            let row_target_type = ExprType::from_variants(
                out_type
                    .get_variants()
                    .iter()
                    .filter(|v| !v.is_none())
                    .cloned(),
            );
            Ok((OutputMode::Optional, shape, row_target_type))
        }
        _ => Err(SqlQueryError::InvalidOutputType(format!(
            "output type must resolve to a list or an optional (None | T), got {} variants",
            resolved.len()
        ))),
    }
}

fn validate_columns(shape: &ResolvedRowShape, columns: &[String]) -> Result<(), SqlQueryError> {
    match shape {
        ResolvedRowShape::Struct(fields) => {
            if fields.len() != columns.len() {
                return Err(SqlQueryError::ColumnMismatch(format!(
                    "expected {} columns, got {}",
                    fields.len(),
                    columns.len()
                )));
            }
            for col in columns {
                if !fields.contains_key(col) {
                    return Err(SqlQueryError::ColumnMismatch(format!(
                        "column \"{}\" not found in struct fields",
                        col
                    )));
                }
            }
            Ok(())
        }
        ResolvedRowShape::Tuple(elements) => {
            if elements.len() != columns.len() {
                return Err(SqlQueryError::ColumnMismatch(format!(
                    "expected {} columns for tuple, got {}",
                    elements.len(),
                    columns.len()
                )));
            }
            Ok(())
        }
    }
}

fn convert_row<T: EvalObject, D: DatabaseConnection>(
    row: &BTreeMap<String, DbValue>,
    columns: &[String],
    shape: &ResolvedRowShape,
    row_idx: usize,
    env: &GlobalEnv,
) -> Result<ExprValue<T, D>, SqlQueryError> {
    match shape {
        ResolvedRowShape::Struct(fields) => {
            let mut struct_fields = BTreeMap::new();
            for (col_name, db_val) in row {
                let field_type = &fields[col_name];
                let val = db_val.clone().to_expr_value(env, field_type).map_err(|_| {
                    SqlQueryError::ResultConversionError {
                        row: row_idx,
                        column: col_name.clone(),
                    }
                })?;
                struct_fields.insert(col_name.clone(), val);
            }
            Ok(ExprValue::Struct(struct_fields))
        }
        ResolvedRowShape::Tuple(elements) => {
            let mut values = Vec::with_capacity(columns.len());
            for (i, col_name) in columns.iter().enumerate() {
                let db_val = &row[col_name];
                let elem_type = &elements[i];
                let val = db_val.clone().to_expr_value(env, elem_type).map_err(|_| {
                    SqlQueryError::ResultConversionError {
                        row: row_idx,
                        column: col_name.clone(),
                    }
                })?;
                values.push(val);
            }
            Ok(ExprValue::Tuple(values))
        }
    }
}

fn build_empty_result<T: EvalObject, D: DatabaseConnection>(
    mode: &OutputMode,
    out_type: &ExprType,
    env: &GlobalEnv,
) -> Result<ExprValue<T, D>, SqlQueryError> {
    let raw = match mode {
        OutputMode::Optional => ExprValue::None,
        OutputMode::List => ExprValue::List(vec![]),
    };
    wrap_expr_value(raw, out_type, env)
        .map_err(|_| SqlQueryError::InvalidOutputType("cannot wrap empty result".to_string()))
}

// =============================================================================
// DatabaseHandle::query
// =============================================================================

impl<D: DatabaseConnection> DatabaseHandle<D> {
    pub async fn query<T: EvalObject>(
        &self,
        sql: &str,
        params: Vec<ExprValue<T, D>>,
        out_type: ExprType,
        global_env: &GlobalEnv,
    ) -> Result<ExprValue<T, D>, SqlQueryError> {
        // 1. Convert params
        let db_params = convert_params(params)?;

        // 2. Classify output type
        let (mode, shape, row_target_type) = classify_output_type(&out_type, global_env)?;

        // 3. Execute raw query
        let (rows, columns) = self.inner.query(sql, db_params).await?;

        // 4. Empty result shortcut
        if rows.is_empty() {
            return build_empty_result(&mode, &out_type, global_env);
        }

        // 5. Validate columns
        validate_columns(&shape, &columns)?;

        // 6. Convert rows
        match mode {
            OutputMode::Optional => {
                let row_val = convert_row(&rows[0], &columns, &shape, 0, global_env)?;
                wrap_expr_value(row_val, &out_type, global_env).map_err(|_| {
                    SqlQueryError::InvalidOutputType("cannot wrap optional result".to_string())
                })
            }
            OutputMode::List => {
                let mut list = Vec::with_capacity(rows.len());
                for (i, row) in rows.iter().enumerate() {
                    let row_val = convert_row(row, &columns, &shape, i, global_env)?;
                    let wrapped =
                        wrap_expr_value(row_val, &row_target_type, global_env).map_err(|_| {
                            SqlQueryError::InvalidOutputType("cannot wrap list row".to_string())
                        })?;
                    list.push(wrapped);
                }
                let list_val = ExprValue::List(list);
                wrap_expr_value(list_val, &out_type, global_env).map_err(|_| {
                    SqlQueryError::InvalidOutputType("cannot wrap list result".to_string())
                })
            }
        }
    }
}
