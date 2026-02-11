//! Database handle and query execution logic for runtime database values.
//!
//! This module defines:
//! - `DatabaseHandle`: Generic wrapper stored in `ExprValue`
//! - Typed query helpers for converting between `DbValue` and `ExprValue`

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub use crate::database::{
    DatabaseConnection, DatabaseDriver, DbValue, SqlQueryError, SqliteDatabaseConnection,
    SqliteDatabaseDriver,
};
use crate::semantics::database::DbConversionError;
use crate::semantics::{ExprType, GlobalEnv, SimpleType};
use crate::traits::EvalObject;

use super::values::{CustomValue, ExprValue};

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

impl DbValue {
    /// Convert into an [`ExprValue`] guided by the expected `target` type,
    /// deep-resolving `Custom` variants through `env`.
    ///
    /// Iterates over `target`'s variants and returns the first successful
    /// conversion. Primitive variants (Int, Bool, String, None) are matched
    /// directly. `Custom(…)` variants are unwrapped via the `GlobalEnv` and
    /// the conversion recurses into the underlying type, wrapping the result
    /// in `ExprValue::Custom`.
    pub fn to_expr_value<T: EvalObject, D: DatabaseDriver>(
        self,
        env: &GlobalEnv<D>,
        target: &ExprType,
    ) -> Result<ExprValue<T, D::Connection>, DbConversionError> {
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
    fn try_convert_to<T: EvalObject, D: DatabaseDriver>(
        self,
        env: &GlobalEnv<D>,
        variant: &SimpleType,
    ) -> Result<ExprValue<T, D::Connection>, DbConversionError> {
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

/// Helper to wrap an ExprValue in a Custom wrapper.
fn wrap_custom<T: EvalObject, D: DatabaseConnection>(
    module: &str,
    root: &str,
    variant: &Option<String>,
    inner: ExprValue<T, D>,
) -> ExprValue<T, D> {
    ExprValue::Custom(Box::new(CustomValue {
        module: module.to_string(),
        type_name: root.to_string(),
        variant: variant.clone(),
        content: inner,
    }))
}

/// Build the query result by walking `out_type` breadth-first, peeling one
/// custom type layer at a time.
///
/// Handles three shapes:
/// - **Single Custom** → peel, recurse, wrap in Custom on unwind
/// - **Single List(inner)** → base case: build list by calling `build_row_value` for each row
/// - **Two variants (one resolving to None)** → optional: if rows empty build None path,
///   else call `build_row_value` on the data variant for rows[0]
fn build_query_result<T: EvalObject, D: DatabaseDriver>(
    out_type: &ExprType,
    rows: &[BTreeMap<String, DbValue>],
    columns: &[String],
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, SqlQueryError> {
    let variants: Vec<_> = out_type.get_variants().iter().cloned().collect();

    match variants.len() {
        1 => {
            let single = &variants[0];
            match single {
                SimpleType::Custom(module, root, variant_name) => {
                    let qualified = match variant_name {
                        Some(v) => format!("{}::{}", root, v),
                        None => root.clone(),
                    };
                    let underlying = env
                        .get_custom_type_underlying(module, &qualified)
                        .ok_or_else(|| {
                            SqlQueryError::InvalidOutputType(format!(
                                "unknown custom type: {}",
                                qualified
                            ))
                        })?;
                    let inner = build_query_result(underlying, rows, columns, env)?;
                    Ok(wrap_custom(module, root, variant_name, inner))
                }
                SimpleType::List(inner_type) => {
                    let mut list = Vec::with_capacity(rows.len());
                    for (i, row) in rows.iter().enumerate() {
                        let row_val = build_row_value(inner_type, row, columns, i, env)?;
                        list.push(row_val);
                    }
                    Ok(ExprValue::List(list))
                }
                _ => Err(SqlQueryError::InvalidOutputType(format!(
                    "expected list or optional type, got single non-list variant: {}",
                    single
                ))),
            }
        }
        2 => {
            // Determine which variant is the None branch and which is the data branch
            let (none_variant, data_variant) = classify_optional_variants(&variants, env)?;

            if rows.is_empty() {
                // Build None through the none_variant path
                build_none_value(&ExprType::simple(none_variant.clone()), env)
            } else {
                // Build value through the data variant for the first row
                let data_type = ExprType::simple(data_variant.clone());
                build_row_value(&data_type, &rows[0], columns, 0, env)
            }
        }
        _ => Err(SqlQueryError::InvalidOutputType(format!(
            "output type must resolve to a list or an optional (None | T), got {} variants",
            variants.len()
        ))),
    }
}

/// Classify the two variants of an optional type into (none_variant, data_variant).
///
/// Uses `resolve_type_until_several_or_not_custom` to determine which variant
/// ultimately resolves to None.
fn classify_optional_variants<D: DatabaseDriver>(
    variants: &[SimpleType],
    env: &GlobalEnv<D>,
) -> Result<(SimpleType, SimpleType), SqlQueryError> {
    assert_eq!(variants.len(), 2);

    let resolved_0 = env
        .resolve_type_until_several_or_not_custom(&ExprType::simple(variants[0].clone()))
        .ok_or_else(|| SqlQueryError::InvalidOutputType("cyclic type".to_string()))?;
    let resolved_1 = env
        .resolve_type_until_several_or_not_custom(&ExprType::simple(variants[1].clone()))
        .ok_or_else(|| SqlQueryError::InvalidOutputType("cyclic type".to_string()))?;

    let is_none_0 = resolved_0.len() == 1 && resolved_0[0].is_none();
    let is_none_1 = resolved_1.len() == 1 && resolved_1[0].is_none();

    match (is_none_0, is_none_1) {
        (true, false) => Ok((variants[0].clone(), variants[1].clone())),
        (false, true) => Ok((variants[1].clone(), variants[0].clone())),
        _ => Err(SqlQueryError::InvalidOutputType(
            "expected exactly one None and one non-None variant for optional type".to_string(),
        )),
    }
}

/// Build a None value by walking through custom type layers until reaching
/// `SimpleType::None`, then wrapping in Custom on unwind.
fn build_none_value<T: EvalObject, D: DatabaseDriver>(
    typ: &ExprType,
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, SqlQueryError> {
    let variants: Vec<_> = typ.get_variants().iter().cloned().collect();

    match variants.len() {
        1 => match &variants[0] {
            SimpleType::None => Ok(ExprValue::None),
            SimpleType::Custom(module, root, variant_name) => {
                let qualified = match variant_name {
                    Some(v) => format!("{}::{}", root, v),
                    None => root.clone(),
                };
                let underlying = env
                    .get_custom_type_underlying(module, &qualified)
                    .ok_or_else(|| {
                        SqlQueryError::InvalidOutputType(format!(
                            "unknown custom type: {}",
                            qualified
                        ))
                    })?;
                let inner = build_none_value(underlying, env)?;
                Ok(wrap_custom(module, root, variant_name, inner))
            }
            other => Err(SqlQueryError::InvalidOutputType(format!(
                "expected None through custom type chain, got: {}",
                other
            ))),
        },
        2 => {
            // Fork — find the None branch and recurse into it
            let (none_variant, _data_variant) = classify_optional_variants(&variants, env)?;
            build_none_value(&ExprType::simple(none_variant), env)
        }
        _ => Err(SqlQueryError::InvalidOutputType(
            "cannot build None value from type with more than 2 variants".to_string(),
        )),
    }
}

/// Build an ExprValue for a single row by walking `row_type` breadth-first.
///
/// Handles:
/// - **Single Custom** → peel, recurse, wrap in Custom on unwind
/// - **Single Struct(fields)** → base case: convert each field via `to_expr_value`
/// - **Single Tuple(elements)** → base case: convert each element via `to_expr_value`
/// - **Single primitive or any fork** → delegate to `to_expr_value` (single column expected)
fn build_row_value<T: EvalObject, D: DatabaseDriver>(
    row_type: &ExprType,
    row: &BTreeMap<String, DbValue>,
    columns: &[String],
    row_idx: usize,
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, SqlQueryError> {
    let variants: Vec<_> = row_type.get_variants().iter().cloned().collect();

    if variants.len() == 1 {
        match &variants[0] {
            SimpleType::Custom(module, root, variant_name) => {
                let qualified = match variant_name {
                    Some(v) => format!("{}::{}", root, v),
                    None => root.clone(),
                };
                let underlying = env
                    .get_custom_type_underlying(module, &qualified)
                    .ok_or_else(|| {
                        SqlQueryError::InvalidOutputType(format!(
                            "unknown custom type: {}",
                            qualified
                        ))
                    })?;
                let inner = build_row_value(underlying, row, columns, row_idx, env)?;
                Ok(wrap_custom(module, root, variant_name, inner))
            }
            SimpleType::Struct(fields) => {
                // Validate columns match struct fields
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
            SimpleType::Tuple(elements) => {
                if elements.len() != columns.len() {
                    return Err(SqlQueryError::ColumnMismatch(format!(
                        "expected {} columns for tuple, got {}",
                        elements.len(),
                        columns.len()
                    )));
                }
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
            // Single primitive — delegate to to_expr_value
            _ => {
                if columns.len() != 1 {
                    return Err(SqlQueryError::ColumnMismatch(format!(
                        "expected 1 column for primitive type, got {}",
                        columns.len()
                    )));
                }
                let col_name = &columns[0];
                let db_val = &row[col_name];
                db_val.clone().to_expr_value(env, row_type).map_err(|_| {
                    SqlQueryError::ResultConversionError {
                        row: row_idx,
                        column: col_name.clone(),
                    }
                })
            }
        }
    } else {
        // Multiple variants (fork) — delegate to to_expr_value (single column expected)
        if columns.len() != 1 {
            return Err(SqlQueryError::ColumnMismatch(format!(
                "expected 1 column for primitive type, got {}",
                columns.len()
            )));
        }
        let col_name = &columns[0];
        let db_val = &row[col_name];
        db_val.clone().to_expr_value(env, row_type).map_err(|_| {
            SqlQueryError::ResultConversionError {
                row: row_idx,
                column: col_name.clone(),
            }
        })
    }
}

// =============================================================================
// DatabaseHandle::query
// =============================================================================

impl<C: DatabaseConnection> DatabaseHandle<C> {
    pub async fn query<T: EvalObject, D: DatabaseDriver<Connection = C>>(
        &self,
        sql: &str,
        params: Vec<ExprValue<T, C>>,
        out_type: ExprType,
        global_env: &GlobalEnv<D>,
    ) -> Result<ExprValue<T, C>, SqlQueryError> {
        // 1. Convert params
        let db_params = convert_params(params)?;

        // 2. Execute raw query
        let (rows, columns) = self.inner.query(sql, db_params).await?;

        // 3. Build result by walking the type tree breadth-first
        build_query_result(&out_type, &rows, &columns, global_env)
    }
}
