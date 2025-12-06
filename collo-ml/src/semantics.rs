use crate::ast::{Expr, Param, Span, Spanned};
use std::collections::{BTreeSet, HashMap};

pub mod string_case;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimpleType {
    Int,
    Bool,
    None,
    LinExpr,
    Constraint,
    List(ExprType),
    Object(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprType {
    variants: BTreeSet<SimpleType>,
}

impl SimpleType {
    pub fn is_primitive_type(&self) -> bool {
        matches!(
            self,
            SimpleType::Int
                | SimpleType::Bool
                | SimpleType::LinExpr
                | SimpleType::Constraint
                | SimpleType::None
        )
    }

    pub fn is_none(&self) -> bool {
        matches!(self, SimpleType::None)
    }

    pub fn is_list(&self) -> bool {
        matches!(self, SimpleType::List(_))
    }

    pub fn is_lin_expr(&self) -> bool {
        matches!(self, SimpleType::LinExpr)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, SimpleType::Int)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, SimpleType::Bool)
    }

    pub fn is_constraint(&self) -> bool {
        matches!(self, SimpleType::Constraint)
    }

    pub fn get_inner_list_type(&self) -> Option<&ExprType> {
        match self {
            SimpleType::List(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn to_inner_list_type(self) -> Option<ExprType> {
        match self {
            SimpleType::List(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, SimpleType::Object(_))
    }

    pub fn get_inner_object_type(&self) -> Option<&String> {
        match self {
            SimpleType::Object(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn to_inner_object_type(self) -> Option<String> {
        match self {
            SimpleType::Object(typ) => Some(typ),
            _ => None,
        }
    }

    /// Checks if type is valid for arithmetic operations
    pub fn is_arithmetic(&self) -> bool {
        matches!(self, SimpleType::Int | SimpleType::LinExpr)
    }

    /// Checks if self can be coerced to target type.
    /// This is DIRECTIONAL: Int can coerce to LinExpr, but not vice versa.
    pub fn can_coerce_to(&self, target: &SimpleType) -> bool {
        match (self, target) {
            // Exact match always works
            (a, b) if a == b => true,

            // Int → LinExpr (but NOT LinExpr → Int)
            (SimpleType::Int, SimpleType::LinExpr) => true,

            // Recursive: [A] → [B] if A can coerce to B
            (SimpleType::List(a), SimpleType::List(b)) => a.can_coerce_to(b),

            // Everything else: no coercion
            _ => false,
        }
    }

    /// Finds a common type that both left and right can coerce to.
    /// This is BIDIRECTIONAL and SYMMETRIC.
    pub fn unify(left: &SimpleType, right: &SimpleType) -> Option<SimpleType> {
        match (left, right) {
            // Exact match (including both EmptyList)
            (a, b) if a == b => Some(a.clone()),

            // Int/LinExpr unify to LinExpr (bidirectional)
            (SimpleType::Int, SimpleType::LinExpr) | (SimpleType::LinExpr, SimpleType::Int) => {
                Some(SimpleType::LinExpr)
            }

            // Lists: unify element types recursively
            (SimpleType::List(l), SimpleType::List(r)) => {
                ExprType::unify(l, r).map(|unified| SimpleType::List(unified))
            }

            // No unification possible
            _ => None,
        }
    }
}

impl TryFrom<crate::ast::SimpleTypeName> for SimpleType {
    type Error = TypeNameError;

    fn try_from(value: crate::ast::SimpleTypeName) -> Result<Self, TypeNameError> {
        use crate::ast::SimpleTypeName;
        match value {
            SimpleTypeName::None => Ok(SimpleType::None),
            SimpleTypeName::Bool => Ok(SimpleType::Bool),
            SimpleTypeName::Int => Ok(SimpleType::Int),
            SimpleTypeName::LinExpr => Ok(SimpleType::LinExpr),
            SimpleTypeName::Constraint => Ok(SimpleType::Constraint),
            SimpleTypeName::Object(name) => Ok(SimpleType::Object(name)),
            SimpleTypeName::List(sub_typ) => Ok(SimpleType::List(sub_typ.try_into()?)),
        }
    }
}

impl std::fmt::Display for SimpleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleType::None => write!(f, "None"),
            SimpleType::Bool => write!(f, "Bool"),
            SimpleType::Int => write!(f, "Int"),
            SimpleType::LinExpr => write!(f, "LinExpr"),
            SimpleType::Constraint => write!(f, "Constraint"),
            SimpleType::List(sub_type) => write!(f, "[{}]", sub_type),
            SimpleType::Object(typ) => write!(f, "{}", typ),
        }
    }
}

impl ExprType {
    pub fn simple(typ: SimpleType) -> ExprType {
        ExprType {
            variants: BTreeSet::from([typ]),
        }
    }

    pub fn maybe(typ: SimpleType) -> Option<ExprType> {
        if typ.is_none() {
            return None;
        }
        Some(ExprType {
            variants: BTreeSet::from([SimpleType::None, typ]),
        })
    }

    pub fn sum(types: impl IntoIterator<Item = SimpleType>) -> Option<Self> {
        let variants: BTreeSet<_> = types.into_iter().collect();

        if variants.is_empty() {
            return None;
        }

        Some(ExprType { variants })
    }

    pub fn is_simple(&self) -> bool {
        assert!(
            self.variants.len() >= 1,
            "ExprType should always carry at least one type"
        );
        self.variants.len() == 1
    }

    pub fn as_simple(&self) -> Option<&SimpleType> {
        if !self.is_simple() {
            return None;
        }
        Some(
            self.variants
                .iter()
                .next()
                .expect("ExprType should always carry at least one type"),
        )
    }

    pub fn to_simple(self) -> Option<SimpleType> {
        if !self.is_simple() {
            return None;
        }
        Some(
            self.variants
                .into_iter()
                .next()
                .expect("ExprType should always carry at least one type"),
        )
    }

    pub fn is_primitive_type(&self) -> bool {
        self.as_simple()
            .map(|x| x.is_primitive_type())
            .unwrap_or(false)
    }

    pub fn contains_one_list(&self) -> bool {
        self.variants.iter().filter(|x| x.is_list()).count() == 1
    }

    pub fn get_inner_list_type(&self) -> Option<&ExprType> {
        self.as_simple().map(|x| x.get_inner_list_type()).flatten()
    }

    pub fn to_inner_list_type(self) -> Option<ExprType> {
        self.to_simple().map(|x| x.to_inner_list_type()).flatten()
    }

    pub fn is_list(&self) -> bool {
        self.as_simple().map(|x| x.is_list()).unwrap_or(false)
    }

    pub fn is_none(&self) -> bool {
        self.as_simple().map(|x| x.is_none()).unwrap_or(false)
    }

    pub fn is_sum_of_objects(&self) -> bool {
        self.variants
            .iter()
            .all(|x| matches!(x, SimpleType::Object(_)))
    }

    pub fn get_inner_object_type(&self) -> Option<&String> {
        self.as_simple()
            .map(|x| x.get_inner_object_type())
            .flatten()
    }

    pub fn to_inner_object_type(self) -> Option<String> {
        self.to_simple().map(|x| x.to_inner_object_type()).flatten()
    }

    pub fn is_object(&self) -> bool {
        self.as_simple().map(|x| x.is_object()).unwrap_or(false)
    }

    pub fn contains(&self, typ: &SimpleType) -> bool {
        self.variants.iter().any(|x| x == typ)
    }

    pub fn is_lin_expr(&self) -> bool {
        self.as_simple().map(|x| x.is_lin_expr()).unwrap_or(false)
    }

    pub fn is_int(&self) -> bool {
        self.as_simple().map(|x| x.is_int()).unwrap_or(false)
    }

    pub fn is_bool(&self) -> bool {
        self.as_simple().map(|x| x.is_bool()).unwrap_or(false)
    }

    pub fn is_constraint(&self) -> bool {
        self.as_simple().map(|x| x.is_constraint()).unwrap_or(false)
    }

    /// Checks if type is valid for arithmetic operations
    pub fn is_arithmetic(&self) -> bool {
        self.as_simple().map(|x| x.is_arithmetic()).unwrap_or(false)
    }

    pub fn get_variants(&self) -> &BTreeSet<SimpleType> {
        &self.variants
    }

    pub fn can_coerce_to(&self, target: &ExprType) -> bool {
        match (self, target) {
            // For SimpleType, we defer to SimpleType::can_coerce_to
            (a, b) if a.is_simple() && b.is_simple() => {
                let typ_a = a.as_simple().unwrap();
                let typ_b = b.as_simple().unwrap();

                typ_a.can_coerce_to(typ_b)
            }

            // If we have sum types, we can only coerce from one sum type to another
            // if the first is included in the other
            //
            // This covers the case of one simple type into a sum type
            (a, b) => a.variants.is_subset(&b.variants),
        }
    }

    pub fn unify(left: &ExprType, right: &ExprType) -> Option<ExprType> {
        match (left, right) {
            // For SimpleType, we defer to SimpleType::unify
            (a, b) if a.is_simple() && b.is_simple() => {
                let typ_a = a.as_simple().unwrap();
                let typ_b = b.as_simple().unwrap();

                SimpleType::unify(typ_a, typ_b).map(ExprType::simple)
            }

            // If we have sum types, we unify to the union of types
            //
            // This covers the case of one simple type with a sum type (this coerces to the sum type)
            (a, b) => Some(ExprType {
                variants: a.variants.union(&b.variants).cloned().collect(),
            }),
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TypeNameError {
    #[error("Only one option marker '?' is allowed on types")]
    MultipleOptionMarkers,
    #[error("Option types are not allowed in sum types")]
    OptionTypeInSumType,
    #[error("All types in sum types should appear once")]
    DuplicatedTypeInSumType,
    #[error("Option marker '?' is forbidden on None")]
    OptionMarkerOnNone,
}

impl TryFrom<crate::ast::TypeName> for ExprType {
    type Error = TypeNameError;

    fn try_from(value: crate::ast::TypeName) -> Result<Self, TypeNameError> {
        match value.types.len() {
            0 => panic!("It should not be possible to form 0-length typenames"),
            1 => {
                let maybe_type = value.types.into_iter().next().unwrap();
                match maybe_type.maybe_count {
                    0 => Ok(ExprType::simple(SimpleType::try_from(maybe_type.inner)?)),
                    1 => Ok(ExprType::maybe(SimpleType::try_from(maybe_type.inner)?)
                        .ok_or(TypeNameError::OptionMarkerOnNone)?),
                    _ => Err(TypeNameError::MultipleOptionMarkers),
                }
            }
            _ => {
                if value.types.iter().any(|x| x.maybe_count > 0) {
                    return Err(TypeNameError::OptionTypeInSumType);
                }

                let init_count = value.types.len();
                let expr_type = ExprType::sum(
                    value
                        .types
                        .into_iter()
                        .map(|x| SimpleType::try_from(x.inner))
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .expect("There should be more than 0 variants");

                if expr_type.variants.len() != init_count {
                    return Err(TypeNameError::DuplicatedTypeInSumType);
                }

                Ok(expr_type)
            }
        }
    }
}

impl From<SimpleType> for ExprType {
    fn from(value: SimpleType) -> Self {
        ExprType::simple(value)
    }
}

impl std::fmt::Display for ExprType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.variants.len() == 1 {
            write!(f, "{}", self.variants.iter().next().unwrap())
        } else {
            let types: Vec<_> = self.variants.iter().map(|t| t.to_string()).collect();
            write!(f, "{}", types.join(" | "))
        }
    }
}

/// Represents an annotated type
///
/// When exploring the AST, we might need more precise information than just types.
/// There are two main cases:
/// - types might be not completely determined. This is the case for [AnnotatedType::UntypedList].
///   This happens for instance with empty lists ("[]") for which we of course do not know the types
///   of the elements.
/// - types might also be forced. This means the coercion rules are made more stringent for such types.
///   This happens for instance after a type coercion ("as") as the user has clearly set an intent on
///   the type and it should be fulfilled.
///   Forced typed have some expiration date: forced caracter is lost when unifying types.
///   Coercion is possible after the next operation but forced type can be reintroduced
///   with a new 'as'. So for instance  in "if cond() { 5 as Int } else { g() } + $Var(x)"
///   "g()" must return a type coercible to Int otherwise this fails. However the result of
///   the "if" branches is coercible to a LinExpr.
///   To reenforce the "Int" we should write : "if cond() { 5 as Int } else { g() } as Int"
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedType {
    Forced(ExprType),
    Regular(ExprType),
    UntypedList,
}

impl AnnotatedType {
    pub fn is_primitive_type(&self) -> bool {
        match self {
            AnnotatedType::Forced(typ) => typ.is_primitive_type(),
            AnnotatedType::Regular(typ) => typ.is_primitive_type(),
            AnnotatedType::UntypedList => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            AnnotatedType::Forced(typ) => typ.is_list(),
            AnnotatedType::Regular(typ) => typ.is_list(),
            AnnotatedType::UntypedList => true,
        }
    }

    /// Checks if type is valid for arithmetic operations
    pub fn is_arithmetic(&self) -> bool {
        match self {
            AnnotatedType::Forced(typ) => typ.is_arithmetic(),
            AnnotatedType::Regular(typ) => typ.is_arithmetic(),
            AnnotatedType::UntypedList => false,
        }
    }

    pub fn is_forced(&self) -> bool {
        match self {
            AnnotatedType::Forced(_) => true,
            AnnotatedType::Regular(_) => false,
            AnnotatedType::UntypedList => false,
        }
    }

    pub fn matches(&self, target: &ExprType) -> bool {
        match self {
            AnnotatedType::Forced(typ) => *typ == *target,
            AnnotatedType::Regular(typ) => *typ == *target,
            AnnotatedType::UntypedList => false,
        }
    }

    pub fn inner(&self) -> Option<&ExprType> {
        match self {
            AnnotatedType::Forced(typ) => Some(&typ),
            AnnotatedType::Regular(typ) => Some(&typ),
            AnnotatedType::UntypedList => None,
        }
    }

    pub fn into_inner(self) -> Option<ExprType> {
        match self {
            AnnotatedType::Forced(typ) => Some(typ),
            AnnotatedType::Regular(typ) => Some(typ),
            AnnotatedType::UntypedList => None,
        }
    }

    pub fn can_coerce_to(&self, target: &ExprType) -> bool {
        match self {
            // Force types don't coerce if these are simple types
            AnnotatedType::Forced(typ) if target.is_simple() => typ == target,
            // But force types still coerce to sum types
            AnnotatedType::Forced(typ) => typ.can_coerce_to(target),
            AnnotatedType::Regular(typ) => typ.can_coerce_to(target),
            // We can automatically cast an empty list to a sum type if
            // exactly one variant is a list
            AnnotatedType::UntypedList => target.contains_one_list(),
        }
    }

    pub fn unify(left: &AnnotatedType, right: &AnnotatedType) -> Option<AnnotatedType> {
        match (left, right) {
            // Force typed don't coerce if these are simple types
            (AnnotatedType::Forced(a), AnnotatedType::Forced(b))
                if a.is_simple() && b.is_simple() =>
            {
                if a == b {
                    Some(AnnotatedType::Regular(a.clone()))
                } else {
                    None
                }
            }
            // If one of the type is not simple (it is a sum type), then we can unify to the union of the sum
            (AnnotatedType::Forced(a), AnnotatedType::Forced(b)) => {
                ExprType::unify(a, b).map(AnnotatedType::Regular)
            }
            (AnnotatedType::Regular(a), AnnotatedType::Regular(b)) => {
                ExprType::unify(a, b).map(AnnotatedType::Regular)
            }
            (AnnotatedType::UntypedList, AnnotatedType::UntypedList) => {
                Some(AnnotatedType::UntypedList)
            }
            // Force type does not coerce if simple, but regular type can
            (AnnotatedType::Forced(a), AnnotatedType::Regular(b))
            | (AnnotatedType::Regular(b), AnnotatedType::Forced(a))
                if a.is_simple() && b.is_simple() =>
            {
                if b.can_coerce_to(a) {
                    Some(AnnotatedType::Regular(a.clone()))
                } else {
                    None
                }
            }
            // If at least one of them is not simple (it is a sum type), we can fall back
            // to standard behavior and unify to the union.
            (AnnotatedType::Forced(a), AnnotatedType::Regular(b))
            | (AnnotatedType::Regular(b), AnnotatedType::Forced(a)) => {
                ExprType::unify(a, b).map(AnnotatedType::Regular)
            }
            // We can unify untyped list to sum types if there is a single list type in
            // the sum type.
            (AnnotatedType::UntypedList, AnnotatedType::Forced(a))
            | (AnnotatedType::Forced(a), AnnotatedType::UntypedList) => {
                if a.contains_one_list() {
                    Some(AnnotatedType::Regular(a.clone()))
                } else {
                    None
                }
            }
            (AnnotatedType::UntypedList, AnnotatedType::Regular(a))
            | (AnnotatedType::Regular(a), AnnotatedType::UntypedList) => {
                if a.contains_one_list() {
                    Some(AnnotatedType::Regular(a.clone()))
                } else {
                    None
                }
            }
        }
    }

    pub fn loosen(&self) -> AnnotatedType {
        match self {
            AnnotatedType::Forced(a) => AnnotatedType::Regular(a.clone()),
            AnnotatedType::Regular(a) => AnnotatedType::Regular(a.clone()),
            AnnotatedType::UntypedList => AnnotatedType::UntypedList,
        }
    }

    pub fn enforce(&self) -> AnnotatedType {
        match self {
            AnnotatedType::Forced(a) => AnnotatedType::Forced(a.clone()),
            AnnotatedType::Regular(a) => AnnotatedType::Forced(a.clone()),
            AnnotatedType::UntypedList => AnnotatedType::UntypedList,
        }
    }
}

impl std::fmt::Display for AnnotatedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnotatedType::Forced(typ) => {
                write!(f, "{} (coercion forbidden)", typ)
            }
            AnnotatedType::Regular(typ) => {
                write!(f, "{}", typ)
            }
            AnnotatedType::UntypedList => {
                write!(f, "[<unknown>]")
            }
        }
    }
}

impl From<ExprType> for AnnotatedType {
    fn from(value: ExprType) -> Self {
        AnnotatedType::Regular(value)
    }
}

impl From<SimpleType> for AnnotatedType {
    fn from(value: SimpleType) -> Self {
        AnnotatedType::Regular(value.into())
    }
}

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
    pub docstring: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDesc {
    pub args: ArgsType,
    pub span: Span,
    pub used: bool,
    pub referenced_fn: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEnv {
    defined_types: HashMap<String, ObjectFields>,
    functions: HashMap<String, FunctionDesc>,
    external_variables: HashMap<String, ArgsType>,
    internal_variables: HashMap<String, VariableDesc>,
    variable_lists: HashMap<String, VariableDesc>,
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
    pub fn validate_simple_type(&self, typ: &SimpleType) -> bool {
        match typ {
            SimpleType::None => true,
            SimpleType::Bool => true,
            SimpleType::Int => true,
            SimpleType::LinExpr => true,
            SimpleType::Constraint => true,
            SimpleType::List(sub_typ) => self.validate_type(sub_typ),
            SimpleType::Object(typ_name) => self.defined_types.contains_key(typ_name),
        }
    }

    pub fn validate_type(&self, typ: &ExprType) -> bool {
        typ.variants.iter().all(|x| self.validate_simple_type(x))
    }

    pub fn get_functions(&self) -> &HashMap<String, FunctionDesc> {
        &self.functions
    }

    pub fn get_predefined_vars(&self) -> &HashMap<String, ArgsType> {
        &self.external_variables
    }

    pub fn get_vars(&self) -> &HashMap<String, VariableDesc> {
        &self.internal_variables
    }

    pub fn get_var_lists(&self) -> &HashMap<String, VariableDesc> {
        &self.variable_lists
    }

    pub fn get_types(&self) -> &HashMap<String, ObjectFields> {
        &self.defined_types
    }

    fn lookup_fn(&mut self, name: &str) -> Option<(FunctionType, Span)> {
        let fn_desc = self.functions.get_mut(name)?;
        fn_desc.used = true;
        Some((fn_desc.typ.clone(), fn_desc.body.span.clone()))
    }

    fn register_fn(
        &mut self,
        name: &str,
        name_span: Span,
        fn_typ: FunctionType,
        public: bool,
        arg_names: Vec<String>,
        body: Spanned<crate::ast::Expr>,
        docstring: Vec<String>,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.functions.contains_key(name));

        self.functions.insert(
            name.to_string(),
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

    fn lookup_var(&mut self, name: &str) -> Option<(ArgsType, Option<Span>)> {
        if let Some(ext_var) = self.external_variables.get(name) {
            return Some((ext_var.clone(), None));
        };

        let var_desc = self.internal_variables.get_mut(name)?;

        var_desc.used = true;

        Some((var_desc.args.clone(), Some(var_desc.span.clone())))
    }

    fn register_var(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        referenced_fn: String,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.external_variables.contains_key(name));
        assert!(!self.internal_variables.contains_key(name));

        self.internal_variables.insert(
            name.to_string(),
            VariableDesc {
                args: args_typ.clone(),
                span: span.clone(),
                used: should_be_used_by_default(name),
                referenced_fn,
            },
        );

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_var_list(&mut self, name: &str) -> Option<(ArgsType, Span)> {
        let var_desc = self.variable_lists.get_mut(name)?;

        var_desc.used = true;

        Some((var_desc.args.clone(), var_desc.span.clone()))
    }

    fn register_var_list(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        referenced_fn: String,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.variable_lists.contains_key(name));

        self.variable_lists.insert(
            name.to_string(),
            VariableDesc {
                args: args_typ.clone(),
                span: span.clone(),
                used: should_be_used_by_default(name),
                referenced_fn,
            },
        );

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_field(&self, obj_type: &str, field: &str) -> Option<ExprType> {
        self.defined_types.get(obj_type)?.get(field).cloned()
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
    #[error("TMultiple option markers '?' in type at {span:?}")]
    MultipleOptionMarkers { span: Span },
    #[error("Option types ('?') in sum type defined at {span:?}")]
    OptionTypeInSumType { span: Span },
    #[error("Sum type at {span:?} has multiple copies of the same type")]
    DuplicatedTypeInSumType { span: Span },
    #[error("Option type at {span:?} has an option marker on None")]
    OptionMarkerOnNone { span: Span },
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
        found: AnnotatedType,
    },
    #[error("Type mismatch at {span:?}: expected {expected} but found {found} ({context})")]
    TypeMismatch {
        span: Span,
        expected: AnnotatedType,
        found: AnnotatedType,
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
        typ: AnnotatedType,
        field: String,
        span: Span,
    },
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
struct LocalEnv {
    scopes: Vec<HashMap<String, (ExprType, Span, bool)>>,
    pending_scope: HashMap<String, (ExprType, Span, bool)>,
}

fn should_be_used_by_default(ident: &str) -> bool {
    assert!(ident.len() > 0);

    ident.chars().next().unwrap() == '_'
}

impl LocalEnv {
    fn new() -> Self {
        LocalEnv::default()
    }

    fn lookup_in_pending_scope(&self, ident: &str) -> Option<(ExprType, Span)> {
        self.pending_scope
            .get(ident)
            .map(|(typ, span, _used)| (typ.clone(), span.clone()))
    }

    fn lookup_ident(&mut self, ident: &str) -> Option<(ExprType, Span)> {
        // We don't look in pending scope as these variables are not yet accessible
        for scope in self.scopes.iter_mut().rev() {
            let Some((typ, span, used)) = scope.get_mut(ident) else {
                continue;
            };
            *used = true;
            return Some((typ.clone(), span.clone()));
        }
        None
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
        ident: &str,
        span: Span,
        typ: ExprType,
        type_info: &mut TypeInfo,
        warnings: &mut Vec<SemWarning>,
    ) {
        assert!(!self.pending_scope.contains_key(ident));

        if let Some((_typ, old_span)) = self.lookup_ident(ident) {
            warnings.push(SemWarning::IdentifierShadowed {
                identifier: ident.to_string(),
                span: span.clone(),
                previous: old_span,
            });
        }

        self.pending_scope.insert(
            ident.to_string(),
            (typ.clone(), span.clone(), should_be_used_by_default(ident)),
        );
        type_info.types.insert(span, typ.into());
    }

    fn check_expr(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &crate::ast::Expr,
        span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, AnnotatedType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<AnnotatedType> {
        let result =
            self.check_expr_internal(global_env, expr, type_info, expr_types, errors, warnings);
        if let Some(typ) = &result {
            expr_types.insert(span.clone(), typ.clone());
        }
        result
    }

    fn check_expr_internal(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &crate::ast::Expr,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, AnnotatedType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<AnnotatedType> {
        use crate::ast::Expr;

        match expr {
            // ========== Literals ==========
            Expr::None => Some(SimpleType::None.into()),
            Expr::Number(_) => Some(SimpleType::Int.into()),
            Expr::Boolean(_) => Some(SimpleType::Bool.into()),

            Expr::Ident(ident) => self
                .check_ident(
                    global_env,
                    &ident.node,
                    &ident.span,
                    type_info,
                    errors,
                    warnings,
                )
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
                // 'as' is forcing explicitly a coercion. So we don't care if we are
                // getting a forced type.
                //
                // This means that chained 'as' are valid (even if a bit odd):
                // "5 as Int as LinExpr"
                let loose_type = expr_type.map(|x| x.loosen());

                // Convert the declared type
                let target_type = match ExprType::try_from(typ.node.clone()) {
                    Ok(t) => t,
                    Err(TypeNameError::MultipleOptionMarkers) => {
                        errors.push(SemError::MultipleOptionMarkers {
                            span: typ.span.clone(),
                        });
                        return loose_type; // Fallback to inferred type
                                           // We don't make it forced as it might not correspond to desired type
                    }
                    Err(TypeNameError::OptionTypeInSumType) => {
                        errors.push(SemError::OptionTypeInSumType {
                            span: typ.span.clone(),
                        });
                        return loose_type; // Fallback to inferred type
                                           // We don't make it forced as it might not correspond to desired type
                    }
                    Err(TypeNameError::DuplicatedTypeInSumType) => {
                        errors.push(SemError::DuplicatedTypeInSumType {
                            span: typ.span.clone(),
                        });
                        return loose_type; // Fallback to inferred type
                                           // We don't make it forced as it might not correspond to desired type
                    }
                    Err(TypeNameError::OptionMarkerOnNone) => {
                        errors.push(SemError::OptionMarkerOnNone {
                            span: typ.span.clone(),
                        });
                        return loose_type; // Fallback to inferred type
                                           // We don't make it forced as it might not correspond to desired type
                    }
                };

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return loose_type; // Fallback to inferred type
                                       // We don't make it forced as it might not correspond to desired type
                }

                match loose_type {
                    Some(inferred) => {
                        // Check if the inferred type can coerce to the target type
                        if inferred.can_coerce_to(&target_type) {
                            // Success: use the target type
                            Some(AnnotatedType::Forced(target_type))
                        } else {
                            // Error: can't coerce
                            errors.push(SemError::TypeMismatch {
                                span: expr.span.clone(),
                                expected: target_type.clone().into(),
                                found: inferred,
                                context: "explicit type annotation".to_string(),
                            });
                            // Return target type anyway (user's intent is clear)
                            Some(AnnotatedType::Forced(target_type))
                        }
                    }
                    None => {
                        // Expression failed to type-check, but we have explicit type
                        // Use the explicit type as a hint for recovery
                        Some(AnnotatedType::Forced(target_type))
                    }
                }
            }

            // ========== Arithmetic Operations ==========
            // Int + Int -> Int
            // LinExpr + Int -> LinExpr (coerce Int to LinExpr)
            // Int + LinExpr -> LinExpr (coerce Int to LinExpr)
            // LinExpr + LinExpr -> LinExpr
            // [Type] + [Type] -> [Type]
            Expr::Add(left, right) | Expr::Sub(left, right) => {
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

                match (left_type.clone(), right_type) {
                    (Some(l), Some(r)) => match AnnotatedType::unify(&l, &r) {
                        Some(unified) if unified.is_arithmetic() || unified.is_list() => {
                            Some(unified)
                        }
                        _ => {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: l.clone(),
                                found: r.clone(),
                                context: format!(
                                    "addition/subtraction/concatenation requires Int, LinExpr or List, got {} and {}",
                                    l, r
                                ),
                            });
                            None
                        }
                    },
                    (Some(t), None) | (None, Some(t)) if t.is_arithmetic() || t.is_list() => {
                        Some(t)
                    }
                    (Some(t), None) | (None, Some(t)) => {
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
                                "addition/subtraction/concatenation requires Int or LinExpr or List"
                                    .to_string(),
                        });
                        None
                    }
                    (None, None) => None,
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

                match (left_type.clone(), right_type) {
                    (Some(l), Some(r)) => {
                        // Special case: LinExpr * LinExpr is non-linear
                        if l.matches(&SimpleType::LinExpr.into())
                            && r.matches(&SimpleType::LinExpr.into())
                        {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: SimpleType::LinExpr.into(),
                                context: "cannot multiply two linear expressions (non-linear)"
                                    .to_string(),
                            });
                            return Some(SimpleType::LinExpr.into()); // Fallback
                        }

                        // Try to unify (handles Int * Int, Int * LinExpr, LinExpr * Int)
                        match AnnotatedType::unify(&l, &r) {
                            Some(unified) if unified.is_arithmetic() => Some(unified),
                            _ => {
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.clone(),
                                    found: r.clone(),
                                    context: format!(
                                        "multiplication requires Int or LinExpr, got {} and {}",
                                        l, r
                                    ),
                                });
                                if l.is_arithmetic() {
                                    Some(l)
                                } else {
                                    Some(SimpleType::Int.into())
                                }
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) if t.is_arithmetic() => Some(t),
                    (Some(t), None) | (None, Some(t)) => {
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
                        Some(t) // Fallback
                    }
                    (None, None) => None,
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
                        // Check if both can coerce to Int
                        let l_ok = l.can_coerce_to(&SimpleType::Int.into());
                        let r_ok = r.can_coerce_to(&SimpleType::Int.into());

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
                        if !t.can_coerce_to(&SimpleType::Int.into()) {
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
                        if !t.can_coerce_to(&SimpleType::Int.into()) {
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

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both can coerce to LinExpr
                        let l_ok = l.can_coerce_to(&SimpleType::LinExpr.into());
                        let r_ok = r.can_coerce_to(&SimpleType::LinExpr.into());

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

                        // Always return Constraint (even on error, intent is clear)
                        Some(SimpleType::Constraint.into())
                    }
                    _ => {
                        // Something failed, but we know user wanted a constraint
                        Some(SimpleType::Constraint.into())
                    }
                }
            }

            // ========== Comparison Operations ==========
            // Int == Int -> Bool
            // LinExpr == LinExpr -> Constraint
            // LinExpr == Int -> Constraint (coerce Int to LinExpr)
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

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Try to unify the types
                        match AnnotatedType::unify(&l, &r) {
                            Some(_unified) => Some(SimpleType::Bool.into()),
                            None => {
                                // Types don't unify - incompatible
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.clone(),
                                    found: r,
                                    context: "equality comparison requires compatible types"
                                        .to_string(),
                                });
                                Some(SimpleType::Bool.into()) // Fallback
                            }
                        }
                    }
                    (Some(_), None) | (None, Some(_)) => Some(SimpleType::Bool.into()), // One side failed
                    (None, None) => None,
                }
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

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both can coerce to Int
                        let l_ok = l.can_coerce_to(&SimpleType::Int.into());
                        let r_ok = r.can_coerce_to(&SimpleType::Int.into());

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

                        // Always return Bool (even on error, intent is clear)
                        Some(SimpleType::Bool.into())
                    }
                    (Some(_), None) | (None, Some(_)) => Some(SimpleType::Bool.into()),
                    (None, None) => None,
                }
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

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Try to unify the types
                        match AnnotatedType::unify(&l, &r) {
                            Some(t) if t.matches(&SimpleType::Bool.into()) => {
                                Some(SimpleType::Bool.into())
                            }
                            Some(t) if t.matches(&SimpleType::Constraint.into()) => {
                                Some(SimpleType::Constraint.into())
                            }
                            Some(unified) => {
                                // Unified to something else - not valid for and/or
                                errors.push(SemError::TypeMismatch {
                                    span: left.span.clone(),
                                    expected: SimpleType::Bool.into(),
                                    found: unified,
                                    context: "and/or requires Bool or Constraint operands"
                                        .to_string(),
                                });
                                None
                            }
                            None => {
                                // Can't unify - incompatible types
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.clone(),
                                    found: r,
                                    context: "and/or requires both operands to have the same type (both Bool or both Constraint)".to_string(),
                                });
                                // Return whatever the left side was if valid
                                if l.matches(&SimpleType::Bool.into())
                                    || l.matches(&SimpleType::Constraint.into())
                                {
                                    Some(l)
                                } else {
                                    None
                                }
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) if t.matches(&SimpleType::Bool.into()) => {
                        Some(SimpleType::Bool.into())
                    }
                    (Some(t), None) | (None, Some(t))
                        if t.matches(&SimpleType::Constraint.into()) =>
                    {
                        Some(SimpleType::Constraint.into())
                    }
                    (Some(t), None) | (None, Some(t)) => {
                        // One side is not Bool/Constraint
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::Bool.into(),
                            found: t.clone(),
                            context: "and/or requires Bool or Constraint operands".to_string(),
                        });
                        None
                    }
                    (None, None) => None,
                }
            }

            Expr::Not(expr) => {
                let expr_type = self.check_expr(
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                );

                match expr_type {
                    Some(typ) if typ.can_coerce_to(&SimpleType::Bool.into()) => {
                        Some(SimpleType::Bool.into())
                    }
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
                    Some(AnnotatedType::Forced(a)) | Some(AnnotatedType::Regular(a))
                        if a.is_list() =>
                    {
                        let elem_t = a.to_inner_list_type().unwrap();
                        if let Some(item_t) = item_type {
                            // Check if item can coerce to the element type
                            if !item_t.can_coerce_to(&elem_t) {
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
                    Some(AnnotatedType::UntypedList) => {
                        // Can't check anything - we don't know the element type
                        // But it does not matter, an element is never in an empty collection
                        // So this will be false and the types don't matter.
                    }
                    Some(t) => {
                        // Not a list at all
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(
                                t.inner().expect("UntypedList case already handled").clone(),
                            )
                            .into(),
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
                    Some(AnnotatedType::Forced(a)) | Some(AnnotatedType::Regular(a))
                        if a.is_list() =>
                    {
                        let elem_t = a.to_inner_list_type().unwrap();
                        // Register the loop variable with the element type
                        self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        );
                    }
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context: "forall collection type must be known (use 'as' for explicit typing)".to_string(),
                        });
                        return Some(SimpleType::Constraint.into()); // Return early
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(
                                t.inner().expect("UntypedList case already handled").clone(),
                            )
                            .into(),
                            found: t,
                            context: "forall collection must be a list".to_string(),
                        });
                        return Some(SimpleType::Constraint.into()); // Return early
                    }
                    None => return None,
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
                        if !typ.can_coerce_to(&SimpleType::Bool.into()) {
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
                    Some(typ) if typ.can_coerce_to(&SimpleType::Constraint.into()) => {
                        Some(SimpleType::Constraint.into())
                    }
                    Some(typ) if typ.can_coerce_to(&SimpleType::Bool.into()) => {
                        Some(SimpleType::Bool.into())
                    }
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
                    Some(AnnotatedType::Forced(a)) | Some(AnnotatedType::Regular(a))
                        if a.is_list() =>
                    {
                        let elem_t = a.to_inner_list_type().unwrap();
                        // Register the loop variable with the element type
                        self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        );
                    }
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context:
                                "sum collection type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        return None; // Return early
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(
                                t.inner().expect("UntypedList case already handled").clone(),
                            )
                            .into(),
                            found: t,
                            context: "sum collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None,
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
                        if !typ.can_coerce_to(&SimpleType::Bool.into()) {
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
                    Some(typ) if typ.is_arithmetic() => Some(typ), // Return Int or LinExpr
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: typ,
                            context: "sum body must be Int or LinExpr".to_string(),
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

                let annotated_acc_type = self.check_expr(
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

                // Extract type info
                match coll_type {
                    Some(AnnotatedType::Forced(a)) | Some(AnnotatedType::Regular(a))
                        if a.is_list() =>
                    {
                        let elem_t = a.to_inner_list_type().unwrap();
                        // Register the loop variable with the element type
                        self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        );
                    }
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context:
                                "sum collection type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        return None; // Return early
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(
                                t.inner().expect("UntypedList case already handled").clone(),
                            )
                            .into(),
                            found: t,
                            context: "sum collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None,
                }

                let acc_type = match annotated_acc_type {
                    Some(AnnotatedType::Forced(t)) | Some(AnnotatedType::Regular(t)) => {
                        // Register the acc variable with the init_value type
                        self.register_identifier(
                            &accumulator.node,
                            accumulator.span.clone(),
                            t.clone(),
                            type_info,
                            warnings,
                        );
                        t
                    }
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: accumulator.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context:
                                "accumulator type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None,
                };

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
                        if !typ.can_coerce_to(&SimpleType::Bool.into()) {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "sum filter must be Bool".to_string(),
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
                    Some(typ) if typ.can_coerce_to(&acc_type) => Some(acc_type.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: acc_type.clone().into(),
                            found: typ,
                            context: "fold|rfold body must match accumulator type".to_string(),
                        });
                        Some(acc_type.into())
                    }
                    None => Some(acc_type.into()), // Intent is expressed by accumulator
                }
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
                    if !typ.can_coerce_to(&SimpleType::Bool.into()) {
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
                    (Some(t), Some(e)) => {
                        match AnnotatedType::unify(&t, &e) {
                            Some(unified) => Some(unified),
                            None => {
                                errors.push(SemError::TypeMismatch {
                                    span: else_expr.span.clone(),
                                    expected: t.clone(),
                                    found: e,
                                    context: "if branches must have compatible types".to_string(),
                                });
                                Some(t) // Fallback to then type
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
            }

            // ========== ILP Variables ==========
            Expr::VarCall { name, args } => {
                match global_env.lookup_var(&name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::LinExpr.into()) // Syntax indicates LinExpr intent
                    }
                    Some((var_args, _)) => {
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
                                if !found_type.can_coerce_to(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone().into(),
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
                match global_env.lookup_var_list(&name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
                        // Syntax indicates [LinExpr] intent
                    }
                    Some((var_args, _)) => {
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
                                if !found_type.can_coerce_to(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone().into(),
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

            // ========== Function Calls ==========
            Expr::FnCall { name, args } => match global_env.lookup_fn(&name.node) {
                None => {
                    errors.push(SemError::UnknownIdentifer {
                        identifier: name.node.clone(),
                        span: name.span.clone(),
                    });
                    None
                }
                Some((fn_type, _)) => {
                    if args.len() != fn_type.args.len() {
                        errors.push(SemError::ArgumentCountMismatch {
                            identifier: name.node.clone(),
                            span: args
                                .last()
                                .map(|a| a.span.clone())
                                .unwrap_or_else(|| name.span.clone()),
                            expected: fn_type.args.len(),
                            found: args.len(),
                        });
                    }

                    for (i, (arg, expected_type)) in
                        args.iter().zip(fn_type.args.iter()).enumerate()
                    {
                        let arg_type = self.check_expr(
                            global_env, &arg.node, &arg.span, type_info, expr_types, errors,
                            warnings,
                        );

                        if let Some(found_type) = arg_type {
                            if !found_type.can_coerce_to(expected_type) {
                                errors.push(SemError::TypeMismatch {
                                    span: arg.span.clone(),
                                    expected: expected_type.clone().into(),
                                    found: found_type,
                                    context: format!(
                                        "argument {} to function {}",
                                        i + 1,
                                        name.node
                                    ),
                                });
                            }
                        }
                    }

                    Some(fn_type.output.into())
                }
            },

            // ========== Collections ==========
            Expr::GlobalList(type_name) => {
                let typ = match ExprType::try_from(type_name.node.clone()) {
                    Ok(t) => t,
                    Err(TypeNameError::MultipleOptionMarkers) => {
                        errors.push(SemError::MultipleOptionMarkers {
                            span: type_name.span.clone(),
                        });
                        return None;
                    }
                    Err(TypeNameError::OptionTypeInSumType) => {
                        errors.push(SemError::OptionTypeInSumType {
                            span: type_name.span.clone(),
                        });
                        return None;
                    }
                    Err(TypeNameError::DuplicatedTypeInSumType) => {
                        errors.push(SemError::DuplicatedTypeInSumType {
                            span: type_name.span.clone(),
                        });
                        return None;
                    }
                    Err(TypeNameError::OptionMarkerOnNone) => {
                        errors.push(SemError::OptionMarkerOnNone {
                            span: type_name.span.clone(),
                        });
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
                    return Some(AnnotatedType::UntypedList);
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
                            match AnnotatedType::unify(&u, &i) {
                                Some(new_unified) => unified_type = Some(new_unified),
                                None => {
                                    errors.push(SemError::TypeMismatch {
                                        span: item.span.clone(),
                                        expected: u.clone(),
                                        found: i,
                                        context: "all list elements must have compatible types"
                                            .to_string(),
                                    });
                                    // Keep the current unified type for further checking
                                }
                            }
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
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: elements[0].span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context:
                                "inner elements type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        None
                    }
                    Some(AnnotatedType::Forced(t)) | Some(AnnotatedType::Regular(t)) => {
                        Some(SimpleType::List(t).into())
                    }
                    None => None,
                }
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

                match (start_type, end_type) {
                    (Some(s), Some(e)) => {
                        // Check if both can coerce to Int
                        let s_ok = s.can_coerce_to(&SimpleType::Int.into());
                        let e_ok = e.can_coerce_to(&SimpleType::Int.into());

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

                        // Always return [Int] (even on error, intent is clear)
                        Some(SimpleType::List(SimpleType::Int.into()).into())
                    }
                    (Some(_), None) | (None, Some(_)) => {
                        Some(SimpleType::List(SimpleType::Int.into()).into())
                    }
                    (None, None) => None,
                }
            }

            Expr::ListComprehension {
                body: expr,
                vars_and_collections,
                filter,
            } => {
                let mut typ_error = false;
                for (var, collection) in vars_and_collections {
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
                        Some(AnnotatedType::Forced(a)) | Some(AnnotatedType::Regular(a))
                            if a.is_list() =>
                        {
                            let elem_t = a.to_inner_list_type().unwrap();
                            // Register the loop variable with the element type
                            self.register_identifier(
                                &var.node,
                                var.span.clone(),
                                elem_t,
                                type_info,
                                warnings,
                            );
                        }
                        Some(AnnotatedType::UntypedList) => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                                found: AnnotatedType::UntypedList,
                                context: "list comprehension collection type must be known (use 'as' for explicit typing)".to_string(),
                            });
                            typ_error = true;
                        }
                        Some(t) => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: SimpleType::List(
                                    t.inner().expect("UntypedList case already handled").clone(),
                                )
                                .into(),
                                found: t,
                                context: "list comprehension collection must be a list".to_string(),
                            });
                            typ_error = true;
                        }
                        None => typ_error = true,
                    }

                    self.push_scope();
                }

                let elem_type = if !typ_error {
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
                            if !typ.can_coerce_to(&SimpleType::Bool.into()) {
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
                    self.check_expr(
                        global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                    )
                } else {
                    None
                };

                for (_var, _collection) in vars_and_collections {
                    self.pop_scope(warnings);
                }

                match elem_type {
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context:
                                "inner elements type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        None
                    }
                    Some(AnnotatedType::Forced(t)) | Some(AnnotatedType::Regular(t)) => {
                        Some(SimpleType::List(t).into())
                    }
                    None => None,
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
                            expected: SimpleType::List(
                                t.inner().expect("UntypedList case already handled").clone(),
                            )
                            .into(),
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
                match value_type {
                    Some(AnnotatedType::Forced(typ)) | Some(AnnotatedType::Regular(typ)) => {
                        self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            typ,
                            type_info,
                            warnings,
                        );
                    }
                    Some(AnnotatedType::UntypedList) => {
                        errors.push(SemError::TypeMismatch {
                            span: value.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(), // placeholder
                            found: AnnotatedType::UntypedList,
                            context: "variable type must be known (use 'as' for explicit typing)"
                                .to_string(),
                        });
                        return None;
                    }
                    None => return None,
                }

                self.push_scope();

                let body_type = self.check_expr(
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
                );

                self.pop_scope(warnings);

                body_type
            }
        }
    }

    fn check_ident(
        &mut self,
        _global_env: &GlobalEnv,
        ident: &String,
        span: &Span,
        _type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        _warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        let typ = match self.lookup_ident(&ident) {
            Some((typ, _)) => typ,
            None => {
                errors.push(SemError::UnknownIdentifer {
                    identifier: ident.clone(),
                    span: span.clone(),
                });
                return None;
            }
        };

        Some(typ)
    }

    fn check_path(
        &mut self,
        global_env: &mut GlobalEnv,
        object: &Spanned<Expr>,
        segments: &Vec<Spanned<String>>,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, AnnotatedType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        assert!(!segments.is_empty(), "Path must have at least one segment");

        // First segment can be an expression
        let initial_expr_type = self.check_expr(
            global_env,
            &object.node,
            &object.span,
            type_info,
            expr_types,
            errors,
            warnings,
        )?;

        let mut current_type = match initial_expr_type {
            AnnotatedType::Forced(t) | AnnotatedType::Regular(t) => t,
            AnnotatedType::UntypedList => {
                // Can't access fields on non-object types
                errors.push(SemError::FieldAccessOnNonObject {
                    typ: AnnotatedType::UntypedList,
                    field: segments[0].node.clone(),
                    span: segments[0].span.clone(),
                });
                return None;
            }
        };

        // Follow the path through fields
        for segment in segments {
            match &current_type {
                a if a.is_object() => {
                    let type_name = a.get_inner_object_type().unwrap();
                    // Look up the field in this object type
                    match global_env.lookup_field(type_name, &segment.node) {
                        Some(field_type) => {
                            current_type = field_type;
                        }
                        None => {
                            errors.push(SemError::UnknownField {
                                object_type: type_name.clone(),
                                field: segment.node.clone(),
                                span: segment.span.clone(),
                            });
                            return None;
                        }
                    }
                }
                _ => {
                    // Can't access fields on non-object types
                    errors.push(SemError::FieldAccessOnNonObject {
                        typ: current_type.clone().into(),
                        field: segment.node.clone(),
                        span: segment.span.clone(),
                    });
                    return None;
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
        defined_types: HashMap<String, ObjectFields>,
        variables: HashMap<String, ArgsType>,
        file: &crate::ast::File,
    ) -> Result<
        (
            Self,
            TypeInfo,
            HashMap<Span, AnnotatedType>,
            Vec<SemError>,
            Vec<SemWarning>,
        ),
        GlobalEnvError,
    > {
        let mut temp_env = GlobalEnv {
            defined_types,
            functions: HashMap::new(),
            external_variables: variables
                .into_iter()
                .map(|(var_name, args_type)| (var_name, args_type))
                .collect(),
            internal_variables: HashMap::new(),
            variable_lists: HashMap::new(),
        };

        for (object_type, field_desc) in &temp_env.defined_types {
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

        for statement in &file.statements {
            temp_env.expand_with_statement(
                &statement.node,
                &mut type_info,
                &mut expr_types,
                &mut errors,
                &mut warnings,
            );
        }

        temp_env.check_unused_fn(&mut warnings);
        temp_env.check_unused_var(&mut warnings);

        Ok((temp_env, type_info, expr_types, errors, warnings))
    }

    fn check_unused_fn(&self, warnings: &mut Vec<SemWarning>) {
        for (name, fn_desc) in &self.functions {
            if !fn_desc.public && !fn_desc.used {
                warnings.push(SemWarning::UnusedFunction {
                    identifier: name.clone(),
                    span: fn_desc.body.span.clone(),
                });
            }
        }
    }

    fn check_unused_var(&self, warnings: &mut Vec<SemWarning>) {
        for (name, var_desc) in &self.internal_variables {
            if !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: name.clone(),
                    span: var_desc.span.clone(),
                });
            }
        }

        for (name, var_desc) in &self.variable_lists {
            if !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: name.clone(),
                    span: var_desc.span.clone(),
                });
            }
        }
    }

    fn expand_with_statement(
        &mut self,
        statement: &crate::ast::Statement,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, AnnotatedType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match statement {
            crate::ast::Statement::Let {
                docstring,
                public,
                name,
                params,
                output_type,
                body,
            } => self.expand_with_let_statement(
                *public,
                name,
                params,
                output_type,
                body,
                docstring,
                type_info,
                expr_types,
                errors,
                warnings,
            ),
            crate::ast::Statement::Reify {
                docstring: _,
                constraint_name,
                name,
                var_list,
            } => self.expand_with_reify_statement(
                constraint_name,
                name,
                *var_list,
                type_info,
                errors,
                warnings,
            ),
        }
    }

    fn expand_with_let_statement(
        &mut self,
        public: bool,
        name: &Spanned<String>,
        params: &Vec<Param>,
        output_type: &Spanned<crate::ast::TypeName>,
        body: &Spanned<Expr>,
        docstring: &Vec<String>,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, AnnotatedType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match self.lookup_fn(&name.node) {
            Some((_fn_type, span)) => {
                errors.push(SemError::FunctionAlreadyDefined {
                    identifier: name.node.clone(),
                    span: name.span.clone(),
                    here: span.clone(),
                });
            }
            None => {
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

                let mut local_env = LocalEnv::new();
                let mut error_in_typs = false;
                let mut params_typ = vec![];
                for param in params {
                    match ExprType::try_from(param.typ.node.clone()) {
                        Err(TypeNameError::MultipleOptionMarkers) => {
                            errors.push(SemError::MultipleOptionMarkers {
                                span: param.typ.span.clone(),
                            });
                            error_in_typs = true;
                        }
                        Err(TypeNameError::OptionTypeInSumType) => {
                            errors.push(SemError::OptionTypeInSumType {
                                span: param.typ.span.clone(),
                            });
                            error_in_typs = true;
                        }
                        Err(TypeNameError::DuplicatedTypeInSumType) => {
                            errors.push(SemError::DuplicatedTypeInSumType {
                                span: param.typ.span.clone(),
                            });
                            error_in_typs = true;
                        }
                        Err(TypeNameError::OptionMarkerOnNone) => {
                            errors.push(SemError::OptionMarkerOnNone {
                                span: param.typ.span.clone(),
                            });
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
                            } else if let Some((_typ, span)) =
                                local_env.lookup_in_pending_scope(&param.name.node)
                            {
                                errors.push(SemError::ParameterAlreadyDefined {
                                    identifier: param.name.node.clone(),
                                    span: param.name.span.clone(),
                                    here: span,
                                });
                            } else {
                                if let Some(suggestion) =
                                    string_case::generate_suggestion_for_naming_convention(
                                        &param.name.node,
                                        string_case::NamingConvention::SnakeCase,
                                    )
                                {
                                    warnings.push(SemWarning::ParameterNamingConvention {
                                        identifier: param.name.node.clone(),
                                        span: param.name.span.clone(),
                                        suggestion,
                                    });
                                }
                                local_env.register_identifier(
                                    &param.name.node,
                                    param.name.span.clone(),
                                    param_typ,
                                    type_info,
                                    warnings,
                                );
                            }
                        }
                    }
                }

                local_env.push_scope();
                let body_type_opt = local_env.check_expr(
                    self, &body.node, &body.span, type_info, expr_types, errors, warnings,
                );
                local_env.pop_scope(warnings);

                match ExprType::try_from(output_type.node.clone()) {
                    Err(TypeNameError::MultipleOptionMarkers) => {
                        errors.push(SemError::MultipleOptionMarkers {
                            span: output_type.span.clone(),
                        });
                    }
                    Err(TypeNameError::OptionTypeInSumType) => {
                        errors.push(SemError::OptionTypeInSumType {
                            span: output_type.span.clone(),
                        });
                    }
                    Err(TypeNameError::DuplicatedTypeInSumType) => {
                        errors.push(SemError::DuplicatedTypeInSumType {
                            span: output_type.span.clone(),
                        });
                    }
                    Err(TypeNameError::OptionMarkerOnNone) => {
                        errors.push(SemError::OptionMarkerOnNone {
                            span: output_type.span.clone(),
                        });
                    }
                    Ok(out_typ) => {
                        if !self.validate_type(&out_typ) {
                            errors.push(SemError::UnknownType {
                                typ: out_typ.to_string(),
                                span: output_type.span.clone(),
                            });
                        } else {
                            if let Some(body_type) = body_type_opt {
                                // Allow coercion
                                let types_match = match (out_typ.clone(), body_type.clone()) {
                                    (a, b) if b.can_coerce_to(&a) => true,
                                    _ => false,
                                };

                                if !types_match {
                                    errors.push(SemError::BodyTypeMismatch {
                                        func: name.node.clone(),
                                        span: body.span.clone(),
                                        expected: out_typ.clone(),
                                        found: body_type,
                                    });
                                }
                            }
                        }

                        if !error_in_typs {
                            let fn_typ = FunctionType {
                                args: params_typ,
                                output: out_typ,
                            };
                            self.register_fn(
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
                };
            }
        }
    }

    fn expand_with_reify_statement(
        &mut self,
        constraint_name: &Spanned<String>,
        name: &Spanned<String>,
        var_list: bool,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match self.lookup_fn(&constraint_name.node) {
            None => errors.push(SemError::UnknownIdentifer {
                identifier: constraint_name.node.clone(),
                span: constraint_name.span.clone(),
            }),
            Some(fn_type) => {
                let needed_output_type = if var_list {
                    SimpleType::List(SimpleType::Constraint.into())
                } else {
                    SimpleType::Constraint
                }
                .into();
                let correct_type = fn_type.0.output.can_coerce_to(&needed_output_type);
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
                    match self.lookup_var_list(&name.node) {
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
                                &name.node,
                                fn_type.0.args.clone(),
                                name.span.clone(),
                                constraint_name.node.clone(),
                                type_info,
                            );
                        }
                    }
                } else {
                    match self.lookup_var(&name.node) {
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
                                &name.node,
                                fn_type.0.args.clone(),
                                name.span.clone(),
                                constraint_name.node.clone(),
                                type_info,
                            );
                        }
                    }
                }
            }
        }
    }
}
