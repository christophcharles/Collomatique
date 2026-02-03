use super::{MatchBranch, NamespacePath, PathSegment, Spanned, TypeName};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    // Quantifiers
    Forall {
        var: Spanned<String>,
        collection: Arc<Spanned<Expr>>,
        filter: Option<Arc<Spanned<Expr>>>,
        body: Arc<Spanned<Expr>>,
    },
    Sum {
        var: Spanned<String>,
        collection: Arc<Spanned<Expr>>,
        filter: Option<Arc<Spanned<Expr>>>,
        body: Arc<Spanned<Expr>>,
    },
    Fold {
        var: Spanned<String>,
        collection: Arc<Spanned<Expr>>,
        accumulator: Spanned<String>,
        init_value: Arc<Spanned<Expr>>,
        filter: Option<Arc<Spanned<Expr>>>,
        body: Arc<Spanned<Expr>>,
        reversed: bool,
    },

    // branches
    If {
        condition: Arc<Spanned<Expr>>,
        then_expr: Arc<Spanned<Expr>>,
        else_expr: Arc<Spanned<Expr>>,
    },
    Match {
        match_expr: Arc<Spanned<Expr>>,
        branches: Vec<MatchBranch>,
    },

    // Expression Let
    Let {
        var: Spanned<String>,
        value: Arc<Spanned<Expr>>,
        body: Arc<Spanned<Expr>>,
    },

    // Calls
    /// Generic call: func(args), Type(value), Enum::Variant(value), mod::func(args)
    /// Unifies fn_call, qualified_type_cast, and module-qualified function calls.
    GenericCall {
        path: Spanned<NamespacePath>,
        args: Vec<Arc<Spanned<Expr>>>,
    },
    /// Variable call: $Var(args) or mod::$Var(args)
    VarCall {
        module: Option<Spanned<String>>,
        name: Spanned<String>,
        args: Vec<Arc<Spanned<Expr>>>,
    },
    /// Variable list call: $[VarList](args) or mod::$[VarList](args)
    VarListCall {
        module: Option<Spanned<String>>,
        name: Spanned<String>,
        args: Vec<Arc<Spanned<Expr>>>,
    },

    // Elements
    None,
    Number(i32),
    Boolean(bool),
    StringLiteral(String),
    /// Identifier path: variable reference, unit variant (Option::None), or qualified path
    /// Single segment = variable or primitive type error, multiple segments = enum unit variant
    IdentPath(Spanned<NamespacePath>),
    Path {
        object: Arc<Spanned<Expr>>, // first segment might be an expression - for "get_group().student.age" this is "get_group()"
        segments: Vec<Spanned<PathSegment>>, // and this is [Field("student"), Field("age")] or [TupleIndex(0)]
    },
    TupleLiteral {
        elements: Vec<Arc<Spanned<Expr>>>, // (expr, expr, ...) - at least 2 elements
    },
    StructLiteral {
        fields: Vec<(Spanned<String>, Arc<Spanned<Expr>>)>, // {field1: expr1, field2: expr2}
    },

    // Arithmetic
    Add(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Sub(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Mul(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Div(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>), // //
    Mod(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>), // %
    Neg(Arc<Spanned<Expr>>),

    // Comparisons
    Eq(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Ne(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Lt(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Le(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Gt(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Ge(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),

    // Constraint building
    ConstraintEq(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    ConstraintLe(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    ConstraintGe(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),

    // Boolean operations
    And(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Or(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),
    Not(Arc<Spanned<Expr>>),

    // Null coalescing
    NullCoalesce(Arc<Spanned<Expr>>, Arc<Spanned<Expr>>),

    // Control flow
    Panic(Arc<Spanned<Expr>>),

    // Collection specific
    In {
        item: Arc<Spanned<Expr>>,
        collection: Arc<Spanned<Expr>>,
    },

    GlobalList(Spanned<TypeName>),
    ListLiteral {
        elements: Vec<Arc<Spanned<Expr>>>,
    },
    ListRange {
        start: Arc<Spanned<Expr>>,
        end: Arc<Spanned<Expr>>,
    },
    ListComprehension {
        body: Arc<Spanned<Expr>>,
        vars_and_collections: Vec<(Spanned<String>, Arc<Spanned<Expr>>)>,
        filter: Option<Arc<Spanned<Expr>>>,
    },

    Cardinality(Arc<Spanned<Expr>>),

    // Typed term
    ExplicitType {
        expr: Arc<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },

    // Narrowing casts
    CastFallible {
        expr: Arc<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },
    CastPanic {
        expr: Arc<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },

    // Type cast with complex type: [LinExpr]([1,2,3]), (Int,Bool)(1,true)
    ComplexTypeCast {
        typ: Spanned<TypeName>,
        args: Vec<Arc<Spanned<Expr>>>,
    },

    /// Struct-style call: Type{fields}, Enum::Variant{fields}
    /// Unifies struct_type_cast and qualified_struct_cast.
    StructCall {
        path: Spanned<NamespacePath>,
        fields: Vec<(Spanned<String>, Arc<Spanned<Expr>>)>,
    },
}
