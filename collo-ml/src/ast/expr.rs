use super::{MatchBranch, NamespacePath, PathSegment, Spanned, TypeName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    // Quantifiers
    Forall {
        var: Spanned<String>,
        collection: Box<Spanned<Expr>>,
        filter: Option<Box<Spanned<Expr>>>,
        body: Box<Spanned<Expr>>,
    },
    Sum {
        var: Spanned<String>,
        collection: Box<Spanned<Expr>>,
        filter: Option<Box<Spanned<Expr>>>,
        body: Box<Spanned<Expr>>,
    },
    Fold {
        var: Spanned<String>,
        collection: Box<Spanned<Expr>>,
        accumulator: Spanned<String>,
        init_value: Box<Spanned<Expr>>,
        filter: Option<Box<Spanned<Expr>>>,
        body: Box<Spanned<Expr>>,
        reversed: bool,
    },

    // branches
    If {
        condition: Box<Spanned<Expr>>,
        then_expr: Box<Spanned<Expr>>,
        else_expr: Box<Spanned<Expr>>,
    },
    Match {
        match_expr: Box<Spanned<Expr>>,
        branches: Vec<MatchBranch>,
    },

    // Expression Let
    Let {
        var: Spanned<String>,
        value: Box<Spanned<Expr>>,
        body: Box<Spanned<Expr>>,
    },

    // Calls
    /// Generic call: func(args), Type(value), Enum::Variant(value), mod::func(args)
    /// Unifies fn_call, qualified_type_cast, and module-qualified function calls.
    GenericCall {
        path: Spanned<NamespacePath>,
        args: Vec<Spanned<Expr>>,
    },
    /// Variable call: $Var(args) or mod::$Var(args)
    VarCall {
        module: Option<Spanned<String>>,
        name: Spanned<String>,
        args: Vec<Spanned<Expr>>,
    },
    /// Variable list call: $[VarList](args) or mod::$[VarList](args)
    VarListCall {
        module: Option<Spanned<String>>,
        name: Spanned<String>,
        args: Vec<Spanned<Expr>>,
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
        object: Box<Spanned<Expr>>, // first segment might be an expression - for "get_group().student.age" this is "get_group()"
        segments: Vec<Spanned<PathSegment>>, // and this is [Field("student"), Field("age")] or [TupleIndex(0)]
    },
    TupleLiteral {
        elements: Vec<Spanned<Expr>>, // (expr, expr, ...) - at least 2 elements
    },
    StructLiteral {
        fields: Vec<(Spanned<String>, Spanned<Expr>)>, // {field1: expr1, field2: expr2}
    },

    // Arithmetic
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Sub(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Mul(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Div(Box<Spanned<Expr>>, Box<Spanned<Expr>>), // //
    Mod(Box<Spanned<Expr>>, Box<Spanned<Expr>>), // %
    Neg(Box<Spanned<Expr>>),

    // Comparisons
    Eq(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Ne(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Lt(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Le(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Gt(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Ge(Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    // Constraint building
    ConstraintEq(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    ConstraintLe(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    ConstraintGe(Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    // Boolean operations
    And(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Or(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Not(Box<Spanned<Expr>>),

    // Null coalescing
    NullCoalesce(Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    // Control flow
    Panic(Box<Spanned<Expr>>),

    // Collection specific
    In {
        item: Box<Spanned<Expr>>,
        collection: Box<Spanned<Expr>>,
    },

    GlobalList(Spanned<TypeName>),
    ListLiteral {
        elements: Vec<Spanned<Expr>>,
    },
    ListRange {
        start: Box<Spanned<Expr>>,
        end: Box<Spanned<Expr>>,
    },
    ListComprehension {
        body: Box<Spanned<Expr>>,
        vars_and_collections: Vec<(Spanned<String>, Spanned<Expr>)>,
        filter: Option<Box<Spanned<Expr>>>,
    },

    Cardinality(Box<Spanned<Expr>>),

    // Typed term
    ExplicitType {
        expr: Box<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },

    // Narrowing casts
    CastFallible {
        expr: Box<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },
    CastPanic {
        expr: Box<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },

    // Type cast with complex type: [LinExpr]([1,2,3]), (Int,Bool)(1,true)
    ComplexTypeCast {
        typ: Spanned<TypeName>,
        args: Vec<Spanned<Expr>>,
    },

    /// Struct-style call: Type{fields}, Enum::Variant{fields}
    /// Unifies struct_type_cast and qualified_struct_cast.
    StructCall {
        path: Spanned<NamespacePath>,
        fields: Vec<(Spanned<String>, Spanned<Expr>)>,
    },
}
