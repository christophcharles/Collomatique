//! Local environment for expression evaluation.
//!
//! This module defines:
//! - `LocalEvalEnv`: Manages local variable scopes during expression evaluation

use super::checked_ast::EvalError;
use super::history::EvalHistory;
use super::values::{CustomValue, ExprValue};
use super::variables::{ExternVar, IlpVar, ScriptVar};
use crate::Hashed;
use crate::ast::{Span, Spanned};
use crate::database::{DatabaseConnection, DatabaseDriver};
use crate::semantics::{LocalEnvCheck, ResolvedPathKind, SimpleType, resolve_path};
use collomatique_ilp::LinExpr;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct LocalEvalEnv<D: DatabaseDriver> {
    scope: HashMap<String, Arc<ExprValue<D::Connection>>>,
    parent: Option<Arc<LocalEvalEnv<D>>>,
    current_module: Arc<str>,
}

pub(crate) struct SubscopeBuilder<D: DatabaseDriver> {
    identifiers: HashMap<String, Arc<ExprValue<D::Connection>>>,
    parent: Arc<LocalEvalEnv<D>>,
}

impl<D: DatabaseDriver> SubscopeBuilder<D> {
    pub(crate) fn register_identifier(
        &mut self,
        ident: &str,
        value: Arc<ExprValue<D::Connection>>,
    ) {
        assert!(!self.identifiers.contains_key(ident));
        self.identifiers.insert(ident.to_string(), value);
    }

    pub(crate) fn build_subscope(self) -> Arc<LocalEvalEnv<D>> {
        let current_module = Arc::clone(&self.parent.current_module);
        Arc::new(LocalEvalEnv {
            scope: self.identifiers,
            parent: Some(self.parent),
            current_module,
        })
    }
}

impl<D: DatabaseDriver> LocalEnvCheck for LocalEvalEnv<D> {
    fn has_ident(&self, ident: &str) -> bool {
        self.lookup_ident(ident).is_some()
    }
}

impl<D: DatabaseDriver> LocalEvalEnv<D> {
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

    fn lookup_ident(&self, ident: &str) -> Option<Arc<ExprValue<D::Connection>>> {
        if let Some(value) = self.scope.get(ident) {
            return Some(Arc::clone(value));
        }
        self.parent.as_ref().and_then(|p| p.lookup_ident(ident))
    }

    pub(crate) fn start_subscope(parent: Arc<Self>) -> SubscopeBuilder<D> {
        SubscopeBuilder {
            identifiers: HashMap::new(),
            parent,
        }
    }

    pub(crate) async fn eval_expr(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        expr: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        use crate::ast::Expr;
        Ok(match &expr.node {
            Expr::None => Arc::new(ExprValue::None),
            Expr::Trivial => Arc::new(ExprValue::Constraint(vec![])),
            Expr::Boolean(val) => Arc::new(ExprValue::Bool(*val)),
            Expr::Number(val) => Arc::new(ExprValue::Int(*val)),
            Expr::StringLiteral(val) => Arc::new(ExprValue::String(val.clone())),
            Expr::IdentPath(path) => self.eval_ident_path(&eval_history.ast.global_env, path),
            Expr::Path { object, segments } => {
                Box::pin(Arc::clone(&self).eval_path(
                    eval_history,
                    Arc::clone(object),
                    segments,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Cardinality(list_expr) => {
                Box::pin(Arc::clone(&self).eval_cardinality(eval_history, Arc::clone(list_expr)))
                    .await?
            }
            Expr::ExplicitType {
                expr: inner,
                typ: _,
            } => {
                Box::pin(Arc::clone(&self).eval_explicit_type(
                    eval_history,
                    Arc::clone(inner),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::ComplexTypeCast { typ, args } => {
                Box::pin(Arc::clone(&self).eval_complex_type_cast(
                    eval_history,
                    typ,
                    args,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::StructCall { path, fields } => {
                Box::pin(Arc::clone(&self).eval_struct_call(
                    eval_history,
                    path,
                    fields,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::CastFallible { expr: inner, typ } => {
                Box::pin(Arc::clone(&self).eval_cast_fallible(
                    eval_history,
                    Arc::clone(inner),
                    typ,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::CastPanic { expr: inner, typ } => {
                Box::pin(Arc::clone(&self).eval_cast_panic(
                    eval_history,
                    Arc::clone(inner),
                    typ,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::ListLiteral { elements } => {
                Box::pin(Arc::clone(&self).eval_list_literal(
                    eval_history,
                    elements,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::ListRange { start, end } => {
                Box::pin(Arc::clone(&self).eval_list_range(
                    eval_history,
                    Arc::clone(start),
                    Arc::clone(end),
                ))
                .await?
            }
            Expr::GenericCall { path, args } => {
                Box::pin(Arc::clone(&self).eval_generic_call(
                    eval_history,
                    path,
                    args,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::VarCall { module, name, args } => {
                Box::pin(Arc::clone(&self).eval_var_call(eval_history, module.as_ref(), name, args))
                    .await?
            }
            Expr::In { item, collection } => {
                Box::pin(Arc::clone(&self).eval_in(
                    eval_history,
                    Arc::clone(item),
                    Arc::clone(collection),
                ))
                .await?
            }
            Expr::And(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_and(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Or(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_or(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Not(not_expr) => {
                Box::pin(Arc::clone(&self).eval_not(eval_history, Arc::clone(not_expr))).await?
            }
            Expr::NullCoalesce(lhs, rhs) => {
                Box::pin(Arc::clone(&self).eval_null_coalesce(
                    eval_history,
                    Arc::clone(lhs),
                    Arc::clone(rhs),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::ConstraintEq(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_constraint_eq(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::ConstraintLe(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_constraint_le(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::ConstraintGe(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_constraint_ge(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Eq(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_eq(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Ne(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_ne(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Lt(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_lt(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Le(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_le(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Gt(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_gt(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Ge(expr1, expr2) => {
                Box::pin(Arc::clone(&self).eval_ge(
                    eval_history,
                    Arc::clone(expr1),
                    Arc::clone(expr2),
                ))
                .await?
            }
            Expr::Add(left, right) => {
                Box::pin(Arc::clone(&self).eval_add(
                    eval_history,
                    Arc::clone(left),
                    Arc::clone(right),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Sub(left, right) => {
                Box::pin(Arc::clone(&self).eval_sub(
                    eval_history,
                    Arc::clone(left),
                    Arc::clone(right),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Neg(term) => {
                Box::pin(Arc::clone(&self).eval_neg(eval_history, Arc::clone(term))).await?
            }
            Expr::Panic(inner_expr) => {
                Box::pin(Arc::clone(&self).eval_panic(eval_history, Arc::clone(inner_expr))).await?
            }
            Expr::Mul(left, right) => {
                Box::pin(Arc::clone(&self).eval_mul(
                    eval_history,
                    Arc::clone(left),
                    Arc::clone(right),
                ))
                .await?
            }
            Expr::Div(left, right) => {
                Box::pin(Arc::clone(&self).eval_div(
                    eval_history,
                    Arc::clone(left),
                    Arc::clone(right),
                ))
                .await?
            }
            Expr::Mod(left, right) => {
                Box::pin(Arc::clone(&self).eval_mod(
                    eval_history,
                    Arc::clone(left),
                    Arc::clone(right),
                ))
                .await?
            }
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                Box::pin(Arc::clone(&self).eval_if(
                    eval_history,
                    Arc::clone(condition),
                    Arc::clone(then_expr),
                    Arc::clone(else_expr),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Match {
                match_expr,
                branches,
            } => {
                Box::pin(Arc::clone(&self).eval_match(
                    eval_history,
                    Arc::clone(match_expr),
                    branches,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                Box::pin(Arc::clone(&self).eval_sum(
                    eval_history,
                    var,
                    Arc::clone(collection),
                    filter.as_ref(),
                    Arc::clone(body),
                    &expr,
                    keep_track_of_origin,
                ))
                .await?
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
                Box::pin(Arc::clone(&self).eval_fold(
                    eval_history,
                    var,
                    Arc::clone(collection),
                    accumulator,
                    Arc::clone(init_value),
                    filter.as_ref(),
                    Arc::clone(body),
                    *reversed,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                Box::pin(Arc::clone(&self).eval_forall(
                    eval_history,
                    var,
                    Arc::clone(collection),
                    filter.as_ref(),
                    Arc::clone(body),
                    &expr,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::ListComprehension {
                body,
                vars_and_collections,
                filter,
            } => {
                Box::pin(Arc::clone(&self).eval_list_comprehension(
                    eval_history,
                    body,
                    vars_and_collections,
                    filter.as_ref(),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::Let { var, value, body } => {
                Box::pin(Arc::clone(&self).eval_let(
                    eval_history,
                    var,
                    Arc::clone(value),
                    Arc::clone(body),
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::TupleLiteral { elements } => {
                Box::pin(Arc::clone(&self).eval_tuple_literal(
                    eval_history,
                    elements,
                    keep_track_of_origin,
                ))
                .await?
            }
            Expr::StructLiteral { fields } => {
                Box::pin(Arc::clone(&self).eval_struct_literal(
                    eval_history,
                    fields,
                    keep_track_of_origin,
                ))
                .await?
            }
        })
    }

    fn eval_ident_path(
        &self,
        global_env: &crate::semantics::GlobalEnv<D>,
        path: &Spanned<crate::ast::NamespacePath>,
    ) -> Arc<ExprValue<D::Connection>> {
        let resolved = resolve_path(path, self.current_module(), global_env, Some(self))
            .expect("Path should be valid in a checked AST");

        match resolved {
            ResolvedPathKind::LocalVariable(name) => self
                .lookup_ident(&name)
                .expect("Local variable should exist"),
            ResolvedPathKind::Function { .. } => {
                panic!("Function reference without call should not appear in IdentPath")
            }
            ResolvedPathKind::Type(simple_type) => match simple_type {
                SimpleType::None => Arc::new(ExprValue::None),
                SimpleType::Custom(module, root, Some(variant)) => {
                    Arc::new(ExprValue::Custom(CustomValue {
                        module,
                        type_name: root,
                        variant: Some(variant),
                        content: Arc::new(ExprValue::None),
                    }))
                }
                _ => panic!("Unexpected type in IdentPath: {:?}", simple_type),
            },
            ResolvedPathKind::Query { .. } => {
                panic!("Query reference without call should not appear in IdentPath")
            }
            ResolvedPathKind::Module(_)
            | ResolvedPathKind::ExternalVariable(_)
            | ResolvedPathKind::InternalVariable { .. } => {
                panic!("Module/Variable should not appear in IdentPath after semantic check")
            }
        }
    }

    async fn eval_tuple_literal(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        elements: &[Arc<Spanned<crate::ast::Expr>>],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let mut element_values = Vec::with_capacity(elements.len());
        for x in elements {
            element_values.push(
                Box::pin(Arc::clone(&self).eval_expr(
                    eval_history,
                    Arc::clone(x),
                    keep_track_of_origin,
                ))
                .await?,
            );
        }
        Ok(Arc::new(ExprValue::Tuple(element_values)))
    }

    async fn eval_struct_literal(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        fields: &[(Spanned<String>, Arc<Spanned<crate::ast::Expr>>)],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let mut field_values = BTreeMap::new();
        for (name, expr) in fields {
            field_values.insert(
                name.node.clone(),
                Box::pin(Arc::clone(&self).eval_expr(
                    eval_history,
                    Arc::clone(expr),
                    keep_track_of_origin,
                ))
                .await?,
            );
        }
        Ok(Arc::new(ExprValue::Struct(field_values)))
    }

    async fn eval_list_comprehension(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        body: &Arc<Spanned<crate::ast::Expr>>,
        vars_and_collections: &[(Spanned<String>, Arc<Spanned<crate::ast::Expr>>)],
        filter: Option<&Arc<Spanned<crate::ast::Expr>>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let list = Box::pin(self.build_naked_list_for_list_comprehension(
            eval_history,
            body,
            vars_and_collections,
            filter,
            keep_track_of_origin,
        ))
        .await?;
        Ok(Arc::new(ExprValue::List(list)))
    }

    async fn eval_let(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        var: &Spanned<String>,
        value: Arc<Spanned<crate::ast::Expr>>,
        body: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, value, keep_track_of_origin))
                .await?;

        let mut builder = Self::start_subscope(Arc::clone(&self));
        builder.register_identifier(&var.node, value_value);
        let subscope = builder.build_subscope();

        Box::pin(subscope.eval_expr(eval_history, body, keep_track_of_origin)).await
    }

    async fn eval_sum(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        var: &Spanned<String>,
        collection: Arc<Spanned<crate::ast::Expr>>,
        filter: Option<&Arc<Spanned<crate::ast::Expr>>>,
        body: Arc<Spanned<crate::ast::Expr>>,
        expr: &Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let target = eval_history
            .ast
            .expr_types
            .get(&expr.span)
            .expect("Semantic analysis should have given a target type");

        let keep_track_of_origin = match target {
            a if a.is_lin_expr() => false,
            a if a.is_int() => false,
            a if a.is_list() => keep_track_of_origin,
            a if a.is_string() => false,
            _ => panic!("Expected Int, LinExpr, String or List output"),
        };

        let collection_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, collection, keep_track_of_origin))
                .await?;
        let list = match &*collection_value {
            ExprValue::List(list) => list,
            other => panic!("Expected collection for sum. Got: {:?}", other),
        };

        let mut output = match target {
            a if a.is_lin_expr() => ExprValue::LinExpr(LinExpr::constant(0.)),
            a if a.is_int() => ExprValue::Int(0),
            a if a.is_list() => ExprValue::List(Vec::with_capacity(list.len())),
            a if a.is_string() => ExprValue::String(String::new()),
            _ => panic!("Expected Int, LinExpr, String or List output"),
        };

        for elem in list {
            let mut builder = Self::start_subscope(Arc::clone(&self));
            builder.register_identifier(&var.node, Arc::clone(elem));
            let subscope = builder.build_subscope();

            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = Box::pin(Arc::clone(&subscope).eval_expr(
                        eval_history,
                        Arc::clone(f),
                        keep_track_of_origin,
                    ))
                    .await?;
                    match *filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            if cond {
                let new_value_arc = Box::pin(Arc::clone(&subscope).eval_expr(
                    eval_history,
                    Arc::clone(&body),
                    keep_track_of_origin,
                ))
                .await?;
                let new_value = Arc::unwrap_or_clone(new_value_arc);
                output = match (output, new_value) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value)) => {
                        let mut result = lin_expr_value;
                        result += LinExpr::constant(int_value as f64);
                        ExprValue::LinExpr(result)
                    }
                    (ExprValue::LinExpr(mut lin_expr_value), ExprValue::Int(int_value)) => {
                        lin_expr_value += LinExpr::constant(int_value as f64);
                        ExprValue::LinExpr(lin_expr_value)
                    }
                    (ExprValue::LinExpr(mut v1), ExprValue::LinExpr(ref v2)) => {
                        v1 += v2;
                        ExprValue::LinExpr(v1)
                    }
                    (ExprValue::String(mut s1), ExprValue::String(ref s2)) => {
                        s1.push_str(s2);
                        ExprValue::String(s1)
                    }
                    (ExprValue::List(mut list), ExprValue::List(new_list)) => {
                        list.extend(new_list);
                        ExprValue::List(list)
                    }
                    (value1, value2) => panic!(
                        "Unexpected types for sum operand: {:?}, {:?}",
                        value1, value2
                    ),
                };
            }
        }

        Ok(Arc::new(output))
    }

    async fn eval_fold(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        var: &Spanned<String>,
        collection: Arc<Spanned<crate::ast::Expr>>,
        accumulator: &Spanned<String>,
        init_value: Arc<Spanned<crate::ast::Expr>>,
        filter: Option<&Arc<Spanned<crate::ast::Expr>>>,
        body: Arc<Spanned<crate::ast::Expr>>,
        reversed: bool,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let collection_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, collection, keep_track_of_origin))
                .await?;
        let list = match &*collection_value {
            ExprValue::List(list) => list,
            other => panic!("Expected collection for fold. Got: {:?}", other),
        };

        let mut output =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, init_value, keep_track_of_origin))
                .await?;

        let len = list.len();
        for i in 0..len {
            let elem = &list[if reversed { len - 1 - i } else { i }];
            let mut builder = Self::start_subscope(Arc::clone(&self));
            builder.register_identifier(&var.node, Arc::clone(elem));
            builder.register_identifier(&accumulator.node, Arc::clone(&output));
            let subscope = builder.build_subscope();

            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = Box::pin(Arc::clone(&subscope).eval_expr(
                        eval_history,
                        Arc::clone(f),
                        keep_track_of_origin,
                    ))
                    .await?;
                    match *filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            if cond {
                output = Box::pin(Arc::clone(&subscope).eval_expr(
                    eval_history,
                    Arc::clone(&body),
                    keep_track_of_origin,
                ))
                .await?;
            }
        }

        Ok(output)
    }

    async fn eval_forall(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        var: &Spanned<String>,
        collection: Arc<Spanned<crate::ast::Expr>>,
        filter: Option<&Arc<Spanned<crate::ast::Expr>>>,
        body: Arc<Spanned<crate::ast::Expr>>,
        expr: &Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let collection_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, collection, keep_track_of_origin))
                .await?;
        let list = match &*collection_value {
            ExprValue::List(list) => list,
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
            builder.register_identifier(&var.node, Arc::clone(elem));
            let subscope = builder.build_subscope();

            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = Box::pin(Arc::clone(&subscope).eval_expr(
                        eval_history,
                        Arc::clone(f),
                        keep_track_of_origin,
                    ))
                    .await?;
                    match *filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            if cond {
                let new_value_arc = Box::pin(Arc::clone(&subscope).eval_expr(
                    eval_history,
                    Arc::clone(&body),
                    keep_track_of_origin,
                ))
                .await?;
                let new_value = Arc::unwrap_or_clone(new_value_arc);
                output = match (output, new_value) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 && v2),
                    (ExprValue::Constraint(mut c1), ExprValue::Constraint(c2)) => {
                        c1.extend(c2);
                        ExprValue::Constraint(c1)
                    }
                    (value1, value2) => panic!(
                        "Unexpected types for forall operand: {:?}, {:?}",
                        value1, value2
                    ),
                };
            }
        }

        Ok(Arc::new(output))
    }

    async fn eval_if(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        condition: Arc<Spanned<crate::ast::Expr>>,
        then_expr: Arc<Spanned<crate::ast::Expr>>,
        else_expr: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let cond_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, condition, keep_track_of_origin))
                .await?;
        let ExprValue::Bool(cond) = *cond_value else {
            panic!("Expected Bool for if condition");
        };

        if cond {
            Box::pin(self.eval_expr(eval_history, then_expr, keep_track_of_origin)).await
        } else {
            Box::pin(self.eval_expr(eval_history, else_expr, keep_track_of_origin)).await
        }
    }

    async fn eval_match(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        match_expr: Arc<Spanned<crate::ast::Expr>>,
        branches: &[crate::ast::MatchBranch],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, match_expr, keep_track_of_origin))
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

            let mut builder = Self::start_subscope(Arc::clone(&self));
            builder.register_identifier(&branch.ident.node, Arc::clone(&value));
            let subscope = builder.build_subscope();

            let where_clause_passes = match &branch.filter {
                None => true,
                Some(filter_expr) => {
                    let cond_value = Box::pin(Arc::clone(&subscope).eval_expr(
                        eval_history,
                        Arc::clone(filter_expr),
                        keep_track_of_origin,
                    ))
                    .await?;
                    let ExprValue::Bool(cond) = *cond_value else {
                        panic!("Expected Bool for where clause");
                    };
                    cond
                }
            };

            if !where_clause_passes {
                continue;
            }

            return Box::pin(Arc::clone(&subscope).eval_expr(
                eval_history,
                Arc::clone(&branch.body),
                keep_track_of_origin,
            ))
            .await;
        }

        panic!("Match should be exhaustive during evaluation");
    }

    async fn eval_panic(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        inner: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value = Box::pin(self.eval_expr(eval_history, inner, false)).await?;
        Err(EvalError::Panic(Box::new((*value).clone())))
    }

    async fn eval_mul(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
            | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                ExprValue::LinExpr(*int_value * lin_expr_value.clone())
            }
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 * v2),
            (value1, value2) => {
                panic!("Unexpected types for * operand: {:?}, {:?}", value1, value2)
            }
        }))
    }

    async fn eval_div(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 / v2),
            (value1, value2) => panic!(
                "Unexpected types for // operand: {:?}, {:?}",
                value1, value2
            ),
        }))
    }

    async fn eval_mod(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 % v2),
            (value1, value2) => {
                panic!("Unexpected types for % operand: {:?}, {:?}", value1, value2)
            }
        }))
    }

    async fn eval_add(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, left, keep_track_of_origin)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, keep_track_of_origin)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
            (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
            | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                let new_lin_expr = LinExpr::constant(*int_value as f64);
                ExprValue::LinExpr(lin_expr_value.clone() + new_lin_expr)
            }
            (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => {
                ExprValue::LinExpr(v1.clone() + v2.clone())
            }
            (ExprValue::String(s1), ExprValue::String(s2)) => ExprValue::String(s1.clone() + s2),
            (ExprValue::List(list1), ExprValue::List(list2)) => {
                let mut result = list1.clone();
                result.reserve(list2.len());
                result.extend(list2.iter().cloned());
                ExprValue::List(result)
            }
            (value1, value2) => {
                panic!("Unexpected types for + operand: {:?}, {:?}", value1, value2)
            }
        }))
    }

    async fn eval_sub(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, left, keep_track_of_origin)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, keep_track_of_origin)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
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
        }))
    }

    async fn eval_neg(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        inner: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value = Box::pin(self.eval_expr(eval_history, inner, false)).await?;
        Ok(Arc::new(match &*value {
            ExprValue::Int(v) => ExprValue::Int(-v),
            ExprValue::LinExpr(v) => ExprValue::LinExpr(-v.clone()),
            value => panic!("Unexpected type for (-) operand: {:?}", value),
        }))
    }

    async fn eval_eq(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(ExprValue::Bool(*value1 == *value2)))
    }

    async fn eval_ne(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(ExprValue::Bool(*value1 != *value2)))
    }

    async fn eval_lt(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 < v2),
            (value1, value2) => {
                panic!("Unexpected types for < operand: {:?}, {:?}", value1, value2)
            }
        }))
    }

    async fn eval_le(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 <= v2),
            (value1, value2) => panic!(
                "Unexpected types for <= operand: {:?}, {:?}",
                value1, value2
            ),
        }))
    }

    async fn eval_gt(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 > v2),
            (value1, value2) => {
                panic!("Unexpected types for > operand: {:?}, {:?}", value1, value2)
            }
        }))
    }

    async fn eval_ge(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 >= v2),
            (value1, value2) => panic!(
                "Unexpected types for >= operand: {:?}, {:?}",
                value1, value2
            ),
        }))
    }

    async fn eval_constraint_eq(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        // Ironically, we don't track the origin because it is going to be overwritten
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;

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

        Ok(Arc::new(ExprValue::Constraint(Vec::from([lin_expr1
            .eq(&lin_expr2)
            .into()]))))
    }

    async fn eval_constraint_le(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        // Ironically, we don't track the origin because it is going to be overwritten
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;

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

        Ok(Arc::new(ExprValue::Constraint(Vec::from([lin_expr1
            .leq(&lin_expr2)
            .into()]))))
    }

    async fn eval_constraint_ge(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        // Ironically, we don't track the origin because it is going to be overwritten
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;

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

        Ok(Arc::new(ExprValue::Constraint(Vec::from([lin_expr1
            .geq(&lin_expr2)
            .into()]))))
    }

    async fn eval_in(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        item: Arc<Spanned<crate::ast::Expr>>,
        collection: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let collection_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, collection, false)).await?;
        let list = match &*collection_value {
            ExprValue::List(list) => list,
            _ => panic!("List expected"),
        };

        let item_value = Box::pin(self.eval_expr(eval_history, item, false)).await?;
        for elt in list {
            if *item_value == **elt {
                return Ok(Arc::new(ExprValue::Bool(true)));
            }
        }
        Ok(Arc::new(ExprValue::Bool(false)))
    }

    async fn eval_and(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, left, keep_track_of_origin)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, keep_track_of_origin)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
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
        }))
    }

    async fn eval_or(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        left: Arc<Spanned<crate::ast::Expr>>,
        right: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value1 = Box::pin(Arc::clone(&self).eval_expr(eval_history, left, false)).await?;
        let value2 = Box::pin(self.eval_expr(eval_history, right, false)).await?;
        Ok(Arc::new(match (&*value1, &*value2) {
            (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(*v1 || *v2),
            (value1, value2) => panic!(
                "Unexpected types for OR operand: {:?}, {:?}",
                value1, value2
            ),
        }))
    }

    async fn eval_not(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        inner: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value = Box::pin(self.eval_expr(eval_history, inner, false)).await?;
        Ok(Arc::new(match &*value {
            ExprValue::Bool(v) => ExprValue::Bool(!v),
            value => panic!("Unexpected type for NOT operand: {:?}", value),
        }))
    }

    async fn eval_null_coalesce(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        lhs: Arc<Spanned<crate::ast::Expr>>,
        rhs: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let lhs_value =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, lhs, keep_track_of_origin)).await?;
        if *lhs_value == ExprValue::None {
            Box::pin(self.eval_expr(eval_history, rhs, keep_track_of_origin)).await
        } else {
            Ok(lhs_value)
        }
    }

    async fn eval_var_call(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        module: Option<&Spanned<String>>,
        name: &Spanned<String>,
        args: &[Arc<Spanned<crate::ast::Expr>>],
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
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

        let mut eval_args: Vec<Arc<ExprValue<D::Connection>>> = Vec::with_capacity(args.len());
        for x in args {
            eval_args.push(
                Box::pin(Arc::clone(&self).eval_expr(eval_history, Arc::clone(x), false)).await?,
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
                Ok(Arc::new(ExprValue::LinExpr(LinExpr::var(IlpVar::Base(
                    ExternVar::new(ext_var_name, args),
                )))))
            }
            Ok(ResolvedPathKind::InternalVariable {
                module: var_module,
                name: var_name,
            }) => {
                eval_history.vars.insert(Hashed::new((
                    var_module.clone(),
                    var_name.clone(),
                    args.clone(),
                )));
                Ok(Arc::new(ExprValue::LinExpr(LinExpr::var(IlpVar::Script(
                    ScriptVar::new(var_module, var_name, args),
                )))))
            }
            _ => panic!("Valid var expected (should have been caught by type checker)"),
        }
    }

    async fn eval_generic_call(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        path: &Spanned<crate::ast::NamespacePath>,
        args: &[Arc<Spanned<crate::ast::Expr>>],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
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
                let mut eval_args = Vec::with_capacity(args.len());
                for x in args {
                    eval_args.push(
                        Box::pin(Arc::clone(&self).eval_expr(
                            eval_history,
                            Arc::clone(x),
                            keep_track_of_origin,
                        ))
                        .await?,
                    );
                }
                Ok(Box::pin(eval_history.add_fn_to_call_history(
                    &module,
                    &func,
                    eval_args,
                    true,
                    keep_track_of_origin,
                ))
                .await?)
            }
            ResolvedPathKind::Type(simple_type) => {
                Box::pin(Arc::clone(&self).eval_generic_call_type_cast(
                    eval_history,
                    &simple_type,
                    args,
                    keep_track_of_origin,
                ))
                .await
            }
            ResolvedPathKind::Query { module, name } => {
                let mut eval_args = Vec::with_capacity(args.len());
                for x in args {
                    eval_args.push(
                        Box::pin(Arc::clone(&self).eval_expr(
                            eval_history,
                            Arc::clone(x),
                            false, // Queries can't return constraints
                        ))
                        .await?,
                    );
                }
                Box::pin(eval_history.add_query_to_call_history(&module, &name, eval_args)).await
            }
            ResolvedPathKind::Module(_)
            | ResolvedPathKind::ExternalVariable(_)
            | ResolvedPathKind::InternalVariable { .. } => {
                panic!("Module/Variable should not appear in GenericCall after semantic check")
            }
        }
    }

    async fn eval_list_literal(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        elements: &[Arc<Spanned<crate::ast::Expr>>],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let mut element_values = Vec::with_capacity(elements.len());
        for x in elements {
            element_values.push(
                Box::pin(Arc::clone(&self).eval_expr(
                    eval_history,
                    Arc::clone(x),
                    keep_track_of_origin,
                ))
                .await?,
            );
        }
        Ok(Arc::new(ExprValue::List(element_values)))
    }

    async fn eval_list_range(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        start: Arc<Spanned<crate::ast::Expr>>,
        end: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let start_value = Box::pin(Arc::clone(&self).eval_expr(eval_history, start, false)).await?;
        let end_value = Box::pin(self.eval_expr(eval_history, end, false)).await?;

        let start_num = match &*start_value {
            ExprValue::Int(v) => *v,
            _ => panic!("Int expected"),
        };
        let end_num = match &*end_value {
            ExprValue::Int(v) => *v,
            _ => panic!("Int expected"),
        };

        Ok(Arc::new(ExprValue::List(
            (start_num..end_num)
                .map(|i| Arc::new(ExprValue::Int(i)))
                .collect(),
        )))
    }

    async fn eval_cast_fallible(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        inner: Arc<Spanned<crate::ast::Expr>>,
        typ: &Spanned<crate::ast::TypeName>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value = Box::pin(self.eval_expr(eval_history, inner, keep_track_of_origin)).await?;
        let target_type = eval_history.ast.get_resolved_type(&typ.span);
        if value.fits_in_typ(target_type) {
            Ok(value)
        } else {
            Ok(Arc::new(ExprValue::None))
        }
    }

    async fn eval_cast_panic(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        inner: Arc<Spanned<crate::ast::Expr>>,
        typ: &Spanned<crate::ast::TypeName>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let value = Box::pin(self.eval_expr(eval_history, inner, keep_track_of_origin)).await?;
        let target_type = eval_history.ast.get_resolved_type(&typ.span);
        if value.fits_in_typ(target_type) {
            Ok(value)
        } else {
            Err(EvalError::Panic(Box::new(ExprValue::String(format!(
                "cast! failed: value {} does not fit in type {}",
                value, target_type
            )))))
        }
    }

    async fn eval_struct_call(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        path: &Spanned<crate::ast::NamespacePath>,
        fields: &[(Spanned<String>, Arc<Spanned<crate::ast::Expr>>)],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
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

        let mut field_values = std::collections::BTreeMap::new();
        for (name, expr) in fields {
            let value = Box::pin(Arc::clone(&self).eval_expr(
                eval_history,
                Arc::clone(expr),
                keep_track_of_origin,
            ))
            .await?;
            field_values.insert(name.node.clone(), value);
        }

        Ok(Arc::new(ExprValue::Custom(CustomValue {
            module,
            type_name,
            variant: variant_name,
            content: Arc::new(ExprValue::Struct(field_values)),
        })))
    }

    async fn eval_complex_type_cast(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        typ: &Spanned<crate::ast::TypeName>,
        args: &[Arc<Spanned<crate::ast::Expr>>],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        if args.len() != 1 {
            panic!("ComplexTypeCast expects exactly one argument");
        }
        let value =
            Box::pin(self.eval_expr(eval_history, Arc::clone(&args[0]), keep_track_of_origin))
                .await?;

        let orig_type = eval_history.ast.get_resolved_type(&typ.span);
        let target_type = orig_type
            .as_simple()
            .expect("ComplexTypeCast should have a simple type as target");

        Ok(Arc::new(unsafe { value.convert_to_unchecked(target_type) }))
    }

    async fn eval_explicit_type(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        inner: Arc<Spanned<crate::ast::Expr>>,
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        Box::pin(self.eval_expr(eval_history, inner, keep_track_of_origin)).await
    }

    async fn eval_cardinality(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        list_expr: Arc<Spanned<crate::ast::Expr>>,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        let list_value = Box::pin(self.eval_expr(eval_history, list_expr, false)).await?;
        let count = match &*list_value {
            ExprValue::List(list) => list.len(),
            _ => panic!("Unexpected type for list expression"),
        };
        Ok(Arc::new(ExprValue::Int(
            i32::try_from(count).expect("List length should not exceed i32 capacity"),
        )))
    }

    async fn eval_path(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        object: Arc<Spanned<crate::ast::Expr>>,
        segments: &[Spanned<crate::ast::PathSegment>],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
        use crate::ast::PathSegment;
        assert!(!segments.is_empty());

        let mut current_value: Arc<ExprValue<D::Connection>> =
            Box::pin(Arc::clone(&self).eval_expr(eval_history, object, keep_track_of_origin))
                .await?;

        // Helper to unwrap Custom values for field/index access
        fn unwrap_custom<D: DatabaseConnection>(value: Arc<ExprValue<D>>) -> Arc<ExprValue<D>> {
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
                    let index_arc = Box::pin(Arc::clone(&self).eval_expr(
                        eval_history,
                        Arc::clone(index_expr),
                        keep_track_of_origin,
                    ))
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
                    let index_arc = Box::pin(Arc::clone(&self).eval_expr(
                        eval_history,
                        Arc::clone(index_expr),
                        keep_track_of_origin,
                    ))
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
                        return Err(EvalError::Panic(Box::new(ExprValue::String(format!(
                            "list index out of bounds: index {} but list has {} elements",
                            i,
                            elements.len()
                        )))));
                    }
                    current_value = Arc::clone(&elements[i as usize]);
                }
            }
        }

        Ok(current_value)
    }

    /// Helper for evaluating type casts in GenericCall expressions
    async fn eval_generic_call_type_cast(
        self: Arc<Self>,
        eval_history: &mut EvalHistory<'_, D>,
        simple_type: &SimpleType,
        args: &[Arc<Spanned<crate::ast::Expr>>],
        keep_track_of_origin: bool,
    ) -> Result<Arc<ExprValue<D::Connection>>, EvalError<D::Connection>> {
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
                let value = Box::pin(Arc::clone(&self).eval_expr(
                    eval_history,
                    Arc::clone(&args[0]),
                    keep_track_of_origin,
                ))
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

                let content: Arc<ExprValue<D::Connection>> = if is_unit {
                    // Unit variant - args should be empty or just `none`
                    if args.is_empty() {
                        Arc::new(ExprValue::None)
                    } else {
                        Box::pin(Arc::clone(&self).eval_expr(
                            eval_history,
                            Arc::clone(&args[0]),
                            keep_track_of_origin,
                        ))
                        .await?
                    }
                } else if is_tuple {
                    // Tuple variant - evaluate all args
                    let mut values = Vec::with_capacity(args.len());
                    for x in args {
                        values.push(
                            Box::pin(Arc::clone(&self).eval_expr(
                                eval_history,
                                Arc::clone(x),
                                keep_track_of_origin,
                            ))
                            .await?,
                        );
                    }
                    Arc::new(ExprValue::Tuple(values))
                } else {
                    // Single value variant
                    Box::pin(Arc::clone(&self).eval_expr(
                        eval_history,
                        Arc::clone(&args[0]),
                        keep_track_of_origin,
                    ))
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
        eval_history: &mut EvalHistory<'_, D>,
        body: &Arc<Spanned<crate::ast::Expr>>,
        vars_and_collections: &[(Spanned<String>, Arc<Spanned<crate::ast::Expr>>)],
        filter: Option<&Arc<Spanned<crate::ast::Expr>>>,
        keep_track_of_origin: bool,
    ) -> Result<Vec<Arc<ExprValue<D::Connection>>>, EvalError<D::Connection>> {
        if vars_and_collections.is_empty() {
            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = Box::pin(Arc::clone(&self).eval_expr(
                        eval_history,
                        Arc::clone(f),
                        keep_track_of_origin,
                    ))
                    .await?;
                    match *filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            return Ok(if cond {
                Vec::from([Box::pin(Arc::clone(&self).eval_expr(
                    eval_history,
                    Arc::clone(body),
                    keep_track_of_origin,
                ))
                .await?])
            } else {
                Vec::new()
            });
        }

        let (var, collection) = &vars_and_collections[0];
        let remaining_v_and_c = &vars_and_collections[1..];

        let collection_value = Box::pin(Arc::clone(&self).eval_expr(
            eval_history,
            Arc::clone(collection),
            keep_track_of_origin,
        ))
        .await?;
        let list = match &*collection_value {
            ExprValue::List(list) => list,
            other => panic!("Expected list. Got: {:?}", other),
        };

        let mut output = Vec::new();

        for elem in list {
            let mut builder = Self::start_subscope(Arc::clone(&self));
            builder.register_identifier(&var.node, Arc::clone(elem));
            let subscope = builder.build_subscope();

            let extension = Box::pin(subscope.build_naked_list_for_list_comprehension(
                eval_history,
                body,
                remaining_v_and_c,
                filter,
                keep_track_of_origin,
            ))
            .await?;

            output.extend(extension);
        }

        Ok(output)
    }
}
