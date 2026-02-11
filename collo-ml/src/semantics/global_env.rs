use derivative::Derivative;

use super::errors::{ArgsType, FunctionType, SemError};
use super::types::{ExprType, SimpleType};
use crate::ast::{DocstringLine, Span, Spanned};
use crate::database::DatabaseDriver;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type ObjectFields = HashMap<String, ExprType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDesc {
    pub name_span: Span,
    pub typ: FunctionType,
    pub public: bool,
    pub used: bool,
    pub arg_names: Vec<String>,
    pub body: Arc<Spanned<crate::ast::Expr>>,
    pub docstring: Vec<DocstringLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDesc {
    pub name_span: Span,
    pub typ: FunctionType, // Reuse - same signature structure
    pub public: bool,
    pub used: bool,
    pub arg_names: Vec<String>,
    pub query_string: Spanned<String>,
    pub docstring: Vec<DocstringLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDesc {
    pub args: ArgsType,
    pub span: Span,
    pub used: bool,
    pub public: bool,
    pub referenced_fn: (String, String), // (module_name, fn_name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDesc {
    pub underlying: ExprType,
    pub public: bool,
}

/// Simplified path for symbol table keys (without spans)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolPath(pub Vec<String>);

/// Map from local path to symbol definition
pub type SymbolMap = HashMap<SymbolPath, Symbol>;

/// A symbol in the symbol table, pointing to its definition location
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    Module(String),               // module name
    Function(String, String),     // (module, name)
    Query(String, String),        // (module, name)
    CustomType(String, String),   // (module, name)
    Variable(String, String),     // (module, name)
    VariableList(String, String), // (module, name)
}

impl Symbol {
    /// Returns the module name this symbol comes from
    pub fn module_name(&self) -> &str {
        match self {
            Symbol::Module(m) => m,
            Symbol::Function(m, _) => m,
            Symbol::Query(m, _) => m,
            Symbol::CustomType(m, _) => m,
            Symbol::Variable(m, _) => m,
            Symbol::VariableList(m, _) => m,
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct GlobalEnv<D: DatabaseDriver> {
    pub module_names: Vec<String>,
    pub(crate) object_types: HashMap<String, ObjectFields>, // external, no module
    pub(crate) custom_types: HashMap<(String, String), TypeDesc>, // (module, name) → desc
    pub(crate) functions: HashMap<(String, String), FunctionDesc>, // (module, name) → desc
    pub(crate) queries: HashMap<(String, String), QueryDesc>, // (module, name) → desc
    pub(crate) external_variables: HashMap<String, ArgsType>, // external, no module
    pub(crate) internal_variables: HashMap<(String, String), VariableDesc>, // (module, name) → desc
    pub(crate) variable_lists: HashMap<(String, String), VariableDesc>, // (module, name) → desc
    pub(crate) symbols: HashMap<String, SymbolMap>,         // module → symbol table
    pub(crate) _phantom: std::marker::PhantomData<D>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeInfo {
    pub(crate) types: HashMap<Span, GenericType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericType {
    Function(FunctionType),
    Variable(ArgsType),
    Expr(ExprType),
}

impl From<FunctionType> for GenericType {
    fn from(value: FunctionType) -> Self {
        GenericType::Function(value)
    }
}

impl From<ExprType> for GenericType {
    fn from(value: ExprType) -> Self {
        GenericType::Expr(value)
    }
}

impl From<ArgsType> for GenericType {
    fn from(value: ArgsType) -> Self {
        GenericType::Variable(value)
    }
}

impl std::fmt::Display for GenericType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericType::Function(func) => write!(f, "{}", func),
            GenericType::Expr(typ) => write!(f, "{}", typ),
            GenericType::Variable(var_args) => {
                let args_types: Vec<_> = var_args.iter().map(|x| x.to_string()).collect();
                write!(f, "$({})", args_types.join(", "))
            }
        }
    }
}

impl<D: DatabaseDriver> GlobalEnv<D> {
    pub fn validate_object_type(&self, obj_name: &str) -> bool {
        self.object_types.contains_key(obj_name)
    }

    pub fn validate_simple_type(&self, typ: &SimpleType) -> bool {
        match typ {
            SimpleType::Never => true,
            SimpleType::None => true,
            SimpleType::Bool => true,
            SimpleType::Int => true,
            SimpleType::LinExpr => true,
            SimpleType::Constraint => true,
            SimpleType::String => true,
            SimpleType::EmptyList => true,
            SimpleType::List(sub_typ) => self.validate_type(sub_typ),
            SimpleType::Object(typ_name) => self.validate_object_type(typ_name),
            SimpleType::Custom(module, root, variant) => {
                let name = match variant {
                    None => root.clone(),
                    Some(v) => format!("{}::{}", root, v),
                };
                self.custom_types.contains_key(&(module.clone(), name))
            }
            SimpleType::Tuple(elements) => elements.iter().all(|e| self.validate_type(e)),
            SimpleType::Struct(fields) => fields.values().all(|t| self.validate_type(t)),
            SimpleType::DatabaseSchema(_) => true, // Database schemas are always valid (self-contained)
        }
    }

    pub fn validate_type(&self, typ: &ExprType) -> bool {
        typ.get_variants()
            .iter()
            .all(|x| self.validate_simple_type(x))
    }

    /// Resolve an AST type to an ExprType using resolve_path for symbol table lookup
    pub fn resolve_type(
        &self,
        typ: &Spanned<crate::ast::TypeName>,
        current_module: &str,
    ) -> Result<ExprType, SemError> {
        ExprType::from_ast(typ.clone(), current_module, self, None)
    }

    /// Resolve an AST type to an ExprType, collecting field naming warnings
    pub fn resolve_type_with_warnings(
        &self,
        typ: &Spanned<crate::ast::TypeName>,
        current_module: &str,
        warnings: &mut Vec<super::errors::SemWarning>,
    ) -> Result<ExprType, SemError> {
        ExprType::from_ast(typ.clone(), current_module, self, Some(warnings))
    }

    /// Get the underlying type for a custom type
    pub fn get_custom_type_underlying(&self, module: &str, name: &str) -> Option<&ExprType> {
        self.custom_types
            .get(&(module.to_string(), name.to_string()))
            .map(|desc| &desc.underlying)
    }

    /// Get all custom types
    pub fn get_custom_types(&self) -> &HashMap<(String, String), TypeDesc> {
        &self.custom_types
    }

    /// Get all variant names for an enum (e.g., for "Result", returns ["Ok", "Error"])
    pub fn get_enum_variants(&self, module: &str, enum_name: &str) -> Vec<String> {
        let prefix = format!("{}::", enum_name);
        self.custom_types
            .iter()
            .filter(|((m, _), _)| m == module)
            .filter_map(|((_, k), _)| k.strip_prefix(&prefix).map(|v| v.to_string()))
            .collect()
    }

    /// Expand a type by replacing root enum types with their variant types.
    /// For example, Custom("main", "Result", None) becomes Custom("main", "Result", Some("Ok")) | Custom("main", "Result", Some("Error"))
    pub fn expand_enum_variants(&self, typ: &ExprType) -> ExprType {
        let expanded: Vec<SimpleType> = typ
            .get_variants()
            .iter()
            .flat_map(|v| {
                if let SimpleType::Custom(module, root, None) = v {
                    let variants = self.get_enum_variants(module, root);
                    if variants.is_empty() {
                        // Not an enum or no variants found, keep as-is
                        vec![v.clone()]
                    } else {
                        // Expand to all variants
                        variants
                            .into_iter()
                            .map(|var| SimpleType::Custom(module.clone(), root.clone(), Some(var)))
                            .collect()
                    }
                } else {
                    vec![v.clone()]
                }
            })
            .collect();
        ExprType::sum(expanded).unwrap_or_else(|| typ.clone())
    }

    /// Subtract types with enum-awareness: expands root enum types before subtracting
    pub fn substract_enum_aware(&self, from: &ExprType, to_remove: &ExprType) -> Option<ExprType> {
        let expanded_from = self.expand_enum_variants(from);
        expanded_from.substract(to_remove)
    }

    pub fn get_functions(&self) -> &HashMap<(String, String), FunctionDesc> {
        &self.functions
    }

    pub fn get_predefined_vars(&self) -> &HashMap<String, ArgsType> {
        &self.external_variables
    }

    pub fn get_vars(&self) -> &HashMap<(String, String), VariableDesc> {
        &self.internal_variables
    }

    pub fn get_var_lists(&self) -> &HashMap<(String, String), VariableDesc> {
        &self.variable_lists
    }

    pub fn get_types(&self) -> &HashMap<String, ObjectFields> {
        &self.object_types
    }

    pub(crate) fn lookup_fn(&self, module: &str, name: &str) -> Option<(FunctionType, Span)> {
        let fn_desc = self
            .functions
            .get(&(module.to_string(), name.to_string()))?;
        Some((fn_desc.typ.clone(), fn_desc.body.span.clone()))
    }

    pub(crate) fn mark_fn_used(&mut self, module: &str, name: &str) {
        if let Some(fn_desc) = self
            .functions
            .get_mut(&(module.to_string(), name.to_string()))
        {
            fn_desc.used = true;
        }
    }

    pub(crate) fn register_fn(
        &mut self,
        module: &str,
        name: &str,
        name_span: Span,
        fn_typ: FunctionType,
        public: bool,
        arg_names: Vec<String>,
        body: Spanned<crate::ast::Expr>,
        docstring: Vec<DocstringLine>,
        type_info: &mut TypeInfo,
    ) {
        let key = (module.to_string(), name.to_string());
        assert!(!self.functions.contains_key(&key));

        let body_span = body.span.clone();
        self.functions.insert(
            key,
            FunctionDesc {
                name_span,
                typ: fn_typ.clone(),
                public,
                used: should_be_used_by_default(name),
                arg_names,
                body: Arc::new(body),
                docstring,
            },
        );

        type_info.types.insert(body_span, fn_typ.into());
    }

    pub fn get_queries(&self) -> &HashMap<(String, String), QueryDesc> {
        &self.queries
    }

    pub(crate) fn lookup_query(&self, module: &str, name: &str) -> Option<(FunctionType, Span)> {
        let query_desc = self.queries.get(&(module.to_string(), name.to_string()))?;
        Some((query_desc.typ.clone(), query_desc.query_string.span.clone()))
    }

    pub(crate) fn mark_query_used(&mut self, module: &str, name: &str) {
        if let Some(query_desc) = self
            .queries
            .get_mut(&(module.to_string(), name.to_string()))
        {
            query_desc.used = true;
        }
    }

    pub(crate) fn register_query(
        &mut self,
        module: &str,
        name: &str,
        name_span: Span,
        query_typ: FunctionType,
        public: bool,
        arg_names: Vec<String>,
        query_string: Spanned<String>,
        docstring: Vec<DocstringLine>,
        type_info: &mut TypeInfo,
    ) {
        let key = (module.to_string(), name.to_string());
        assert!(!self.queries.contains_key(&key));

        self.queries.insert(
            key,
            QueryDesc {
                name_span,
                typ: query_typ.clone(),
                public,
                used: should_be_used_by_default(name),
                arg_names,
                query_string: query_string.clone(),
                docstring,
            },
        );

        type_info.types.insert(query_string.span, query_typ.into());
    }

    pub(crate) fn lookup_var(&self, module: &str, name: &str) -> Option<(ArgsType, Option<Span>)> {
        if let Some(ext_var) = self.external_variables.get(name) {
            return Some((ext_var.clone(), None));
        };

        let var_desc = self
            .internal_variables
            .get(&(module.to_string(), name.to_string()))?;

        Some((var_desc.args.clone(), Some(var_desc.span.clone())))
    }

    pub(crate) fn mark_var_used(&mut self, module: &str, name: &str) {
        // External variables don't track usage
        if let Some(var_desc) = self
            .internal_variables
            .get_mut(&(module.to_string(), name.to_string()))
        {
            var_desc.used = true;
        }
    }

    pub(crate) fn register_var(
        &mut self,
        module: &str,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        public: bool,
        referenced_fn: (String, String),
        type_info: &mut TypeInfo,
    ) {
        let key = (module.to_string(), name.to_string());
        assert!(!self.external_variables.contains_key(name));
        assert!(!self.internal_variables.contains_key(&key));

        self.internal_variables.insert(
            key,
            VariableDesc {
                args: args_typ.clone(),
                span: span.clone(),
                used: should_be_used_by_default(name),
                public,
                referenced_fn,
            },
        );

        type_info.types.insert(span, args_typ.into());
    }

    pub(crate) fn lookup_var_list(&self, module: &str, name: &str) -> Option<(ArgsType, Span)> {
        let var_desc = self
            .variable_lists
            .get(&(module.to_string(), name.to_string()))?;

        Some((var_desc.args.clone(), var_desc.span.clone()))
    }

    pub(crate) fn mark_var_list_used(&mut self, module: &str, name: &str) {
        if let Some(var_desc) = self
            .variable_lists
            .get_mut(&(module.to_string(), name.to_string()))
        {
            var_desc.used = true;
        }
    }

    /// Look up a symbol path in the symbol table for a given module
    pub fn lookup_symbol(&self, module: &str, path: &SymbolPath) -> Option<&Symbol> {
        self.symbols.get(module)?.get(path)
    }

    pub(crate) fn register_var_list(
        &mut self,
        module: &str,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        public: bool,
        referenced_fn: (String, String),
        type_info: &mut TypeInfo,
    ) {
        let key = (module.to_string(), name.to_string());
        assert!(!self.variable_lists.contains_key(&key));

        self.variable_lists.insert(
            key,
            VariableDesc {
                args: args_typ.clone(),
                span: span.clone(),
                used: should_be_used_by_default(name),
                public,
                referenced_fn,
            },
        );

        type_info.types.insert(span, args_typ.into());
    }

    pub(crate) fn lookup_field(&self, obj_type: &str, field: &str) -> Option<ExprType> {
        self.object_types.get(obj_type)?.get(field).cloned()
    }

    pub(crate) fn module_exists(&self, module: &str) -> bool {
        self.module_names.contains(&module.to_string())
    }

    /// Get the argument types for an internal variable
    pub(crate) fn get_internal_variable_args(&self, module: &str, name: &str) -> Option<ArgsType> {
        self.internal_variables
            .get(&(module.to_string(), name.to_string()))
            .map(|desc| desc.args.clone())
    }

    /// Get the argument types for an external variable
    pub(crate) fn get_external_variable_args(&self, name: &str) -> Option<ArgsType> {
        self.external_variables.get(name).cloned()
    }

    /// Get the argument types for a variable list
    pub(crate) fn get_variable_list_args(&self, module: &str, name: &str) -> Option<ArgsType> {
        self.variable_lists
            .get(&(module.to_string(), name.to_string()))
            .map(|desc| desc.args.clone())
    }

    /// Breadth-first resolution: peel custom type wrappers one level at a time.
    /// Stops when the ExprType has more than 1 variant (returns all as-is),
    /// or the single variant is not a Custom type (returns it).
    /// Returns None if a cycle is detected.
    pub fn resolve_type_until_several_or_not_custom(
        &self,
        typ: &ExprType,
    ) -> Option<Vec<SimpleType>> {
        let mut visited = HashSet::new();
        self.resolve_type_until_several_or_not_custom_inner(typ, &mut visited)
    }

    fn resolve_type_until_several_or_not_custom_inner(
        &self,
        typ: &ExprType,
        visited: &mut HashSet<(String, String)>,
    ) -> Option<Vec<SimpleType>> {
        let variants = typ.get_variants();
        if variants.len() != 1 {
            return Some(variants.iter().cloned().collect());
        }

        let single = variants.iter().next().unwrap();
        match single {
            SimpleType::Custom(module, root, None) => {
                let key = (module.clone(), root.clone());
                if !visited.insert(key.clone()) {
                    return None; // cycle
                }
                let enum_variants = self.get_enum_variants(module, root);
                let result = if !enum_variants.is_empty() {
                    // Enum root → get underlying (union of variant custom types)
                    if let Some(underlying) = self.get_custom_type_underlying(module, root) {
                        self.resolve_type_until_several_or_not_custom_inner(underlying, visited)
                    } else {
                        Some(vec![single.clone()])
                    }
                } else if let Some(underlying) = self.get_custom_type_underlying(module, root) {
                    // Type alias → unwrap and recurse
                    self.resolve_type_until_several_or_not_custom_inner(underlying, visited)
                } else {
                    Some(vec![single.clone()])
                };
                visited.remove(&key);
                result
            }
            SimpleType::Custom(module, root, Some(variant)) => {
                let qualified = format!("{}::{}", root, variant);
                let key = (module.clone(), qualified.clone());
                if !visited.insert(key.clone()) {
                    return None; // cycle
                }
                let result =
                    if let Some(underlying) = self.get_custom_type_underlying(module, &qualified) {
                        self.resolve_type_until_several_or_not_custom_inner(underlying, visited)
                    } else {
                        Some(vec![single.clone()])
                    };
                visited.remove(&key);
                result
            }
            other => Some(vec![other.clone()]),
        }
    }

    /// Recursively resolve an ExprType by unwrapping Custom type aliases
    /// and expanding enum variants. Returns a Vec<SimpleType> to preserve
    /// duplicate variants (e.g., multiple unit enum variants all resolving
    /// to None). Returns None if a cycle is detected.
    pub fn resolve_type_deep(&self, typ: &ExprType) -> Option<Vec<SimpleType>> {
        let mut visited = HashSet::new();
        self.resolve_type_deep_inner(typ, &mut visited)
    }

    fn resolve_type_deep_inner(
        &self,
        typ: &ExprType,
        visited: &mut HashSet<(String, String)>,
    ) -> Option<Vec<SimpleType>> {
        let mut result = Vec::new();
        for variant in typ.get_variants() {
            let resolved = self.resolve_simple_type_deep(variant, visited)?;
            result.extend(resolved);
        }
        Some(result)
    }

    fn resolve_simple_type_deep(
        &self,
        typ: &SimpleType,
        visited: &mut HashSet<(String, String)>,
    ) -> Option<Vec<SimpleType>> {
        match typ {
            SimpleType::Custom(module, root, None) => {
                let key = (module.clone(), root.clone());
                if !visited.insert(key.clone()) {
                    return None; // cycle
                }
                let variants = self.get_enum_variants(module, root);
                let result = if !variants.is_empty() {
                    // Enum root → expand each variant's underlying
                    let mut all = Vec::new();
                    for var_name in &variants {
                        let qualified = format!("{}::{}", root, var_name);
                        if let Some(underlying) =
                            self.get_custom_type_underlying(module, &qualified)
                        {
                            all.extend(self.resolve_type_deep_inner(underlying, visited)?);
                        }
                    }
                    Some(all)
                } else if let Some(underlying) = self.get_custom_type_underlying(module, root) {
                    // Type alias → unwrap
                    self.resolve_type_deep_inner(underlying, visited)
                } else {
                    Some(vec![typ.clone()])
                };
                visited.remove(&key);
                result
            }
            SimpleType::Custom(module, root, Some(variant)) => {
                let qualified = format!("{}::{}", root, variant);
                let key = (module.clone(), qualified.clone());
                if !visited.insert(key.clone()) {
                    return None; // cycle
                }
                let result =
                    if let Some(underlying) = self.get_custom_type_underlying(module, &qualified) {
                        self.resolve_type_deep_inner(underlying, visited)
                    } else {
                        Some(vec![typ.clone()])
                    };
                visited.remove(&key);
                result
            }
            // Do NOT resolve inside List — checked separately
            other => Some(vec![other.clone()]),
        }
    }
}

impl TypeInfo {
    pub fn new() -> Self {
        TypeInfo::default()
    }
}

fn ident_can_be_unused(ident: &str) -> bool {
    assert!(!ident.is_empty());
    ident.starts_with('_')
}

pub(crate) fn should_be_used_by_default(ident: &str) -> bool {
    ident_can_be_unused(ident)
}

pub(crate) fn ident_can_be_shadowed(ident: &str) -> bool {
    ident_can_be_unused(ident)
}
