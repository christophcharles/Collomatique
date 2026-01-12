use crate::ast::{DocstringLine, Expr, Param, Span, Spanned};
use std::collections::{BTreeSet, HashMap, HashSet};

pub mod string_case;
#[cfg(test)]
mod tests;

mod types;
pub use types::{ConcreteType, ExprType, SimpleType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub args: ArgsType,
    pub output: ExprType,
}

impl std::fmt::Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args_types: Vec<_> = self.args.iter().map(|x| x.to_string()).collect();
        write!(f, "({}) -> {}", args_types.join(", "), self.output)
    }
}

pub type ArgsType = Vec<ExprType>;

pub type ObjectFields = HashMap<String, ExprType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDesc {
    pub name_span: Span,
    pub typ: FunctionType,
    pub public: bool,
    pub used: bool,
    pub arg_names: Vec<String>,
    pub body: Spanned<crate::ast::Expr>,
    pub docstring: Vec<DocstringLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDesc {
    pub args: ArgsType,
    pub span: Span,
    pub used: bool,
    pub public: bool,
    pub referenced_fn: String,
}

/// A module with its name and AST
pub struct Module {
    pub name: String,
    pub file: crate::ast::File,
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
            Symbol::CustomType(m, _) => m,
            Symbol::Variable(m, _) => m,
            Symbol::VariableList(m, _) => m,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEnv {
    pub module_names: Vec<String>,
    object_types: HashMap<String, ObjectFields>, // external, no module
    custom_types: HashMap<(String, String), ExprType>, // (module, name) → type
    functions: HashMap<(String, String), FunctionDesc>, // (module, name) → desc
    external_variables: HashMap<String, ArgsType>, // external, no module
    internal_variables: HashMap<(String, String), VariableDesc>, // (module, name) → desc
    variable_lists: HashMap<(String, String), VariableDesc>, // (module, name) → desc
    symbols: HashMap<String, SymbolMap>,         // module → symbol table
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeInfo {
    types: HashMap<crate::ast::Span, GenericType>,
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

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum GlobalEnvError {
    #[error("Field {field} of object type {object_type} has unknown type {unknown_type}")]
    UnknownTypeInField {
        object_type: String,
        field: String,
        unknown_type: String,
    },
    #[error("Parameter number {param} for ILP variable {var} has unknown type {unknown_type}")]
    UnknownTypeForVariableArg {
        var: String,
        param: usize,
        unknown_type: String,
    },
}

impl GlobalEnv {
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
            SimpleType::Object(typ_name) => self.validate_object_type(&typ_name),
            SimpleType::Custom(module, root, variant) => {
                let name = match variant {
                    None => root.clone(),
                    Some(v) => format!("{}::{}", root, v),
                };
                self.custom_types.contains_key(&(module.clone(), name))
            }
            SimpleType::Tuple(elements) => elements.iter().all(|e| self.validate_type(e)),
            SimpleType::Struct(fields) => fields.values().all(|t| self.validate_type(t)),
        }
    }

    pub fn validate_type(&self, typ: &ExprType) -> bool {
        typ.get_variants()
            .iter()
            .all(|x| self.validate_simple_type(x))
    }

    /// Resolve an AST type to an ExprType using the current context (objects + custom types)
    pub fn resolve_type(
        &self,
        typ: &Spanned<crate::ast::TypeName>,
        current_module: &str,
    ) -> Result<ExprType, SemError> {
        let object_types: std::collections::HashSet<String> =
            self.object_types.keys().cloned().collect();
        let custom_type_names: std::collections::HashSet<(String, String)> =
            self.custom_types.keys().cloned().collect();
        ExprType::from_ast(
            typ.clone(),
            current_module,
            &object_types,
            &custom_type_names,
        )
    }

    /// Get the underlying type for a custom type
    pub fn get_custom_type_underlying(&self, module: &str, name: &str) -> Option<&ExprType> {
        self.custom_types
            .get(&(module.to_string(), name.to_string()))
    }

    /// Get all custom types
    pub fn get_custom_types(&self) -> &HashMap<(String, String), ExprType> {
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
        ExprType::sum(expanded.into_iter()).unwrap_or_else(|| typ.clone())
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

    fn lookup_fn(&self, module: &str, name: &str) -> Option<(FunctionType, Span)> {
        let fn_desc = self
            .functions
            .get(&(module.to_string(), name.to_string()))?;
        Some((fn_desc.typ.clone(), fn_desc.body.span.clone()))
    }

    fn mark_fn_used(&mut self, module: &str, name: &str) {
        if let Some(fn_desc) = self
            .functions
            .get_mut(&(module.to_string(), name.to_string()))
        {
            fn_desc.used = true;
        }
    }

    fn register_fn(
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

        self.functions.insert(
            key,
            FunctionDesc {
                name_span,
                typ: fn_typ.clone(),
                public,
                used: should_be_used_by_default(name),
                arg_names,
                body: body.clone(),
                docstring,
            },
        );

        type_info.types.insert(body.span, fn_typ.into());
    }

    fn lookup_var(&self, module: &str, name: &str) -> Option<(ArgsType, Option<Span>)> {
        if let Some(ext_var) = self.external_variables.get(name) {
            return Some((ext_var.clone(), None));
        };

        let var_desc = self
            .internal_variables
            .get(&(module.to_string(), name.to_string()))?;

        Some((var_desc.args.clone(), Some(var_desc.span.clone())))
    }

    fn mark_var_used(&mut self, module: &str, name: &str) {
        // External variables don't track usage
        if let Some(var_desc) = self
            .internal_variables
            .get_mut(&(module.to_string(), name.to_string()))
        {
            var_desc.used = true;
        }
    }

    fn register_var(
        &mut self,
        module: &str,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        public: bool,
        referenced_fn: String,
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

    fn lookup_var_list(&self, module: &str, name: &str) -> Option<(ArgsType, Span)> {
        let var_desc = self
            .variable_lists
            .get(&(module.to_string(), name.to_string()))?;

        Some((var_desc.args.clone(), var_desc.span.clone()))
    }

    fn mark_var_list_used(&mut self, module: &str, name: &str) {
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

    fn register_var_list(
        &mut self,
        module: &str,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        public: bool,
        referenced_fn: String,
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

    fn lookup_field(&self, obj_type: &str, field: &str) -> Option<ExprType> {
        self.object_types.get(obj_type)?.get(field).cloned()
    }
}

#[derive(Debug, Error, Clone)]
pub enum SemError {
    #[error("Unknown identifier \"{identifier}\" at {span:?}")]
    UnknownIdentifer { identifier: String, span: Span },
    #[error("Unknown variable \"{var}\" at {span:?}")]
    UnknownVariable { var: String, span: Span },
    #[error("Function type mismatch: \"{identifier}\" at {span:?} has type {found} but type {expected} expected.")]
    FunctionTypeMismatch {
        identifier: String,
        span: Span,
        expected: FunctionType,
        found: FunctionType,
    },
    #[error("Variable \"{identifier}\" at {span:?} is already defined ({here:?})")]
    VariableAlreadyDefined {
        identifier: String,
        span: Span,
        here: Option<Span>,
    },
    #[error("Function \"{identifier}\" at {span:?} is already defined ({here:?})")]
    FunctionAlreadyDefined {
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Type {typ} at {span:?} is unknown")]
    UnknownType { typ: String, span: Span },
    #[error("Multiple option markers '?' on {typ} (at {span:?}) - only one option marker '?' is allowed")]
    MultipleOptionMarkers { typ: SimpleType, span: Span },
    #[error("Type {typ} appears multiple time in the sum (at {span1:?} and {span2:?} in sum at {sum_span:?})")]
    MultipleTypeInSum {
        typ: SimpleType,
        span1: Span,
        span2: Span,
        sum_span: Span,
    },
    #[error(
        "Type {typ1} (at {span1:?}) is a subtype of {typ2} (at {span2:?} in sum at {sum_span:?})"
    )]
    SubtypeAndTypePresent {
        typ1: SimpleType,
        span1: Span,
        typ2: SimpleType,
        span2: Span,
        sum_span: Span,
    },
    #[error("Option marker '?' is forbidden on None (at {0:?})")]
    OptionMarkerOnNone(Span),
    #[error("Type {typ} at {span:?} is not a sum type of objects. This is disallowed in global collections")]
    GlobalCollectionsMustBeAListOfObjects { typ: String, span: Span },
    #[error("Parameter \"{identifier}\" is already defined ({here:?}).")]
    ParameterAlreadyDefined {
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Body type mismatch: body for function {func} at {span:?} has type {found} but type {expected} expected.")]
    BodyTypeMismatch {
        func: String,
        span: Span,
        expected: ExprType,
        found: ExprType,
    },
    #[error("Type mismatch at {span:?}: expected {expected} but found {found} ({context})")]
    TypeMismatch {
        span: Span,
        expected: ExprType,
        found: ExprType,
        context: String,
    },
    #[error("Argument count mismatch for \"{identifier}\" at {span:?}: expected {expected} arguments but found {found}")]
    ArgumentCountMismatch {
        identifier: String,
        span: Span,
        expected: usize,
        found: usize,
    },
    #[error("Unknown field \"{field}\" on type {object_type} at {span:?}")]
    UnknownField {
        object_type: String,
        field: String,
        span: Span,
    },
    #[error("Cannot access field \"{field}\" on non-object type {typ} at {span:?}")]
    FieldAccessOnNonObject {
        typ: ExprType,
        field: String,
        span: Span,
    },
    #[error(
        "Duplicate field \"{field}\" in struct literal at {span:?} (first defined at {previous:?})"
    )]
    DuplicateStructField {
        field: String,
        span: Span,
        previous: Span,
    },
    #[error("Unknown field \"{field}\" on struct type {struct_type} at {span:?}")]
    UnknownStructField {
        struct_type: String,
        field: String,
        span: Span,
    },
    #[error("Tuple index {index} out of bounds for tuple of size {size} at {span:?}")]
    TupleIndexOutOfBounds {
        index: usize,
        size: usize,
        span: Span,
    },
    #[error("Cannot access tuple index {index} on non-tuple type {typ} at {span:?}")]
    TupleIndexOnNonTuple {
        typ: ExprType,
        index: usize,
        span: Span,
    },
    #[error("Type at {span:?}: found {found} which is not a concrete type ({context})")]
    NonConcreteType {
        span: Span,
        found: ExprType,
        context: String,
    },
    #[error("Type at {span:?}: found {found} which cannot be converted into {target}")]
    ImpossibleConversion {
        span: Span,
        found: ExprType,
        target: ConcreteType,
    },
    #[error("Local variable \"{identifier}\" at {span:?} is already defined in the same scope ({here:?})")]
    LocalIdentAlreadyDeclared {
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Local variable \"{identifier}\" at {span:?} shadows a function with the same name")]
    LocalIdentShadowsFunction { identifier: String, span: Span },
    #[error("Branch for match at {span:?} has a too large type ({found:?}). Maximum type is {expected:?}")]
    OverMatching {
        span: Span,
        expected: Option<ExprType>,
        found: Option<ExprType>,
    },
    #[error("Match at {span:?} does not have exhaustive checking. The case {remaining_types} is not covered")]
    NonExhaustiveMatching {
        span: Span,
        remaining_types: ExprType,
    },
    #[error("Null coalescing operator '??' at {span:?} requires a maybe type (containing None), but found {found}")]
    NullCoalesceOnNonMaybe { span: Span, found: ExprType },
    #[error("List index at {span:?} requires Int type, but found {found}")]
    ListIndexNotInt { span: Span, found: ExprType },
    #[error("Cannot index into non-list type {typ} at {span:?}")]
    IndexOnNonList { typ: ExprType, span: Span },
    #[error("Type \"{type_name}\" at {span:?} shadows a primitive type")]
    TypeShadowsPrimitive { type_name: String, span: Span },
    #[error("Type \"{type_name}\" at {span:?} shadows an object type")]
    TypeShadowsObject { type_name: String, span: Span },
    #[error("Type \"{type_name}\" at {span:?} shadows a previously defined custom type")]
    TypeShadowsCustomType { type_name: String, span: Span },
    #[error(
        "Type \"{type_name}\" at {span:?} has unguarded recursion (must be inside a list or tuple)"
    )]
    UnguardedRecursiveType { type_name: String, span: Span },
    #[error("Module \"{module}\" at {span:?} is unknown")]
    UnknownModule { module: String, span: Span },
    #[error("Cannot import own module at {span:?}")]
    SelfImport { span: Span },
    #[error("Symbol \"{path}\" at {span:?} conflicts with existing symbol from module \"{existing_module}\"")]
    SymbolConflict {
        path: String,
        span: Span,
        existing_module: String,
    },
    #[error("Qualified module access at {span:?} is not yet implemented")]
    QualifiedAccessNotYetSupported { span: Span },
    #[error("Primitive type \"{type_name}\" at {span:?} cannot be used as a value (use a conversion like {type_name}(x))")]
    PrimitiveTypeAsValue { type_name: String, span: Span },
    #[error("Enum variant {enum_name}::{variant_name} at {span:?} requires arguments")]
    MissingEnumVariantArguments {
        enum_name: String,
        variant_name: String,
        span: Span,
    },
    #[error("Unsupported feature: {feature} at {span:?}")]
    UnsupportedFeature { feature: String, span: Span },
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SemWarning {
    #[error("Identifier \"{identifier}\" at {span:?} shadows previous definition at {previous:?}")]
    IdentifierShadowed {
        identifier: String,
        span: Span,
        previous: Span,
    },

    #[error(
        "Function \"{identifier}\" at {span:?} should use snake_case (consider \"{suggestion}\")"
    )]
    FunctionNamingConvention {
        identifier: String,
        span: Span,
        suggestion: String,
    },

    #[error(
        "Variable \"{identifier}\" at {span:?} should use PascalCase (consider \"{suggestion}\")"
    )]
    VariableNamingConvention {
        identifier: String,
        span: Span,
        suggestion: String,
    },

    #[error(
        "Parameter \"{identifier}\" at {span:?} should use snake_case (consider \"{suggestion}\")"
    )]
    ParameterNamingConvention {
        identifier: String,
        span: Span,
        suggestion: String,
    },
    #[error("Unused identifier \"{identifier}\" at {span:?}")]
    UnusedIdentifier { identifier: String, span: Span },
    #[error("Unused function \"{identifier}\" at {span:?}")]
    UnusedFunction { identifier: String, span: Span },
    #[error("Unused variable \"{identifier}\" at {span:?}")]
    UnusedVariable { identifier: String, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LocalEnv {
    scopes: Vec<HashMap<String, (ExprType, Span, bool)>>,
    pending_scope: HashMap<String, (ExprType, Span, bool)>,
    current_module: String,
}

fn ident_can_be_unused(ident: &str) -> bool {
    assert!(ident.len() > 0);

    ident.chars().next().unwrap() == '_'
}

fn should_be_used_by_default(ident: &str) -> bool {
    ident_can_be_unused(ident)
}

fn ident_can_be_shadowed(ident: &str) -> bool {
    ident_can_be_unused(ident)
}

impl LocalEnv {
    fn new(current_module: &str) -> Self {
        LocalEnv {
            scopes: Vec::new(),
            pending_scope: HashMap::new(),
            current_module: current_module.to_string(),
        }
    }

    fn current_module(&self) -> &str {
        &self.current_module
    }

    fn lookup_ident(&self, ident: &str) -> Option<(ExprType, Span)> {
        // We don't look in pending scope as these variables are not yet accessible
        for scope in self.scopes.iter().rev() {
            if let Some((typ, span, _)) = scope.get(ident) {
                return Some((typ.clone(), span.clone()));
            }
        }
        None
    }

    fn mark_ident_used(&mut self, ident: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((_, _, used)) = scope.get_mut(ident) {
                *used = true;
                return;
            }
        }
    }

    fn push_scope(&mut self) {
        let mut old_scope = HashMap::new();
        std::mem::swap(&mut old_scope, &mut self.pending_scope);

        self.scopes.push(old_scope);
    }

    fn pop_scope(&mut self, warnings: &mut Vec<SemWarning>) {
        assert!(!self.scopes.is_empty());

        self.pending_scope = self.scopes.pop().unwrap();

        for (name, (_typ, span, used)) in &self.pending_scope {
            if !*used {
                warnings.push(SemWarning::UnusedIdentifier {
                    identifier: name.clone(),
                    span: span.clone(),
                });
            }
        }

        self.pending_scope.clear();
    }

    fn register_identifier(
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
                identifier: ident.to_string(),
                span,
                here: old_ident_span.clone(),
            });
        }

        // Check if identifier shadows a function name
        let fn_key = (self.current_module().to_string(), ident.to_string());
        if global_env.get_functions().contains_key(&fn_key) {
            return Err(SemError::LocalIdentShadowsFunction {
                identifier: ident.to_string(),
                span,
            });
        }

        if let Some((_typ, old_span)) = self.lookup_ident(ident) {
            if !ident_can_be_shadowed(ident) {
                warnings.push(SemWarning::IdentifierShadowed {
                    identifier: ident.to_string(),
                    span: span.clone(),
                    previous: old_span,
                });
            }
        }

        self.pending_scope.insert(
            ident.to_string(),
            (typ.clone(), span.clone(), should_be_used_by_default(ident)),
        );
        type_info.types.insert(span, typ.into());

        Ok(())
    }

    /// Register an identifier without duplicate checking (for pass 2 where we already validated)
    fn register_identifier_no_check(&mut self, ident: &str, typ: ExprType) {
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

// =============================================================================
// PATH RESOLUTION
// =============================================================================

/// What kind of entity a namespace path refers to
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPathKind {
    /// A local variable or parameter: `x`, `student`
    LocalVariable(String),

    /// A function in scope: `foo`, `helper`
    Function { module: String, func: String },

    /// A type (built-in, custom, or enum variant)
    /// - Built-in: `Int` → Type(SimpleType::Int)
    /// - Custom: `MyType` → Type(SimpleType::Custom("main", "MyType", None))
    /// - Enum variant: `Result::Ok` → Type(SimpleType::Custom("main", "Result", Some("Ok")))
    Type(SimpleType),

    /// A module reference (from import with alias)
    Module(String),

    /// An external variable (defined by runtime, not in source)
    ExternalVariable(String),

    /// An internal variable (defined in source with $var)
    InternalVariable { module: String, name: String },

    /// A variable list (defined in source with $$var_list)
    VariableList { module: String, name: String },
}

/// Errors that can occur during path resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathResolutionError {
    /// Identifier not found in any scope
    UnknownIdentifier { name: String, span: Span },
    /// Qualified path (e.g., `Foo::Bar`) not found
    UnknownQualifiedPath { path: String, span: Span },
    /// Path has too many segments (> 2)
    UnsupportedPathLength { len: usize, span: Span },
}

impl PathResolutionError {
    pub fn into_sem_error(self) -> SemError {
        match self {
            PathResolutionError::UnknownIdentifier { name, span } => SemError::UnknownIdentifer {
                identifier: name,
                span,
            },
            PathResolutionError::UnknownQualifiedPath { path, span } => {
                SemError::UnknownIdentifer {
                    identifier: path,
                    span,
                }
            }
            PathResolutionError::UnsupportedPathLength { len, span } => {
                SemError::UnknownIdentifer {
                    identifier: format!("<path with {} segments>", len),
                    span,
                }
            }
        }
    }
}

/// Resolve a namespace path to its absolute meaning.
///
/// This is the single source of truth for what a path refers to.
/// Resolution priority:
/// 1. Built-in types: Int, Bool, String, LinExpr, Constraint, None, Never
/// 2. External entities: object_types, external_variables (treated like primitives)
/// 3. Symbol table: custom types, functions, internal variables, variable lists
/// 4. Local variables (from LocalEnv) - checked last for defensive programming
///
/// Paths can be single-segment (`foo`) or multi-segment (`Result::Ok`, `mod::func`).
pub fn resolve_path(
    path: &Spanned<crate::ast::NamespacePath>,
    current_module: &str,
    global_env: &GlobalEnv,
    local_env: Option<&LocalEnv>,
) -> Result<ResolvedPathKind, PathResolutionError> {
    let segments: Vec<&str> = path.node.segments.iter().map(|s| s.node.as_str()).collect();

    // 1. Check built-in types (single segment only)
    if segments.len() == 1 {
        if let Some(builtin) = try_resolve_builtin_type(segments[0]) {
            return Ok(ResolvedPathKind::Type(builtin));
        }
    }

    // 2. Check external entities (single segment only, treated like primitives)
    if segments.len() == 1 {
        let name = segments[0];
        if global_env.object_types.contains_key(name) {
            return Ok(ResolvedPathKind::Type(SimpleType::Object(name.to_string())));
        }
        if global_env.external_variables.contains_key(name) {
            return Ok(ResolvedPathKind::ExternalVariable(name.to_string()));
        }
    }

    // 3. Check symbol table (any path length)
    let symbol_path = SymbolPath(segments.iter().map(|s| s.to_string()).collect());
    if let Some(symbol) = global_env.lookup_symbol(current_module, &symbol_path) {
        return match symbol {
            Symbol::Module(m) => Ok(ResolvedPathKind::Module(m.clone())),
            Symbol::CustomType(m, n) => {
                // Handle enum variants: path ["Result", "Ok"] → Custom("main", "Result", Some("Ok"))
                // The type name in the symbol table stores "Result::Ok" for variants
                if n.contains("::") {
                    // This is an enum variant, parse it
                    let parts: Vec<&str> = n.split("::").collect();
                    Ok(ResolvedPathKind::Type(SimpleType::Custom(
                        m.clone(),
                        parts[0].to_string(),
                        Some(parts[1].to_string()),
                    )))
                } else {
                    Ok(ResolvedPathKind::Type(SimpleType::Custom(
                        m.clone(),
                        n.clone(),
                        None,
                    )))
                }
            }
            Symbol::Function(m, n) => Ok(ResolvedPathKind::Function {
                module: m.clone(),
                func: n.clone(),
            }),
            Symbol::Variable(m, n) => Ok(ResolvedPathKind::InternalVariable {
                module: m.clone(),
                name: n.clone(),
            }),
            Symbol::VariableList(m, n) => Ok(ResolvedPathKind::VariableList {
                module: m.clone(),
                name: n.clone(),
            }),
        };
    }

    // 4. Check local variables (single segment only, last for defensive programming)
    if segments.len() == 1 {
        if let Some(local) = local_env {
            if local.lookup_ident(segments[0]).is_some() {
                return Ok(ResolvedPathKind::LocalVariable(segments[0].to_string()));
            }
        }
    }

    Err(PathResolutionError::UnknownIdentifier {
        name: segments.join("::"),
        span: path.span.clone(),
    })
}

/// Try to resolve a name as a built-in type
fn try_resolve_builtin_type(name: &str) -> Option<SimpleType> {
    match name {
        "Int" => Some(SimpleType::Int),
        "Bool" => Some(SimpleType::Bool),
        "String" => Some(SimpleType::String),
        "LinExpr" => Some(SimpleType::LinExpr),
        "Constraint" => Some(SimpleType::Constraint),
        "None" => Some(SimpleType::None),
        "Never" => Some(SimpleType::Never),
        _ => None,
    }
}

impl LocalEnv {
    fn check_expr(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &crate::ast::Expr,
        span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        let result = self.check_expr_internal(
            global_env, expr, span, type_info, expr_types, errors, warnings,
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
        expr: &crate::ast::Expr,
        global_span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        use crate::ast::Expr;

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
                    global_env, &object, segments, type_info, expr_types, errors, warnings,
                )
                .map(|x| x.into()),

            // ========== As construct ==========
            Expr::ExplicitType { expr, typ } => {
                // Check the inner expression
                let expr_type = self.check_expr(
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
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
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

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
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
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
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

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
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
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
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

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
                        global_env, &arg.node, &arg.span, type_info, expr_types, errors, warnings,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                            &r,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                            &r,
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
                                    context: format!(
                                        "Cannot remove elements from empty list",
                                    ),
                                }),
                                (SimpleType::List(_inner), SimpleType::EmptyList) => Err(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::List(SimpleType::Int.into()).into(),
                                    found: SimpleType::EmptyList.into(),
                                    context: format!(
                                        "Removing empty list is a no-op",
                                    ),
                                }),
                                (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                                    if inner1.overlaps_with(inner2) {
                                        Ok(SimpleType::List(inner1.clone()))
                                    } else {
                                        Err(SemError::TypeMismatch {
                                            span: right.span.clone(),
                                            expected: inner1.clone(),
                                            found: inner2.clone(),
                                            context: format!(
                                                "Types must overlap in set difference",
                                            ),
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
                    global_env, &term.node, &term.span, type_info, expr_types, errors, warnings,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                    (Some(l), Some(r)) => l.cross_check(&r, errors, |v1, v2| match (v1, v2) {
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
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
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
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
                    global_env, &lhs.node, &lhs.span, type_info, expr_types, errors, warnings,
                );
                let rhs_type = self.check_expr(
                    global_env, &rhs.node, &rhs.span, type_info, expr_types, errors, warnings,
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
                    global_env, &item.node, &item.span, type_info, expr_types, errors, warnings,
                );
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
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
                    errors,
                    warnings,
                );

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
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
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
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
                    errors,
                    warnings,
                );

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
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
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
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
                    errors,
                    warnings,
                );

                let acc_type = self.check_expr(
                    global_env,
                    &init_value.node,
                    &init_value.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                // Check naming conventions
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
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
                        global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
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
                    errors,
                    warnings,
                );
                let else_type = self.check_expr(
                    global_env,
                    &else_expr.node,
                    &else_expr.span,
                    type_info,
                    expr_types,
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
                                        typ: t.to_string(),
                                        span: bt.span.clone(),
                                    });
                                    continue;
                                }
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
            Expr::VarCall { name, args } => {
                match global_env.lookup_var(self.current_module(), &name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::LinExpr.into()) // Syntax indicates LinExpr intent
                    }
                    Some((var_args, _)) => {
                        // Mark variable as used
                        global_env.mark_var_used(self.current_module(), &name.node);

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

                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type = self.check_expr(
                                global_env, &arg.node, &arg.span, type_info, expr_types, errors,
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
                }
            }

            Expr::VarListCall { name, args } => {
                match global_env.lookup_var_list(self.current_module(), &name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
                        // Syntax indicates [LinExpr] intent
                    }
                    Some((var_args, _)) => {
                        // Mark variable list as used
                        global_env.mark_var_list_used(self.current_module(), &name.node);

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

                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type = self.check_expr(
                                global_env, &arg.node, &arg.span, type_info, expr_types, errors,
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

                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
                    }
                }
            }

            // ========== Qualified Module Variable Calls (not yet implemented) ==========
            Expr::QualifiedVarCall { module, .. } => {
                errors.push(SemError::QualifiedAccessNotYetSupported {
                    span: module.span.clone(),
                });
                Some(SimpleType::LinExpr.into())
            }

            Expr::QualifiedVarListCall { module, .. } => {
                errors.push(SemError::QualifiedAccessNotYetSupported {
                    span: module.span.clone(),
                });
                Some(SimpleType::List(SimpleType::LinExpr.into()).into())
            }

            // ========== Generic Calls: func(args), Type(x), Enum::Variant(x) ==========
            Expr::GenericCall { path, args } => {
                // Use resolve_path to determine what this path refers to
                let resolved =
                    match resolve_path(path, self.current_module(), global_env, Some(self)) {
                        Ok(r) => r,
                        Err(e) => {
                            errors.push(e.into_sem_error());
                            return None;
                        }
                    };

                match resolved {
                    ResolvedPathKind::LocalVariable(name) => {
                        // Cannot call a local variable
                        errors.push(SemError::UnknownIdentifer {
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
                                        global_env, &arg.node, &arg.span, type_info, expr_types,
                                        errors, warnings,
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
                            errors,
                            warnings,
                        )
                    }
                    ResolvedPathKind::Module(name) => {
                        // Modules cannot be called
                        errors.push(SemError::UnknownIdentifer {
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
                            errors.push(e.into_sem_error());
                            return None;
                        }
                    };

                match resolved {
                    ResolvedPathKind::LocalVariable(name) => {
                        // Cannot use struct syntax with variables
                        errors.push(SemError::UnknownType {
                            typ: name,
                            span: path.span.clone(),
                        });
                        None
                    }
                    ResolvedPathKind::Function { func, .. } => {
                        // Cannot use struct syntax with functions
                        errors.push(SemError::UnknownType {
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
                        errors,
                        warnings,
                    ),
                    ResolvedPathKind::Module(name) => {
                        // Cannot use struct syntax with modules
                        errors.push(SemError::UnknownType {
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
                    errors,
                    warnings,
                );

                for item in &elements[1..] {
                    let item_type = self.check_expr(
                        global_env, &item.node, &item.span, type_info, expr_types, errors, warnings,
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
                            global_env, &elem.node, &elem.span, type_info, expr_types, errors,
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
                use std::collections::{BTreeMap, HashMap};
                let mut field_types: BTreeMap<String, ExprType> = BTreeMap::new();
                let mut seen_fields: HashMap<String, Span> = HashMap::new();
                let mut all_ok = true;

                for (field_name, field_expr) in fields {
                    // Check for duplicate field names
                    if let Some(prev_span) = seen_fields.get(&field_name.node) {
                        errors.push(SemError::DuplicateStructField {
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
                    errors,
                    warnings,
                );
                let end_type = self.check_expr(
                    global_env, &end.node, &end.span, type_info, expr_types, errors, warnings,
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
                        errors,
                        warnings,
                    );

                    // Check naming convention
                    if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                        &var.node,
                        string_case::NamingConvention::SnakeCase,
                    ) {
                        warnings.push(SemWarning::ParameterNamingConvention {
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
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
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
                    errors,
                    warnings,
                );

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
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
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
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
                    global_env, &arg.node, &arg.span, type_info, expr_types, errors, warnings,
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
                            global_env, &arg.node, &arg.span, type_info, expr_types, errors,
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
                                global_env, &arg.node, &arg.span, type_info, expr_types, errors,
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
                            global_env, &arg.node, &arg.span, type_info, expr_types, errors,
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

                // Return the custom type
                Some(
                    SimpleType::Custom(self.current_module().to_string(), type_name, variant_name)
                        .into(),
                )
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
                errors.push(e.into_sem_error());
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
                                    typ: qualified_name,
                                    span: path.span.clone(),
                                });
                                None
                            }
                        } else {
                            // Root custom type without variant - cannot be used as value
                            errors.push(SemError::PrimitiveTypeAsValue {
                                type_name: root.clone(),
                                span: path.span.clone(),
                            });
                            None
                        }
                    }
                    // Other types shouldn't appear from resolve_path for identifiers
                    _ => {
                        errors.push(SemError::UnknownIdentifer {
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

impl TypeInfo {
    pub fn new() -> Self {
        TypeInfo::default()
    }
}

impl GlobalEnv {
    pub fn new(
        object_types: HashMap<String, ObjectFields>,
        variables: HashMap<String, ArgsType>,
        file: &crate::ast::File,
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
        // Module list (placeholder until modules become a parameter)
        let modules = [Module {
            name: "main".into(),
            file: file.clone(),
        }];

        let mut temp_env = GlobalEnv {
            module_names: modules.iter().map(|m| m.name.clone()).collect(),
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
        for module in &modules {
            let current_module = &module.name;
            for statement in &module.file.statements {
                match &statement.node {
                    crate::ast::Statement::TypeDecl { name, .. } => {
                        temp_env.expand_with_type_decl_pass1(current_module, name, &mut errors);
                    }
                    crate::ast::Statement::EnumDecl { name, variants, .. } => {
                        temp_env.expand_with_enum_decl_pass1(
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
        // PASS 1b: Resolve all type definitions
        // Now all type names are known, we can resolve underlying types
        // ====================================================================
        for module in &modules {
            let current_module = &module.name;
            for statement in &module.file.statements {
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
        // PASS 1c: Populate type symbols for current module + imports
        // ====================================================================
        for module in &modules {
            let current_module = &module.name;
            temp_env.import_type_symbols(current_module, current_module, None, None, &mut errors);

            // Import type symbols from foreign modules
            for statement in &module.file.statements {
                if let crate::ast::Statement::Import { module_path, alias } = &statement.node {
                    if !temp_env.module_exists(&module_path.node) {
                        errors.push(SemError::UnknownModule {
                            module: module_path.node.clone(),
                            span: module_path.span.clone(),
                        });
                        continue;
                    }
                    if module_path.node == *current_module {
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
        // PASS 2a: Register all function signatures
        // Now all types are resolved, we can build function signatures
        // ====================================================================
        for module in &modules {
            let current_module = &module.name;
            for statement in &module.file.statements {
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
        for module in &modules {
            let current_module = &module.name;
            temp_env.import_function_symbols(
                current_module,
                current_module,
                None,
                None,
                &mut errors,
            );

            // Import function symbols from foreign modules (skip validation, done in PASS 1c)
            for statement in &module.file.statements {
                if let crate::ast::Statement::Import { module_path, alias } = &statement.node {
                    if temp_env.module_exists(&module_path.node)
                        && module_path.node != *current_module
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
        for module in &modules {
            let current_module = &module.name;
            for statement in &module.file.statements {
                if let crate::ast::Statement::Reify {
                    constraint_name,
                    name,
                    var_list,
                    public,
                    ..
                } = &statement.node
                {
                    temp_env.expand_with_reify_statement(
                        current_module,
                        constraint_name,
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
        for module in &modules {
            let current_module = &module.name;
            temp_env.import_variable_symbols(
                current_module,
                current_module,
                None,
                None,
                &mut errors,
            );

            // Import variable symbols from foreign modules (skip validation, done in PASS 1c)
            for statement in &module.file.statements {
                if let crate::ast::Statement::Import { module_path, alias } = &statement.node {
                    if temp_env.module_exists(&module_path.node)
                        && module_path.node != *current_module
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
        for module in &modules {
            let current_module = &module.name;
            for statement in &module.file.statements {
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
                    identifier: format!("{}::{}", module, fn_name),
                    span: fn_desc.body.span.clone(),
                });
            }
        }
    }

    fn check_unused_var(&self, warnings: &mut Vec<SemWarning>) {
        for ((module, var_name), var_desc) in &self.internal_variables {
            if !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: format!("{}::{}", module, var_name),
                    span: var_desc.span.clone(),
                });
            }
        }

        for ((module, var_name), var_desc) in &self.variable_lists {
            if !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: format!("{}::{}", module, var_name),
                    span: var_desc.span.clone(),
                });
            }
        }
    }

    // ========================================================================
    // Symbol Table Helpers
    // ========================================================================

    /// Check if a module exists
    fn module_exists(&self, module_name: &str) -> bool {
        self.module_names.contains(&module_name.to_string())
    }

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
        let types_to_add: Vec<_> = self
            .custom_types
            .keys()
            .filter(|(mod_name, _)| mod_name == source_module)
            .map(|(mod_name, type_name)| (mod_name.clone(), type_name.clone()))
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
        let fns_to_add: Vec<_> = self
            .functions
            .keys()
            .filter(|(mod_name, _)| mod_name == source_module)
            .map(|(mod_name, fn_name)| (mod_name.clone(), fn_name.clone()))
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
        let vars_to_add: Vec<_> = self
            .internal_variables
            .keys()
            .filter(|(mod_name, _)| mod_name == source_module)
            .map(|(mod_name, var_name)| (mod_name.clone(), var_name.clone()))
            .collect();

        let symbol_map = self.symbols.entry(target_module.to_string()).or_default();
        for (mod_name, var_name) in vars_to_add {
            let path = Self::make_symbol_path(prefix, &var_name);
            if let Some(existing) = symbol_map.get(&path) {
                conflicts.push((path.0.join("::"), existing.module_name().to_string()));
            } else {
                symbol_map.insert(path, Symbol::Variable(mod_name, var_name));
            }
        }

        // Collect variable lists to add
        let var_lists_to_add: Vec<_> = self
            .variable_lists
            .keys()
            .filter(|(mod_name, _)| mod_name == source_module)
            .map(|(mod_name, var_name)| (mod_name.clone(), var_name.clone()))
            .collect();

        let symbol_map = self.symbols.entry(target_module.to_string()).or_default();
        for (mod_name, var_name) in var_lists_to_add {
            let path = Self::make_symbol_path(prefix, &var_name);
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
        local_env: &mut LocalEnv,
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

        // Build LocalEnv with parameters
        let mut local_env = LocalEnv::new(current_module);
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
        constraint_name: &Spanned<String>,
        name: &Spanned<String>,
        var_list: bool,
        public: bool,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match self.lookup_fn(current_module, &constraint_name.node) {
            None => errors.push(SemError::UnknownIdentifer {
                identifier: constraint_name.node.clone(),
                span: constraint_name.span.clone(),
            }),
            Some(fn_type) => {
                // Mark function as used
                self.mark_fn_used(current_module, &constraint_name.node);

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
                    errors.push(SemError::FunctionTypeMismatch {
                        identifier: constraint_name.node.clone(),
                        span: constraint_name.span.clone(),
                        expected: expected_type,
                        found: fn_type.0,
                    });
                    return;
                }

                if var_list {
                    match self.lookup_var_list(current_module, &name.node) {
                        Some((_args, span)) => errors.push(SemError::VariableAlreadyDefined {
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
                                constraint_name.node.clone(),
                                type_info,
                            );
                        }
                    }
                } else {
                    match self.lookup_var(current_module, &name.node) {
                        Some((_args, span_opt)) => errors.push(SemError::VariableAlreadyDefined {
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
                                constraint_name.node.clone(),
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
        name: &Spanned<String>,
        errors: &mut Vec<SemError>,
    ) {
        // Check if type name shadows a primitive type
        if Self::is_primitive_type_name(&name.node) {
            errors.push(SemError::TypeShadowsPrimitive {
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Check if type name shadows an object type
        if self.object_types.contains_key(&name.node) {
            errors.push(SemError::TypeShadowsObject {
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Check if type name shadows a previous custom type (duplicate in same file)
        let type_key = (current_module.to_string(), name.node.clone());
        if self.custom_types.contains_key(&type_key) {
            errors.push(SemError::TypeShadowsCustomType {
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Register with placeholder - will be resolved in pass 2
        self.custom_types
            .insert(type_key, ExprType::simple(SimpleType::Never));
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
        if !self.custom_types.contains_key(&type_key) {
            return;
        }

        // Build the context for type resolution - all type names are now known
        let object_types: std::collections::HashSet<String> =
            self.object_types.keys().cloned().collect();
        let custom_type_names: std::collections::HashSet<(String, String)> =
            self.custom_types.keys().cloned().collect();

        // Resolve the underlying type
        let underlying_type = match ExprType::from_ast(
            underlying.clone(),
            current_module,
            &object_types,
            &custom_type_names,
        ) {
            Ok(typ) => typ,
            Err(e) => {
                errors.push(e);
                return;
            }
        };

        // Check for unguarded recursive type (type references itself without being inside a container)
        if self.has_unguarded_reference(&underlying_type, &name.node) {
            errors.push(SemError::UnguardedRecursiveType {
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Update the placeholder with the actual type
        self.custom_types.insert(type_key, underlying_type);
    }

    /// Pass 1 for enum declarations: Register the enum name and all variant names with placeholders
    fn expand_with_enum_decl_pass1(
        &mut self,
        current_module: &str,
        name: &Spanned<String>,
        variants: &[Spanned<crate::ast::EnumVariant>],
        errors: &mut Vec<SemError>,
    ) {
        // Check if enum name shadows a primitive type
        if Self::is_primitive_type_name(&name.node) {
            errors.push(SemError::TypeShadowsPrimitive {
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        // Check for shadowing existing object or custom types
        if self.object_types.contains_key(&name.node) {
            errors.push(SemError::TypeShadowsObject {
                type_name: name.node.clone(),
                span: name.span.clone(),
            });
            return;
        }

        let type_key = (current_module.to_string(), name.node.clone());
        if self.custom_types.contains_key(&type_key) {
            errors.push(SemError::TypeShadowsCustomType {
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
        self.custom_types
            .insert(type_key, ExprType::simple(SimpleType::Never));

        // Register all variant types with placeholders
        for variant in variants {
            let qualified_name = format!("{}::{}", name.node, variant.node.name.node);
            let variant_key = (current_module.to_string(), qualified_name.clone());

            self.custom_types
                .insert(variant_key, ExprType::simple(SimpleType::Never));
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
        if !self.custom_types.contains_key(&type_key) {
            return;
        }

        // Build the context for type resolution
        let object_types: std::collections::HashSet<String> =
            self.object_types.keys().cloned().collect();
        let custom_type_names: std::collections::HashSet<(String, String)> =
            self.custom_types.keys().cloned().collect();

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
                        match ExprType::from_ast(
                            types[0].clone(),
                            current_module,
                            &object_types,
                            &custom_type_names,
                        ) {
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
                            .map(|t| {
                                ExprType::from_ast(
                                    t.clone(),
                                    current_module,
                                    &object_types,
                                    &custom_type_names,
                                )
                            })
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
                                    ExprType::from_ast(
                                        ftype.clone(),
                                        current_module,
                                        &object_types,
                                        &custom_type_names,
                                    )
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
                    type_name: qualified_name.clone(),
                    span: variant.node.name.span.clone(),
                });
                continue;
            }

            // Update the variant's underlying type
            self.custom_types.insert(variant_key, underlying_type);

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
            self.custom_types.insert(type_key, enum_type);
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
                if let Some(underlying) = self.custom_types.get(&type_key) {
                    // Skip if it's still a placeholder (Never type used during pass 1)
                    let placeholder = ExprType::simple(SimpleType::Never);
                    if *underlying == placeholder {
                        false
                    } else {
                        self.has_unguarded_reference(underlying, type_name)
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
