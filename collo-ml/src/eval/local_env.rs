//! Local environment for expression evaluation.
//!
//! This module defines:
//! - `LocalEvalEnv`: Manages local variable scopes during expression evaluation

use super::checked_ast::EvalError;
use super::history::EvalHistory;
use super::values::{CustomValue, ExprValue};
use super::variables::{ExternVar, IlpVar, ScriptVar};
use crate::ast::{Span, Spanned};
use crate::database::{DatabaseConnection, DatabaseDriver};
use crate::semantics::{LocalEnvCheck, ResolvedPathKind, SimpleType, resolve_path};
use crate::traits::EvalObject;
use collomatique_ilp::LinExpr;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct LocalEvalEnv<T: EvalObject, D: DatabaseDriver> {
    scope: HashMap<String, Arc<ExprValue<T, D::Connection>>>,
    parent: Option<Arc<LocalEvalEnv<T, D>>>,
    current_module: Arc<str>,
}

pub(crate) struct SubscopeBuilder<T: EvalObject, D: DatabaseDriver> {
    identifiers: HashMap<String, Arc<ExprValue<T, D::Connection>>>,
    parent: Arc<LocalEvalEnv<T, D>>,
}

impl<T: EvalObject, D: DatabaseDriver> SubscopeBuilder<T, D> {
    pub(crate) fn register_identifier(
        &mut self,
        ident: &str,
        value: Arc<ExprValue<T, D::Connection>>,
    ) {
        assert!(!self.identifiers.contains_key(ident));
        self.identifiers.insert(ident.to_string(), value);
    }

    pub(crate) fn build_subscope(self) -> Arc<LocalEvalEnv<T, D>> {
        let current_module = Arc::clone(&self.parent.current_module);
        Arc::new(LocalEvalEnv {
            scope: self.identifiers,
            parent: Some(self.parent),
            current_module,
        })
    }
}

impl<T: EvalObject, D: DatabaseDriver> LocalEnvCheck for LocalEvalEnv<T, D> {
    fn has_ident(&self, ident: &str) -> bool {
        self.lookup_ident(ident).is_some()
    }
}

impl<T: EvalObject, D: DatabaseDriver> LocalEvalEnv<T, D> {
    pub(crate) fn new(current_module: &str) -> Arc<Self> {
        Arc::new(LocalEvalEnv {
            scope: HashMap::new(),
            parent: None,
            current_module: Arc::from(current_module),
        })
    }

    pub(crate) fn current_module(&self) -> &str {
        &self.current_module
    }

    fn lookup_ident(&self, ident: &str) -> Option<Arc<ExprValue<T, D::Connection>>> {
        if let Some(value) = self.scope.get(ident) {
            return Some(Arc::clone(value));
        }
        self.parent.as_ref().and_then(|p| p.lookup_ident(ident))
    }

    pub(crate) fn start_subscope(parent: Arc<Self>) -> SubscopeBuilder<T, D> {
        SubscopeBuilder {
            identifiers: HashMap::new(),
            parent,
        }
    }

    pub(crate) async fn eval_expr(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, T, D>,
        expr: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<T, D::Connection>>, EvalError<T, D::Connection>> {
        use crate::ast::Expr;
        Ok(match &expr.node {
            Expr::None => Arc::new(ExprValue::None),
            Expr::Boolean(val) => Arc::new(ExprValue::Bool(*val)),
            Expr::Number(val) => Arc::new(ExprValue::Int(*val)),
            Expr::StringLiteral(val) => Arc::new(ExprValue::String(val.clone())),
            Expr::IdentPath(path) => {
                // Use resolve_path for unified resolution
                let resolved = resolve_path(
                    path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(&*self),
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
                            SimpleType::None => Arc::new(ExprValue::None),
                            SimpleType::Custom(module, root, Some(variant)) => {
                                // Qualified unit value: Enum::UnitVariant
                                Arc::new(ExprValue::Custom(CustomValue {
                                    module,
                                    type_name: root,
                                    variant: Some(variant),
                                    content: Arc::new(ExprValue::None),
                                }))
                            }
                            _ => panic!("Unexpected type in IdentPath: {:?}", simple_type),
                        }
                    }
                    ResolvedPathKind::Query { .. } => {
                        panic!("Query reference without call should not appear in IdentPath")
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

                let mut current_value: Arc<ExprValue<T, D::Connection>> =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(object))).await?;

                // Helper to unwrap Custom values for field/index access
                fn unwrap_custom<T: EvalObject, D: DatabaseConnection>(
                    value: Arc<ExprValue<T, D>>,
                ) -> Arc<ExprValue<T, D>> {
                    match &*value {
                        ExprValue::Custom(custom) => unwrap_custom(Arc::clone(&custom.content)),
                        _ => value,
                    }
                }

                for segment in segments {
                    match &segment.node {
                        PathSegment::Field(field_name) => {
                            // Unwrap Custom types for field access
                            let unwrapped = unwrap_custom(current_value);
                            match &*unwrapped {
                                ExprValue::Struct(fields) => {
                                    current_value = Arc::clone(
                                        fields
                                            .get(field_name)
                                            .expect("Field should exist after type checking"),
                                    );
                                }
                                _ => panic!("Struct expected for field access"),
                            }
                        }
                        PathSegment::TupleIndex(index) => {
                            // Unwrap Custom types for tuple index access
                            let unwrapped = unwrap_custom(current_value);
                            match &*unwrapped {
                                ExprValue::Tuple(elements) => {
                                    current_value = Arc::clone(&elements[*index]);
                                }
                                _ => panic!("Tuple expected for index access"),
                            };
                        }
                        PathSegment::ListIndexFallible(index_expr) => {
                            let index_arc = Box::pin(
                                Arc::clone(&self).eval_expr(eval_history, Arc::clone(index_expr)),
                            )
                            .await?;
                            let ExprValue::Int(i) = *index_arc else {
                                panic!("Index should be Int after type checking");
                            };

                            // Unwrap Custom types for list index access
                            let unwrapped = unwrap_custom(current_value);
                            let ExprValue::List(elements) = &*unwrapped else {
                                panic!("Should be list after type checking");
                            };

                            // Bounds check - return None if out of bounds
                            if i < 0 || (i as usize) >= elements.len() {
                                current_value = Arc::new(ExprValue::None);
                            } else {
                                current_value = Arc::clone(&elements[i as usize]);
                            }
                        }
                        PathSegment::ListIndexPanic(index_expr) => {
                            let index_arc = Box::pin(
                                Arc::clone(&self).eval_expr(eval_history, Arc::clone(index_expr)),
                            )
                            .await?;
                            let ExprValue::Int(i) = *index_arc else {
                                panic!("Index should be Int after type checking");
                            };

                            // Unwrap Custom types for list index access
                            let unwrapped = unwrap_custom(current_value);
                            let ExprValue::List(elements) = &*unwrapped else {
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
                            current_value = Arc::clone(&elements[i as usize]);
                        }
                    }
                }

                current_value
            }
            Expr::Cardinality(list_expr) => {
                let list_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(list_expr)))
                        .await?;
                let count = match &*list_value {
                    ExprValue::List(list) => list.len(),
                    _ => panic!("Unexpected type for list expression"),
                };
                Arc::new(ExprValue::Int(
                    i32::try_from(count).expect("List length should not exceed i32 capacity"),
                ))
            }
            Expr::ExplicitType { expr, typ: _ } => {
                // we do nothing: the semantic analysis has already checked everything
                // and types are relevant only in the semantic phase
                Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr))).await?
            }
            Expr::ComplexTypeCast { typ, args } => {
                // For type casts like [LinExpr]([1,2,3]) or (Int, Bool)(1, true)
                // We expect exactly one argument
                if args.len() != 1 {
                    panic!("ComplexTypeCast expects exactly one argument");
                }
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(&args[0])))
                        .await?;

                let orig_type = eval_history.ast.get_resolved_type(&typ.span);
                let target_type = orig_type
                    .as_simple()
                    .expect("ComplexTypeCast should have a simple type as target");

                Arc::new(unsafe { value.convert_to_unchecked(target_type) })
            }
            Expr::StructCall { path, fields } => {
                // Use resolve_path to determine what this path refers to
                let resolved = resolve_path(
                    path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(&*self),
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
                    let value =
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr)))
                            .await?;
                    field_values.insert(name.node.clone(), value);
                }

                // Wrap in custom type
                Arc::new(ExprValue::Custom(CustomValue {
                    module,
                    type_name,
                    variant: variant_name,
                    content: Arc::new(ExprValue::Struct(field_values)),
                }))
            }
            Expr::CastFallible { expr, typ } => {
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr))).await?;
                let target_type = eval_history.ast.get_resolved_type(&typ.span);

                // Check if value fits in target type
                if value.fits_in_typ(target_type) {
                    value
                } else {
                    Arc::new(ExprValue::None)
                }
            }
            Expr::CastPanic { expr, typ } => {
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr))).await?;
                let target_type = eval_history.ast.get_resolved_type(&typ.span);

                // Check if value fits in target type
                if value.fits_in_typ(target_type) {
                    value
                } else {
                    return Err(EvalError::Panic(Box::new(ExprValue::String(format!(
                        "cast! failed: value {} does not fit in type {}",
                        value, target_type
                    )))));
                }
            }
            Expr::ListLiteral { elements } => {
                let mut element_values = Vec::with_capacity(elements.len());
                for x in elements {
                    element_values.push(
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x))).await?,
                    );
                }

                Arc::new(ExprValue::List(element_values))
            }
            Expr::ListRange { start, end } => {
                let start_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(start))).await?;
                let end_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(end))).await?;

                let start_num = match &*start_value {
                    ExprValue::Int(v) => *v,
                    _ => panic!("Int expected"),
                };
                let end_num = match &*end_value {
                    ExprValue::Int(v) => *v,
                    _ => panic!("Int expected"),
                };

                Arc::new(ExprValue::List(
                    (start_num..end_num)
                        .map(|i| Arc::new(ExprValue::Int(i)))
                        .collect(),
                ))
            }
            Expr::GenericCall { path, args } => {
                // Use resolve_path to determine what this path refers to
                let resolved = resolve_path(
                    path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(&*self),
                )
                .expect("Path should be valid in a checked AST");

                match resolved {
                    ResolvedPathKind::LocalVariable(_) => {
                        panic!("Cannot call a local variable")
                    }
                    ResolvedPathKind::Function { module, func } => {
                        // Function call
                        let mut eval_args = Vec::with_capacity(args.len());
                        for x in args {
                            eval_args.push(
                                Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x)))
                                    .await?,
                            );
                        }
                        Box::pin(
                            eval_history.add_fn_to_call_history(&module, &func, eval_args, true),
                        )
                        .await?
                        .0
                    }
                    ResolvedPathKind::Type(simple_type) => {
                        // Type cast: BuiltinType(x), CustomType(x), Enum::Variant(x)
                        Box::pin(Arc::clone(&self).eval_generic_call_type_cast(
                            eval_history,
                            &simple_type,
                            args,
                        ))
                        .await?
                    }
                    ResolvedPathKind::Query { module, name } => {
                        let mut eval_args = Vec::with_capacity(args.len());
                        for x in args {
                            eval_args.push(
                                Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x)))
                                    .await?,
                            );
                        }
                        Box::pin(eval_history.add_query_to_call_history(&module, &name, eval_args))
                            .await?
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

                let mut eval_args: Vec<Arc<ExprValue<T, D::Connection>>> =
                    Vec::with_capacity(args.len());
                for x in args {
                    eval_args.push(
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x))).await?,
                    );
                }
                let args = eval_args;

                match resolve_path(
                    &path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(&*self),
                ) {
                    Ok(ResolvedPathKind::ExternalVariable(ext_var_name)) => {
                        Arc::new(ExprValue::LinExpr(LinExpr::var(IlpVar::Base(
                            ExternVar::new(ext_var_name, args),
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
                        Box::pin(eval_history.add_fn_to_call_history(
                            &var_desc.referenced_fn.0,
                            &var_desc.referenced_fn.1,
                            args.clone(),
                            true,
                        ))
                        .await?;
                        Arc::new(ExprValue::LinExpr(LinExpr::var(IlpVar::Script(
                            ScriptVar::new(var_module, var_name, None, args),
                        ))))
                    }
                    _ => panic!("Valid var expected (should have been caught by type checker)"),
                }
            }
            Expr::In { item, collection } => {
                let collection_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(collection)))
                        .await?;
                let list = match &*collection_value {
                    ExprValue::List(list) => list,
                    _ => panic!("List expected"),
                };

                let item_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(item))).await?;
                for elt in list {
                    if *item_value == **elt {
                        return Ok(Arc::new(ExprValue::Bool(true)));
                    }
                }
                Arc::new(ExprValue::Bool(false))
            }
            Expr::And(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(*v1 && *v2),
                    (ExprValue::Constraint(c1), ExprValue::Constraint(c2)) => {
                        let mut result = c1.clone();
                        result.reserve(c2.len());
                        result.extend(c2.iter().cloned());
                        ExprValue::Constraint(result)
                    }
                    (value1, value2) => panic!(
                        "Unexpected types for AND operand: {:?}, {:?}",
                        value1, value2
                    ),
                })
            }
            Expr::Or(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(*v1 || *v2),
                    (value1, value2) => panic!(
                        "Unexpected types for OR operand: {:?}, {:?}",
                        value1, value2
                    ),
                })
            }
            Expr::Not(not_expr) => {
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(not_expr)))
                        .await?;

                Arc::new(match &*value {
                    ExprValue::Bool(v) => ExprValue::Bool(!v),
                    value => panic!("Unexpected type for NOT operand: {:?}", value),
                })
            }
            Expr::NullCoalesce(lhs, rhs) => {
                let lhs_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(lhs))).await?;
                if *lhs_value == ExprValue::None {
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(rhs))).await?
                } else {
                    lhs_value
                }
            }
            Expr::ConstraintEq(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                let ExprValue::LinExpr(lin_expr1) =
                    (unsafe { value1.convert_to_unchecked(&SimpleType::LinExpr) })
                else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) =
                    (unsafe { value2.convert_to_unchecked(&SimpleType::LinExpr) })
                else {
                    panic!("Should be a LinExpr result")
                };

                Arc::new(ExprValue::Constraint(Vec::from([lin_expr1
                    .eq(&lin_expr2)
                    .into()])))
            }
            Expr::ConstraintLe(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                let ExprValue::LinExpr(lin_expr1) =
                    (unsafe { value1.convert_to_unchecked(&SimpleType::LinExpr) })
                else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) =
                    (unsafe { value2.convert_to_unchecked(&SimpleType::LinExpr) })
                else {
                    panic!("Should be a LinExpr result")
                };

                Arc::new(ExprValue::Constraint(Vec::from([lin_expr1
                    .leq(&lin_expr2)
                    .into()])))
            }
            Expr::ConstraintGe(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                let ExprValue::LinExpr(lin_expr1) =
                    (unsafe { value1.convert_to_unchecked(&SimpleType::LinExpr) })
                else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) =
                    (unsafe { value2.convert_to_unchecked(&SimpleType::LinExpr) })
                else {
                    panic!("Should be a LinExpr result")
                };

                Arc::new(ExprValue::Constraint(Vec::from([lin_expr1
                    .geq(&lin_expr2)
                    .into()])))
            }
            Expr::Eq(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;
                Arc::new(ExprValue::Bool(*value1 == *value2))
            }
            Expr::Ne(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;
                Arc::new(ExprValue::Bool(*value1 != *value2))
            }
            Expr::Lt(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 < v2),
                    (value1, value2) => {
                        panic!("Unexpected types for < operand: {:?}, {:?}", value1, value2)
                    }
                })
            }
            Expr::Le(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 <= v2),
                    (value1, value2) => panic!(
                        "Unexpected types for <= operand: {:?}, {:?}",
                        value1, value2
                    ),
                })
            }
            Expr::Gt(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 > v2),
                    (value1, value2) => {
                        panic!("Unexpected types for > operand: {:?}, {:?}", value1, value2)
                    }
                })
            }
            Expr::Ge(expr1, expr2) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr1))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr2))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 >= v2),
                    (value1, value2) => panic!(
                        "Unexpected types for >= operand: {:?}, {:?}",
                        value1, value2
                    ),
                })
            }
            Expr::Add(left, right) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(left))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(right))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                    | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                        let new_lin_expr = LinExpr::constant(*int_value as f64);
                        ExprValue::LinExpr(lin_expr_value.clone() + new_lin_expr)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => {
                        ExprValue::LinExpr(v1.clone() + v2.clone())
                    }
                    (ExprValue::String(s1), ExprValue::String(s2)) => {
                        ExprValue::String(s1.clone() + s2)
                    }
                    (ExprValue::List(list1), ExprValue::List(list2)) => {
                        let mut result = list1.clone();
                        result.reserve(list2.len());
                        result.extend(list2.iter().cloned());
                        ExprValue::List(result)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for + operand: {:?}, {:?}", value1, value2)
                    }
                })
            }
            Expr::Sub(left, right) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(left))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(right))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 - v2),
                    (ExprValue::Int(v1), ExprValue::LinExpr(v2)) => {
                        let new_lin_expr = LinExpr::constant(*v1 as f64);
                        ExprValue::LinExpr(new_lin_expr - v2.clone())
                    }
                    (ExprValue::LinExpr(v1), ExprValue::Int(v2)) => {
                        let new_lin_expr = LinExpr::constant(*v2 as f64);
                        ExprValue::LinExpr(v1.clone() - new_lin_expr)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => {
                        ExprValue::LinExpr(v1.clone() - v2.clone())
                    }
                    (ExprValue::List(list1), ExprValue::List(list2)) => {
                        let list = list1
                            .iter()
                            .filter(|x| !list2.contains(x))
                            .cloned()
                            .collect();
                        ExprValue::List(list)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for - operand: {:?}, {:?}", value1, value2)
                    }
                })
            }
            Expr::Neg(term) => {
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(term))).await?;

                Arc::new(match &*value {
                    ExprValue::Int(v) => ExprValue::Int(-v),
                    ExprValue::LinExpr(v) => ExprValue::LinExpr(-v.clone()),
                    value => panic!("Unexpected type for (-) operand: {:?}", value),
                })
            }
            Expr::Panic(inner_expr) => {
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(inner_expr)))
                        .await?;
                return Err(EvalError::Panic(Box::new((*value).clone())));
            }
            Expr::Mul(left, right) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(left))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(right))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                    | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                        ExprValue::LinExpr(*int_value * lin_expr_value.clone())
                    }
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 * v2),
                    (value1, value2) => {
                        panic!("Unexpected types for * operand: {:?}, {:?}", value1, value2)
                    }
                })
            }
            Expr::Div(left, right) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(left))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(right))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 / v2),
                    (value1, value2) => panic!(
                        "Unexpected types for // operand: {:?}, {:?}",
                        value1, value2
                    ),
                })
            }
            Expr::Mod(left, right) => {
                let value1 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(left))).await?;
                let value2 =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(right))).await?;

                Arc::new(match (&*value1, &*value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 % v2),
                    (value1, value2) => {
                        panic!("Unexpected types for % operand: {:?}, {:?}", value1, value2)
                    }
                })
            }
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(condition)))
                        .await?;
                let ExprValue::Bool(cond) = *cond_value else {
                    panic!("Expected Bool for if condition");
                };

                if cond {
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(then_expr)))
                        .await?
                } else {
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(else_expr)))
                        .await?
                }
            }
            Expr::Match {
                match_expr,
                branches,
            } => {
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(match_expr)))
                        .await?;

                for branch in branches {
                    let does_typ_match = match &branch.as_typ {
                        Some(t) => {
                            let target_type = eval_history.ast.get_resolved_type(&t.span);
                            value.fits_in_typ(target_type)
                        }
                        None => true,
                    };

                    if !does_typ_match {
                        continue;
                    }

                    // Let's add the identifier to a subscope
                    let mut builder = Self::start_subscope(Arc::clone(&self));
                    builder.register_identifier(&branch.ident.node, Arc::clone(&value));
                    let subscope = builder.build_subscope();

                    // Now we check the where clause
                    let where_clause_passes = match &branch.filter {
                        None => true,
                        Some(filter_expr) => {
                            let cond_value = Box::pin(
                                Arc::clone(&subscope)
                                    .eval_expr(eval_history, Arc::clone(filter_expr)),
                            )
                            .await?;
                            let ExprValue::Bool(cond) = *cond_value else {
                                panic!("Expected Bool for where clause");
                            };
                            cond
                        }
                    };

                    if !where_clause_passes {
                        // Where clause failed, subscope is dropped, move to the next branch
                        continue;
                    }

                    let output = Box::pin(
                        Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(&branch.body)),
                    )
                    .await;
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
                let collection_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(collection)))
                        .await?;
                let list = match &*collection_value {
                    ExprValue::List(list) => list.clone(),
                    other => panic!("Expected collection for sum. Got: {:?}", other),
                };

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let mut output = match target {
                    a if a.is_lin_expr() => ExprValue::LinExpr(LinExpr::constant(0.)),
                    a if a.is_int() => ExprValue::Int(0),
                    a if a.is_list() => ExprValue::List(Vec::with_capacity(list.len())),
                    a if a.is_string() => ExprValue::String(String::new()),
                    _ => panic!("Expected Int, LinExpr, String or List output"),
                };

                for elem in list {
                    let mut builder = Self::start_subscope(Arc::clone(&self));
                    builder.register_identifier(&var.node, elem);
                    let subscope = builder.build_subscope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = Box::pin(
                                Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(f)),
                            )
                            .await?;
                            match *filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        let new_value_arc = Box::pin(
                            Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(body)),
                        )
                        .await?;
                        output = match (output, &*new_value_arc) {
                            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                            (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value)) => {
                                let new_lin_expr = LinExpr::constant(int_value as f64);
                                ExprValue::LinExpr(lin_expr_value.clone() + new_lin_expr)
                            }
                            (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                                let new_lin_expr = LinExpr::constant(*int_value as f64);
                                ExprValue::LinExpr(lin_expr_value + new_lin_expr)
                            }
                            (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => {
                                ExprValue::LinExpr(v1 + v2.clone())
                            }
                            (ExprValue::String(s1), ExprValue::String(s2)) => {
                                ExprValue::String(s1 + s2)
                            }
                            (ExprValue::List(mut list), ExprValue::List(new_list)) => {
                                list.reserve(new_list.len());
                                list.extend(new_list.iter().cloned());
                                ExprValue::List(list)
                            }
                            (value1, value2) => panic!(
                                "Unexpected types for sum operand: {:?}, {:?}",
                                value1, value2
                            ),
                        };
                    }
                }

                Arc::new(output)
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
                let collection_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(collection)))
                        .await?;
                let mut list = match &*collection_value {
                    ExprValue::List(list) => list.clone(),
                    other => panic!("Expected collection for fold. Got: {:?}", other),
                };
                if *reversed {
                    list.reverse();
                }

                let mut output =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(init_value)))
                        .await?;

                for elem in list {
                    let mut builder = Self::start_subscope(Arc::clone(&self));
                    builder.register_identifier(&var.node, elem);
                    builder.register_identifier(&accumulator.node, Arc::clone(&output));
                    let subscope = builder.build_subscope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = Box::pin(
                                Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(f)),
                            )
                            .await?;
                            match *filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        output = Box::pin(
                            Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(body)),
                        )
                        .await?;
                    }
                }

                output
            }
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(collection)))
                        .await?;
                let list = match &*collection_value {
                    ExprValue::List(list) => list.clone(),
                    other => panic!("Expected collection for forall. Got: {:?}", other),
                };

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let mut output = match target {
                    a if a.is_bool() => ExprValue::Bool(true),
                    a if a.is_constraint() => ExprValue::Constraint(Vec::with_capacity(list.len())),
                    _ => panic!("Expected Bool or Constraint output"),
                };

                for elem in list {
                    let mut builder = Self::start_subscope(Arc::clone(&self));
                    builder.register_identifier(&var.node, elem);
                    let subscope = builder.build_subscope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = Box::pin(
                                Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(f)),
                            )
                            .await?;
                            match *filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        let new_value_arc = Box::pin(
                            Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(body)),
                        )
                        .await?;
                        output = match (output, &*new_value_arc) {
                            (ExprValue::Bool(v1), ExprValue::Bool(v2)) => {
                                ExprValue::Bool(v1 && *v2)
                            }
                            (ExprValue::Constraint(mut c1), ExprValue::Constraint(c2)) => {
                                c1.reserve(c2.len());
                                c1.extend(c2.iter().cloned());
                                ExprValue::Constraint(c1)
                            }
                            (value1, value2) => panic!(
                                "Unexpected types for forall operand: {:?}, {:?}",
                                value1, value2
                            ),
                        };
                    }
                }

                Arc::new(output)
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

                let mut evaluated_args: Vec<Arc<ExprValue<T, D::Connection>>> =
                    Vec::with_capacity(args.len());
                for x in args {
                    evaluated_args.push(
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x))).await?,
                    );
                }

                match resolve_path(
                    &path,
                    self.current_module(),
                    &eval_history.ast.global_env,
                    Some(&*self),
                ) {
                    Ok(ResolvedPathKind::VariableList {
                        module: var_module,
                        name: var_name,
                    }) => {
                        let var_lists = eval_history.ast.get_var_lists();
                        let var_list_fn = var_lists
                            .get(&(var_module.clone(), var_name.clone()))
                            .expect("Var list should be declared");

                        let (constraints, _origin) = Box::pin(eval_history.add_fn_to_call_history(
                            &var_list_fn.0,
                            &var_list_fn.1,
                            evaluated_args.clone(),
                            true,
                        ))
                        .await?;
                        eval_history.var_lists.insert(
                            (var_module.clone(), var_name.clone(), evaluated_args.clone()),
                            var_list_fn.clone(),
                        );

                        let constraint_count = match &*constraints {
                            ExprValue::List(list) => list.len(),
                            _ => panic!("Expected [Constraint]"),
                        };

                        Arc::new(ExprValue::List(
                            (0..constraint_count)
                                .map(|i| {
                                    Arc::new(ExprValue::LinExpr(LinExpr::var(IlpVar::Script(
                                        ScriptVar::new(
                                            var_module.clone(),
                                            var_name.clone(),
                                            Some(i),
                                            evaluated_args.clone(),
                                        ),
                                    ))))
                                })
                                .collect(),
                        ))
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
                let list = Box::pin(Arc::clone(&self).build_naked_list_for_list_comprehension(
                    eval_history,
                    body,
                    &vars_and_collections[..],
                    filter.as_ref(),
                ))
                .await?;

                Arc::new(ExprValue::List(list))
            }
            Expr::Let { var, value, body } => {
                let value_value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(value))).await?;

                let mut builder = Self::start_subscope(Arc::clone(&self));
                builder.register_identifier(&var.node, value_value);
                let subscope = builder.build_subscope();

                let body_value =
                    Box::pin(Arc::clone(&subscope).eval_expr(eval_history, Arc::clone(body))).await;

                body_value?
            }
            Expr::TupleLiteral { elements } => {
                let mut element_values = Vec::with_capacity(elements.len());
                for x in elements {
                    element_values.push(
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x))).await?,
                    );
                }

                Arc::new(ExprValue::Tuple(element_values))
            }

            Expr::StructLiteral { fields } => {
                let mut field_values = BTreeMap::new();
                for (name, expr) in fields {
                    field_values.insert(
                        name.node.clone(),
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(expr)))
                            .await?,
                    );
                }

                Arc::new(ExprValue::Struct(field_values))
            }
        })
    }

    /// Helper for evaluating type casts in GenericCall expressions
    async fn eval_generic_call_type_cast(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, T, D>,
        simple_type: &SimpleType,
        args: &Vec<Arc<Spanned<crate::ast::Expr>>>,
    ) -> Result<Arc<ExprValue<T, D::Connection>>, EvalError<T, D::Connection>> {
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
                let value =
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(&args[0])))
                        .await?;
                Ok(Arc::new(unsafe { value.convert_to_unchecked(simple_type) }))
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

                let content: Arc<ExprValue<T, D::Connection>> = if is_unit {
                    // Unit variant - args should be empty or just `none`
                    if args.is_empty() {
                        Arc::new(ExprValue::None)
                    } else {
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(&args[0])))
                            .await?
                    }
                } else if is_tuple {
                    // Tuple variant - evaluate all args
                    let mut values = Vec::with_capacity(args.len());
                    for x in args {
                        values.push(
                            Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x)))
                                .await?,
                        );
                    }
                    Arc::new(ExprValue::Tuple(values))
                } else {
                    // Single value variant
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(&args[0])))
                        .await?
                };

                Ok(Arc::new(ExprValue::Custom(CustomValue {
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

    async fn build_naked_list_for_list_comprehension(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, T, D>,
        body: &Arc<Spanned<crate::ast::Expr>>,
        vars_and_collections: &[(Spanned<String>, Arc<Spanned<crate::ast::Expr>>)],
        filter: Option<&Arc<Spanned<crate::ast::Expr>>>,
    ) -> Result<Vec<Arc<ExprValue<T, D::Connection>>>, EvalError<T, D::Connection>> {
        if vars_and_collections.is_empty() {
            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value =
                        Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(f))).await?;
                    match *filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            return Ok(if cond {
                Vec::from([
                    Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(body))).await?,
                ])
            } else {
                Vec::new()
            });
        }

        let (var, collection) = &vars_and_collections[0];
        let remaining_v_and_c = &vars_and_collections[1..];

        let collection_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(collection))).await?;
        let list = match &*collection_value {
            ExprValue::List(list) => list.clone(),
            other => panic!("Expected list. Got: {:?}", other),
        };

        let mut output = Vec::new();

        for elem in list {
            let mut builder = Self::start_subscope(Arc::clone(&self));
            builder.register_identifier(&var.node, elem);
            let subscope = builder.build_subscope();

            let extension = Box::pin(subscope.build_naked_list_for_list_comprehension(
                eval_history,
                body,
                remaining_v_and_c,
                filter,
            ))
            .await?;

            output.extend(extension);
        }

        Ok(output)
    }
}
