use super::errors::SemError;
use super::global_env::{GlobalEnv, Symbol, SymbolPath};
use super::local_env::LocalEnvCheck;
use super::types::SimpleType;
use crate::ast::{Span, Spanned};

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
}

impl PathResolutionError {
    pub fn into_sem_error(self, module: &str) -> SemError {
        match self {
            PathResolutionError::UnknownIdentifier { name, span } => SemError::UnknownIdentifer {
                module: module.to_string(),
                identifier: name,
                span,
            },
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
    local_env: Option<&dyn LocalEnvCheck>,
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
        if !name.is_empty() {
            let first_symbol = name.chars().next().unwrap();
            if first_symbol == '$' {
                let no_dollar_name = &name[1..];
                if global_env.external_variables.contains_key(no_dollar_name) {
                    return Ok(ResolvedPathKind::ExternalVariable(
                        no_dollar_name.to_string(),
                    ));
                }
            }
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
            if local.has_ident(segments[0]) {
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
pub fn try_resolve_builtin_type(name: &str) -> Option<SimpleType> {
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
