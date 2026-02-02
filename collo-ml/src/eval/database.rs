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

use super::values::ExprValue;

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

/// Wrap an `ExprValue` to match a target `ExprType`, adding `Custom` wrappers as needed.
///
/// Iterates over the target's variants and returns the first successful wrapping.
fn wrap_expr_value<T: EvalObject, D: DatabaseDriver>(
    value: ExprValue<T, D::Connection>,
    target: &ExprType,
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, DbConversionError> {
    for variant in target.get_variants() {
        if let Ok(v) = try_wrap_to(value.clone(), variant, env) {
            return Ok(v);
        }
    }
    Err(DbConversionError)
}

/// Try to wrap an `ExprValue` to match a single `SimpleType` variant.
fn try_wrap_to<T: EvalObject, D: DatabaseDriver>(
    value: ExprValue<T, D::Connection>,
    variant: &SimpleType,
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, DbConversionError> {
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

fn classify_output_type<D: DatabaseDriver>(
    out_type: &ExprType,
    env: &GlobalEnv<D>,
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

fn convert_row<T: EvalObject, D: DatabaseDriver>(
    row: &BTreeMap<String, DbValue>,
    columns: &[String],
    shape: &ResolvedRowShape,
    row_idx: usize,
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, SqlQueryError> {
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

fn build_empty_result<T: EvalObject, D: DatabaseDriver>(
    mode: &OutputMode,
    out_type: &ExprType,
    env: &GlobalEnv<D>,
) -> Result<ExprValue<T, D::Connection>, SqlQueryError> {
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
