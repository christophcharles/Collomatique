use super::errors::{SemError, SemWarning};
use super::global_env::{ident_can_be_shadowed, GlobalEnv, TypeInfo};
use super::types::ExprType;
use crate::ast::Span;
use std::collections::HashMap;

/// Trait for checking if an identifier exists in a local environment.
/// This allows resolve_path to work with different LocalEnv implementations.
pub trait LocalEnvCheck {
    fn has_ident(&self, ident: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalCheckEnv {
    scopes: Vec<HashMap<String, (ExprType, Span, bool)>>,
    pending_scope: HashMap<String, (ExprType, Span, bool)>,
    current_module: String,
}

impl LocalEnvCheck for LocalCheckEnv {
    fn has_ident(&self, ident: &str) -> bool {
        self.lookup_ident(ident).is_some()
    }
}

impl LocalCheckEnv {
    pub(crate) fn new(current_module: &str) -> Self {
        LocalCheckEnv {
            scopes: Vec::new(),
            pending_scope: HashMap::new(),
            current_module: current_module.to_string(),
        }
    }

    pub(crate) fn current_module(&self) -> &str {
        &self.current_module
    }

    pub(crate) fn lookup_ident(&self, ident: &str) -> Option<(ExprType, Span)> {
        // We don't look in pending scope as these variables are not yet accessible
        for scope in self.scopes.iter().rev() {
            if let Some((typ, span, _)) = scope.get(ident) {
                return Some((typ.clone(), span.clone()));
            }
        }
        None
    }

    pub(crate) fn mark_ident_used(&mut self, ident: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((_, _, used)) = scope.get_mut(ident) {
                *used = true;
                return;
            }
        }
    }

    pub(crate) fn push_scope(&mut self) {
        let mut old_scope = HashMap::new();
        std::mem::swap(&mut old_scope, &mut self.pending_scope);

        self.scopes.push(old_scope);
    }

    pub(crate) fn pop_scope(&mut self, warnings: &mut Vec<SemWarning>) {
        assert!(!self.scopes.is_empty());

        self.pending_scope = self.scopes.pop().unwrap();

        for (name, (_typ, span, used)) in &self.pending_scope {
            if !*used {
                warnings.push(SemWarning::UnusedIdentifier {
                    module: self.current_module().to_string(),
                    identifier: name.clone(),
                    span: span.clone(),
                });
            }
        }

        self.pending_scope.clear();
    }

    pub(crate) fn register_identifier(
        &mut self,
        global_env: &GlobalEnv,
        ident: &str,
        span: Span,
        typ: ExprType,
        type_info: &mut TypeInfo,
        warnings: &mut Vec<SemWarning>,
    ) -> Result<(), SemError> {
        if let Some((_, old_ident_span, _)) = self.pending_scope.get(ident) {
            return Err(SemError::LocalIdentAlreadyDeclared {
                module: self.current_module().to_string(),
                identifier: ident.to_string(),
                span,
                here: old_ident_span.clone(),
            });
        }

        // Check if this identifier shadows a function in global env
        if global_env.lookup_fn(self.current_module(), ident).is_some() {
            return Err(SemError::LocalIdentShadowsFunction {
                module: self.current_module().to_string(),
                identifier: ident.to_string(),
                span,
            });
        }

        // Check if there's a shadowed identifier in outer scopes
        if !ident_can_be_shadowed(ident) {
            for scope in self.scopes.iter().rev() {
                if let Some((_, old_ident_span, _)) = scope.get(ident) {
                    warnings.push(SemWarning::IdentifierShadowed {
                        module: self.current_module().to_string(),
                        identifier: ident.to_string(),
                        span: span.clone(),
                        previous: old_ident_span.clone(),
                    });
                    break;
                }
            }
        }

        let should_be_used_by_default = ident.chars().next().unwrap() == '_';

        type_info.types.insert(span.clone(), typ.clone().into());
        self.pending_scope
            .insert(ident.to_string(), (typ, span, should_be_used_by_default));

        Ok(())
    }

    /// Register an identifier without duplicate checking (for pass 2 where we already validated)
    pub(crate) fn register_identifier_no_check(&mut self, ident: &str, typ: ExprType) {
        use super::global_env::should_be_used_by_default;
        // Use a dummy span since params were already registered in type_info during pass 1
        self.pending_scope.insert(
            ident.to_string(),
            (
                typ,
                Span { start: 0, end: 0 },
                should_be_used_by_default(ident),
            ),
        );
    }
}
