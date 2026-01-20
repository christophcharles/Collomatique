//! Module processing for the ColloML DSL.
//!
//! This module implements the multi-pass compilation of ColloML modules,
//! including type declarations, enum declarations, function signatures,
//! reify statements, and function body validation.

use super::errors::{ArgsType, FunctionType, GlobalEnvError, SemError, SemWarning};
use super::global_env::{GlobalEnv, ObjectFields, Symbol, SymbolPath, TypeDesc, TypeInfo};
use super::local_env::LocalCheckEnv;
use super::path_resolution::{resolve_path, ResolvedPathKind};
use super::string_case;
use super::types::{ExprType, SimpleType};
use crate::ast::{DocstringLine, Expr, Param, Span, Spanned};
use std::collections::{BTreeMap, HashMap};

impl GlobalEnv {
    /// Create a GlobalEnv from modules
    pub fn new(
        object_types: HashMap<String, ObjectFields>,
        variables: HashMap<String, ArgsType>,
        modules: &BTreeMap<&str, crate::ast::File>,
    ) -> Result<
        (
            Self,
            TypeInfo,
            HashMap<Span, ExprType>,
            Vec<SemError>,
            Vec<SemWarning>,
        ),
        GlobalEnvError,
    > {
        let mut temp_env = GlobalEnv {
            module_names: modules.keys().map(|name| name.to_string()).collect(),
            object_types,
            custom_types: HashMap::new(),
            functions: HashMap::new(),
            external_variables: variables
                .into_iter()
                .map(|(var_name, args_type)| (var_name, args_type))
                .collect(),
            internal_variables: HashMap::new(),
            variable_lists: HashMap::new(),
            symbols: HashMap::new(),
        };

        for (object_type, field_desc) in &temp_env.object_types {
            for (field, typ) in field_desc {
                if !temp_env.validate_type(typ) {
                    return Err(GlobalEnvError::UnknownTypeInField {
                        object_type: object_type.clone(),
                        field: field.clone(),
                        unknown_type: typ.to_string(),
                    });
                }
            }
        }

        for (var, args) in &temp_env.external_variables {
            for (param, typ) in args.iter().enumerate() {
                if !temp_env.validate_type(typ) {
                    return Err(GlobalEnvError::UnknownTypeForVariableArg {
                        var: var.clone(),
                        param,
                        unknown_type: typ.to_string(),
                    });
                }
            }
        }

        let mut type_info = TypeInfo::new();
        let mut expr_types = HashMap::new();
        let mut errors = vec![];
        let mut warnings = vec![];

        // ====================================================================
        // PASS 1a: Register all type names with placeholders
        // This allows forward references between types
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            for statement in &module_file.statements {
                match &statement.node {
                    crate::ast::Statement::TypeDecl { public, name, .. } => {
                        temp_env.expand_with_type_decl_pass1(
                            current_module,
                            *public,
                            name,
                            &mut errors,
                        );
                    }
                    crate::ast::Statement::EnumDecl {
                        public,
                        name,
                        variants,
                    } => {
                        temp_env.expand_with_enum_decl_pass1(
                            current_module,
                            *public,
                            name,
                            variants,
                            &mut errors,
                        );
                    }
                    _ => {}
                }
            }
        }

        // ====================================================================
        // PASS 1b: Populate type symbols for current module + imports
        // (Moved before type resolution so resolve_path can be used)
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            temp_env.import_type_symbols(current_module, current_module, None, None, &mut errors);

            // Import type symbols from foreign modules
            for statement in &module_file.statements {
                if let crate::ast::Statement::Import { module_path, alias } = &statement.node {
                    if !temp_env.module_exists(&module_path.node) {
                        errors.push(SemError::UnknownModule {
                            module: module_path.node.clone(),
                            span: module_path.span.clone(),
                        });
                        continue;
                    }
                    if module_path.node == current_module {
                        errors.push(SemError::SelfImport {
                            span: module_path.span.clone(),
                        });
                        continue;
                    }
                    let prefix = match alias {
                        crate::ast::ImportAlias::Named(name) => Some(name.node.as_str()),
                        crate::ast::ImportAlias::Wildcard(_) => None,
                    };
                    let import_span = match alias {
                        crate::ast::ImportAlias::Named(name) => &name.span,
                        crate::ast::ImportAlias::Wildcard(span) => span,
                    };
                    temp_env.import_type_symbols(
                        current_module,
                        &module_path.node,
                        prefix,
                        Some(import_span),
                        &mut errors,
                    );
                }
            }
        }

        // ====================================================================
        // PASS 1c: Resolve all type definitions
        // Now symbols are populated, we can use resolve_path for types
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            for statement in &module_file.statements {
                match &statement.node {
                    crate::ast::Statement::TypeDecl {
                        name, underlying, ..
                    } => {
                        temp_env.expand_with_type_decl_pass2(
                            current_module,
                            name,
                            underlying,
                            &mut errors,
                        );
                    }
                    crate::ast::Statement::EnumDecl { name, variants, .. } => {
                        temp_env.expand_with_enum_decl_pass2(
                            current_module,
                            name,
                            variants,
                            &mut errors,
                        );
                    }
                    _ => {}
                }
            }
        }

        // ====================================================================
        // PASS 2a: Register all function signatures
        // Now all types are resolved, we can build function signatures
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            for statement in &module_file.statements {
                if let crate::ast::Statement::Let {
                    public,
                    name,
                    params,
                    output_type,
                    body,
                    docstring,
                } = &statement.node
                {
                    temp_env.expand_with_let_statement_pass1(
                        current_module,
                        *public,
                        name,
                        params,
                        output_type,
                        body,
                        docstring,
                        &mut type_info,
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }

        // ====================================================================
        // PASS 2a+: Populate function symbols for current module + imports
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            temp_env.import_function_symbols(
                current_module,
                current_module,
                None,
                None,
                &mut errors,
            );

            // Import function symbols from foreign modules (skip validation, done in PASS 1b)
            for statement in &module_file.statements {
                if let crate::ast::Statement::Import { module_path, alias } = &statement.node {
                    if temp_env.module_exists(&module_path.node)
                        && module_path.node != current_module
                    {
                        let prefix = match alias {
                            crate::ast::ImportAlias::Named(name) => Some(name.node.as_str()),
                            crate::ast::ImportAlias::Wildcard(_) => None,
                        };
                        let import_span = match alias {
                            crate::ast::ImportAlias::Named(name) => &name.span,
                            crate::ast::ImportAlias::Wildcard(span) => span,
                        };
                        temp_env.import_function_symbols(
                            current_module,
                            &module_path.node,
                            prefix,
                            Some(import_span),
                            &mut errors,
                        );
                    }
                }
            }
        }

        // ====================================================================
        // PASS 2b: Process reify statements
        // Now all function signatures are known, reify can verify them
        // Variables created here can be used in function bodies
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            for statement in &module_file.statements {
                if let crate::ast::Statement::Reify {
                    constraint_path,
                    name,
                    var_list,
                    public,
                    ..
                } = &statement.node
                {
                    temp_env.expand_with_reify_statement(
                        current_module,
                        constraint_path,
                        name,
                        *var_list,
                        *public,
                        &mut type_info,
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }

        // ====================================================================
        // PASS 2b+: Populate variable symbols for current module + imports
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            temp_env.import_variable_symbols(
                current_module,
                current_module,
                None,
                None,
                &mut errors,
            );

            // Import variable symbols from foreign modules (skip validation, done in PASS 1b)
            for statement in &module_file.statements {
                if let crate::ast::Statement::Import { module_path, alias } = &statement.node {
                    if temp_env.module_exists(&module_path.node)
                        && module_path.node != current_module
                    {
                        let prefix = match alias {
                            crate::ast::ImportAlias::Named(name) => Some(name.node.as_str()),
                            crate::ast::ImportAlias::Wildcard(_) => None,
                        };
                        let import_span = match alias {
                            crate::ast::ImportAlias::Named(name) => &name.span,
                            crate::ast::ImportAlias::Wildcard(span) => span,
                        };
                        temp_env.import_variable_symbols(
                            current_module,
                            &module_path.node,
                            prefix,
                            Some(import_span),
                            &mut errors,
                        );
                    }
                }
            }
        }

        // ====================================================================
        // PASS 2c: Validate all function bodies
        // Now all function signatures AND variables are known
        // ====================================================================
        for (module_name, module_file) in modules {
            let current_module = *module_name;
            for statement in &module_file.statements {
                if let crate::ast::Statement::Let { name, body, .. } = &statement.node {
                    temp_env.expand_with_let_statement_pass2(
                        current_module,
                        name,
                        body,
                        &mut type_info,
                        &mut expr_types,
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }

        temp_env.check_unused_fn(&mut warnings);
        temp_env.check_unused_var(&mut warnings);

        Ok((temp_env, type_info, expr_types, errors, warnings))
    }

    fn check_unused_fn(&self, warnings: &mut Vec<SemWarning>) {
        for ((module, fn_name), fn_desc) in &self.functions {
            if !fn_desc.public && !fn_desc.used {
                warnings.push(SemWarning::UnusedFunction {
                    module: module.clone(),
                    identifier: format!("{}::{}", module, fn_name),
                    span: fn_desc.body.span.clone(),
                });
            }
        }
    }

    fn check_unused_var(&self, warnings: &mut Vec<SemWarning>) {
        for ((module, var_name), var_desc) in &self.internal_variables {
            if !var_desc.public && !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    module: module.clone(),
                    identifier: format!("{}::{}", module, var_name),
                    span: var_desc.span.clone(),
                });
            }
        }

        for ((module, var_name), var_desc) in &self.variable_lists {
            if !var_desc.public && !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    module: module.clone(),
                    identifier: format!("{}::{}", module, var_name),
                    span: var_desc.span.clone(),
                });
            }
        }
    }

    // ========================================================================
    // Symbol Table Helpers
    // ========================================================================

    /// Build a SymbolPath from optional prefix and name
    /// For enum variants like "Result::Ok", splits into ["prefix?", "Result", "Ok"]
    fn make_symbol_path(prefix: Option<&str>, name: &str) -> SymbolPath {
        let mut segments: Vec<String> = prefix.map(|p| vec![p.to_string()]).unwrap_or_default();
        if name.contains("::") {
            segments.extend(name.split("::").map(|s| s.to_string()));
        } else {
            segments.push(name.to_string());
        }
        SymbolPath(segments)
    }

    /// Import type symbols from source_module into target_module's symbol table
    fn import_type_symbols(
        &mut self,
        target_module: &str,
        source_module: &str,
        prefix: Option<&str>,
        import_span: Option<&Span>,
        errors: &mut Vec<SemError>,
    ) {
        // Collect conflicts and types to add (to avoid borrow conflict)
        let mut conflicts: Vec<(String, String)> = Vec::new(); // (path_str, existing_module)

        {
            let symbol_map = self.symbols.entry(target_module.to_string()).or_default();

            // If prefix given, register the module symbol first
            if let Some(p) = prefix {
                let module_path = SymbolPath(vec![p.to_string()]);
                if let Some(existing) = symbol_map.get(&module_path) {
                    conflicts.push((p.to_string(), existing.module_name().to_string()));
                } else {
                    symbol_map.insert(module_path, Symbol::Module(source_module.to_string()));
                }
            }
        }

        // Collect types to add
        // Skip private types when importing from another module
        let types_to_add: Vec<_> = self
            .custom_types
            .iter()
            .filter(|((mod_name, _), type_desc)| {
                mod_name == source_module && (import_span.is_none() || type_desc.public)
            })
            .map(|((mod_name, type_name), _)| (mod_name.clone(), type_name.clone()))
            .collect();

        let symbol_map = self.symbols.entry(target_module.to_string()).or_default();
        for (mod_name, type_name) in types_to_add {
            let path = Self::make_symbol_path(prefix, &type_name);
            if let Some(existing) = symbol_map.get(&path) {
                conflicts.push((path.0.join("::"), existing.module_name().to_string()));
            } else {
                symbol_map.insert(path, Symbol::CustomType(mod_name, type_name));
            }
        }

        // Report conflicts
        if let Some(span) = import_span {
            for (path_str, existing_module) in conflicts {
                errors.push(SemError::SymbolConflict {
                    path: path_str,
                    span: span.clone(),
                    existing_module,
                });
            }
        }
    }

    /// Import function symbols from source_module into target_module's symbol table
    fn import_function_symbols(
        &mut self,
        target_module: &str,
        source_module: &str,
        prefix: Option<&str>,
        import_span: Option<&Span>,
        errors: &mut Vec<SemError>,
    ) {
        let mut conflicts: Vec<(String, String)> = Vec::new();

        // Collect functions to add (to avoid borrow conflict)
        // Skip private functions when importing from another module
        let fns_to_add: Vec<_> = self
            .functions
            .iter()
            .filter(|((mod_name, _), func_desc)| {
                mod_name == source_module && (import_span.is_none() || func_desc.public)
            })
            .map(|((mod_name, fn_name), _)| (mod_name.clone(), fn_name.clone()))
            .collect();

        let symbol_map = self.symbols.entry(target_module.to_string()).or_default();
        for (mod_name, fn_name) in fns_to_add {
            let path = Self::make_symbol_path(prefix, &fn_name);
            if let Some(existing) = symbol_map.get(&path) {
                conflicts.push((path.0.join("::"), existing.module_name().to_string()));
            } else {
                symbol_map.insert(path, Symbol::Function(mod_name, fn_name));
            }
        }

        // Report conflicts
        if let Some(span) = import_span {
            for (path_str, existing_module) in conflicts {
                errors.push(SemError::SymbolConflict {
                    path: path_str,
                    span: span.clone(),
                    existing_module,
                });
            }
        }
    }

    /// Import variable symbols from source_module into target_module's symbol table
    fn import_variable_symbols(
        &mut self,
        target_module: &str,
        source_module: &str,
        prefix: Option<&str>,
        import_span: Option<&Span>,
        errors: &mut Vec<SemError>,
    ) {
        let mut conflicts: Vec<(String, String)> = Vec::new();

        // Collect internal variables to add
        // Skip private variables when importing from another module
        let vars_to_add: Vec<_> = self
            .internal_variables
            .iter()
            .filter(|((mod_name, _), var_desc)| {
                mod_name == source_module && (import_span.is_none() || var_desc.public)
            })
            .map(|((mod_name, var_name), _)| (mod_name.clone(), var_name.clone()))
            .collect();

        let symbol_map = self.symbols.entry(target_module.to_string()).or_default();
        for (mod_name, var_name) in vars_to_add {
            let dollar_var_name = format!("${}", var_name);
            let path = Self::make_symbol_path(prefix, &dollar_var_name);
            if let Some(existing) = symbol_map.get(&path) {
                conflicts.push((path.0.join("::"), existing.module_name().to_string()));
            } else {
                symbol_map.insert(path, Symbol::Variable(mod_name, var_name));
            }
        }

        // Collect variable lists to add
        // Skip private variable lists when importing from another module
        let var_lists_to_add: Vec<_> = self
            .variable_lists
            .iter()
            .filter(|((mod_name, _), var_desc)| {
                mod_name == source_module && (import_span.is_none() || var_desc.public)
            })
            .map(|((mod_name, var_name), _)| (mod_name.clone(), var_name.clone()))
            .collect();

        let symbol_map = self.symbols.entry(target_module.to_string()).or_default();
        for (mod_name, var_name) in var_lists_to_add {
            let dollar_var_name = format!("$[{}]", var_name);
            let path = Self::make_symbol_path(prefix, &dollar_var_name);
            if let Some(existing) = symbol_map.get(&path) {
                conflicts.push((path.0.join("::"), existing.module_name().to_string()));
            } else {
                symbol_map.insert(path, Symbol::VariableList(mod_name, var_name));
            }
        }

        // Report conflicts
        if let Some(span) = import_span {
            for (path_str, existing_module) in conflicts {
                errors.push(SemError::SymbolConflict {
                    path: path_str,
                    span: span.clone(),
                    existing_module,
                });
            }
        }
    }

    /// Pass 1: Register function signature (for forward references)
    /// Does NOT validate the function body - that happens in pass 2
    fn expand_with_let_statement_pass1(
        &mut self,
        current_module: &str,
        public: bool,
        name: &Spanned<String>,
        params: &Vec<Param>,
        output_type: &Spanned<crate::ast::TypeName>,
        body: &Spanned<Expr>,
        docstring: &Vec<DocstringLine>,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        // Check for duplicate function name
        if let Some((_fn_type, span)) = self.lookup_fn(current_module, &name.node) {
            errors.push(SemError::FunctionAlreadyDefined {
                module: current_module.to_string(),
                identifier: name.node.clone(),
                span: name.span.clone(),
                here: span.clone(),
            });
            return;
        }

        // Naming convention warning for function
        if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
            &name.node,
            string_case::NamingConvention::SnakeCase,
        ) {
            warnings.push(SemWarning::FunctionNamingConvention {
                module: current_module.to_string(),
                identifier: name.node.clone(),
                span: name.span.clone(),
                suggestion,
            });
        }

        // Resolve and validate parameter types
        let mut error_in_typs = false;
        let mut params_typ = vec![];
        let mut seen_param_names: HashMap<String, Span> = HashMap::new();

        for param in params {
            match self.resolve_type(&param.typ, current_module) {
                Err(e) => {
                    errors.push(e);
                    error_in_typs = true;
                }
                Ok(param_typ) => {
                    params_typ.push(param_typ.clone());
                    if !self.validate_type(&param_typ) {
                        errors.push(SemError::UnknownType {
                            module: current_module.to_string(),
                            typ: param_typ.to_string(),
                            span: param.typ.span.clone(),
                        });
                        error_in_typs = true;
                    }
                }
            }

            // Check for duplicate parameter names
            if let Some(prev_span) = seen_param_names.get(&param.name.node) {
                errors.push(SemError::ParameterAlreadyDefined {
                    module: current_module.to_string(),
                    identifier: param.name.node.clone(),
                    span: param.name.span.clone(),
                    here: prev_span.clone(),
                });
            } else {
                seen_param_names.insert(param.name.node.clone(), param.name.span.clone());

                // Naming convention warning for parameter
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &param.name.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        module: current_module.to_string(),
                        identifier: param.name.node.clone(),
                        span: param.name.span.clone(),
                        suggestion,
                    });
                }
            }
        }

        // Resolve and validate output type
        let out_typ = match self.resolve_type(output_type, current_module) {
            Err(e) => {
                errors.push(e);
                return;
            }
            Ok(typ) => {
                if !self.validate_type(&typ) {
                    errors.push(SemError::UnknownType {
                        module: current_module.to_string(),
                        typ: typ.to_string(),
                        span: output_type.span.clone(),
                    });
                    return;
                }
                typ
            }
        };

        // Register the function (body will be validated in pass 2)
        if !error_in_typs {
            let fn_typ = FunctionType {
                args: params_typ,
                output: out_typ,
            };
            self.register_fn(
                current_module,
                &name.node,
                name.span.clone(),
                fn_typ,
                public,
                params.iter().map(|x| x.name.node.clone()).collect(),
                body.clone(),
                docstring.clone(),
                type_info,
            );
        }
    }

    /// Check docstring expressions for semantic validity
    fn check_docstring_expressions(
        &mut self,
        docstring: &Vec<DocstringLine>,
        local_env: &mut LocalCheckEnv,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        for line in docstring {
            for part in line {
                if let Some(expr) = &part.expr {
                    // Check the expression - it's already wrapped in String(...)
                    // so it should type-check if the inner expression can convert to String
                    local_env.check_expr(
                        self, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                    );
                }
            }
        }
    }

    /// Pass 2: Validate function body (now all function signatures are known)
    fn expand_with_let_statement_pass2(
        &mut self,
        current_module: &str,
        name: &Spanned<String>,
        body: &Spanned<Expr>,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        // Skip if function wasn't registered in pass 1
        let fn_key = (current_module.to_string(), name.node.clone());
        let fn_desc = match self.functions.get(&fn_key) {
            Some(desc) => desc.clone(),
            None => return,
        };

        // Build LocalCheckEnv with parameters
        let mut local_env = LocalCheckEnv::new(current_module);
        for (param_name, param_typ) in fn_desc.arg_names.iter().zip(fn_desc.typ.args.iter()) {
            // We don't need to check for duplicate params here - already done in pass 1
            // Just register them in the local environment
            local_env.register_identifier_no_check(param_name, param_typ.clone());
        }

        // Validate body and docstring expressions (parameters available in same scope)
        local_env.push_scope();

        // Check docstring expressions first
        self.check_docstring_expressions(
            &fn_desc.docstring,
            &mut local_env,
            type_info,
            expr_types,
            errors,
            warnings,
        );

        // Then validate body
        let body_type_opt = local_env.check_expr(
            self, &body.node, &body.span, type_info, expr_types, errors, warnings,
        );
        local_env.pop_scope(warnings);

        // Check body type matches declared return type
        if let Some(body_type) = body_type_opt {
            let types_match = body_type.is_subtype_of(&fn_desc.typ.output);
            if !types_match {
                errors.push(SemError::BodyTypeMismatch {
                    func: name.node.clone(),
                    span: body.span.clone(),
                    expected: fn_desc.typ.output.clone(),
                    found: body_type,
                });
            }
        }
    }

    fn expand_with_reify_statement(
        &mut self,
        current_module: &str,
        constraint_path: &Spanned<crate::ast::NamespacePath>,
        name: &Spanned<String>,
        var_list: bool,
        public: bool,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        // Resolve the constraint path to find the function
        let resolved = resolve_path(constraint_path, current_module, self, None);

        let (fn_module, fn_name) = match resolved {
            Ok(ResolvedPathKind::Function { module, func }) => (module, func),
            Ok(_) => {
                // Path resolved to something that's not a function
                let path_str = constraint_path
                    .node
                    .segments
                    .iter()
                    .map(|s| s.node.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                errors.push(SemError::UnknownIdentifer {
                    module: current_module.to_string(),
                    identifier: path_str,
                    span: constraint_path.span.clone(),
                });
                return;
            }
            Err(e) => {
                errors.push(e.into_sem_error(current_module));
                return;
            }
        };

        match self.lookup_fn(&fn_module, &fn_name) {
            None => {
                // This shouldn't happen if resolve_path returned Function, but handle it
                let path_str = constraint_path
                    .node
                    .segments
                    .iter()
                    .map(|s| s.node.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                errors.push(SemError::UnknownIdentifer {
                    module: current_module.to_string(),
                    identifier: path_str,
                    span: constraint_path.span.clone(),
                });
            }
            Some(fn_type) => {
                // Mark function as used
                self.mark_fn_used(&fn_module, &fn_name);

                let needed_output_type = ExprType::simple(if var_list {
                    SimpleType::List(SimpleType::Constraint.into())
                } else {
                    SimpleType::Constraint
                });
                let correct_type = fn_type.0.output == needed_output_type;
                if !correct_type {
                    let expected_type = FunctionType {
                        output: needed_output_type,
                        ..fn_type.0.clone()
                    };
                    let path_str = constraint_path
                        .node
                        .segments
                        .iter()
                        .map(|s| s.node.as_str())
                        .collect::<Vec<_>>()
                        .join("::");
                    errors.push(SemError::FunctionTypeMismatch {
                        module: current_module.to_string(),
                        identifier: path_str,
                        span: constraint_path.span.clone(),
                        expected: expected_type,
                        found: fn_type.0,
                    });
                    return;
                }

                if var_list {
                    match self.lookup_var_list(current_module, &name.node) {
                        Some((_args, span)) => errors.push(SemError::VariableAlreadyDefined {
                            module: current_module.to_string(),
                            identifier: name.node.clone(),
                            span: name.span.clone(),
                            here: Some(span),
                        }),
                        None => {
                            if let Some(suggestion) =
                                string_case::generate_suggestion_for_naming_convention(
                                    &name.node,
                                    string_case::NamingConvention::PascalCase,
                                )
                            {
                                warnings.push(SemWarning::VariableNamingConvention {
                                    module: current_module.to_string(),
                                    identifier: name.node.clone(),
                                    span: name.span.clone(),
                                    suggestion,
                                });
                            }
                            self.register_var_list(
                                current_module,
                                &name.node,
                                fn_type.0.args.clone(),
                                name.span.clone(),
                                public,
                                (fn_module.clone(), fn_name.clone()),
                                type_info,
                            );
                        }
                    }
                } else {
                    match self.lookup_var(current_module, &name.node) {
                        Some((_args, span_opt)) => errors.push(SemError::VariableAlreadyDefined {
                            module: current_module.to_string(),
                            identifier: name.node.clone(),
                            span: name.span.clone(),
                            here: span_opt,
                        }),
                        None => {
                            if let Some(suggestion) =
                                string_case::generate_suggestion_for_naming_convention(
                                    &name.node,
                                    string_case::NamingConvention::PascalCase,
                                )
                            {
                                warnings.push(SemWarning::VariableNamingConvention {
                                    module: current_module.to_string(),
                                    identifier: name.node.clone(),
                                    span: name.span.clone(),
                                    suggestion,
                                });
                            }
                            self.register_var(
                                current_module,
                                &name.node,
                                fn_type.0.args.clone(),
                                name.span.clone(),
                                public,
                                (fn_module.clone(), fn_name.clone()),
                                type_info,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Pass 1: Register type name with placeholder (for forward references)
    fn expand_with_type_decl_pass1(
        &mut self,
        current_module: &str,
        public: bool,
        name: &Spanned<String>,
        errors: &mut Vec<SemError>,
    ) {
        // Check if type name shadows a primitive type
        if Self::is_primitive_type_name(&name.node) {
            errors.push(SemError::TypeShadowsPrimitive {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Check if type name shadows an object type
        if self.object_types.contains_key(&name.node) {
            errors.push(SemError::TypeShadowsObject {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Check if type name shadows a previous custom type (duplicate in same file)
        let type_key = (current_module.to_string(), name.node.clone());
        if self.custom_types.contains_key(&type_key) {
            errors.push(SemError::TypeShadowsCustomType {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Register with placeholder - will be resolved in pass 2
        self.custom_types.insert(
            type_key,
            TypeDesc {
                underlying: ExprType::simple(SimpleType::Never),
                public,
            },
        );
    }

    /// Pass 2: Resolve underlying type and check for unguarded recursion
    fn expand_with_type_decl_pass2(
        &mut self,
        current_module: &str,
        name: &Spanned<String>,
        underlying: &Spanned<crate::ast::TypeName>,
        errors: &mut Vec<SemError>,
    ) {
        // Skip if pass 1 failed (type wasn't registered)
        let type_key = (current_module.to_string(), name.node.clone());
        let public = match self.custom_types.get(&type_key) {
            Some(desc) => desc.public,
            None => return,
        };

        // Resolve the underlying type using the symbol table
        let underlying_type = match ExprType::from_ast(underlying.clone(), current_module, self) {
            Ok(typ) => typ,
            Err(e) => {
                errors.push(e);
                return;
            }
        };

        // Check for unguarded recursive type (type references itself without being inside a container)
        if self.has_unguarded_reference(&underlying_type, &name.node) {
            errors.push(SemError::UnguardedRecursiveType {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Update the placeholder with the actual type
        self.custom_types.insert(
            type_key,
            TypeDesc {
                underlying: underlying_type,
                public,
            },
        );
    }

    /// Pass 1 for enum declarations: Register the enum name and all variant names with placeholders
    fn expand_with_enum_decl_pass1(
        &mut self,
        current_module: &str,
        public: bool,
        name: &Spanned<String>,
        variants: &[Spanned<crate::ast::EnumVariant>],
        errors: &mut Vec<SemError>,
    ) {
        // Check if enum name shadows a primitive type
        if Self::is_primitive_type_name(&name.node) {
            errors.push(SemError::TypeShadowsPrimitive {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Check for shadowing existing object or custom types
        if self.object_types.contains_key(&name.node) {
            errors.push(SemError::TypeShadowsObject {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        let type_key = (current_module.to_string(), name.node.clone());
        if self.custom_types.contains_key(&type_key) {
            errors.push(SemError::TypeShadowsCustomType {
                module: current_module.to_string(),
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        let mut variant_failure = false;

        for variant in variants {
            let qualified_name = format!("{}::{}", name.node, variant.node.name.node);
            let variant_key = (current_module.to_string(), qualified_name.clone());

            // Note: Primitive type names ARE allowed as variant names since the qualified name
            // (e.g., "MyType::Int", "Option::None") is distinct from the primitive type.
            // Only the root enum name must not shadow primitives.

            if self.custom_types.contains_key(&variant_key) {
                errors.push(SemError::TypeShadowsCustomType {
                    module: current_module.to_string(),
                    type_name: qualified_name.clone(),
                    span: variant.node.name.span.clone(),
                });
                variant_failure = true;
                continue;
            }
        }

        // If any variant has failed, do not register type
        if variant_failure {
            return;
        }

        // Register the root enum type with a placeholder
        self.custom_types.insert(
            type_key,
            TypeDesc {
                underlying: ExprType::simple(SimpleType::Never),
                public,
            },
        );

        // Register all variant types with placeholders (variants inherit parent visibility)
        for variant in variants {
            let qualified_name = format!("{}::{}", name.node, variant.node.name.node);
            let variant_key = (current_module.to_string(), qualified_name.clone());

            self.custom_types.insert(
                variant_key,
                TypeDesc {
                    underlying: ExprType::simple(SimpleType::Never),
                    public,
                },
            );
        }
    }

    /// Pass 2 for enum declarations: Resolve underlying types for all variants
    /// and build the root enum type as a union of all variants
    fn expand_with_enum_decl_pass2(
        &mut self,
        current_module: &str,
        name: &Spanned<String>,
        variants: &[Spanned<crate::ast::EnumVariant>],
        errors: &mut Vec<SemError>,
    ) {
        use crate::ast::EnumVariantType;

        // Skip if pass 1 failed
        let type_key = (current_module.to_string(), name.node.clone());
        let public = match self.custom_types.get(&type_key) {
            Some(desc) => desc.public,
            None => return,
        };

        // Process each variant and collect their SimpleTypes for the root enum
        let mut variant_simple_types = Vec::new();

        for variant in variants {
            let qualified_name = format!("{}::{}", name.node, variant.node.name.node);
            let variant_key = (current_module.to_string(), qualified_name.clone());

            // Skip if this variant wasn't registered in pass 1
            if !self.custom_types.contains_key(&variant_key) {
                continue;
            }

            // Determine the underlying type for this variant
            let underlying_type = match &variant.node.underlying {
                None => {
                    // Unit variant - underlying type is None
                    ExprType::simple(SimpleType::None)
                }
                Some(variant_type) => match &variant_type.node {
                    EnumVariantType::Tuple(types) if types.is_empty() => {
                        // Empty parens () - also a unit variant
                        ExprType::simple(SimpleType::None)
                    }
                    EnumVariantType::Tuple(types) if types.len() == 1 => {
                        // Single type like Ok(Int) - underlying is just that type
                        match ExprType::from_ast(types[0].clone(), current_module, self) {
                            Ok(typ) => typ,
                            Err(e) => {
                                errors.push(e);
                                continue;
                            }
                        }
                    }
                    EnumVariantType::Tuple(types) => {
                        // Multiple types like TupleCase(Int, Bool) - underlying is a tuple
                        let tuple_types: Result<Vec<ExprType>, _> = types
                            .iter()
                            .map(|t| ExprType::from_ast(t.clone(), current_module, self))
                            .collect();
                        match tuple_types {
                            Ok(ts) => ExprType::simple(SimpleType::Tuple(ts)),
                            Err(e) => {
                                errors.push(e);
                                continue;
                            }
                        }
                    }
                    EnumVariantType::Struct(fields) => {
                        // Struct variant like StructCase { field: Type }
                        let struct_fields: Result<std::collections::BTreeMap<String, ExprType>, _> =
                            fields
                                .iter()
                                .map(|(fname, ftype)| {
                                    ExprType::from_ast(ftype.clone(), current_module, self)
                                        .map(|t| (fname.node.clone(), t))
                                })
                                .collect();
                        match struct_fields {
                            Ok(fs) => ExprType::simple(SimpleType::Struct(fs)),
                            Err(e) => {
                                errors.push(e);
                                continue;
                            }
                        }
                    }
                },
            };

            // Check for unguarded recursion (though enums should be guarded by the enum wrapper)
            if self.has_unguarded_reference(&underlying_type, &qualified_name) {
                errors.push(SemError::UnguardedRecursiveType {
                    module: current_module.to_string(),
                    type_name: qualified_name.clone(),
                    span: variant.node.name.span.clone(),
                });
                continue;
            }

            // Update the variant's underlying type
            self.custom_types.insert(
                variant_key,
                TypeDesc {
                    underlying: underlying_type,
                    public,
                },
            );

            // Add this variant to the list for the root enum type
            variant_simple_types.push(SimpleType::Custom(
                current_module.to_string(),
                name.node.clone(),
                Some(variant.node.name.node.clone()),
            ));
        }

        // Build the root enum type as a union of all variants
        if !variant_simple_types.is_empty() {
            let enum_type = ExprType::from_variants(variant_simple_types);
            self.custom_types.insert(
                type_key,
                TypeDesc {
                    underlying: enum_type,
                    public,
                },
            );
        }
    }

    fn is_primitive_type_name(name: &str) -> bool {
        matches!(
            name,
            "Int" | "Bool" | "String" | "None" | "LinExpr" | "Constraint" | "Never"
        )
    }

    /// Check if type_name appears as an unguarded variant in typ.
    /// Unguarded = direct union variant, not inside List/Tuple.
    /// This detects invalid recursive types like `type A = Int | A;`
    /// but allows guarded recursion like `type A = Int | [A];`
    fn has_unguarded_reference(&self, typ: &ExprType, type_name: &str) -> bool {
        for variant in typ.get_variants() {
            if self.simple_type_has_unguarded_reference(variant, type_name) {
                return true;
            }
        }
        false
    }

    fn simple_type_has_unguarded_reference(&self, typ: &SimpleType, type_name: &str) -> bool {
        match typ {
            SimpleType::Custom(module, root, variant) => {
                let key = match variant {
                    None => root.clone(),
                    Some(v) => format!("{}::{}", root, v),
                };
                if key == type_name {
                    // Direct reference to the type we're defining - unguarded!
                    return true;
                }
                // Check if this custom type transitively has unguarded reference
                // But only if it's already resolved (not a placeholder)
                let type_key = (module.clone(), key);
                if let Some(type_desc) = self.custom_types.get(&type_key) {
                    // Skip if it's still a placeholder (Never type used during pass 1)
                    let placeholder = ExprType::simple(SimpleType::Never);
                    if type_desc.underlying == placeholder {
                        false
                    } else {
                        self.has_unguarded_reference(&type_desc.underlying, type_name)
                    }
                } else {
                    false
                }
            }
            SimpleType::List(_) | SimpleType::Tuple(_) => {
                // Guarded - recursion inside containers is allowed
                // Don't recurse into containers
                false
            }
            _ => false,
        }
    }
}
