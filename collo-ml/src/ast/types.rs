use super::{Expr, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub statements: Vec<Spanned<Statement>>,
}

impl File {
    pub fn new() -> Self {
        File { statements: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let {
        docstring: Vec<DocstringLine>,
        public: bool,
        name: Spanned<String>,
        params: Vec<Param>,
        output_type: Spanned<TypeName>, // Declared type
        body: Spanned<Expr>,            // Body
    },
    Reify {
        docstring: Vec<DocstringLine>,
        public: bool,
        constraint_path: Spanned<NamespacePath>,
        var_list: bool,
        name: Spanned<String>,
    },
    TypeDecl {
        public: bool,
        name: Spanned<String>,
        underlying: Spanned<TypeName>,
    },
    EnumDecl {
        public: bool,
        name: Spanned<String>,
        variants: Vec<Spanned<EnumVariant>>,
    },
    /// Import statement: import "module_name" as mod; or import "module_name" as *;
    Import {
        module_path: Spanned<String>,
        alias: ImportAlias,
    },
}

/// Import alias for import statements
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportAlias {
    /// Named import: import "foo" as bar;
    Named(Spanned<String>),
    /// Wildcard import: import "foo" as *;
    Wildcard(Span),
}

/// Represents a single enum variant
/// e.g., Ok(Int), Error(String), None, TupleCase(Int, Bool), StructCase { field: Int }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: Spanned<String>,
    /// The underlying type for this variant:
    /// - None for unit variants (no payload)
    /// - Some with single type for simple variants like Ok(Int)
    /// - Some with tuple type for multi-value variants like TupleCase(Int, Bool)
    /// - Some with struct type for struct variants like StructCase { field: Int }
    pub underlying: Option<Spanned<EnumVariantType>>,
}

/// The type specification for an enum variant
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumVariantType {
    /// Single type or tuple: (Int) or (Int, Bool)
    Tuple(Vec<Spanned<TypeName>>),
    /// Struct type: { field1: Type1, field2: Type2 }
    Struct(Vec<(Spanned<String>, Spanned<TypeName>)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Spanned<String>,
    pub typ: Spanned<TypeName>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName {
    pub types: Vec<Spanned<MaybeTypeName>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaybeTypeName {
    pub maybe_count: usize,
    pub inner: SimpleTypeName,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimpleTypeName {
    /// Type path - could be simple (Int, Bool) or qualified (Result::Ok, module::Type)
    /// Resolution happens in the semantics layer.
    Path(Spanned<NamespacePath>),
    EmptyList,
    List(Spanned<TypeName>),       // [Student], [[Int]], etc.
    Tuple(Vec<Spanned<TypeName>>), // (Int, Bool), (Int, Bool, String), etc.
    Struct(Vec<(Spanned<String>, Spanned<TypeName>)>), // {field1: Type1, field2: Type2}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Field(String),
    TupleIndex(usize),
    ListIndexFallible(Box<Spanned<Expr>>), // [expr]?
    ListIndexPanic(Box<Spanned<Expr>>),    // [expr]!
}

/// A namespace path with one or more segments: ident or ident::ident::...
/// Used for variable references, function calls, type casts, and enum variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamespacePath {
    pub segments: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchBranch {
    pub ident: Spanned<String>,
    pub as_typ: Option<Spanned<TypeName>>,
    pub filter: Option<Spanned<Expr>>,
    pub body: Spanned<Expr>,
}

/// A part of a docstring line, either plain text or an expression to evaluate
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocstringPart {
    /// Text before the expression (or the entire text if no expression)
    pub prefix: String,
    /// Optional expression to evaluate, wrapped in String(...)
    pub expr: Option<Spanned<Expr>>,
}

/// A complete docstring line with all its parts
pub type DocstringLine = Vec<DocstringPart>;
