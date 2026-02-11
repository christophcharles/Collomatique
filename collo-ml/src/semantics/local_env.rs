use super::errors::{SemError, SemWarning};
use super::global_env::{GlobalEnv, TypeInfo, ident_can_be_shadowed};
use super::types::ExprType;
use crate::ast::Span;
use crate::database::DatabaseDriver;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Trait for checking if an identifier exists in a local environment.
/// This allows resolve_path to work with different LocalEnv implementations.
pub trait LocalEnvCheck {
    fn has_ident(&self, ident: &str) -> bool;
}

#[derive(Debug)]
pub struct LocalCheckEnv {
    scope: HashMap<String, (ExprType, Span, AtomicBool)>,
    parent: Option<Arc<LocalCheckEnv>>,
    current_module: Arc<str>,
    warnings: Arc<Mutex<Vec<SemWarning>>>,
}

impl LocalEnvCheck for LocalCheckEnv {
    fn has_ident(&self, ident: &str) -> bool {
        self.lookup_ident(ident).is_some()
    }
}

impl Drop for LocalCheckEnv {
    fn drop(&mut self) {
        for (name, (_, span, used)) in &self.scope {
            if !used.load(Ordering::Relaxed) {
                self.warnings
                    .lock()
                    .unwrap()
                    .push(SemWarning::UnusedIdentifier {
                        module: self.current_module.to_string(),
                        identifier: name.clone(),
                        span: span.clone(),
                    });
            }
        }
    }
}

impl LocalCheckEnv {
    pub(crate) fn new(current_module: &str) -> Arc<Self> {
        Arc::new(LocalCheckEnv {
            scope: HashMap::new(),
            parent: None,
            current_module: Arc::from(current_module),
            warnings: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub(crate) fn current_module(&self) -> &str {
        &self.current_module
    }

    pub(crate) fn lookup_ident(&self, ident: &str) -> Option<(ExprType, Span)> {
        if let Some((typ, span, _)) = self.scope.get(ident) {
            return Some((typ.clone(), span.clone()));
        }
        if let Some(parent) = &self.parent {
            return parent.lookup_ident(ident);
        }
        None
    }

    pub(crate) fn mark_ident_used(&self, ident: &str) {
        if let Some((_, _, used)) = self.scope.get(ident) {
            used.store(true, Ordering::Relaxed);
            return;
        }
        if let Some(parent) = &self.parent {
            parent.mark_ident_used(ident);
        }
    }

    pub(crate) fn start_subscope(parent: Arc<Self>) -> CheckSubscopeBuilder {
        CheckSubscopeBuilder {
            identifiers: HashMap::new(),
            parent,
        }
    }

    /// Push a warning into the shared warning sink.
    pub(crate) fn push_warning(&self, warning: SemWarning) {
        self.warnings.lock().unwrap().push(warning);
    }

    /// Consume the env and return all collected warnings.
    /// This triggers the Drop cascade — each scope pushes unused-variable warnings.
    pub(crate) fn to_warnings(self: Arc<Self>) -> Vec<SemWarning> {
        let warnings_sink = Arc::clone(&self.warnings);
        drop(self); // triggers Drop cascade
        Arc::try_unwrap(warnings_sink)
            .expect("all scopes should be dropped")
            .into_inner()
            .unwrap()
    }
}

pub(crate) struct CheckSubscopeBuilder {
    identifiers: HashMap<String, (ExprType, Span, AtomicBool)>,
    parent: Arc<LocalCheckEnv>,
}

impl CheckSubscopeBuilder {
    pub(crate) fn register_identifier<D: DatabaseDriver>(
        &mut self,
        global_env: &GlobalEnv<D>,
        ident: &str,
        span: Span,
        typ: ExprType,
        type_info: &mut TypeInfo,
    ) -> Result<(), SemError> {
        if let Some((_, old_ident_span, _)) = self.identifiers.get(ident) {
            return Err(SemError::LocalIdentAlreadyDeclared {
                module: self.parent.current_module().to_string(),
                identifier: ident.to_string(),
                span,
                here: old_ident_span.clone(),
            });
        }

        // Check if this identifier shadows a function in global env
        if global_env
            .lookup_fn(self.parent.current_module(), ident)
            .is_some()
        {
            return Err(SemError::LocalIdentShadowsFunction {
                module: self.parent.current_module().to_string(),
                identifier: ident.to_string(),
                span,
            });
        }

        // Check if there's a shadowed identifier in outer scopes (including parent chain)
        if !ident_can_be_shadowed(ident)
            && let Some((_, old_ident_span)) = self.parent.lookup_ident(ident)
        {
            self.parent
                .warnings
                .lock()
                .unwrap()
                .push(SemWarning::IdentifierShadowed {
                    module: self.parent.current_module().to_string(),
                    identifier: ident.to_string(),
                    span: span.clone(),
                    previous: old_ident_span,
                });
        }

        let should_be_used_by_default = ident.starts_with('_');

        type_info.types.insert(span.clone(), typ.clone().into());
        self.identifiers.insert(
            ident.to_string(),
            (typ, span, AtomicBool::new(should_be_used_by_default)),
        );

        Ok(())
    }

    /// Register an identifier without duplicate checking (for pass 2 where we already validated)
    pub(crate) fn register_identifier_no_check(&mut self, ident: &str, typ: ExprType) {
        use super::global_env::should_be_used_by_default;
        // Use a dummy span since params were already registered in type_info during pass 1
        self.identifiers.insert(
            ident.to_string(),
            (
                typ,
                Span { start: 0, end: 0 },
                AtomicBool::new(should_be_used_by_default(ident)),
            ),
        );
    }

    pub(crate) fn build_subscope(self) -> Arc<LocalCheckEnv> {
        Arc::new(LocalCheckEnv {
            scope: self.identifiers,
            current_module: Arc::clone(&self.parent.current_module),
            warnings: Arc::clone(&self.parent.warnings),
            parent: Some(self.parent),
        })
    }
}
