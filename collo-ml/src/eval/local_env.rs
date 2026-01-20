//! Local environment for expression evaluation.
//!
//! This module defines:
//! - `LocalEvalEnv`: Manages local variable scopes during expression evaluation

use super::checked_ast::EvalError;
use super::history::EvalHistory;
use super::values::{CustomValue, ExprValue};
use super::variables::{ExternVar, IlpVar, ScriptVar};
use crate::ast::{Span, Spanned};
use crate::semantics::{resolve_path, LocalEnvCheck, ResolvedPathKind, SimpleType};
use crate::traits::EvalObject;
use collomatique_ilp::LinExpr;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalEvalEnv<T: EvalObject> {
    scopes: Vec<HashMap<String, ExprValue<T>>>,
    pending_scope: HashMap<String, ExprValue<T>>,
    current_module: String,
}

impl<T: EvalObject> LocalEnvCheck for LocalEvalEnv<T> {
    fn has_ident(&self, ident: &str) -> bool {
        self.lookup_ident(ident).is_some()
    }
}

impl<T: EvalObject> LocalEvalEnv<T> {
    pub(crate) fn new(current_module: &str) -> Self {
        LocalEvalEnv {
            scopes: vec![],
            pending_scope: HashMap::new(),
            current_module: current_module.to_string(),
        }
    }

    pub(crate) fn current_module(&self) -> &str {
        &self.current_module
    }

    fn lookup_ident(&self, ident: &str) -> Option<ExprValue<T>> {
        // We don't look in pending scope as these variables are not yet accessible
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(ident) {
                return Some(value.clone());
            };
        }
        None
    }

    pub(crate) fn push_scope(&mut self) {
        let mut old_scope = HashMap::new();
        std::mem::swap(&mut old_scope, &mut self.pending_scope);

        self.scopes.push(old_scope);
    }

    pub(crate) fn pop_scope(&mut self) {
        assert!(!self.scopes.is_empty());

        self.pending_scope = self.scopes.pop().unwrap();
        self.pending_scope.clear();
    }

    pub(crate) fn register_identifier(&mut self, ident: &str, value: ExprValue<T>) {
        assert!(!self.pending_scope.contains_key(ident));

        self.pending_scope.insert(ident.to_string(), value);
    }

    pub(crate) fn eval_expr(
        &mut self,
        eval_history: &mut EvalHistory<'_, T>,
        expr: &Spanned<crate::ast::Expr>,
    ) -> Result<ExprValue<T>, EvalError<T>> {
        use crate::ast::Expr;
        Ok(match &expr.node {
            Expr::None => ExprValue::None,
            Expr::Boolean(val) => ExprValue::Bool(*val),
            Expr::Number(val) => ExprValue::Int(*val),
            Expr::StringLiteral(val) => ExprValue::String(val.clone()),
            Expr::IdentPath(path) => {
                // Use resolve_path for unified resolution
                let resolved = resolve_path(
                    path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(self),
                )
                .expect("Path should be valid in a checked AST");

                match resolved {
                    ResolvedPathKind::LocalVariable(name) => self
                        .lookup_ident(&name)
                        .expect("Local variable should exist"),
                    ResolvedPathKind::Function { .. } => {
                        panic!("Function reference without call should not appear in IdentPath")
                    }
                    ResolvedPathKind::Type(simple_type) => {
                        // Unit enum variant or None type
                        match simple_type {
                            SimpleType::None => ExprValue::None,
                            SimpleType::Custom(module, root, Some(variant)) => {
                                // Qualified unit value: Enum::UnitVariant
                                ExprValue::Custom(Box::new(CustomValue {
                                    module,
                                    type_name: root,
                                    variant: Some(variant),
                                    content: ExprValue::None,
                                }))
                            }
                            _ => panic!("Unexpected type in IdentPath: {:?}", simple_type),
                        }
                    }
                    ResolvedPathKind::Module(_)
                    | ResolvedPathKind::ExternalVariable(_)
                    | ResolvedPathKind::InternalVariable { .. }
                    | ResolvedPathKind::VariableList { .. } => {
                        panic!(
                            "Module/Variable should not appear in IdentPath after semantic check"
                        )
                    }
                }
            }
            Expr::Path { object, segments } => {
                use crate::ast::PathSegment;
                assert!(!segments.is_empty());

                let mut current_value = self.eval_expr(eval_history, &object)?;

                // Helper to unwrap Custom values for field/index access
                fn unwrap_custom<T: EvalObject>(value: ExprValue<T>) -> ExprValue<T> {
                    match value {
                        ExprValue::Custom(custom) => unwrap_custom(custom.content),
                        other => other,
                    }
                }

                for segment in segments {
                    match &segment.node {
                        PathSegment::Field(field_name) => {
                            // Unwrap Custom types for field access
                            let unwrapped = unwrap_custom(current_value);
                            match unwrapped {
                                ExprValue::Object(obj) => {
                                    current_value = obj
                                        .field_access(
                                            &eval_history.env,
                                            &mut eval_history.cache,
                                            field_name,
                                        )
                                        .ok_or(EvalError::MissingObjectField {
                                            object: format!("{:?}", obj),
                                            typ: obj.typ_name(&eval_history.env),
                                            field: field_name.clone(),
                                        })?;
                                }
                                ExprValue::Struct(fields) => {
                                    current_value = fields
                                        .get(field_name)
                                        .cloned()
                                        .expect("Field should exist after type checking");
                                }
                                _ => panic!("Object or Struct expected for field access"),
                            }
                        }
                        PathSegment::TupleIndex(index) => {
                            // Unwrap Custom types for tuple index access
                            let unwrapped = unwrap_custom(current_value);
                            let tuple = match unwrapped {
                                ExprValue::Tuple(elements) => elements,
                                _ => panic!("Tuple expected for index access"),
                            };
                            current_value = tuple
                                .into_iter()
                                .nth(*index)
                                .expect("Index should be valid after type checking");
                        }
                        PathSegment::ListIndexFallible(index_expr) => {
                            let index = self.eval_expr(eval_history, index_expr)?;
                            let ExprValue::Int(i) = index else {
                                panic!("Index should be Int after type checking");
                            };

                            // Unwrap Custom types for list index access
                            let unwrapped = unwrap_custom(current_value);
                            let ExprValue::List(elements) = unwrapped else {
                                panic!("Should be list after type checking");
                            };

                            // Bounds check - return None if out of bounds
                            if i < 0 || (i as usize) >= elements.len() {
                                current_value = ExprValue::None;
                            } else {
                                current_value = elements.into_iter().nth(i as usize).unwrap();
                            }
                        }
                        PathSegment::ListIndexPanic(index_expr) => {
                            let index = self.eval_expr(eval_history, index_expr)?;
                            let ExprValue::Int(i) = index else {
                                panic!("Index should be Int after type checking");
                            };

                            // Unwrap Custom types for list index access
                            let unwrapped = unwrap_custom(current_value);
                            let ExprValue::List(elements) = unwrapped else {
                                panic!("Should be list after type checking");
                            };

                            // Bounds check - panic if out of bounds
                            if i < 0 || (i as usize) >= elements.len() {
                                return Err(EvalError::Panic(Box::new(ExprValue::String(
                                    format!(
                                    "list index out of bounds: index {} but list has {} elements",
                                    i,
                                    elements.len()
                                ),
                                ))));
                            }
                            current_value = elements.into_iter().nth(i as usize).unwrap();
                        }
                    }
                }

                current_value
            }
            Expr::Cardinality(list_expr) => {
                let list_value = self.eval_expr(eval_history, &list_expr)?;
                let count = match list_value {
                    ExprValue::List(list) => list.len(),
                    _ => panic!("Unexpected type for list expression"),
                };
                ExprValue::Int(
                    i32::try_from(count).expect("List length should not exceed i32 capacity"),
                )
            }
            Expr::ExplicitType { expr, typ: _ } => {
                let value = self.eval_expr(eval_history, &expr)?;
                // we do nothing: the semantic analysis has already checked everything
                // and types are relevant only in the semantic phase
                value
            }
            Expr::ComplexTypeCast { typ, args } => {
                // For type casts like [LinExpr]([1,2,3]) or (Int, Bool)(1, true)
                // We expect exactly one argument
                if args.len() != 1 {
                    panic!("ComplexTypeCast expects exactly one argument");
                }
                let value = self.eval_expr(eval_history, &args[0])?;

                let orig_type = eval_history.ast.get_resolved_type(&typ.span);
                let target_type = orig_type
                    .as_simple()
                    .expect("ComplexTypeCast should have a simple type as target");

                unsafe {
                    value.convert_to_unchecked(
                        eval_history.env,
                        &mut eval_history.cache,
                        target_type,
                    )
                }
            }
            Expr::StructCall { path, fields } => {
                // Use resolve_path to determine what this path refers to
                let resolved = resolve_path(
                    path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(self),
                )
                .expect("Path should be valid in a checked AST");

                let (module, type_name, variant_name) = match resolved {
                    ResolvedPathKind::Type(SimpleType::Custom(module, root, variant)) => {
                        (module, root, variant)
                    }
                    _ => panic!("StructCall should resolve to a Custom type"),
                };

                // Evaluate all fields
                let mut field_values = std::collections::BTreeMap::new();
                for (name, expr) in fields {
                    let value = self.eval_expr(eval_history, &expr)?;
                    field_values.insert(name.node.clone(), value);
                }

                // Wrap in custom type
                ExprValue::Custom(Box::new(CustomValue {
                    module,
                    type_name,
                    variant: variant_name,
                    content: ExprValue::Struct(field_values),
                }))
            }
            Expr::CastFallible { expr, typ } => {
                let value = self.eval_expr(eval_history, &expr)?;
                let target_type = eval_history.ast.get_resolved_type(&typ.span);

                // Check if value fits in target type
                if value.fits_in_typ(eval_history.env, target_type) {
                    value
                } else {
                    ExprValue::None
                }
            }
            Expr::CastPanic { expr, typ } => {
                let value = self.eval_expr(eval_history, &expr)?;
                let target_type = eval_history.ast.get_resolved_type(&typ.span);

                // Check if value fits in target type
                if value.fits_in_typ(eval_history.env, target_type) {
                    value
                } else {
                    return Err(EvalError::Panic(Box::new(ExprValue::String(format!(
                        "cast! failed: value {} does not fit in type {}",
                        value, target_type
                    )))));
                }
            }
            Expr::ListLiteral { elements } => {
                let element_values: Vec<_> = elements
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;

                ExprValue::List(element_values)
            }
            Expr::ListRange { start, end } => {
                let start_value = self.eval_expr(eval_history, &start)?;
                let end_value = self.eval_expr(eval_history, &end)?;

                let start_num = match start_value {
                    ExprValue::Int(v) => v,
                    _ => panic!("Int expected"),
                };
                let end_num = match end_value {
                    ExprValue::Int(v) => v,
                    _ => panic!("Int expected"),
                };

                ExprValue::List(
                    (start_num..end_num)
                        .into_iter()
                        .map(ExprValue::Int)
                        .collect(),
                )
            }
            Expr::GlobalList(typ_name) => {
                let expr_type = eval_history.ast.get_resolved_type(&typ_name.span);

                let mut collection = vec![];
                for variant in expr_type.get_variants() {
                    let typ_as_str = match &variant {
                        SimpleType::Object(obj) => obj.clone(),
                        _ => panic!("Object expected"),
                    };
                    let objects = T::objects_with_typ(&eval_history.env, &typ_as_str);
                    collection.extend(objects.into_iter().map(|x| ExprValue::Object(x)));
                }

                ExprValue::List(collection)
            }
            Expr::GenericCall { path, args } => {
                // Use resolve_path to determine what this path refers to
                let resolved = resolve_path(
                    path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(self),
                )
                .expect("Path should be valid in a checked AST");

                match resolved {
                    ResolvedPathKind::LocalVariable(_) => {
                        panic!("Cannot call a local variable")
                    }
                    ResolvedPathKind::Function { module, func } => {
                        // Function call
                        let args = args
                            .iter()
                            .map(|x| self.eval_expr(eval_history, &x))
                            .collect::<Result<_, _>>()?;
                        eval_history
                            .add_fn_to_call_history(&module, &func, args, true)?
                            .0
                            .into()
                    }
                    ResolvedPathKind::Type(simple_type) => {
                        // Type cast: BuiltinType(x), CustomType(x), Enum::Variant(x)
                        self.eval_generic_call_type_cast(eval_history, &simple_type, args)?
                    }
                    ResolvedPathKind::Module(_)
                    | ResolvedPathKind::ExternalVariable(_)
                    | ResolvedPathKind::InternalVariable { .. }
                    | ResolvedPathKind::VariableList { .. } => {
                        panic!(
                            "Module/Variable should not appear in GenericCall after semantic check"
                        )
                    }
                }
            }
            Expr::VarCall { module, name, args } => {
                // Build NamespacePath with $ prefix on the variable name
                let var_name_with_dollar = format!("${}", name.node);

                let segments = match module {
                    Some(mod_span) => vec![
                        mod_span.clone(),
                        Spanned::new(var_name_with_dollar, name.span.clone()),
                    ],
                    None => vec![Spanned::new(var_name_with_dollar, name.span.clone())],
                };

                let full_span = match module {
                    Some(mod_span) => Span {
                        start: mod_span.span.start,
                        end: name.span.end,
                    },
                    None => name.span.clone(),
                };

                let path = Spanned::new(crate::ast::NamespacePath { segments }, full_span);

                let args: Vec<_> = args
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;

                match resolve_path(
                    &path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(self),
                ) {
                    Ok(ResolvedPathKind::ExternalVariable(ext_var_name)) => {
                        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new(
                            eval_history.env,
                            &mut eval_history.cache,
                            &mut eval_history.var_str_cache,
                            ext_var_name,
                            args,
                        ))))
                    }
                    Ok(ResolvedPathKind::InternalVariable {
                        module: var_module,
                        name: var_name,
                    }) => {
                        let var_desc = eval_history
                            .ast
                            .global_env
                            .get_vars()
                            .get(&(var_module.clone(), var_name.clone()))
                            .expect("Internal variable should exist after resolution");

                        eval_history.vars.insert(
                            (var_module.clone(), var_name.clone(), args.clone()),
                            var_desc.referenced_fn.clone(),
                        );
                        eval_history.add_fn_to_call_history(
                            &var_desc.referenced_fn.0,
                            &var_desc.referenced_fn.1,
                            args.clone(),
                            true,
                        )?;
                        ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar::new(
                            eval_history.env,
                            &mut eval_history.cache,
                            &mut eval_history.var_str_cache,
                            var_module,
                            var_name,
                            None,
                            args,
                        ))))
                    }
                    _ => panic!("Valid var expected (should have been caught by type checker)"),
                }
            }
            Expr::In { item, collection } => {
                let collection_value = self.eval_expr(eval_history, &*collection)?;
                let list = match collection_value {
                    ExprValue::List(list) => list,
                    _ => panic!("List expected"),
                };

                let item_value = self.eval_expr(eval_history, &*item)?;
                for elt in list {
                    if item_value == elt {
                        return Ok(ExprValue::Bool(true));
                    }
                }
                ExprValue::Bool(false)
            }
            Expr::And(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 && v2),
                    (ExprValue::Constraint(mut c1), ExprValue::Constraint(c2)) => {
                        c1.reserve(c2.len());
                        c1.extend(c2);
                        ExprValue::Constraint(c1)
                    }
                    (value1, value2) => panic!(
                        "Unexpected types for AND operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Or(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 || v2),
                    (value1, value2) => panic!(
                        "Unexpected types for OR operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Not(not_expr) => {
                let value = self.eval_expr(eval_history, &*not_expr)?;

                match value {
                    ExprValue::Bool(v) => ExprValue::Bool(!v),
                    value => panic!("Unexpected type for NOT operand: {:?}", value),
                }
            }
            Expr::NullCoalesce(lhs, rhs) => {
                let lhs_value = self.eval_expr(eval_history, &*lhs)?;
                if lhs_value == ExprValue::None {
                    self.eval_expr(eval_history, &*rhs)?
                } else {
                    lhs_value
                }
            }
            Expr::ConstraintEq(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                let ExprValue::LinExpr(lin_expr1) = (unsafe {
                    value1.convert_to_unchecked(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &SimpleType::LinExpr,
                    )
                }) else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) = (unsafe {
                    value2.convert_to_unchecked(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &SimpleType::LinExpr,
                    )
                }) else {
                    panic!("Should be a LinExpr result")
                };

                ExprValue::Constraint(Vec::from([lin_expr1.eq(&lin_expr2).into()]))
            }
            Expr::ConstraintLe(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                let ExprValue::LinExpr(lin_expr1) = (unsafe {
                    value1.convert_to_unchecked(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &SimpleType::LinExpr,
                    )
                }) else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) = (unsafe {
                    value2.convert_to_unchecked(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &SimpleType::LinExpr,
                    )
                }) else {
                    panic!("Should be a LinExpr result")
                };

                ExprValue::Constraint(Vec::from([lin_expr1.leq(&lin_expr2).into()]))
            }
            Expr::ConstraintGe(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                let ExprValue::LinExpr(lin_expr1) = (unsafe {
                    value1.convert_to_unchecked(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &SimpleType::LinExpr,
                    )
                }) else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) = (unsafe {
                    value2.convert_to_unchecked(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &SimpleType::LinExpr,
                    )
                }) else {
                    panic!("Should be a LinExpr result")
                };

                ExprValue::Constraint(Vec::from([lin_expr1.geq(&lin_expr2).into()]))
            }
            Expr::Eq(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;
                ExprValue::Bool(value1 == value2)
            }
            Expr::Ne(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;
                ExprValue::Bool(value1 != value2)
            }
            Expr::Lt(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 < v2),
                    (value1, value2) => {
                        panic!("Unexpected types for < operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Le(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 <= v2),
                    (value1, value2) => panic!(
                        "Unexpected types for <= operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Gt(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 > v2),
                    (value1, value2) => {
                        panic!("Unexpected types for > operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Ge(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 >= v2),
                    (value1, value2) => panic!(
                        "Unexpected types for >= operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Add(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                    | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                        let new_lin_expr = LinExpr::constant(int_value as f64);
                        ExprValue::LinExpr(lin_expr_value + new_lin_expr)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => ExprValue::LinExpr(v1 + v2),
                    (ExprValue::String(s1), ExprValue::String(s2)) => ExprValue::String(s1 + &s2),
                    (ExprValue::List(mut list1), ExprValue::List(list2)) => {
                        list1.reserve(list2.len());
                        list1.extend(list2);
                        ExprValue::List(list1)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for + operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Sub(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 - v2),
                    (ExprValue::Int(v1), ExprValue::LinExpr(v2)) => {
                        let new_lin_expr = LinExpr::constant(v1 as f64);
                        ExprValue::LinExpr(new_lin_expr - v2)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::Int(v2)) => {
                        let new_lin_expr = LinExpr::constant(v2 as f64);
                        ExprValue::LinExpr(v1 - new_lin_expr)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => ExprValue::LinExpr(v1 - v2),
                    (ExprValue::List(list1), ExprValue::List(list2)) => {
                        let list = list1.into_iter().filter(|x| !list2.contains(x)).collect();
                        ExprValue::List(list)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for - operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Neg(term) => {
                let value = self.eval_expr(eval_history, &*term)?;

                match value {
                    ExprValue::Int(v) => ExprValue::Int(-v),
                    ExprValue::LinExpr(v) => ExprValue::LinExpr(-v),
                    value => panic!("Unexpected type for (-) operand: {:?}", value),
                }
            }
            Expr::Panic(inner_expr) => {
                let value = self.eval_expr(eval_history, &*inner_expr)?;
                return Err(EvalError::Panic(Box::new(value)));
            }
            Expr::Mul(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 * v2),
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                    | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                        ExprValue::LinExpr(int_value * lin_expr_value)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for * operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Div(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 / v2),
                    (value1, value2) => panic!(
                        "Unexpected types for // operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Mod(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 % v2),
                    (value1, value2) => {
                        panic!("Unexpected types for % operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_value = self.eval_expr(eval_history, &condition)?;
                let ExprValue::Bool(cond) = cond_value else {
                    panic!("Expected Bool for if condition");
                };

                if cond {
                    self.eval_expr(eval_history, &then_expr)?
                } else {
                    self.eval_expr(eval_history, &else_expr)?
                }
            }
            Expr::Match {
                match_expr,
                branches,
            } => {
                let value = self.eval_expr(eval_history, match_expr)?;

                for branch in branches {
                    let does_typ_match = match &branch.as_typ {
                        Some(t) => {
                            let target_type = eval_history.ast.get_resolved_type(&t.span);
                            value.fits_in_typ(&eval_history.env, target_type)
                        }
                        None => true,
                    };

                    if !does_typ_match {
                        continue;
                    }

                    // Let's add the identifier to the scope
                    self.register_identifier(&branch.ident.node, value.clone());
                    self.push_scope();

                    // Now we check the where clause
                    let where_clause_passes = match &branch.filter {
                        None => true,
                        Some(filter_expr) => {
                            let cond_value = match self.eval_expr(eval_history, &filter_expr) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            let ExprValue::Bool(cond) = cond_value else {
                                panic!("Expected Bool for where clause");
                            };
                            cond
                        }
                    };

                    if !where_clause_passes {
                        // Where clause failed, we remove the scope and move to the next branch
                        self.pop_scope();
                        continue;
                    }

                    let output = self.eval_expr(eval_history, &branch.body);

                    self.pop_scope();
                    return output;
                }

                panic!("Match should be exhaustive during evaluation");
            }
            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;
                let ExprValue::List(list) = collection_value else {
                    panic!("Expected collection for sum. Got: {:?}", collection_value);
                };

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let mut output = match target {
                    a if a.is_lin_expr() => ExprValue::LinExpr(LinExpr::constant(0.)),
                    a if a.is_int() => ExprValue::Int(0),
                    a if a.is_list() => ExprValue::List(Vec::with_capacity(list.len())), // Heuristic for length
                    a if a.is_string() => ExprValue::String(String::new()),
                    _ => panic!("Expected Int, LinExpr, String or List output"),
                };

                for elem in list {
                    self.register_identifier(&var.node, elem);
                    self.push_scope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = match self.eval_expr(eval_history, &f) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            match filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        let new_value = match self.eval_expr(eval_history, &body) {
                            Ok(v) => v,
                            Err(e) => {
                                self.pop_scope();
                                return Err(e);
                            }
                        };
                        output = match (output, new_value) {
                            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                            (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                            | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                                let new_lin_expr = LinExpr::constant(int_value as f64);
                                ExprValue::LinExpr(lin_expr_value + new_lin_expr)
                            }
                            (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => {
                                ExprValue::LinExpr(v1 + v2)
                            }
                            (ExprValue::String(s1), ExprValue::String(s2)) => {
                                ExprValue::String(s1 + &s2)
                            }
                            (ExprValue::List(mut list), ExprValue::List(new_list)) => {
                                list.reserve(new_list.len());
                                list.extend(new_list);
                                ExprValue::List(list)
                            }
                            (value1, value2) => panic!(
                                "Unexpected types for sum operand: {:?}, {:?}",
                                value1, value2
                            ),
                        };
                    }

                    self.pop_scope();
                }

                output
            }
            Expr::Fold {
                var,
                collection,
                accumulator,
                init_value,
                filter,
                body,
                reversed,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;
                let ExprValue::List(mut list) = collection_value else {
                    panic!("Expected collection for sum. Got: {:?}", collection_value);
                };
                if *reversed {
                    list.reverse();
                }

                let mut output = self.eval_expr(eval_history, &init_value)?;

                for elem in list {
                    self.register_identifier(&var.node, elem);
                    self.register_identifier(&accumulator.node, output.clone());
                    self.push_scope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = match self.eval_expr(eval_history, &f) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            match filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        output = match self.eval_expr(eval_history, &body) {
                            Ok(v) => v,
                            Err(e) => {
                                self.pop_scope();
                                return Err(e);
                            }
                        };
                    }

                    self.pop_scope();
                }

                output
            }
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;
                let ExprValue::List(list) = collection_value else {
                    panic!("Expected collection for sum. Got: {:?}", collection_value);
                };

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let mut output = match target {
                    a if a.is_bool() => ExprValue::Bool(true),
                    a if a.is_constraint() => ExprValue::Constraint(Vec::with_capacity(list.len())), // Heuristic for length
                    _ => panic!("Expected Bool or Constraint output"),
                };

                for elem in list {
                    self.register_identifier(&var.node, elem);
                    self.push_scope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = match self.eval_expr(eval_history, &f) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            match filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        let new_value = match self.eval_expr(eval_history, &body) {
                            Ok(v) => v,
                            Err(e) => {
                                self.pop_scope();
                                return Err(e);
                            }
                        };
                        output = match (output, new_value) {
                            (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 && v2),
                            (ExprValue::Constraint(mut c1), ExprValue::Constraint(c2)) => {
                                c1.reserve(c2.len());
                                c1.extend(c2);
                                ExprValue::Constraint(c1)
                            }
                            (value1, value2) => panic!(
                                "Unexpected types for forall operand: {:?}, {:?}",
                                value1, value2
                            ),
                        };
                    }

                    self.pop_scope();
                }

                output
            }
            Expr::VarListCall { module, name, args } => {
                // Build NamespacePath with $[name] format
                let var_name_with_dollar = format!("$[{}]", name.node);

                let segments = match module {
                    Some(mod_span) => vec![
                        mod_span.clone(),
                        Spanned::new(var_name_with_dollar, name.span.clone()),
                    ],
                    None => vec![Spanned::new(var_name_with_dollar, name.span.clone())],
                };

                let full_span = match module {
                    Some(mod_span) => Span {
                        start: mod_span.span.start,
                        end: name.span.end,
                    },
                    None => name.span.clone(),
                };

                let path = Spanned::new(crate::ast::NamespacePath { segments }, full_span);

                let evaluated_args: Vec<_> = args
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;

                match resolve_path(
                    &path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(self),
                ) {
                    Ok(ResolvedPathKind::VariableList {
                        module: var_module,
                        name: var_name,
                    }) => {
                        let var_lists = eval_history.ast.get_var_lists();
                        let var_list_fn = var_lists
                            .get(&(var_module.clone(), var_name.clone()))
                            .expect("Var list should be declared");

                        let (constraints, _origin) = eval_history.add_fn_to_call_history(
                            &var_list_fn.0,
                            &var_list_fn.1,
                            evaluated_args.clone(),
                            true,
                        )?;
                        eval_history.var_lists.insert(
                            (var_module.clone(), var_name.clone(), evaluated_args.clone()),
                            var_list_fn.clone(),
                        );

                        let constraint_count = match constraints {
                            ExprValue::List(list) => list.len(),
                            _ => panic!("Expected [Constraint]"),
                        };

                        ExprValue::List(
                            (0..constraint_count)
                                .map(|i| {
                                    ExprValue::LinExpr(LinExpr::var(IlpVar::Script(
                                        ScriptVar::new(
                                            eval_history.env,
                                            &mut eval_history.cache,
                                            &mut eval_history.var_str_cache,
                                            var_module.clone(),
                                            var_name.clone(),
                                            Some(i),
                                            evaluated_args.clone(),
                                        ),
                                    )))
                                })
                                .collect(),
                        )
                    }
                    _ => {
                        panic!("Valid var list expected (should have been caught by type checker)")
                    }
                }
            }
            Expr::ListComprehension {
                body,
                vars_and_collections,
                filter,
            } => {
                let list = self.build_naked_list_for_list_comprehension(
                    eval_history,
                    &body,
                    &vars_and_collections[..],
                    filter.as_ref().map(|x| x.as_ref()),
                )?;

                ExprValue::List(list)
            }
            Expr::Let { var, value, body } => {
                let value_value = self.eval_expr(eval_history, &value)?;

                self.register_identifier(&var.node, value_value);
                self.push_scope();

                let body_value = self.eval_expr(eval_history, &body);

                self.pop_scope();

                body_value?
            }
            Expr::TupleLiteral { elements } => {
                let element_values: Vec<_> = elements
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;

                ExprValue::Tuple(element_values)
            }

            Expr::StructLiteral { fields } => {
                let field_values: BTreeMap<_, _> = fields
                    .iter()
                    .map(|(name, expr)| {
                        Ok((name.node.clone(), self.eval_expr(eval_history, expr)?))
                    })
                    .collect::<Result<_, EvalError<T>>>()?;

                ExprValue::Struct(field_values)
            }
        })
    }

    /// Helper for evaluating type casts in GenericCall expressions
    fn eval_generic_call_type_cast(
        &mut self,
        eval_history: &mut EvalHistory<'_, T>,
        simple_type: &SimpleType,
        args: &Vec<Spanned<crate::ast::Expr>>,
    ) -> Result<ExprValue<T>, EvalError<T>> {
        match simple_type {
            // Built-in type casts: Int(x), Bool(x), String(x), etc.
            SimpleType::Int
            | SimpleType::Bool
            | SimpleType::String
            | SimpleType::LinExpr
            | SimpleType::Constraint
            | SimpleType::None
            | SimpleType::Never => {
                assert!(
                    args.len() == 1,
                    "Built-in type cast should have exactly 1 argument"
                );
                let value = self.eval_expr(eval_history, &args[0])?;
                Ok(unsafe {
                    value.convert_to_unchecked(
                        eval_history.env,
                        &mut eval_history.cache,
                        &simple_type,
                    )
                })
            }

            // Custom type casts: CustomType(x), Enum::Variant(x)
            SimpleType::Custom(module, root, variant_opt) => {
                let qualified_name = match variant_opt {
                    Some(v) => format!("{}::{}", root, v),
                    None => root.clone(),
                };

                let underlying_type = eval_history
                    .ast
                    .global_env
                    .get_custom_type_underlying(module, &qualified_name)
                    .expect("Semantic analysis should have validated this type exists")
                    .clone();

                // Check if underlying type is None (unit variant like Option::None)
                let is_unit = underlying_type
                    .as_simple()
                    .map(|s| s.is_none())
                    .unwrap_or(false);
                // Check if it's a tuple type
                let is_tuple = matches!(underlying_type.to_simple(), Some(SimpleType::Tuple(_)));

                let content = if is_unit {
                    // Unit variant - args should be empty or just `none`
                    if args.is_empty() {
                        ExprValue::None
                    } else {
                        self.eval_expr(eval_history, &args[0])?
                    }
                } else if is_tuple {
                    // Tuple variant - evaluate all args
                    let values: Vec<ExprValue<T>> = args
                        .iter()
                        .map(|x| self.eval_expr(eval_history, &x))
                        .collect::<Result<_, _>>()?;
                    ExprValue::Tuple(values)
                } else {
                    // Single value variant
                    self.eval_expr(eval_history, &args[0])?
                };

                Ok(ExprValue::Custom(Box::new(CustomValue {
                    module: module.clone(),
                    type_name: root.clone(),
                    variant: variant_opt.clone(),
                    content,
                })))
            }

            // Other types shouldn't appear in GenericCall
            _ => panic!("Unexpected type in GenericCall: {:?}", simple_type),
        }
    }

    fn build_naked_list_for_list_comprehension(
        &mut self,
        eval_history: &mut EvalHistory<'_, T>,
        body: &Spanned<crate::ast::Expr>,
        vars_and_collections: &[(Spanned<String>, Spanned<crate::ast::Expr>)],
        filter: Option<&Spanned<crate::ast::Expr>>,
    ) -> Result<Vec<ExprValue<T>>, EvalError<T>> {
        if vars_and_collections.is_empty() {
            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = self.eval_expr(eval_history, &f)?;
                    match filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            return Ok(if cond {
                Vec::from([self.eval_expr(eval_history, &body)?])
            } else {
                Vec::new()
            });
        }

        let (var, collection) = &vars_and_collections[0];
        let remaining_v_and_c = &vars_and_collections[1..];

        let collection_value = self.eval_expr(eval_history, &collection)?;
        let ExprValue::List(list) = collection_value else {
            panic!("Expected list. Got: {:?}", collection_value);
        };

        let mut output = Vec::new();

        for elem in list {
            self.register_identifier(&var.node, elem);
            self.push_scope();

            let extension = self.build_naked_list_for_list_comprehension(
                eval_history,
                body,
                remaining_v_and_c,
                filter,
            );

            self.pop_scope();

            output.extend(extension?);
        }

        Ok(output)
    }
}
