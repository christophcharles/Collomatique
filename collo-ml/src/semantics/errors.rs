use super::types::{ConcreteType, ExprType, SimpleType};
use crate::ast::Span;
use thiserror::Error;

/// Type used to represent function signatures
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

#[derive(Debug, Error, Clone)]
pub enum SemError {
    #[error("Unknown identifier \"{identifier}\" in module \"{module}\" at {span:?}")]
    UnknownIdentifer {
        module: String,
        identifier: String,
        span: Span,
    },
    #[error("Unknown variable \"{var}\" in module \"{module}\" at {span:?}")]
    UnknownVariable {
        module: String,
        var: String,
        span: Span,
    },
    #[error("Function type mismatch in module \"{module}\": \"{identifier}\" at {span:?} has type {found} but type {expected} expected.")]
    FunctionTypeMismatch {
        module: String,
        identifier: String,
        span: Span,
        expected: FunctionType,
        found: FunctionType,
    },
    #[error("Variable \"{identifier}\" in module \"{module}\" at {span:?} is already defined ({here:?})")]
    VariableAlreadyDefined {
        module: String,
        identifier: String,
        span: Span,
        here: Option<Span>,
    },
    #[error("Function \"{identifier}\" in module \"{module}\" at {span:?} is already defined ({here:?})")]
    FunctionAlreadyDefined {
        module: String,
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Type {typ} in module \"{module}\" at {span:?} is unknown")]
    UnknownType {
        module: String,
        typ: String,
        span: Span,
    },
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
    #[error("Parameter \"{identifier}\" in module \"{module}\" is already defined ({here:?}).")]
    ParameterAlreadyDefined {
        module: String,
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
        "Duplicate field \"{field}\" in module \"{module}\" in struct literal at {span:?} (first defined at {previous:?})"
    )]
    DuplicateStructField {
        module: String,
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
    #[error("Local variable \"{identifier}\" in module \"{module}\" at {span:?} is already defined in the same scope ({here:?})")]
    LocalIdentAlreadyDeclared {
        module: String,
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Local variable \"{identifier}\" in module \"{module}\" at {span:?} shadows a function with the same name")]
    LocalIdentShadowsFunction {
        module: String,
        identifier: String,
        span: Span,
    },
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
    #[error("Type \"{type_name}\" in module \"{module}\" at {span:?} shadows a primitive type")]
    TypeShadowsPrimitive {
        module: String,
        type_name: String,
        span: Span,
    },
    #[error("Type \"{type_name}\" in module \"{module}\" at {span:?} shadows an object type")]
    TypeShadowsObject {
        module: String,
        type_name: String,
        span: Span,
    },
    #[error("Type \"{type_name}\" in module \"{module}\" at {span:?} shadows a previously defined custom type")]
    TypeShadowsCustomType {
        module: String,
        type_name: String,
        span: Span,
    },
    #[error(
        "Type \"{type_name}\" in module \"{module}\" at {span:?} has unguarded recursion (must be inside a list or tuple)"
    )]
    UnguardedRecursiveType {
        module: String,
        type_name: String,
        span: Span,
    },
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
    #[error("Primitive type \"{type_name}\" in module \"{module}\" at {span:?} cannot be used as a value (use a conversion like {type_name}(x))")]
    PrimitiveTypeAsValue {
        module: String,
        type_name: String,
        span: Span,
    },
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
    #[error("Identifier \"{identifier}\" in module \"{module}\" at {span:?} shadows previous definition at {previous:?}")]
    IdentifierShadowed {
        module: String,
        identifier: String,
        span: Span,
        previous: Span,
    },

    #[error(
        "Function \"{identifier}\" in module \"{module}\" at {span:?} should use snake_case (consider \"{suggestion}\")"
    )]
    FunctionNamingConvention {
        module: String,
        identifier: String,
        span: Span,
        suggestion: String,
    },

    #[error(
        "Variable \"{identifier}\" in module \"{module}\" at {span:?} should use PascalCase (consider \"{suggestion}\")"
    )]
    VariableNamingConvention {
        module: String,
        identifier: String,
        span: Span,
        suggestion: String,
    },

    #[error(
        "Parameter \"{identifier}\" in module \"{module}\" at {span:?} should use snake_case (consider \"{suggestion}\")"
    )]
    ParameterNamingConvention {
        module: String,
        identifier: String,
        span: Span,
        suggestion: String,
    },
    #[error("Unused identifier \"{identifier}\" in module \"{module}\" at {span:?}")]
    UnusedIdentifier {
        module: String,
        identifier: String,
        span: Span,
    },
    #[error("Unused function \"{identifier}\" in module \"{module}\" at {span:?}")]
    UnusedFunction {
        module: String,
        identifier: String,
        span: Span,
    },
    #[error("Unused variable \"{identifier}\" in module \"{module}\" at {span:?}")]
    UnusedVariable {
        module: String,
        identifier: String,
        span: Span,
    },
}
