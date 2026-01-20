//! Expression type checking for the ColloML DSL.
//!
//! This module implements the core type-checking logic for expressions.

use super::errors::{SemError, SemWarning};
use super::global_env::{GlobalEnv, TypeInfo};
use super::local_env::LocalCheckEnv;
use super::path_resolution::{resolve_path, ResolvedPathKind};
use super::string_case;
use super::types::{ConcreteType, ExprType, SimpleType};
use crate::ast::{Expr, Span, Spanned};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

impl LocalCheckEnv {
    pub(crate) fn check_expr(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &Expr,
        span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        resolved_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        let result = self.check_expr_internal(
            global_env,
            expr,
            span,
            type_info,
            expr_types,
            resolved_types,
            errors,
            warnings,
        );
        if let Some(typ) = &result {
            expr_types.insert(span.clone(), typ.clone());
        }
        result
    }

    /// Check if a type can be converted considering custom type wrapping/unwrapping
    fn can_convert_with_custom_types(
        &self,
        global_env: &GlobalEnv,
        from: &ExprType,
        to: &ConcreteType,
    ) -> bool {
        let to_simple = to.inner();

        // Check if target is a custom type and source can convert to underlying
        if let SimpleType::Custom(module, root, variant) = to_simple {
            let key = match variant {
                None => root.clone(),
                Some(v) => format!("{}::{}", root, v),
            };
            if let Some(underlying) = global_env.get_custom_type_underlying(module, &key) {
                // Can convert if source can convert to the underlying type
                if underlying.is_concrete() {
                    let underlying_concrete = underlying
                        .clone()
                        .to_simple()
                        .unwrap()
                        .into_concrete()
                        .unwrap();
                    if from.can_convert_to(&underlying_concrete) {
                        return true;
                    }
                }
            }
        }

        // Check if source is a custom type and underlying can convert to target
        if from.is_concrete() {
            if let Some(from_simple) = from.clone().to_simple() {
                if let SimpleType::Custom(module, root, variant) = from_simple {
                    let key = match variant {
                        None => root.clone(),
                        Some(v) => format!("{}::{}", root, v),
                    };
                    if let Some(underlying) = global_env.get_custom_type_underlying(&module, &key) {
                        // Can convert if the underlying type can convert to target
                        if underlying.can_convert_to(to) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Resolve all variants of an ExprType, unwrapping any Custom types to their leaf types.
    /// For Custom types, recursively resolves through the underlying ExprType (which may be a union).
    /// Returns all the leaf SimpleTypes after resolving custom types.
    fn resolve_type_for_access(&self, global_env: &GlobalEnv, typ: &ExprType) -> Vec<SimpleType> {
        let mut visited = HashSet::new();
        self.resolve_type_for_access_impl(global_env, typ, &mut visited)
    }

    fn resolve_type_for_access_impl(
        &self,
        global_env: &GlobalEnv,
        typ: &ExprType,
        visited: &mut HashSet<String>,
    ) -> Vec<SimpleType> {
        let mut result = Vec::new();
        for variant in typ.get_variants() {
            result.extend(self.resolve_simple_type_for_access_impl(global_env, variant, visited));
        }
        result
    }

    /// Resolve a single SimpleType, unwrapping Custom types recursively.
    fn resolve_simple_type_for_access_impl(
        &self,
        global_env: &GlobalEnv,
        typ: &SimpleType,
        visited: &mut HashSet<String>,
    ) -> Vec<SimpleType> {
        match typ {
            SimpleType::Custom(module, root, variant) => {
                let key = match variant {
                    None => root.clone(),
                    Some(v) => format!("{}::{}", root, v),
                };
                if visited.contains(&key) {
                    // If has_unguarded_reference is correct, we should NEVER hit this.
                    // Panic to catch bugs in validation.
                    panic!(
                        "Cycle detected in type resolution for '{}'. \
                         This indicates a bug in has_unguarded_reference validation.",
                        key
                    );
                }
                visited.insert(key.clone());

                if let Some(underlying) = global_env.get_custom_type_underlying(module, &key) {
                    // Recursively resolve the underlying ExprType
                    self.resolve_type_for_access_impl(global_env, underlying, visited)
                } else {
                    // Unknown custom type - return empty (error will be caught elsewhere)
                    vec![]
                }
            }
            other => vec![other.clone()],
        }
    }

    fn check_expr_internal(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &Expr,
        global_span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        resolved_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        match expr {
            // ========== Literals ==========
            Expr::None => Some(SimpleType::None.into()),
            Expr::Number(_) => Some(SimpleType::Int.into()),
            Expr::Boolean(_) => Some(SimpleType::Bool.into()),
            Expr::StringLiteral(_) => Some(SimpleType::String.into()),

            Expr::IdentPath(path) => self
                .check_ident_path(global_env, path, type_info, errors, warnings)
                .map(|x| x.into()),
            Expr::Path { object, segments } => self
                .check_path(
                    global_env,
                    object,
                    segments,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                )
                .map(|x| x.into()),

            // ========== As construct ==========
            Expr::ExplicitType { expr, typ } => {
                // Check the inner expression
                let expr_type = self.check_expr(
                    global_env,
                    &expr.node,
                    &expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Convert the declared type
                let target_type = match global_env.resolve_type(typ, self.current_module()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return expr_type; // Fallback to inferred type
                    }
                };

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        module: self.current_module().to_string(),
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

                // Cache the resolved type for use during evaluation
                resolved_types.insert(typ.span.clone(), target_type.clone());

                if let Some(inferred) = expr_type {
                    // Check if the inferred type can convert to the target type
                    if !inferred.is_subtype_of(&target_type) {
                        // Error: can't convert
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: inferred,
                            found: target_type.clone(),
                            context: "Type coercion can only be done to super-types".into(),
                        });
                    }
                }
                Some(target_type) // Propagate target in all cases
            }

            // ========== Cast constructs ==========
            // cast? narrows a type, returns None if the value doesn't fit
            Expr::CastFallible { expr, typ } => {
                // Check the inner expression
                let expr_type = self.check_expr(
                    global_env,
                    &expr.node,
                    &expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Convert the declared type
                let target_type = match global_env.resolve_type(typ, self.current_module()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return expr_type; // Fallback to inferred type
                    }
                };

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        module: self.current_module().to_string(),
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

                // Cache the resolved type for use during evaluation
                resolved_types.insert(typ.span.clone(), target_type.clone());

                if let Some(inferred) = expr_type {
                    // For cast?, target must be subtype of expr type (narrowing)
                    if !target_type.is_subtype_of(&inferred) {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: target_type.clone(),
                            found: inferred,
                            context: "cast? can only narrow types (target must be subtype of expression type)".into(),
                        });
                    }
                }
                // Return type is always ?TargetType
                Some(target_type.make_optional())
            }

            // cast! narrows a type, panics if the value doesn't fit
            Expr::CastPanic { expr, typ } => {
                // Check the inner expression
                let expr_type = self.check_expr(
                    global_env,
                    &expr.node,
                    &expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Convert the declared type
                let target_type = match global_env.resolve_type(typ, self.current_module()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return expr_type; // Fallback to inferred type
                    }
                };

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        module: self.current_module().to_string(),
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

                // Cache the resolved type for use during evaluation
                resolved_types.insert(typ.span.clone(), target_type.clone());

                if let Some(inferred) = expr_type {
                    // For cast!, target must be subtype of expr type (narrowing)
                    if !target_type.is_subtype_of(&inferred) {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: target_type.clone(),
                            found: inferred,
                            context: "cast! can only narrow types (target must be subtype of expression type)".into(),
                        });
                    }
                }
                // Return type is the target type (panics on failure)
                Some(target_type)
            }

            // ========== Complex Type Cast: [Type](expr) or (Type, Type)(expr) ==========
            Expr::ComplexTypeCast { typ, args } => {
                // Check all args
                let mut arg_types = Vec::new();
                for arg in args {
                    let arg_type = self.check_expr(
                        global_env,
                        &arg.node,
                        &arg.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );
                    arg_types.push((arg, arg_type));
                }

                // Resolve the target type
                let target_type = match global_env.resolve_type(typ, self.current_module()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return None;
                    }
                };

                // Validate that the target type is concrete
                if !target_type.is_concrete() {
                    errors.push(SemError::NonConcreteType {
                        span: typ.span.clone(),
                        found: target_type,
                        context: "Type cast requires a concrete target type".to_string(),
                    });
                    return None;
                }

                // Cache the resolved type for use during evaluation
                resolved_types.insert(typ.span.clone(), target_type.clone());

                let concrete_target = target_type.to_simple().unwrap().into_concrete().unwrap();

                // For type conversion, we expect exactly one argument
                if args.len() != 1 {
                    errors.push(SemError::ArgumentCountMismatch {
                        identifier: format!("{}", concrete_target),
                        span: typ.span.clone(),
                        expected: 1,
                        found: args.len(),
                    });
                }

                // Check if the arg can convert to target
                if let Some((arg, Some(inferred))) = arg_types.first() {
                    let can_convert = inferred.can_convert_to(&concrete_target)
                        || self.can_convert_with_custom_types(
                            global_env,
                            inferred,
                            &concrete_target,
                        );

                    if !can_convert {
                        errors.push(SemError::ImpossibleConversion {
                            span: arg.span.clone(),
                            found: inferred.clone(),
                            target: concrete_target.clone(),
                        });
                    }
                }

                Some(concrete_target.into_inner().into())
            }

            // ========== Arithmetic Operations ==========
            // Int + Int -> Int
            // LinExpr + Int -> LinExpr (auto convert Int to LinExpr)
            // Int + LinExpr -> LinExpr (auto convert Int to LinExpr)
            // LinExpr + LinExpr -> LinExpr
            // [Type] + [Type] -> [Type]
            // String + String -> String
            Expr::Add(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (None, None) => None,
                    (Some(t), None) |
                    (None, Some(t)) => {
                        if t.is_int() || t.is_lin_expr() || t.is_list() || t.is_string() {
                            Some(t.clone())
                        } else if t.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap()) {
                            Some(SimpleType::LinExpr.into())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context:
                                    "addition/concatenation requires Int, LinExpr, String or List"
                                        .to_string(),
                            });
                            None
                        }
                    }
                    (Some(l), Some(r)) => {
                        l.cross_check(
                            r,
                            errors,
                            |v1,v2| match (v1,v2) {
                                (SimpleType::Int, SimpleType::Int) => Ok(SimpleType::Int),
                                (SimpleType::LinExpr, SimpleType::Int) |
                                (SimpleType::Int, SimpleType::LinExpr) |
                                (SimpleType::LinExpr, SimpleType::LinExpr) => Ok(SimpleType::LinExpr),
                                (SimpleType::String, SimpleType::String) => Ok(SimpleType::String),
                                (SimpleType::EmptyList, SimpleType::EmptyList) => Ok(SimpleType::EmptyList),
                                (SimpleType::List(inner), SimpleType::EmptyList) |
                                (SimpleType::EmptyList, SimpleType::List(inner)) => Ok(
                                    SimpleType::List(inner.clone())
                                ),
                                (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                                    Ok(SimpleType::List(inner1.unify_with(inner2)))
                                }
                                (a,b) => {
                                    let is_a_valid = a.is_arithmetic() || a.is_list() || a.is_string();
                                    let is_b_valid = b.is_arithmetic() || b.is_list() || b.is_string();
                                    let span = if is_a_valid {
                                        right.span.clone()
                                    } else {
                                        left.span.clone()
                                    };
                                    let expected = if is_a_valid {
                                        a.clone()
                                    } else if is_b_valid {
                                        b.clone()
                                    } else {
                                        SimpleType::Int
                                    };
                                    let found = if is_a_valid {
                                        b.clone()
                                    } else {
                                        a.clone()
                                    };
                                    Err(SemError::TypeMismatch {
                                        span,
                                        expected: expected.into(),
                                        found: found.into(),
                                        context: format!(
                                            "addition/concatenation requires Int, LinExpr, String or List, got {} and {}",
                                            a, b
                                        ),
                                    })
                                }
                            }
                        )
                    }
                }
            }
            // Int - Int -> Int
            // LinExpr - Int -> LinExpr (auto convert Int to LinExpr)
            // Int - LinExpr -> LinExpr (auto convert Int to LinExpr)
            // LinExpr - LinExpr -> LinExpr
            // [Type1] - [Type2] -> [Type1] if Type1 and Type2 overlap
            Expr::Sub(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (None, None) => None,
                    (Some(t), None) |
                    (None, Some(t)) => {
                        if t.is_int() || t.is_lin_expr() || t.is_list() {
                            Some(t.clone())
                        } else if t.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap()) {
                            Some(SimpleType::LinExpr.into())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context:
                                    "substraction/difference requires Int or LinExpr or List"
                                        .to_string(),
                            });
                            None
                        }
                    }
                    (Some(l), Some(r)) => {
                        l.cross_check(
                            r,
                            errors,
                            |v1,v2| match (v1,v2) {
                                (SimpleType::Int, SimpleType::Int) => Ok(SimpleType::Int),
                                (SimpleType::LinExpr, SimpleType::Int) |
                                (SimpleType::Int, SimpleType::LinExpr) |
                                (SimpleType::LinExpr, SimpleType::LinExpr) => Ok(SimpleType::LinExpr),
                                (SimpleType::EmptyList, _) => Err(SemError::TypeMismatch {
                                    span: left.span.clone(),
                                    expected: SimpleType::List(SimpleType::Int.into()).into(),
                                    found: SimpleType::EmptyList.into(),
                                    context: "Cannot remove elements from empty list".to_string(),
                                }),
                                (SimpleType::List(_inner), SimpleType::EmptyList) => Err(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::List(SimpleType::Int.into()).into(),
                                    found: SimpleType::EmptyList.into(),
                                    context: "Removing empty list is a no-op".to_string(),
                                }),
                                (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                                    if inner1.overlaps_with(inner2) {
                                        Ok(SimpleType::List(inner1.clone()))
                                    } else {
                                        Err(SemError::TypeMismatch {
                                            span: right.span.clone(),
                                            expected: inner1.clone(),
                                            found: inner2.clone(),
                                            context: "Types must overlap in set difference".to_string(),
                                        })
                                    }
                                }
                                (a,b) => {
                                    let is_a_valid = a.is_arithmetic() || a.is_list();
                                    let is_b_valid = b.is_arithmetic() || b.is_list();
                                    let span = if is_a_valid {
                                        right.span.clone()
                                    } else {
                                        left.span.clone()
                                    };
                                    let expected = if is_a_valid {
                                        a.clone()
                                    } else if is_b_valid {
                                        b.clone()
                                    } else {
                                        SimpleType::Int
                                    };
                                    let found = if is_a_valid {
                                        b.clone()
                                    } else {
                                        a.clone()
                                    };
                                    Err(SemError::TypeMismatch {
                                        span,
                                        expected: expected.into(),
                                        found: found.into(),
                                        context: format!(
                                            "subtraction/difference requires Int, LinExpr or List, got {} and {}",
                                            a, b
                                        ),
                                    })
                                }
                            }
                        )
                    }
                }
            }
            // Unary negation - for LinExpr and Int
            Expr::Neg(term) => {
                let term_type = self.check_expr(
                    global_env,
                    &term.node,
                    &term.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match term_type.clone() {
                    Some(t) if t.is_arithmetic() => Some(t),
                    Some(t) => {
                        let span = term.span.clone();
                        errors.push(SemError::TypeMismatch {
                            span,
                            expected: SimpleType::Int.into(),
                            found: t.clone(),
                            context: "negation requires Int or LinExpr".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }
            // Multiplication: Int * Int -> Int, Int * LinExpr -> LinExpr, LinExpr * Int -> LinExpr
            // But NOT LinExpr * LinExpr (non-linear!)
            Expr::Mul(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (None, None) => None,
                    (Some(t), None) | (None, Some(t)) => {
                        if t.is_int() || t.is_lin_expr() {
                            Some(t.clone())
                        } else if t.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap()) {
                            Some(SimpleType::LinExpr.into())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context: "multiplication requires Int or LinExpr".to_string(),
                            });
                            None
                        }
                    }
                    (Some(l), Some(r)) => l.cross_check(r, errors, |v1, v2| match (v1, v2) {
                        (SimpleType::Int, SimpleType::Int) => Ok(SimpleType::Int),
                        (SimpleType::LinExpr, SimpleType::Int)
                        | (SimpleType::Int, SimpleType::LinExpr) => Ok(SimpleType::LinExpr),
                        (SimpleType::LinExpr, SimpleType::LinExpr) => Err(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: SimpleType::LinExpr.into(),
                            context: "cannot multiply two linear expressions (non-linear)"
                                .to_string(),
                        }),
                        (a, b) => {
                            let is_a_valid = a.is_arithmetic();
                            let is_b_valid = b.is_arithmetic();
                            let span = if is_a_valid {
                                right.span.clone()
                            } else {
                                left.span.clone()
                            };
                            let expected = if is_a_valid {
                                a.clone()
                            } else if is_b_valid {
                                b.clone()
                            } else {
                                SimpleType::Int
                            };
                            let found = if is_a_valid { b.clone() } else { a.clone() };
                            Err(SemError::TypeMismatch {
                                span,
                                expected: expected.into(),
                                found: found.into(),
                                context: format!(
                                    "multiplication requires Int or LinExpr, got {} and {}",
                                    a, b
                                ),
                            })
                        }
                    }),
                }
            }
            // Division and modulo: Int // Int -> Int, Int % Int -> Int
            // These are NOT allowed on LinExpr
            Expr::Div(left, right) | Expr::Mod(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both are Int
                        let l_ok = l.is_int();
                        let r_ok = r.is_int();

                        if !l_ok {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: l.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                        }
                        if !r_ok {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: r.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                        }

                        if l_ok || r_ok {
                            Some(SimpleType::Int.into())
                        } else {
                            None
                        }
                    }
                    (Some(t), None) => {
                        if !t.is_int() {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                            None
                        } else {
                            Some(SimpleType::Int.into())
                        }
                    }
                    (None, Some(t)) => {
                        if !t.is_int() {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                            None
                        } else {
                            Some(SimpleType::Int.into())
                        }
                    }
                    (None, None) => None,
                }
            }

            // ========== Constraints operators ==========
            Expr::ConstraintEq(left, right)
            | Expr::ConstraintLe(left, right)
            | Expr::ConstraintGe(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                if let (Some(l), Some(r)) = (left_type, right_type) {
                    // Check if both can convert to LinExpr
                    let l_ok = l.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap());
                    let r_ok = r.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap());

                    if !l_ok {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::LinExpr.into(),
                            found: l,
                            context: "constraint operator requires LinExpr or Int operands"
                                .to_string(),
                        });
                    }
                    if !r_ok {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: SimpleType::LinExpr.into(),
                            found: r,
                            context: "constraint operator requires LinExpr or Int operands"
                                .to_string(),
                        });
                    }
                }

                // Always return Constraint (even on error, intent is clear)
                Some(SimpleType::Constraint.into())
            }

            // ========== Comparison Operations ==========
            Expr::Eq(left, right) | Expr::Ne(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                if let (Some(l), Some(r)) = (left_type, right_type) {
                    if !l.overlaps_with(&r) {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l.clone(),
                            found: r.clone(),
                            context: "equality can never happens with incompatible types"
                                .to_string(),
                        });
                    }
                }
                Some(SimpleType::Bool.into())
            }

            // Relational: Int < Int -> Bool
            Expr::Le(left, right)
            | Expr::Ge(left, right)
            | Expr::Lt(left, right)
            | Expr::Gt(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                if let (Some(l), Some(r)) = (left_type, right_type) {
                    // Check if both can coerce to Int
                    let l_ok = l.can_convert_to(&SimpleType::Int.into_concrete().unwrap());
                    let r_ok = r.can_convert_to(&SimpleType::Int.into_concrete().unwrap());

                    if !l_ok {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: l,
                            context: "relational comparison requires Int operands".to_string(),
                        });
                    }
                    if !r_ok {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: r,
                            context: "relational comparison requires Int operands".to_string(),
                        });
                    }
                }
                Some(SimpleType::Bool.into())
            }

            // ========== Boolean Operations ==========
            // Bool and Bool -> Bool, Constraint and Constraint -> Constraint
            Expr::And(left, right) | Expr::Or(left, right) => {
                let left_type = self.check_expr(
                    global_env,
                    &left.node,
                    &left.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (Some(l), Some(r)) => {
                        if l.is_bool() {
                            if r.is_bool() {
                                Some(SimpleType::Bool.into())
                            } else {
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::Bool.into(),
                                    found: r.clone(),
                                    context: "and/or requires Bool or Constraint operands"
                                        .to_string(),
                                });
                                None
                            }
                        } else if l.is_constraint() {
                            if r.is_constraint() {
                                Some(SimpleType::Constraint.into())
                            } else {
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::Constraint.into(),
                                    found: r.clone(),
                                    context: "and/or requires Bool or Constraint operands"
                                        .to_string(),
                                });
                                None
                            }
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: l.clone(),
                                context: "and/or requires Bool or Constraint operands".to_string(),
                            });
                            None
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => {
                        if t.is_bool() || t.is_constraint() {
                            Some(t.clone())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Bool.into(),
                                found: t.clone(),
                                context: "and/or requires Bool or Constraint operands".to_string(),
                            });
                            None
                        }
                    }
                    (None, None) => None,
                }
            }

            Expr::Not(expr) => {
                let expr_type = self.check_expr(
                    global_env,
                    &expr.node,
                    &expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match expr_type {
                    Some(typ) if typ.is_bool() => Some(SimpleType::Bool.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: SimpleType::Bool.into(),
                            found: typ.clone(),
                            context: "not requires Bool operand".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            // ========== Null Coalescing ==========
            // x ?? default -> (typeof x - None) | typeof default
            Expr::NullCoalesce(lhs, rhs) => {
                let lhs_type = self.check_expr(
                    global_env,
                    &lhs.node,
                    &lhs.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let rhs_type = self.check_expr(
                    global_env,
                    &rhs.node,
                    &rhs.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (lhs_type, rhs_type) {
                    (Some(l), Some(r)) => {
                        // LHS must contain None
                        if !l.contains(&SimpleType::None) {
                            errors.push(SemError::NullCoalesceOnNonMaybe {
                                span: lhs.span.clone(),
                                found: l.clone(),
                            });
                            return None;
                        }
                        // Result type is (LHS - None) | RHS
                        match l.substract(&SimpleType::None.into()) {
                            Some(lhs_without_none) => Some(lhs_without_none.unify_with(&r)),
                            // LHS was just None, result is RHS type
                            None => Some(r),
                        }
                    }
                    (Some(l), None) => {
                        // Still check that LHS contains None
                        if !l.contains(&SimpleType::None) {
                            errors.push(SemError::NullCoalesceOnNonMaybe {
                                span: lhs.span.clone(),
                                found: l.clone(),
                            });
                        }
                        None
                    }
                    _ => None,
                }
            }

            // ========== Panic ==========
            // panic! expr -> Never (always diverges)
            Expr::Panic(inner_expr) => {
                // Type-check the inner expression (any type is allowed)
                let _ = self.check_expr(
                    global_env,
                    &inner_expr.node,
                    &inner_expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Panic always returns Never type
                Some(SimpleType::Never.into())
            }

            // ========== Membership Test ==========
            // x in collection -> Bool
            Expr::In { item, collection } => {
                let item_type = self.check_expr(
                    global_env,
                    &item.node,
                    &item.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match coll_type {
                    Some(a) if a.is_list() => {
                        let elem_t_opt = a.get_inner_list_type();
                        if let Some(elem_t) = elem_t_opt {
                            // The list might not be empty so we check the inner type
                            if let Some(item_t) = item_type {
                                if !item_t.overlaps_with(&elem_t) {
                                    errors.push(SemError::TypeMismatch {
                                        span: item.span.clone(),
                                        expected: elem_t.into(),
                                        found: item_t,
                                        context: "item type must match collection element type"
                                            .to_string(),
                                    });
                                }
                            }
                        }
                    }
                    Some(t) => {
                        // Not a list at all
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "membership test requires a list".to_string(),
                        });
                    }
                    None => {
                        // Collection failed to type-check
                    }
                }

                // Always returns Bool
                Some(SimpleType::Bool.into())
            }

            // ========== Quantifiers ==========
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        module: self.current_module().to_string(),
                        identifier: var.node.clone(),
                        span: var.span.clone(),
                        suggestion,
                    });
                }

                // Extract element type from collection
                match coll_type {
                    Some(a) if a.is_empty_list() => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(),
                            found: a.clone(),
                            context: "forall collection inner type must be known (use 'as' for explicit typing)".to_string(),
                        });
                        return None; // Return early
                    }
                    Some(a) if a.is_list() => {
                        let elem_t = a
                            .get_inner_list_type()
                            .expect("The list should not be empty at this point");
                        // Register the loop variable with the element type
                        if let Err(e) = self.register_identifier(
                            global_env,
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        ) {
                            errors.push(e);
                            return None;
                        }
                    }

                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "forall collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None, // Return early
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_expr(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if !typ.is_bool() {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "forall filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be Constraint or Bool)
                let body_type = self.check_expr(
                    global_env,
                    &body.node,
                    &body.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                self.pop_scope(warnings);

                match body_type {
                    Some(typ) if typ.is_constraint() => Some(SimpleType::Constraint.into()),
                    Some(typ) if typ.is_bool() => Some(SimpleType::Bool.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: SimpleType::Constraint.into(),
                            found: typ,
                            context: "forall body must be Constraint or Bool".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        module: self.current_module().to_string(),
                        identifier: var.node.clone(),
                        span: var.span.clone(),
                        suggestion,
                    });
                }

                // Extract element type from collection
                match coll_type {
                    Some(a) if a.is_empty_list() => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(),
                            found: a.clone(),
                            context:
                                "sum collection inner type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        return None; // Return early
                    }
                    Some(a) if a.is_list() => {
                        let elem_t = a
                            .get_inner_list_type()
                            .expect("List should not be empty at this point");
                        // Register the loop variable with the element type
                        if let Err(e) = self.register_identifier(
                            global_env,
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        ) {
                            errors.push(e);
                            return None;
                        }
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "sum collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None, // Return early
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_expr(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if !typ.is_bool() {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "sum filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be arithmetic: Int or LinExpr)
                let body_type = self.check_expr(
                    global_env,
                    &body.node,
                    &body.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                self.pop_scope(warnings);

                match body_type {
                    Some(typ) if typ.is_arithmetic() || typ.is_list() || typ.is_string() => {
                        Some(typ)
                    }
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: typ,
                            context: "sum body must be Int, LinExpr, String or List".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            Expr::Fold {
                var,
                collection,
                accumulator,
                init_value,
                filter,
                body,
                reversed: _,
            } => {
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                let acc_type = self.check_expr(
                    global_env,
                    &init_value.node,
                    &init_value.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Check naming conventions
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        module: self.current_module().to_string(),
                        identifier: var.node.clone(),
                        span: var.span.clone(),
                        suggestion,
                    });
                }

                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &accumulator.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        module: self.current_module().to_string(),
                        identifier: accumulator.node.clone(),
                        span: accumulator.span.clone(),
                        suggestion,
                    });
                }

                // Extract type info for elements in the collection
                let elem_t = match coll_type {
                    Some(a) if a.is_empty_list() => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(),
                            found: a.clone(),
                            context: "fold|rfold collection inner type must be known (use 'as' for explicit typing)".to_string(),
                        });
                        return None; // Return early
                    }
                    Some(a) if a.is_list() => a
                        .get_inner_list_type()
                        .expect("List should not be empty at this point"),
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "fold|rfold collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None, // Return early
                };

                let mut current_acc_type = match acc_type {
                    Some(t) => t,
                    None => return None,
                };
                let mut has_to_iterate = true;
                while has_to_iterate {
                    // Register the loop variable with the element type
                    if let Err(e) = self.register_identifier(
                        global_env,
                        &var.node,
                        var.span.clone(),
                        elem_t.clone(),
                        type_info,
                        warnings,
                    ) {
                        errors.push(e);
                    }

                    // Register the accumulator variable with the current computed type
                    if let Err(e) = self.register_identifier(
                        global_env,
                        &accumulator.node,
                        accumulator.span.clone(),
                        current_acc_type.clone(),
                        type_info,
                        warnings,
                    ) {
                        errors.push(e);
                    }

                    self.push_scope();

                    // Check filter (must be Bool)
                    if let Some(filter_expr) = filter {
                        let filter_type = self.check_expr(
                            global_env,
                            &filter_expr.node,
                            &filter_expr.span,
                            type_info,
                            expr_types,
                            resolved_types,
                            errors,
                            warnings,
                        );

                        if let Some(typ) = filter_type {
                            if !typ.is_bool() {
                                errors.push(SemError::TypeMismatch {
                                    span: filter_expr.span.clone(),
                                    expected: SimpleType::Bool.into(),
                                    found: typ,
                                    context: "fold|rfold filter must be Bool".to_string(),
                                });
                            }
                        }
                    }

                    // Check body (must match accumulator)
                    let body_type = self.check_expr(
                        global_env,
                        &body.node,
                        &body.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );

                    self.pop_scope(warnings);

                    match body_type {
                        Some(typ) => {
                            has_to_iterate = !typ.is_subtype_of(&current_acc_type);
                            if has_to_iterate {
                                current_acc_type = current_acc_type.unify_with(&typ);
                            }
                        }
                        None => {
                            // This will end the loop and return the last known type
                            // for the accumulator
                            has_to_iterate = false;
                        }
                    }
                }

                Some(current_acc_type)
            }

            // ========== If Expression ==========
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_type = self.check_expr(
                    global_env,
                    &condition.node,
                    &condition.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                if let Some(typ) = cond_type {
                    if !typ.is_bool() {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: SimpleType::Bool.into(),
                            found: typ,
                            context: "if condition must be Bool".to_string(),
                        });
                    }
                }

                let then_type = self.check_expr(
                    global_env,
                    &then_expr.node,
                    &then_expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let else_type = self.check_expr(
                    global_env,
                    &else_expr.node,
                    &else_expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                match (then_type, else_type) {
                    (Some(t), Some(e)) => Some(t.unify_with(&e)),
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
            }
            Expr::Match {
                match_expr,
                branches,
            } => {
                let Some(expr_type) = self.check_expr(
                    global_env,
                    &match_expr.node,
                    &match_expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                ) else {
                    // Cannot type check anything, propagate underspecified type
                    return None;
                };

                let mut output = Option::<ExprType>::None;
                let mut current_type = Some(expr_type);

                for branch in branches {
                    let as_type = if let Some(bt) = &branch.as_typ {
                        match global_env.resolve_type(bt, self.current_module()) {
                            Ok(t) => {
                                if !global_env.validate_type(&t) {
                                    errors.push(SemError::UnknownType {
                                        module: self.current_module().to_string(),
                                        typ: t.to_string(),
                                        span: bt.span.clone(),
                                    });
                                    continue;
                                }
                                // Cache the resolved type for use during evaluation
                                resolved_types.insert(bt.span.clone(), t.clone());
                                Some(t)
                            }
                            Err(e) => {
                                errors.push(e);
                                continue; // Can't evaluate branch
                            }
                        }
                    } else {
                        None
                    };

                    let bad_branch_typ = match &current_type {
                        Some(typ) => match &as_type {
                            Some(b_typ) => !b_typ.is_subtype_of(typ),
                            None => false,
                        },
                        None => true,
                    };

                    if bad_branch_typ {
                        errors.push(SemError::OverMatching {
                            span: match &branch.as_typ {
                                Some(t) => t.span.clone(),
                                None => branch.ident.span.clone(),
                            },
                            expected: current_type.clone(),
                            found: as_type.clone(),
                        });
                    }

                    let actual_branch_typ_opt = match as_type {
                        Some(typ) => Some(typ),
                        None => current_type.clone(),
                    };

                    if let Some(actual_branch_typ) = actual_branch_typ_opt {
                        if let Err(e) = self.register_identifier(
                            global_env,
                            &branch.ident.node,
                            branch.ident.span.clone(),
                            actual_branch_typ.clone(),
                            type_info,
                            warnings,
                        ) {
                            panic!("There should be no other identifier in the current scope. But got: {:?}", e);
                        }

                        self.push_scope();

                        if let Some(filter_expr) = &branch.filter {
                            let filter_type = self.check_expr(
                                global_env,
                                &filter_expr.node,
                                &filter_expr.span,
                                type_info,
                                expr_types,
                                resolved_types,
                                errors,
                                warnings,
                            );

                            if let Some(typ) = filter_type {
                                if !typ.is_bool() {
                                    errors.push(SemError::TypeMismatch {
                                        span: filter_expr.span.clone(),
                                        expected: SimpleType::Bool.into(),
                                        found: typ,
                                        context: "where filter must be Bool".to_string(),
                                    });
                                }
                            }
                        }

                        let body_typ = self.check_expr(
                            global_env,
                            &branch.body.node,
                            &branch.body.span,
                            type_info,
                            expr_types,
                            resolved_types,
                            errors,
                            warnings,
                        );

                        self.pop_scope(warnings);

                        // Update output type
                        if let Some(typ) = body_typ {
                            output = Some(match output {
                                Some(x) => x.unify_with(&typ),
                                None => typ,
                            });
                        }

                        // Update remaining type
                        if branch.filter.is_none() {
                            if let Some(typ) = current_type {
                                // Use enum-aware subtraction to properly handle enum variants
                                current_type =
                                    global_env.substract_enum_aware(&typ, &actual_branch_typ);
                            }
                        }
                    }
                }

                if let Some(remaining_types) = current_type {
                    errors.push(SemError::NonExhaustiveMatching {
                        span: global_span.clone(),
                        remaining_types,
                    });
                }

                output
            }

            // ========== ILP Variables ==========
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

                // Build the full span (from module start if present, to name end)
                let full_span = match module {
                    Some(mod_span) => Span {
                        start: mod_span.span.start,
                        end: name.span.end,
                    },
                    None => name.span.clone(),
                };

                let path = Spanned::new(crate::ast::NamespacePath { segments }, full_span);

                // Use resolve_path instead of lookup_var
                match resolve_path(&path, self.current_module(), global_env, None) {
                    Ok(ResolvedPathKind::InternalVariable {
                        module: var_module,
                        name: var_name,
                    }) => {
                        // Mark variable as used
                        global_env.mark_var_used(&var_module, &var_name);

                        // Get variable args from internal_variables
                        let var_args = global_env
                            .get_internal_variable_args(&var_module, &var_name)
                            .expect("Internal variable should exist after resolution");

                        // Check argument count
                        if args.len() != var_args.len() {
                            errors.push(SemError::ArgumentCountMismatch {
                                identifier: name.node.clone(),
                                span: args
                                    .last()
                                    .map(|a| a.span.clone())
                                    .unwrap_or_else(|| name.span.clone()),
                                expected: var_args.len(),
                                found: args.len(),
                            });
                        }

                        // Type-check each argument
                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type = self.check_expr(
                                global_env,
                                &arg.node,
                                &arg.span,
                                type_info,
                                expr_types,
                                resolved_types,
                                errors,
                                warnings,
                            );

                            if let Some(found_type) = arg_type {
                                if !found_type.is_subtype_of(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone(),
                                        found: found_type,
                                        context: format!(
                                            "argument {} to variable ${}",
                                            i + 1,
                                            name.node
                                        ),
                                    });
                                }
                            }
                        }

                        Some(SimpleType::LinExpr.into())
                    }
                    Ok(ResolvedPathKind::ExternalVariable(ext_var_name)) => {
                        // Get variable args from external_variables
                        let var_args = global_env
                            .get_external_variable_args(&ext_var_name)
                            .expect("External variable should exist after resolution");

                        // Check argument count
                        if args.len() != var_args.len() {
                            errors.push(SemError::ArgumentCountMismatch {
                                identifier: name.node.clone(),
                                span: args
                                    .last()
                                    .map(|a| a.span.clone())
                                    .unwrap_or_else(|| name.span.clone()),
                                expected: var_args.len(),
                                found: args.len(),
                            });
                        }

                        // Type-check each argument
                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type = self.check_expr(
                                global_env,
                                &arg.node,
                                &arg.span,
                                type_info,
                                expr_types,
                                resolved_types,
                                errors,
                                warnings,
                            );

                            if let Some(found_type) = arg_type {
                                if !found_type.is_subtype_of(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone(),
                                        found: found_type,
                                        context: format!(
                                            "argument {} to variable ${}",
                                            i + 1,
                                            name.node
                                        ),
                                    });
                                }
                            }
                        }

                        Some(SimpleType::LinExpr.into())
                    }
                    Err(_) | Ok(_) => {
                        // Path not found or resolved to something that's not a variable
                        errors.push(SemError::UnknownVariable {
                            module: self.current_module().to_string(),
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::LinExpr.into())
                    }
                }
            }

            Expr::VarListCall { module, name, args } => {
                // Build NamespacePath with $[name] format for the variable list name
                let var_name_with_dollar = format!("$[{}]", name.node);

                let segments = match module {
                    Some(mod_span) => vec![
                        mod_span.clone(),
                        Spanned::new(var_name_with_dollar, name.span.clone()),
                    ],
                    None => vec![Spanned::new(var_name_with_dollar, name.span.clone())],
                };

                // Build the full span (from module start if present, to name end)
                let full_span = match module {
                    Some(mod_span) => Span {
                        start: mod_span.span.start,
                        end: name.span.end,
                    },
                    None => name.span.clone(),
                };

                let path = Spanned::new(crate::ast::NamespacePath { segments }, full_span);

                // Use resolve_path instead of lookup_var_list
                match resolve_path(&path, self.current_module(), global_env, None) {
                    Ok(ResolvedPathKind::VariableList {
                        module: var_module,
                        name: var_name,
                    }) => {
                        // Mark variable list as used
                        global_env.mark_var_list_used(&var_module, &var_name);

                        // Get variable args from variable_lists
                        let var_args = global_env
                            .get_variable_list_args(&var_module, &var_name)
                            .expect("Variable list should exist after resolution");

                        // Check argument count
                        if args.len() != var_args.len() {
                            errors.push(SemError::ArgumentCountMismatch {
                                identifier: name.node.clone(),
                                span: args
                                    .last()
                                    .map(|a| a.span.clone())
                                    .unwrap_or_else(|| name.span.clone()),
                                expected: var_args.len(),
                                found: args.len(),
                            });
                        }

                        // Type-check each argument
                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type = self.check_expr(
                                global_env,
                                &arg.node,
                                &arg.span,
                                type_info,
                                expr_types,
                                resolved_types,
                                errors,
                                warnings,
                            );

                            if let Some(found_type) = arg_type {
                                if !found_type.is_subtype_of(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone(),
                                        found: found_type,
                                        context: format!(
                                            "argument {} to variable list $[{}]",
                                            i + 1,
                                            name.node
                                        ),
                                    });
                                }
                            }
                        }

                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
                    }
                    Err(_) | Ok(_) => {
                        // Path not found or resolved to something that's not a variable list
                        errors.push(SemError::UnknownVariable {
                            module: self.current_module().to_string(),
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
                    }
                }
            }

            // ========== Generic Calls: func(args), Type(x), Enum::Variant(x) ==========
            Expr::GenericCall { path, args } => {
                // Use resolve_path to determine what this path refers to
                let resolved =
                    match resolve_path(path, self.current_module(), global_env, Some(self)) {
                        Ok(r) => r,
                        Err(e) => {
                            errors.push(e.into_sem_error(self.current_module()));
                            return None;
                        }
                    };

                match resolved {
                    ResolvedPathKind::LocalVariable(name) => {
                        // Cannot call a local variable
                        errors.push(SemError::UnknownIdentifer {
                            module: self.current_module().to_string(),
                            identifier: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                    ResolvedPathKind::Function { module, func } => {
                        // Function call
                        match global_env.lookup_fn(&module, &func) {
                            None => {
                                // Shouldn't happen: resolve_path said it's a function
                                errors.push(SemError::UnknownIdentifer {
                                    module: self.current_module().to_string(),
                                    identifier: func,
                                    span: path.span.clone(),
                                });
                                None
                            }
                            Some((fn_type, _)) => {
                                // Mark function as used
                                global_env.mark_fn_used(&module, &func);

                                if args.len() != fn_type.args.len() {
                                    errors.push(SemError::ArgumentCountMismatch {
                                        identifier: func.clone(),
                                        span: args
                                            .last()
                                            .map(|a| a.span.clone())
                                            .unwrap_or_else(|| path.span.clone()),
                                        expected: fn_type.args.len(),
                                        found: args.len(),
                                    });
                                }

                                for (i, (arg, expected_type)) in
                                    args.iter().zip(fn_type.args.iter()).enumerate()
                                {
                                    let arg_type = self.check_expr(
                                        global_env,
                                        &arg.node,
                                        &arg.span,
                                        type_info,
                                        expr_types,
                                        resolved_types,
                                        errors,
                                        warnings,
                                    );

                                    if let Some(found_type) = arg_type {
                                        if !found_type.is_subtype_of(expected_type) {
                                            errors.push(SemError::TypeMismatch {
                                                span: arg.span.clone(),
                                                expected: expected_type.clone(),
                                                found: found_type,
                                                context: format!(
                                                    "argument {} to function {}",
                                                    i + 1,
                                                    func
                                                ),
                                            });
                                        }
                                    }
                                }

                                Some(fn_type.output)
                            }
                        }
                    }
                    ResolvedPathKind::Type(simple_type) => {
                        // Type cast: BuiltinType(x), CustomType(x), Enum::Variant(x)
                        self.check_generic_call_type_cast(
                            global_env,
                            &simple_type,
                            args,
                            &path.span,
                            type_info,
                            expr_types,
                            resolved_types,
                            errors,
                            warnings,
                        )
                    }
                    ResolvedPathKind::Module(name) => {
                        // Modules cannot be called
                        errors.push(SemError::UnknownIdentifer {
                            module: self.current_module().to_string(),
                            identifier: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                    ResolvedPathKind::ExternalVariable(name)
                    | ResolvedPathKind::InternalVariable { name, .. }
                    | ResolvedPathKind::VariableList { name, .. } => {
                        // Variables use $name or $$name syntax, not function call syntax
                        errors.push(SemError::UnknownIdentifer {
                            module: self.current_module().to_string(),
                            identifier: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                }
            }

            // ========== Struct Calls: Type { field: value } ==========
            Expr::StructCall { path, fields } => {
                // Use resolve_path to determine what this path refers to
                let resolved =
                    match resolve_path(path, self.current_module(), global_env, Some(self)) {
                        Ok(r) => r,
                        Err(e) => {
                            errors.push(e.into_sem_error(self.current_module()));
                            return None;
                        }
                    };

                match resolved {
                    ResolvedPathKind::LocalVariable(name) => {
                        // Cannot use struct syntax with variables
                        errors.push(SemError::UnknownType {
                            module: self.current_module().to_string(),
                            typ: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                    ResolvedPathKind::Function { func, .. } => {
                        // Cannot use struct syntax with functions
                        errors.push(SemError::UnknownType {
                            module: self.current_module().to_string(),
                            typ: func,
                            span: path.span.clone(),
                        });
                        None
                    }
                    ResolvedPathKind::Type(simple_type) => self.check_struct_call_type(
                        global_env,
                        &simple_type,
                        fields,
                        &path.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    ),
                    ResolvedPathKind::Module(name) => {
                        // Cannot use struct syntax with modules
                        errors.push(SemError::UnknownType {
                            module: self.current_module().to_string(),
                            typ: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                    ResolvedPathKind::ExternalVariable(name)
                    | ResolvedPathKind::InternalVariable { name, .. }
                    | ResolvedPathKind::VariableList { name, .. } => {
                        // Cannot use struct syntax with variables
                        errors.push(SemError::UnknownType {
                            module: self.current_module().to_string(),
                            typ: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                }
            }

            // ========== Collections ==========
            Expr::GlobalList(type_name) => {
                let typ = match global_env.resolve_type(type_name, self.current_module()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return None;
                    }
                };
                if !global_env.validate_type(&typ) {
                    errors.push(SemError::UnknownType {
                        module: self.current_module().to_string(),
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else if !typ.is_sum_of_objects() {
                    errors.push(SemError::GlobalCollectionsMustBeAListOfObjects {
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else {
                    // Cache the resolved type for use during evaluation
                    resolved_types.insert(type_name.span.clone(), typ.clone());
                    Some(SimpleType::List(typ).into())
                }
            }

            Expr::ListLiteral { elements } => {
                if elements.is_empty() {
                    return Some(SimpleType::EmptyList.into());
                }

                // Check all elements and unify their types
                let mut unified_type = self.check_expr(
                    global_env,
                    &elements[0].node,
                    &elements[0].span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                for item in &elements[1..] {
                    let item_type = self.check_expr(
                        global_env,
                        &item.node,
                        &item.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );

                    match (unified_type.clone(), item_type) {
                        (Some(u), Some(i)) => {
                            unified_type = Some(u.unify_with(&i));
                        }
                        (Some(u), None) => {
                            // Item failed to type-check, keep unified type
                            unified_type = Some(u);
                        }
                        (None, Some(i)) => {
                            // First element failed, use this item's type
                            unified_type = Some(i);
                        }
                        (None, None) => {
                            // Both failed
                            unified_type = None;
                        }
                    }
                }

                match unified_type {
                    Some(t) => Some(SimpleType::List(t).into()),
                    None => None, // Inner None does not imply [<unknown>] because this is reserved for empty lists
                }
            }

            Expr::TupleLiteral { elements } => {
                // Tuples must have at least 2 elements (enforced by grammar)
                let element_types: Vec<_> = elements
                    .iter()
                    .filter_map(|elem| {
                        self.check_expr(
                            global_env,
                            &elem.node,
                            &elem.span,
                            type_info,
                            expr_types,
                            resolved_types,
                            errors,
                            warnings,
                        )
                    })
                    .collect();

                // If any element failed to type-check, we can't form a valid tuple type
                if element_types.len() != elements.len() {
                    return None;
                }

                Some(SimpleType::Tuple(element_types).into())
            }

            Expr::StructLiteral { fields } => {
                let mut field_types: BTreeMap<String, ExprType> = BTreeMap::new();
                let mut seen_fields: HashMap<String, Span> = HashMap::new();
                let mut all_ok = true;

                for (field_name, field_expr) in fields {
                    // Check for duplicate field names
                    if let Some(prev_span) = seen_fields.get(&field_name.node) {
                        errors.push(SemError::DuplicateStructField {
                            module: self.current_module().to_string(),
                            field: field_name.node.clone(),
                            span: field_name.span.clone(),
                            previous: prev_span.clone(),
                        });
                        all_ok = false;
                        continue;
                    }
                    seen_fields.insert(field_name.node.clone(), field_name.span.clone());

                    // Type-check the field expression
                    if let Some(field_type) = self.check_expr(
                        global_env,
                        &field_expr.node,
                        &field_expr.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    ) {
                        field_types.insert(field_name.node.clone(), field_type);
                    } else {
                        all_ok = false;
                    }
                }

                // If any field failed to type-check, we can't form a valid struct type
                if !all_ok {
                    return None;
                }

                Some(SimpleType::Struct(field_types).into())
            }

            Expr::ListRange { start, end } => {
                let start_type = self.check_expr(
                    global_env,
                    &start.node,
                    &start.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                let end_type = self.check_expr(
                    global_env,
                    &end.node,
                    &end.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                if let (Some(s), Some(e)) = (start_type, end_type) {
                    // Check if both can coerce to Int
                    let s_ok = s.is_int();
                    let e_ok = e.is_int();

                    if !s_ok {
                        errors.push(SemError::TypeMismatch {
                            span: start.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: s,
                            context: "list range requires Int limits".to_string(),
                        });
                    }
                    if !e_ok {
                        errors.push(SemError::TypeMismatch {
                            span: end.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: e,
                            context: "list range requires Int limits".to_string(),
                        });
                    }
                }
                // Always return [Int] (even on error, intent is clear)
                Some(SimpleType::List(SimpleType::Int.into()).into())
            }

            Expr::ListComprehension {
                body: expr,
                vars_and_collections,
                filter,
            } => {
                for (i, (var, collection)) in vars_and_collections.iter().enumerate() {
                    let mut typ_error = false;

                    let coll_type = self.check_expr(
                        global_env,
                        &collection.node,
                        &collection.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );

                    // Check naming convention
                    if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                        &var.node,
                        string_case::NamingConvention::SnakeCase,
                    ) {
                        warnings.push(SemWarning::ParameterNamingConvention {
                            module: self.current_module().to_string(),
                            identifier: var.node.clone(),
                            span: var.span.clone(),
                            suggestion,
                        });
                    }

                    // Extract element type from collection
                    match coll_type {
                        Some(a) if a.is_empty_list() => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: SimpleType::List(SimpleType::Int.into()).into(),
                                found: a.clone(),
                                context: "list comprehension collection inner type must be known (use 'as' for explicit typing)".to_string(),
                            });
                            typ_error = true;
                        }
                        Some(a) if a.is_list() => {
                            let elem_t = a
                                .get_inner_list_type()
                                .expect("List should not be empty at this point");
                            // Register the loop variable with the element type
                            if let Err(e) = self.register_identifier(
                                global_env,
                                &var.node,
                                var.span.clone(),
                                elem_t,
                                type_info,
                                warnings,
                            ) {
                                errors.push(e);
                                typ_error = true;
                            }
                        }
                        Some(t) => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: SimpleType::List(t.clone()).into(),
                                found: t,
                                context: "list comprehension collection must be a list".to_string(),
                            });
                            typ_error = true;
                        }
                        None => typ_error = true,
                    }

                    if typ_error {
                        for _j in 0..i {
                            let mut ignored_warnings = vec![];
                            self.pop_scope(&mut ignored_warnings);
                        }
                        return None;
                    }

                    self.push_scope();
                }

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_expr(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if !typ.is_bool() {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "list comprehension filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check the output expression - this determines the result element type
                let elem_type = self.check_expr(
                    global_env,
                    &expr.node,
                    &expr.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                for (_var, _collection) in vars_and_collections {
                    self.pop_scope(warnings);
                }

                match elem_type {
                    Some(t) => Some(SimpleType::List(t).into()),
                    None => None, // Inner None does not imply [<unknown>] because this is reserved for empty lists
                }
            }

            // ========== Cardinality ==========
            Expr::Cardinality(collection) => {
                let elem_t = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );
                match elem_t {
                    Some(t) if t.is_list() => (),
                    None => (),
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "cardinality is always computed on a collection".to_string(),
                        });
                    }
                }
                Some(SimpleType::Int.into()) // Cardinality always gives an Int
            }

            // ========== Let construct ==========
            Expr::Let { var, value, body } => {
                let value_type = self.check_expr(
                    global_env,
                    &value.node,
                    &value.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        module: self.current_module().to_string(),
                        identifier: var.node.clone(),
                        span: var.span.clone(),
                        suggestion,
                    });
                }

                // Extract element type from collection
                // Track if registration succeeded to determine return value
                let registration_failed = match value_type {
                    Some(typ) => {
                        if let Err(e) = self.register_identifier(
                            global_env,
                            &var.node,
                            var.span.clone(),
                            typ,
                            type_info,
                            warnings,
                        ) {
                            errors.push(e);
                            true
                        } else {
                            false
                        }
                    }
                    None => true,
                };

                self.push_scope();

                let body_type = self.check_expr(
                    global_env,
                    &body.node,
                    &body.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                self.pop_scope(warnings);

                // Return None if registration failed, otherwise return body type
                if registration_failed {
                    None
                } else {
                    body_type
                }
            }
        }
    }

    /// Handle type casts in GenericCall expressions: BuiltinType(x), CustomType(x), Enum::Variant(x)
    fn check_generic_call_type_cast(
        &mut self,
        global_env: &mut GlobalEnv,
        simple_type: &SimpleType,
        args: &Vec<Spanned<Expr>>,
        span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        resolved_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        match simple_type {
            // Built-in type casts: Int(x), Bool(x), String(x), etc.
            SimpleType::Int
            | SimpleType::Bool
            | SimpleType::String
            | SimpleType::LinExpr
            | SimpleType::Constraint
            | SimpleType::None
            | SimpleType::Never => {
                let type_name = simple_type.to_string();

                // Built-in type casts require exactly 1 argument
                if args.len() != 1 {
                    errors.push(SemError::ArgumentCountMismatch {
                        identifier: type_name,
                        span: span.clone(),
                        expected: 1,
                        found: args.len(),
                    });
                    return Some(simple_type.clone().into());
                }

                // Check the argument type and validate conversion
                let arg = &args[0];
                let arg_type = self.check_expr(
                    global_env,
                    &arg.node,
                    &arg.span,
                    type_info,
                    expr_types,
                    resolved_types,
                    errors,
                    warnings,
                );

                if let Some(inferred) = arg_type {
                    if let Some(concrete_target) = simple_type.clone().into_concrete() {
                        let can_convert = inferred.can_convert_to(&concrete_target)
                            || self.can_convert_with_custom_types(
                                global_env,
                                &inferred,
                                &concrete_target,
                            );

                        if !can_convert {
                            errors.push(SemError::ImpossibleConversion {
                                span: arg.span.clone(),
                                found: inferred,
                                target: concrete_target,
                            });
                        }
                    }
                }

                Some(simple_type.clone().into())
            }

            // Custom type casts: CustomType(x), Enum::Variant(x)
            SimpleType::Custom(module, root, variant_opt) => {
                let qualified_name = match variant_opt {
                    Some(v) => format!("{}::{}", root, v),
                    None => root.clone(),
                };

                let target_type =
                    match global_env.get_custom_type_underlying(module, &qualified_name) {
                        Some(t) => t.clone(),
                        None => {
                            // Shouldn't happen: resolve_path said it exists
                            errors.push(SemError::UnknownType {
                                module: self.current_module().to_string(),
                                typ: qualified_name,
                                span: span.clone(),
                            });
                            return None;
                        }
                    };

                let underlying_simple = target_type.to_simple();
                let is_unit = underlying_simple
                    .as_ref()
                    .map(|s| s.is_none())
                    .unwrap_or(false);
                let is_tuple = matches!(underlying_simple, Some(SimpleType::Tuple(_)));

                if is_unit {
                    // Unit variant: Enum::None()
                    if args.len() > 1 {
                        errors.push(SemError::ArgumentCountMismatch {
                            identifier: qualified_name.clone(),
                            span: span.clone(),
                            expected: 0,
                            found: args.len(),
                        });
                    }
                    if let Some(arg) = args.first() {
                        let arg_type = self.check_expr(
                            global_env,
                            &arg.node,
                            &arg.span,
                            type_info,
                            expr_types,
                            resolved_types,
                            errors,
                            warnings,
                        );
                        if let Some(inferred) = arg_type {
                            if !inferred.is_none() {
                                let none_concrete = SimpleType::None.into_concrete().unwrap();
                                if !inferred.can_convert_to(&none_concrete) {
                                    errors.push(SemError::ImpossibleConversion {
                                        span: arg.span.clone(),
                                        found: inferred,
                                        target: none_concrete,
                                    });
                                }
                            }
                        }
                    }
                } else if is_tuple {
                    // Tuple type: check argument count matches tuple element count
                    if let Some(SimpleType::Tuple(tuple_types)) = underlying_simple.as_ref() {
                        if args.len() != tuple_types.len() {
                            errors.push(SemError::ArgumentCountMismatch {
                                identifier: qualified_name.clone(),
                                span: span.clone(),
                                expected: tuple_types.len(),
                                found: args.len(),
                            });
                        }
                        for arg in args {
                            self.check_expr(
                                global_env,
                                &arg.node,
                                &arg.span,
                                type_info,
                                expr_types,
                                resolved_types,
                                errors,
                                warnings,
                            );
                        }
                    }
                } else {
                    // Regular type cast: expects 1 argument
                    if args.len() != 1 {
                        errors.push(SemError::ArgumentCountMismatch {
                            identifier: qualified_name.clone(),
                            span: span.clone(),
                            expected: 1,
                            found: args.len(),
                        });
                    }

                    if let (Some(arg), Some(underlying)) =
                        (args.first(), underlying_simple.as_ref())
                    {
                        let arg_type = self.check_expr(
                            global_env,
                            &arg.node,
                            &arg.span,
                            type_info,
                            expr_types,
                            resolved_types,
                            errors,
                            warnings,
                        );

                        if let Some(inferred) = arg_type {
                            if let Some(concrete_target) = underlying.clone().into_concrete() {
                                if !inferred.can_convert_to(&concrete_target) {
                                    errors.push(SemError::ImpossibleConversion {
                                        span: arg.span.clone(),
                                        found: inferred,
                                        target: concrete_target,
                                    });
                                }
                            }
                        }
                    }
                }

                Some(simple_type.clone().into())
            }

            // Other types (Object, List, Tuple, Struct) can't be used as GenericCall targets
            _ => {
                errors.push(SemError::UnknownType {
                    module: self.current_module().to_string(),
                    typ: simple_type.to_string(),
                    span: span.clone(),
                });
                None
            }
        }
    }

    /// Handle struct-style type casts: Type { field: value }
    fn check_struct_call_type(
        &mut self,
        global_env: &mut GlobalEnv,
        simple_type: &SimpleType,
        fields: &Vec<(Spanned<String>, Spanned<Expr>)>,
        span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        resolved_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        let (module, type_name, variant_name, qualified_name) = match simple_type {
            SimpleType::Custom(module, root, variant) => (
                module.clone(),
                root.clone(),
                variant.clone(),
                match variant {
                    Some(v) => format!("{}::{}", root, v),
                    None => root.clone(),
                },
            ),
            _ => {
                // Only Custom types can use struct syntax
                errors.push(SemError::UnknownType {
                    module: self.current_module().to_string(),
                    typ: simple_type.to_string(),
                    span: span.clone(),
                });
                return None;
            }
        };

        // Look up the custom type
        let underlying = match global_env.get_custom_type_underlying(&module, &qualified_name) {
            Some(t) => t.clone(),
            None => {
                errors.push(SemError::UnknownType {
                    module: self.current_module().to_string(),
                    typ: qualified_name.clone(),
                    span: span.clone(),
                });
                return None;
            }
        };

        // The underlying type should be a struct
        let expected_struct = match underlying.clone().to_simple() {
            Some(SimpleType::Struct(fields)) => Some(fields),
            _ => None,
        };

        match expected_struct {
            None => {
                errors.push(SemError::NonConcreteType {
                    span: span.clone(),
                    found: underlying,
                    context: "Struct-style type cast requires a type that wraps a struct"
                        .to_string(),
                });
                None
            }
            Some(expected_fields) => {
                // Check fields match
                let mut found_fields: HashMap<String, Span> = HashMap::new();
                for (field_name, field_expr) in fields {
                    if let Some(prev_span) = found_fields.get(&field_name.node) {
                        errors.push(SemError::ParameterAlreadyDefined {
                            module: self.current_module().to_string(),
                            identifier: field_name.node.clone(),
                            span: field_name.span.clone(),
                            here: prev_span.clone(),
                        });
                        continue;
                    }
                    found_fields.insert(field_name.node.clone(), field_name.span.clone());

                    // Check if field exists
                    let expected_type = expected_fields.get(&field_name.node);

                    match expected_type {
                        None => {
                            errors.push(SemError::UnknownField {
                                object_type: qualified_name.clone(),
                                field: field_name.node.clone(),
                                span: field_name.span.clone(),
                            });
                        }
                        Some(exp_typ) => {
                            let inferred = self.check_expr(
                                global_env,
                                &field_expr.node,
                                &field_expr.span,
                                type_info,
                                expr_types,
                                resolved_types,
                                errors,
                                warnings,
                            );

                            if let Some(inf) = inferred {
                                if !inf.is_subtype_of(exp_typ) {
                                    errors.push(SemError::TypeMismatch {
                                        span: field_expr.span.clone(),
                                        expected: exp_typ.clone(),
                                        found: inf,
                                        context: format!(
                                            "Field '{}' has wrong type",
                                            field_name.node
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }

                // Check for missing fields
                for exp_name in expected_fields.keys() {
                    if !found_fields.contains_key(exp_name) {
                        errors.push(SemError::UnknownField {
                            object_type: qualified_name.clone(),
                            field: exp_name.clone(),
                            span: span.clone(),
                        });
                    }
                }

                // Return the custom type (using the resolved module, not current_module)
                Some(SimpleType::Custom(module, type_name, variant_name).into())
            }
        }
    }

    fn check_ident_path(
        &mut self,
        global_env: &GlobalEnv,
        path: &Spanned<crate::ast::NamespacePath>,
        _type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        _warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        // Use resolve_path to determine what this path refers to
        let resolved = match resolve_path(path, self.current_module(), global_env, Some(self)) {
            Ok(r) => r,
            Err(e) => {
                errors.push(e.into_sem_error(self.current_module()));
                return None;
            }
        };

        match resolved {
            ResolvedPathKind::LocalVariable(name) => {
                // Look up to get the type
                let (typ, _) = self
                    .lookup_ident(&name)
                    .expect("resolve_path said this is a local variable, but lookup_ident failed");
                // Mark as used
                self.mark_ident_used(&name);
                Some(typ)
            }
            ResolvedPathKind::Function { func, .. } => {
                // Functions cannot be used as values without calling them
                // This currently falls through to UnknownIdentifier in the original code
                // because functions weren't in the lookup path for IdentPath.
                // To maintain compatibility, we'll error similarly.
                errors.push(SemError::UnknownIdentifer {
                    module: self.current_module().to_string(),
                    identifier: func,
                    span: path.span.clone(),
                });
                None
            }
            ResolvedPathKind::Type(simple_type) => {
                // Validate based on the type
                match &simple_type {
                    // Primitive types (except None) cannot be used as values
                    SimpleType::Int
                    | SimpleType::Bool
                    | SimpleType::String
                    | SimpleType::LinExpr
                    | SimpleType::Constraint
                    | SimpleType::Never => {
                        errors.push(SemError::PrimitiveTypeAsValue {
                            module: self.current_module().to_string(),
                            type_name: simple_type.to_string(),
                            span: path.span.clone(),
                        });
                        None
                    }
                    // None is valid as a unit value
                    SimpleType::None => Some(SimpleType::None.into()),
                    // Custom types: check if it's a unit variant
                    SimpleType::Custom(module, root, variant_opt) => {
                        if let Some(variant) = variant_opt {
                            // Enum variant: check if it's a unit variant
                            let qualified_name = format!("{}::{}", root, variant);
                            if let Some(target_type) =
                                global_env.get_custom_type_underlying(module, &qualified_name)
                            {
                                let underlying_simple = target_type.clone().to_simple();
                                let is_unit = underlying_simple
                                    .as_ref()
                                    .map(|s| s.is_none())
                                    .unwrap_or(false);

                                if is_unit {
                                    Some(simple_type.into())
                                } else {
                                    // Non-unit variant requires arguments
                                    errors.push(SemError::ArgumentCountMismatch {
                                        identifier: qualified_name,
                                        span: path.span.clone(),
                                        expected: 1,
                                        found: 0,
                                    });
                                    None
                                }
                            } else {
                                // Shouldn't happen: resolve_path said it exists
                                errors.push(SemError::UnknownType {
                                    module: self.current_module().to_string(),
                                    typ: qualified_name,
                                    span: path.span.clone(),
                                });
                                None
                            }
                        } else {
                            // Root custom type without variant - cannot be used as value
                            errors.push(SemError::PrimitiveTypeAsValue {
                                module: self.current_module().to_string(),
                                type_name: root.clone(),
                                span: path.span.clone(),
                            });
                            None
                        }
                    }
                    // Other types shouldn't appear from resolve_path for identifiers
                    _ => {
                        errors.push(SemError::UnknownIdentifer {
                            module: self.current_module().to_string(),
                            identifier: simple_type.to_string(),
                            span: path.span.clone(),
                        });
                        None
                    }
                }
            }
            ResolvedPathKind::Module(name) => {
                // Modules cannot be used as values
                errors.push(SemError::UnknownIdentifer {
                    module: self.current_module().to_string(),
                    identifier: name,
                    span: path.span.clone(),
                });
                None
            }
            ResolvedPathKind::ExternalVariable(name)
            | ResolvedPathKind::InternalVariable { name, .. }
            | ResolvedPathKind::VariableList { name, .. } => {
                // Variables use $name or $$name syntax, not identifier syntax
                errors.push(SemError::UnknownIdentifer {
                    module: self.current_module().to_string(),
                    identifier: name,
                    span: path.span.clone(),
                });
                None
            }
        }
    }

    fn check_path(
        &mut self,
        global_env: &mut GlobalEnv,
        object: &Spanned<Expr>,
        segments: &Vec<Spanned<crate::ast::PathSegment>>,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        resolved_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        use crate::ast::PathSegment;
        assert!(!segments.is_empty(), "Path must have at least one segment");

        // First segment can be an expression
        let mut current_type = self.check_expr(
            global_env,
            &object.node,
            &object.span,
            type_info,
            expr_types,
            resolved_types,
            errors,
            warnings,
        )?;

        // Follow the path through fields or tuple indices
        for segment in segments {
            match &segment.node {
                PathSegment::Field(field_name) => {
                    // Resolve all custom types to their underlying leaf types
                    let resolved_variants = self.resolve_type_for_access(global_env, &current_type);

                    if resolved_variants.is_empty() {
                        errors.push(SemError::FieldAccessOnNonObject {
                            typ: current_type.clone().into(),
                            field: field_name.clone(),
                            span: segment.span.clone(),
                        });
                        return None;
                    }

                    let mut variants = BTreeSet::new();
                    for resolved_variant in resolved_variants {
                        match resolved_variant {
                            SimpleType::Object(type_name) => {
                                // Look up the field in this object type
                                match global_env.lookup_field(&type_name, field_name) {
                                    Some(field_type) => {
                                        variants.extend(field_type.into_variants());
                                    }
                                    None => {
                                        errors.push(SemError::UnknownField {
                                            object_type: type_name.clone(),
                                            field: field_name.clone(),
                                            span: segment.span.clone(),
                                        });
                                        return None;
                                    }
                                }
                            }
                            SimpleType::Struct(fields) => {
                                // Look up the field in this struct type
                                match fields.get(field_name) {
                                    Some(field_type) => {
                                        variants.extend(field_type.clone().into_variants());
                                    }
                                    None => {
                                        errors.push(SemError::UnknownStructField {
                                            struct_type: SimpleType::Struct(fields).to_string(),
                                            field: field_name.clone(),
                                            span: segment.span.clone(),
                                        });
                                        return None;
                                    }
                                }
                            }
                            _ => {
                                // Can't access fields on non-object/non-struct types
                                errors.push(SemError::FieldAccessOnNonObject {
                                    typ: current_type.clone().into(),
                                    field: field_name.clone(),
                                    span: segment.span.clone(),
                                });
                                return None;
                            }
                        }
                    }
                    current_type =
                        ExprType::sum(variants).expect("There should be at least one variant");
                }

                PathSegment::TupleIndex(index) => {
                    // Resolve all custom types to their underlying leaf types
                    let resolved_variants = self.resolve_type_for_access(global_env, &current_type);

                    if resolved_variants.is_empty() {
                        errors.push(SemError::TupleIndexOnNonTuple {
                            typ: current_type.clone(),
                            index: *index,
                            span: segment.span.clone(),
                        });
                        return None;
                    }

                    let mut variants = BTreeSet::new();
                    for resolved_variant in resolved_variants {
                        match resolved_variant {
                            SimpleType::Tuple(elements) => {
                                if *index >= elements.len() {
                                    errors.push(SemError::TupleIndexOutOfBounds {
                                        index: *index,
                                        size: elements.len(),
                                        span: segment.span.clone(),
                                    });
                                    return None;
                                }
                                variants.extend(elements[*index].clone().into_variants());
                            }
                            _ => {
                                errors.push(SemError::TupleIndexOnNonTuple {
                                    typ: current_type.clone(),
                                    index: *index,
                                    span: segment.span.clone(),
                                });
                                return None;
                            }
                        }
                    }
                    current_type =
                        ExprType::sum(variants).expect("There should be at least one variant");
                }

                PathSegment::ListIndexFallible(index_expr)
                | PathSegment::ListIndexPanic(index_expr) => {
                    // 1. Check index expression is Int
                    let index_type = self.check_expr(
                        global_env,
                        &index_expr.node,
                        &index_expr.span,
                        type_info,
                        expr_types,
                        resolved_types,
                        errors,
                        warnings,
                    )?;

                    if !index_type.is_int() {
                        errors.push(SemError::ListIndexNotInt {
                            span: index_expr.span.clone(),
                            found: index_type,
                        });
                        return None;
                    }

                    // 2. Check current_type is a list (or union of lists)
                    let mut element_types = BTreeSet::new();
                    for variant in current_type.get_variants() {
                        match variant {
                            SimpleType::List(elem_type) => {
                                element_types.extend(elem_type.clone().into_variants());
                            }
                            _ => {
                                errors.push(SemError::IndexOnNonList {
                                    typ: current_type.clone(),
                                    span: segment.span.clone(),
                                });
                                return None;
                            }
                        }
                    }

                    // 3. Compute result type
                    let element_type = ExprType::sum(element_types)
                        .expect("There should be at least one element type");

                    current_type = match &segment.node {
                        PathSegment::ListIndexFallible(_) => element_type.make_optional(),
                        PathSegment::ListIndexPanic(_) => element_type,
                        _ => unreachable!(),
                    };
                }
            }
        }

        Some(current_type)
    }
}
