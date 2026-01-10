use crate::parser::{ColloMLParser, Rule};
use pest::iterators::Pair;
use pest::Parser;

// ============= Span and Spanned =============

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn from_pest(pair: &Pair<Rule>) -> Self {
        let span = pair.as_span();
        Span {
            start: span.start(),
            end: span.end(),
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Spanned { node, span }
    }
}

// ============= Docstrings =============

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

/// Parse a docstring line and extract expressions delimited by backticks.
/// Supports multiple backticks for escaping: `` `x` ``, ``` ``x with `backticks` `` ```, etc.
/// Each expression is automatically wrapped in String(...) for evaluation.
pub fn parse_docstring_line(
    content: &str,
    base_span_start: usize,
) -> Result<DocstringLine, AstError> {
    let mut parts = Vec::new();
    let mut current_pos = 0;
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Look for backticks
        if bytes[i] == b'`' {
            // Count opening backticks
            let backtick_start = i;
            let mut backtick_count = 0;
            while i < bytes.len() && bytes[i] == b'`' {
                backtick_count += 1;
                i += 1;
            }

            // Find matching closing backticks (same count)
            let expr_start = i;
            let mut found_closing = false;
            let mut expr_end = i;

            while i < bytes.len() {
                if bytes[i] == b'`' {
                    // Count consecutive backticks
                    let closing_start = i;
                    let mut closing_count = 0;
                    while i < bytes.len() && bytes[i] == b'`' {
                        closing_count += 1;
                        i += 1;
                    }

                    if closing_count == backtick_count {
                        // Found matching closing backticks
                        expr_end = closing_start;
                        found_closing = true;
                        break;
                    }
                    // Not matching count, continue searching
                } else {
                    i += 1;
                }
            }

            if !found_closing {
                return Err(AstError::UnmatchedBackticks {
                    span: Span {
                        start: base_span_start + backtick_start,
                        end: base_span_start + content.len(),
                    },
                });
            }

            let expr_text = &content[expr_start..expr_end];
            let expr_span_start = base_span_start + expr_start;

            // Parse expression using pest
            let parsed = ColloMLParser::parse(Rule::expr_complete, expr_text).map_err(|e| {
                AstError::DocstringExpressionParse {
                    text: expr_text.to_string(),
                    error: format!("{}", e),
                    span: Span {
                        start: expr_span_start,
                        end: expr_span_start + expr_text.len(),
                    },
                }
            })?;

            let expr_pair = parsed
                .into_iter()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .ok_or_else(|| AstError::DocstringExpressionParse {
                    text: expr_text.to_string(),
                    error: "Empty expression".to_string(),
                    span: Span {
                        start: expr_span_start,
                        end: expr_span_start + expr_text.len(),
                    },
                })?;

            let inner_expr = Expr::from_pest(expr_pair)?;

            // Wrap in String(...) type cast
            let string_type = TypeName {
                types: vec![Spanned::new(
                    MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::String,
                    },
                    Span {
                        start: expr_span_start,
                        end: expr_span_start,
                    },
                )],
            };

            let wrapped_expr = Expr::ComplexTypeCast {
                typ: Spanned::new(
                    string_type,
                    Span {
                        start: expr_span_start,
                        end: expr_span_start,
                    },
                ),
                args: vec![Spanned::new(
                    inner_expr,
                    Span {
                        start: expr_span_start,
                        end: expr_span_start + expr_text.len(),
                    },
                )],
            };

            // Add prefix text part (if any)
            let prefix = content[current_pos..backtick_start].to_string();
            if !prefix.is_empty() {
                parts.push(DocstringPart { prefix, expr: None });
            }

            // Add expression part
            parts.push(DocstringPart {
                prefix: String::new(),
                expr: Some(Spanned::new(
                    wrapped_expr,
                    Span {
                        start: expr_span_start,
                        end: expr_span_start + expr_text.len(),
                    },
                )),
            });

            current_pos = i;
        } else {
            i += 1;
        }
    }

    // Add remaining text
    if current_pos < content.len() {
        parts.push(DocstringPart {
            prefix: content[current_pos..].to_string(),
            expr: None,
        });
    }

    // Handle empty docstring - return empty vec (no parts)
    Ok(parts)
}

// ============= Top Level =============

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
        constraint_name: Spanned<String>,
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
    Never,
    LinExpr,
    Constraint,
    None,
    Int,
    Bool,
    String,
    Other(String), // At parse time, we don't know if it's an object or custom type
    /// Qualified type name like Result::Ok - (root, variant)
    Qualified(String, String),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchBranch {
    pub ident: Spanned<String>,
    pub as_typ: Option<Spanned<TypeName>>,
    pub filter: Option<Spanned<Expr>>,
    pub body: Spanned<Expr>,
}

// ============= Expressions =============

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
    FnCall {
        name: Spanned<String>,
        args: Vec<Spanned<Expr>>,
    },
    VarCall {
        name: Spanned<String>,
        args: Vec<Spanned<Expr>>,
    },
    VarListCall {
        name: Spanned<String>,
        args: Vec<Spanned<Expr>>,
    },

    // Elements
    None,
    Number(i32),
    Boolean(bool),
    StringLiteral(String),
    Ident(Spanned<String>),
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

    // Struct-style type cast: TypeName {field: value}
    StructTypeCast {
        type_name: Spanned<String>,
        fields: Vec<(Spanned<String>, Spanned<Expr>)>,
    },

    // Qualified type cast: Result::Ok(x), Option::None(), Option::None
    // For enum variant construction
    QualifiedTypeCast {
        root: Spanned<String>,    // The enum name (e.g., "Result")
        variant: Spanned<String>, // The variant name (e.g., "Ok")
        args: Vec<Spanned<Expr>>, // Arguments (empty for unit variants)
    },

    // Qualified struct cast: MyEnum::StructCase { x: 1, y: 2 }
    // For enum struct variant construction
    QualifiedStructCast {
        root: Spanned<String>,                         // The enum name
        variant: Spanned<String>,                      // The variant name
        fields: Vec<(Spanned<String>, Spanned<Expr>)>, // Field name-value pairs
    },
}

// ============= Error Type =============

use thiserror::Error;
#[derive(Debug, Error, Clone)]
pub enum AstError {
    #[error("Expected {expected}, found {found:?} at {span:?}")]
    UnexpectedRule {
        expected: &'static str,
        found: Rule,
        span: Span,
    },
    #[error("Missing name at {0:?}")]
    MissingName(Span),
    #[error("Missing type name at {0:?}")]
    MissingTypeName(Span),
    #[error("Missing statement body at {0:?}")]
    MissingBody(Span),
    #[error("Missing expression at {0:?}")]
    MissingExpr(Span),
    #[error("Failed to parse integer at {span:?}: {error}")]
    ParseIntError {
        span: Span,
        error: std::num::ParseIntError,
    },
    #[error("Unclosed backticks in docstring expression at {span:?}")]
    UnmatchedBackticks { span: Span },
    #[error("Failed to parse docstring expression `{text}` at {span:?}: {error}")]
    DocstringExpressionParse {
        text: String,
        error: String,
        span: Span,
    },
}

impl File {
    pub fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::file {
            return Err(AstError::UnexpectedRule {
                expected: "file",
                found: pair.as_rule(),
                span,
            });
        }

        let mut statements = Vec::new();
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::statement => {
                    // statement is a wrapper containing let_statement, reify_statement, or type_statement
                    let span = Span::from_pest(&inner_pair);
                    let actual_stmt = inner_pair.into_inner().next().unwrap();
                    let stmt = Statement::from_pest(actual_stmt)?;
                    statements.push(Spanned::new(stmt, span));
                }
                Rule::EOI => {}
                _ => {
                    return Err(AstError::UnexpectedRule {
                        expected: "let_statement, reify_statement, type_statement, or EOI",
                        found: inner_pair.as_rule(),
                        span: Span::from_pest(&inner_pair),
                    });
                }
            }
        }

        Ok(File { statements })
    }
}

impl Statement {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        match pair.as_rule() {
            Rule::let_statement => Self::from_let_pest(pair),
            Rule::reify_statement => Self::from_reify_pest(pair),
            Rule::type_statement => Self::from_type_pest(pair),
            Rule::enum_statement => Self::from_enum_pest(pair),
            _ => Err(AstError::UnexpectedRule {
                expected: "let_statement, reify_statement, type_statement, or enum_statement",
                found: pair.as_rule(),
                span: Span::from_pest(&pair),
            }),
        }
    }

    fn from_let_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut docstring = Vec::new();
        let mut public = false;
        let mut name = None;
        let mut params = Vec::new();
        let mut output_type = None;
        let mut body = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::docstring => {
                    // docstring contains docstring_content
                    let docstring_span = Span::from_pest(&inner_pair);
                    let content = inner_pair
                        .into_inner()
                        .next()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_default();
                    let parsed_line = parse_docstring_line(&content, docstring_span.start)?;
                    docstring.push(parsed_line);
                }
                Rule::pub_modifier => {
                    public = true;
                }
                Rule::ident => {
                    if name.is_none() {
                        name = Some(Spanned::new(
                            inner_pair.as_str().to_string(),
                            Span::from_pest(&inner_pair),
                        ));
                    }
                }
                Rule::params => {
                    params = parse_params(inner_pair)?;
                }
                Rule::type_name => {
                    let type_name_span = Span::from_pest(&inner_pair);
                    output_type = Some(Spanned::new(
                        TypeName::from_pest(inner_pair)?,
                        type_name_span,
                    ));
                }
                Rule::expr => {
                    let output_span = Span::from_pest(&inner_pair);
                    body = Some(Spanned::new(Expr::from_pest(inner_pair)?, output_span));
                }
                _ => {}
            }
        }

        Ok(Statement::Let {
            docstring,
            public,
            name: name.ok_or(AstError::MissingName(span.clone()))?,
            params,
            output_type: output_type.ok_or(AstError::MissingTypeName(span.clone()))?,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_reify_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut docstring = Vec::new();
        let mut public = false;
        let mut constraint_name = None;
        let mut name = None;
        let mut var_list = false;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::docstring => {
                    let docstring_span = Span::from_pest(&inner_pair);
                    let content = inner_pair
                        .into_inner()
                        .next()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_default();
                    let parsed_line = parse_docstring_line(&content, docstring_span.start)?;
                    docstring.push(parsed_line);
                }
                Rule::pub_modifier => {
                    public = true;
                }
                Rule::ident => {
                    if constraint_name.is_none() {
                        constraint_name = Some(Spanned::new(
                            inner_pair.as_str().to_string(),
                            Span::from_pest(&inner_pair),
                        ));
                    }
                }
                Rule::var_name => {
                    // var_name is "$" ~ ident, so we need to strip the $
                    name = Some(Spanned::new(
                        inner_pair.as_str().trim_start_matches('$').to_string(),
                        Span::from_pest(&inner_pair),
                    ));
                }
                Rule::var_list_name => {
                    // var_name is "$[" ~ ident ~ "]", so we need to strip the $[...]
                    name = Some(Spanned::new(
                        inner_pair
                            .as_str()
                            .trim_start_matches("$[")
                            .trim_end_matches("]")
                            .to_string(),
                        Span::from_pest(&inner_pair),
                    ));
                    var_list = true;
                }
                _ => {}
            }
        }

        Ok(Statement::Reify {
            docstring,
            public,
            constraint_name: constraint_name.ok_or(AstError::MissingName(span.clone()))?,
            name: name.ok_or(AstError::MissingName(span))?,
            var_list,
        })
    }

    fn from_type_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        // type_statement = { pub_modifier? ~ "type" ~ ident ~ "=" ~ type_name ~ ";" }
        let span = Span::from_pest(&pair);
        let mut public = false;
        let mut name = None;
        let mut underlying = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::pub_modifier => {
                    public = true;
                }
                Rule::ident => {
                    if name.is_none() {
                        name = Some(Spanned::new(
                            inner_pair.as_str().to_string(),
                            Span::from_pest(&inner_pair),
                        ));
                    }
                }
                Rule::type_name => {
                    let type_name_span = Span::from_pest(&inner_pair);
                    underlying = Some(Spanned::new(
                        TypeName::from_pest(inner_pair)?,
                        type_name_span,
                    ));
                }
                _ => {}
            }
        }

        Ok(Statement::TypeDecl {
            public,
            name: name.ok_or(AstError::MissingName(span.clone()))?,
            underlying: underlying.ok_or(AstError::MissingTypeName(span))?,
        })
    }

    fn from_enum_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        // enum_statement = { pub_modifier? ~ "enum" ~ ident ~ "=" ~ enum_variants ~ ";" }
        let span = Span::from_pest(&pair);
        let mut public = false;
        let mut name = None;
        let mut variants = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::pub_modifier => {
                    public = true;
                }
                Rule::ident => {
                    if name.is_none() {
                        name = Some(Spanned::new(
                            inner_pair.as_str().to_string(),
                            Span::from_pest(&inner_pair),
                        ));
                    }
                }
                Rule::enum_variants => {
                    variants = parse_enum_variants(inner_pair)?;
                }
                _ => {}
            }
        }

        Ok(Statement::EnumDecl {
            public,
            name: name.ok_or(AstError::MissingName(span.clone()))?,
            variants,
        })
    }
}

fn parse_enum_variants(pair: Pair<Rule>) -> Result<Vec<Spanned<EnumVariant>>, AstError> {
    // enum_variants = { enum_variant ~ ("|" ~ enum_variant)* }
    let mut variants = Vec::new();
    for variant_pair in pair.into_inner() {
        if variant_pair.as_rule() == Rule::enum_variant {
            let variant_span = Span::from_pest(&variant_pair);
            let variant = parse_enum_variant(variant_pair)?;
            variants.push(Spanned::new(variant, variant_span));
        }
    }
    Ok(variants)
}

fn parse_enum_variant(pair: Pair<Rule>) -> Result<EnumVariant, AstError> {
    // enum_variant = { variant_name ~ enum_variant_type? }
    // variant_name = @{ "None" ~ !ident_char | ident }
    let span = Span::from_pest(&pair);
    let mut name = None;
    let mut underlying = None;

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::variant_name | Rule::ident => {
                name = Some(Spanned::new(
                    inner_pair.as_str().to_string(),
                    Span::from_pest(&inner_pair),
                ));
            }
            Rule::enum_variant_type => {
                let type_span = Span::from_pest(&inner_pair);
                let variant_type = parse_enum_variant_type(inner_pair)?;
                underlying = Some(Spanned::new(variant_type, type_span));
            }
            _ => {}
        }
    }

    Ok(EnumVariant {
        name: name.ok_or(AstError::MissingName(span))?,
        underlying,
    })
}

fn parse_enum_variant_type(pair: Pair<Rule>) -> Result<EnumVariantType, AstError> {
    // enum_variant_type = { "(" ~ (type_name ~ ("," ~ type_name)* ~ ","?)? ~ ")" | struct_type }
    let mut types = vec![];
    let mut is_struct = false;
    let mut struct_fields = vec![];

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::type_name => {
                let type_span = Span::from_pest(&inner_pair);
                types.push(Spanned::new(TypeName::from_pest(inner_pair)?, type_span));
            }
            Rule::struct_type => {
                is_struct = true;
                let fields = SimpleTypeName::from_struct_type(inner_pair)?;
                if let SimpleTypeName::Struct(field_list) = fields {
                    struct_fields = field_list;
                }
            }
            _ => {}
        }
    }

    if is_struct {
        Ok(EnumVariantType::Struct(struct_fields))
    } else {
        Ok(EnumVariantType::Tuple(types))
    }
}

fn parse_params(pair: Pair<Rule>) -> Result<Vec<Param>, AstError> {
    let mut params = Vec::new();
    for param_pair in pair.into_inner() {
        let span = Span::from_pest(&param_pair);
        if param_pair.as_rule() == Rule::param {
            let mut name = None;
            let mut typ = None;

            for inner in param_pair.into_inner() {
                match inner.as_rule() {
                    Rule::ident => {
                        let inner_span = Span::from_pest(&inner);
                        name = Some(Spanned::new(inner.as_str().to_string(), inner_span));
                    }
                    Rule::type_name => {
                        let inner_span = Span::from_pest(&inner);
                        typ = Some(Spanned::new(TypeName::from_pest(inner)?, inner_span));
                    }
                    _ => {}
                }
            }

            let param = Param {
                name: name.ok_or(AstError::MissingName(span.clone()))?,
                typ: typ.ok_or(AstError::MissingTypeName(span.clone()))?,
            };
            params.push(param);
        }
    }
    Ok(params)
}

impl TypeName {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::type_name {
            return Err(AstError::UnexpectedRule {
                expected: "type_name",
                found: pair.as_rule(),
                span,
            });
        }

        // type_name is: maybe_type ~ ( "|" ~ maybe_type )*
        let types: Result<Vec<_>, _> = pair
            .into_inner()
            .map(|maybe_type_pair| MaybeTypeName::from_pest(maybe_type_pair))
            .collect();

        Ok(TypeName { types: types? })
    }
}

impl MaybeTypeName {
    fn from_pest(pair: Pair<Rule>) -> Result<Spanned<Self>, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::maybe_type {
            return Err(AstError::UnexpectedRule {
                expected: "maybe_type",
                found: pair.as_rule(),
                span,
            });
        }

        // maybe_type is: maybe_op* ~ list_type
        let inner_pairs = pair.into_inner();

        // Count the maybe_ops ("?")
        let mut maybe_count = 0;
        let mut list_type_pair = None;

        for inner_pair in inner_pairs {
            match inner_pair.as_rule() {
                Rule::maybe_op => maybe_count += 1,
                Rule::list_type => {
                    list_type_pair = Some(inner_pair);
                    break;
                }
                _ => {
                    return Err(AstError::UnexpectedRule {
                        expected: "maybe_op or list_type",
                        found: inner_pair.as_rule(),
                        span: Span::from_pest(&inner_pair),
                    });
                }
            }
        }

        let list_type_pair = list_type_pair.ok_or(AstError::MissingTypeName(span.clone()))?;
        let inner = SimpleTypeName::from_pest(list_type_pair)?;

        Ok(Spanned::new(MaybeTypeName { maybe_count, inner }, span))
    }
}

impl SimpleTypeName {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::list_type {
            return Err(AstError::UnexpectedRule {
                expected: "list_type",
                found: pair.as_rule(),
                span,
            });
        }

        let mut inner_pairs = pair.into_inner();
        let inner = inner_pairs.next();

        match inner {
            None => {
                // No inner content means we have "[]" - empty list
                Ok(SimpleTypeName::EmptyList)
            }
            Some(inner_pair) => {
                match inner_pair.as_rule() {
                    Rule::primitive_type => Self::from_primitive_type(inner_pair),
                    Rule::type_name => {
                        // It's a list type: [type_name]
                        let inner_span = Span::from_pest(&inner_pair);
                        Ok(SimpleTypeName::List(Spanned::new(
                            TypeName::from_pest(inner_pair)?,
                            inner_span,
                        )))
                    }
                    Rule::tuple_type => Self::from_tuple_type(inner_pair),
                    Rule::struct_type => Self::from_struct_type(inner_pair),
                    _ => Err(AstError::UnexpectedRule {
                        expected: "primitive_type, type_name, tuple_type, or struct_type",
                        found: inner_pair.as_rule(),
                        span: Span::from_pest(&inner_pair),
                    }),
                }
            }
        }
    }

    fn from_tuple_type(pair: Pair<Rule>) -> Result<Self, AstError> {
        // tuple_type = { "(" ~ type_name ~ "," ~ type_name ~ ("," ~ type_name)* ~ ","? ~ ")" }
        let elements: Result<Vec<_>, _> = pair
            .into_inner()
            .filter(|p| p.as_rule() == Rule::type_name)
            .map(|type_pair| {
                let type_span = Span::from_pest(&type_pair);
                Ok(Spanned::new(TypeName::from_pest(type_pair)?, type_span))
            })
            .collect();

        Ok(SimpleTypeName::Tuple(elements?))
    }

    fn from_struct_type(pair: Pair<Rule>) -> Result<Self, AstError> {
        // struct_type = { "{" ~ (struct_field_type ~ ("," ~ struct_field_type)* ~ ","?)? ~ "}" }
        // struct_field_type = { ident ~ ":" ~ type_name }
        let mut fields = Vec::new();

        for field_pair in pair.into_inner() {
            if field_pair.as_rule() == Rule::struct_field_type {
                let mut inner = field_pair.into_inner();

                let name_pair = inner.next().unwrap();
                let name_span = Span::from_pest(&name_pair);
                let name = Spanned::new(name_pair.as_str().to_string(), name_span);

                let type_pair = inner.next().unwrap();
                let type_span = Span::from_pest(&type_pair);
                let field_type = Spanned::new(TypeName::from_pest(type_pair)?, type_span);

                fields.push((name, field_type));
            }
        }

        Ok(SimpleTypeName::Struct(fields))
    }

    fn from_primitive_type(pair: Pair<Rule>) -> Result<Self, AstError> {
        match pair.as_rule() {
            Rule::primitive_type => {
                // Check if there's a nested qualified_type_name
                let mut inner = pair.clone().into_inner();
                if let Some(first) = inner.next() {
                    if first.as_rule() == Rule::qualified_type_name {
                        // It's a qualified type like Result::Ok
                        let mut idents = first.into_inner();
                        let root = idents.next().unwrap().as_str().to_string();
                        let variant = idents.next().unwrap().as_str().to_string();
                        return Ok(SimpleTypeName::Qualified(root, variant));
                    }
                }

                // Otherwise, it's a simple type name
                let type_name = pair.as_str();
                match type_name {
                    "None" => Ok(SimpleTypeName::None),
                    "Int" => Ok(SimpleTypeName::Int),
                    "Bool" => Ok(SimpleTypeName::Bool),
                    "LinExpr" => Ok(SimpleTypeName::LinExpr),
                    "Constraint" => Ok(SimpleTypeName::Constraint),
                    "String" => Ok(SimpleTypeName::String),
                    "Never" => Ok(SimpleTypeName::Never),
                    _ => Ok(SimpleTypeName::Other(type_name.to_string())),
                }
            }
            _ => Err(AstError::UnexpectedRule {
                expected: "primitive_type",
                found: pair.as_rule(),
                span: Span::from_pest(&pair),
            }),
        }
    }
}

impl Expr {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::expr {
            return Err(AstError::UnexpectedRule {
                expected: "expr",
                found: pair.as_rule(),
                span,
            });
        }

        // expr -> null_coalesce_expr
        let null_coalesce_expr = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;
        Self::from_null_coalesce_expr(null_coalesce_expr)
    }

    fn from_null_coalesce_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_or_expr(first)?;

        while let Some(_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_or_expr(right_pair)?;

            let result_span = span.clone();
            result = Expr::NullCoalesce(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_or_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_and_expr(first)?;

        while let Some(_or_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_and_expr(right_pair)?;

            let result_span = span.clone();
            result = Expr::Or(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_and_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First not_expr
        let first = inner.next().unwrap();
        let mut result = Self::from_not_expr(first)?;

        // Chain together with 'and'
        while let Some(_and_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_not_expr(right_pair)?;

            let result_span = span.clone();
            result = Expr::And(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_not_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::not_op => {
                // It's a not expression
                let expr_pair = inner.next().unwrap();
                let expr_span = Span::from_pest(&expr_pair);
                let expr = Self::from_not_expr(expr_pair)?;
                Ok(Expr::Not(Box::new(Spanned::new(expr, expr_span))))
            }
            Rule::forall_expr => Self::from_forall_expr(first),
            _ => Err(AstError::UnexpectedRule {
                expected: "not_expr or forall_expr",
                found: first.as_rule(),
                span: Span::from_pest(&first),
            }),
        }
    }

    fn from_forall_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::forall => Self::from_forall(inner),
            Rule::comparison_expr => Self::from_comparison_expr(inner),
            _ => Err(AstError::UnexpectedRule {
                expected: "forall or comparison_expr",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }

    fn from_forall(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut var = None;
        let mut collection = None;
        let mut filter = None;
        let mut body = None;
        let mut has_filter = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let var_span = Span::from_pest(&inner);
                    var = Some(Spanned::new(inner.as_str().to_string(), var_span));
                }
                Rule::where_op => {
                    has_filter = true;
                }
                Rule::expr => {
                    let expr_span = Span::from_pest(&inner);
                    let expr = Box::new(Spanned::new(Expr::from_pest(inner)?, expr_span));
                    if collection.is_none() {
                        collection = Some(expr);
                    } else if has_filter && filter.is_none() {
                        filter = Some(expr);
                    } else if body.is_none() {
                        body = Some(expr);
                    }
                }
                _ => {}
            }
        }

        Ok(Expr::Forall {
            var: var.ok_or(AstError::MissingName(span.clone()))?,
            collection: collection.ok_or(AstError::MissingBody(span.clone()))?,
            filter,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_comparison_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::in_expr => Self::from_in_expr(first),
            Rule::relational_expr => Self::from_relational_expr(first),
            _ => Err(AstError::UnexpectedRule {
                expected: "in_expr or relation_expr",
                found: first.as_rule(),
                span: Span::from_pest(&first),
            }),
        }
    }

    fn from_in_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let item_pair = inner.next().unwrap();
        let item_span = Span::from_pest(&item_pair);
        let item = Box::new(Spanned::new(
            Self::from_relational_expr(item_pair)?,
            item_span,
        ));

        let _in_op = inner.next().unwrap(); // consume "in"

        let coll_pair = inner.next().unwrap();
        let coll_span = Span::from_pest(&coll_pair);
        let collection = Box::new(Spanned::new(
            Self::from_relational_expr(coll_pair)?,
            coll_span,
        ));

        Ok(Expr::In { item, collection })
    }

    fn from_relational_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let left_pair = inner.next().unwrap();
        let left_span = Span::from_pest(&left_pair);
        let left = Self::from_add_sub_expr(left_pair)?;

        // Check if there's a comparison operator
        if let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right_span = Span::from_pest(&right_pair);
            let right = Self::from_add_sub_expr(right_pair)?;

            let left_spanned = Box::new(Spanned::new(left, left_span));
            let right_spanned = Box::new(Spanned::new(right, right_span));

            match op_pair.as_str() {
                "===" => Ok(Expr::ConstraintEq(left_spanned, right_spanned)),
                "<==" => Ok(Expr::ConstraintLe(left_spanned, right_spanned)),
                ">==" => Ok(Expr::ConstraintGe(left_spanned, right_spanned)),
                "==" => Ok(Expr::Eq(left_spanned, right_spanned)),
                "!=" => Ok(Expr::Ne(left_spanned, right_spanned)),
                "<" => Ok(Expr::Lt(left_spanned, right_spanned)),
                "<=" => Ok(Expr::Le(left_spanned, right_spanned)),
                ">" => Ok(Expr::Gt(left_spanned, right_spanned)),
                ">=" => Ok(Expr::Ge(left_spanned, right_spanned)),
                _ => Err(AstError::UnexpectedRule {
                    expected: "comparison operator",
                    found: op_pair.as_rule(),
                    span: Span::from_pest(&op_pair),
                }),
            }
        } else {
            // No comparison, just the expression
            Ok(left)
        }
    }

    fn from_add_sub_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_mul_div_mod_expr(first)?;

        while let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_mul_div_mod_expr(right_pair)?;

            let result_span = span.clone();
            result = match op_pair.as_rule() {
                Rule::add_op => Expr::Add(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::sub_op => Expr::Sub(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                _ => {
                    return Err(AstError::UnexpectedRule {
                        expected: "add_op or sub_op",
                        found: op_pair.as_rule(),
                        span: Span::from_pest(&op_pair),
                    })
                }
            };
        }

        Ok(result)
    }

    fn from_mul_div_mod_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_cast_expr(first)?;

        while let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_cast_expr(right_pair)?;

            let result_span = span.clone();
            result = match op_pair.as_rule() {
                Rule::mul_op => Expr::Mul(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::div_op => Expr::Div(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::mod_op => Expr::Mod(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                _ => {
                    return Err(AstError::UnexpectedRule {
                        expected: "mul_op, div_op, or mod_op",
                        found: op_pair.as_rule(),
                        span: Span::from_pest(&op_pair),
                    })
                }
            };
        }

        Ok(result)
    }

    fn from_cast_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::cast_expr {
            return Err(AstError::UnexpectedRule {
                expected: "cast_expr",
                found: pair.as_rule(),
                span,
            });
        }

        let mut inner = pair.into_inner();

        // First is always explicit_type
        let expr_pair = inner.next().unwrap();
        let expr_span = Span::from_pest(&expr_pair);
        let expr = Self::from_explicit_type(expr_pair)?;

        // Check if there's a cast operator
        if let Some(cast_op_pair) = inner.next() {
            let cast_op = cast_op_pair.as_str();
            let type_pair = inner.next().unwrap();
            let type_span = Span::from_pest(&type_pair);
            let typ = TypeName::from_pest(type_pair)?;

            match cast_op {
                "cast?" => Ok(Expr::CastFallible {
                    expr: Box::new(Spanned::new(expr, expr_span)),
                    typ: Spanned::new(typ, type_span),
                }),
                "cast!" => Ok(Expr::CastPanic {
                    expr: Box::new(Spanned::new(expr, expr_span)),
                    typ: Spanned::new(typ, type_span),
                }),
                _ => panic!("Unknown cast operator: {}", cast_op),
            }
        } else {
            // No cast, just return the expression
            Ok(expr)
        }
    }

    fn from_explicit_type(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::explicit_type {
            return Err(AstError::UnexpectedRule {
                expected: "explicit_type",
                found: pair.as_rule(),
                span,
            });
        }

        let mut inner = pair.into_inner();

        // First is always path
        let expr_pair = inner.next().unwrap();
        let expr_span = Span::from_pest(&expr_pair);
        let expr = Self::from_path(expr_pair)?;

        // Check if there's a type annotation
        if let Some(type_pair) = inner.next() {
            // This is the type_name after "as"
            let type_span = Span::from_pest(&type_pair);
            let typ = TypeName::from_pest(type_pair)?;

            Ok(Expr::ExplicitType {
                expr: Box::new(Spanned::new(expr, expr_span)),
                typ: Spanned::new(typ, type_span),
            })
        } else {
            // No type annotation, just return the expression
            Ok(expr)
        }
    }

    fn from_path(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::path {
            return Err(AstError::UnexpectedRule {
                expected: "path",
                found: pair.as_rule(),
                span,
            });
        }

        let mut inner = pair.into_inner();

        // First is always primary_expr
        let expr_pair = inner.next().unwrap();
        let expr_span = Span::from_pest(&expr_pair);
        let expr = Self::from_primary_expr(expr_pair)?;

        let mut segments = Vec::new();
        for segment in inner {
            match segment.as_rule() {
                Rule::path_segment => {
                    let inner_segment = segment.into_inner().next().unwrap();
                    let segment_span = Span::from_pest(&inner_segment);
                    match inner_segment.as_rule() {
                        Rule::ident => {
                            segments.push(Spanned::new(
                                PathSegment::Field(inner_segment.as_str().to_string()),
                                segment_span,
                            ));
                        }
                        Rule::tuple_index => {
                            let index: usize = inner_segment.as_str().parse().map_err(|e| {
                                AstError::ParseIntError {
                                    span: segment_span.clone(),
                                    error: e,
                                }
                            })?;
                            segments
                                .push(Spanned::new(PathSegment::TupleIndex(index), segment_span));
                        }
                        _ => {}
                    }
                }
                Rule::index_segment => {
                    // index_segment = { "[" ~ expr ~ index_suffix }
                    // index_suffix = ${ "]" ~ index_op }
                    let segment_span = Span::from_pest(&segment);
                    let mut index_inner = segment.into_inner();

                    // First is the index expression
                    let index_expr_pair = index_inner.next().unwrap();
                    let index_expr_span = Span::from_pest(&index_expr_pair);
                    let index_expr = Self::from_pest(index_expr_pair)?;
                    let boxed_index = Box::new(Spanned::new(index_expr, index_expr_span));

                    // Second is index_suffix which contains "]" and index_op
                    let suffix_pair = index_inner.next().unwrap();
                    let suffix_str = suffix_pair.as_str();

                    // suffix_str is "]?" or "]!"
                    let path_segment = if suffix_str.ends_with('?') {
                        PathSegment::ListIndexFallible(boxed_index)
                    } else {
                        PathSegment::ListIndexPanic(boxed_index)
                    };

                    segments.push(Spanned::new(path_segment, segment_span));
                }
                _ => {}
            }
        }

        if segments.is_empty() {
            Ok(expr)
        } else {
            Ok(Expr::Path {
                object: Box::new(Spanned::new(expr, expr_span)),
                segments,
            })
        }
    }

    fn from_primary_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::primary_expr {
            return Err(AstError::UnexpectedRule {
                expected: "primary_expr",
                found: pair.as_rule(),
                span,
            });
        }

        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::let_expr => Self::from_let_expr(inner),
            Rule::if_expr => Self::from_if_expr(inner),
            Rule::match_expr => Self::from_match_expr(inner),
            Rule::sum => Self::from_sum(inner),
            Rule::fold => Self::from_fold(inner, false),
            Rule::rfold => Self::from_fold(inner, true),
            Rule::cardinality => Self::from_cardinality(inner),
            Rule::empty_typed_list => Self::from_empty_typed_list(inner),
            Rule::list_comprehension => Self::from_list_comprehension(inner),
            Rule::list_range => Self::from_list_range(inner),
            Rule::list_literal => Self::from_list_literal(inner),
            Rule::global_collection => Self::from_global_collection(inner),
            Rule::var_call => Self::from_var_call(inner),
            Rule::var_list_call => Self::from_var_list_call(inner),
            Rule::qualified_type_cast => Self::from_qualified_type_cast(inner),
            Rule::qualified_struct_cast => Self::from_qualified_struct_cast(inner),
            Rule::qualified_unit_value => Self::from_qualified_unit_value(inner),
            Rule::complex_type_cast => Self::from_complex_type_cast(inner),
            Rule::struct_type_cast => Self::from_struct_type_cast(inner),
            Rule::primitive_type_cast => Self::from_primitive_type_cast(inner),
            Rule::fn_call => Self::from_fn_call(inner),
            Rule::string_literal => Self::from_string_literal(inner),
            Rule::boolean => Self::from_boolean(inner),
            Rule::none => Self::from_none(inner),
            Rule::neg => Self::from_neg(inner),
            Rule::panic => Self::from_panic(inner),
            Rule::struct_literal => Self::from_struct_literal(inner),
            Rule::tuple_literal => Self::from_tuple_literal(inner),
            Rule::number => {
                let num_str = inner.as_str();
                let value = num_str
                    .parse::<i32>()
                    .map_err(|e| AstError::ParseIntError {
                        span: Span::from_pest(&inner),
                        error: e,
                    })?;
                Ok(Expr::Number(value))
            }
            Rule::ident => {
                let ident_span = Span::from_pest(&inner);
                let ident = inner.as_str().to_string();
                Ok(Expr::Ident(Spanned::new(ident, ident_span)))
            }
            Rule::expr => {
                // Parenthesized expr
                Self::from_pest(inner)
            }
            _ => Err(AstError::UnexpectedRule {
                expected: "if_expr match_expr sum cardinality list_comprehension list_literal global_collection var_call fn_call string_literal boolean number path expr",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }

    fn from_if_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let condition_pair = inner.next().unwrap();
        let condition_span = Span::from_pest(&condition_pair);
        let condition = Box::new(Spanned::new(
            Self::from_pest(condition_pair)?,
            condition_span,
        ));

        let then_pair = inner.next().unwrap();
        let then_span = Span::from_pest(&then_pair);
        let then_expr = Box::new(Spanned::new(Self::from_pest(then_pair)?, then_span));

        let else_pair = inner.next().unwrap();
        let else_span = Span::from_pest(&else_pair);
        let else_expr = Box::new(Spanned::new(Self::from_pest(else_pair)?, else_span));

        Ok(Expr::If {
            condition,
            then_expr,
            else_expr,
        })
    }

    fn from_match_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        // match_expr = { "match" ~ expr ~ "{" ~ match_branch* ~ "}" }
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First inner element is the expression being matched
        let expr_pair = inner.next().ok_or(AstError::MissingExpr(span.clone()))?;
        let expr_span = Span::from_pest(&expr_pair);
        let match_expr = Box::new(Spanned::new(Self::from_pest(expr_pair)?, expr_span));

        // Remaining elements are match branches
        let mut branches = Vec::new();
        for branch_pair in inner {
            if branch_pair.as_rule() == Rule::match_branch {
                branches.push(Self::from_match_branch(branch_pair)?);
            }
        }

        Ok(Expr::Match {
            match_expr,
            branches,
        })
    }

    fn from_match_branch(pair: Pair<Rule>) -> Result<MatchBranch, AstError> {
        // match_branch = { ident ~ (as_op ~ type_name)? ~ (where_op ~ expr)? ~ "{" ~ expr ~ "}" }
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First element is always the identifier
        let ident_pair = inner.next().ok_or(AstError::MissingName(span.clone()))?;
        let ident_span = Span::from_pest(&ident_pair);
        let ident = Spanned::new(ident_pair.as_str().to_string(), ident_span);

        let mut as_typ = None;
        let mut filter = None;
        let mut body = None;
        let mut has_filter = false;

        // Track which operator we just saw
        let mut last_op_was_as = false;

        for element in inner {
            match element.as_rule() {
                Rule::as_op => {
                    last_op_was_as = true;
                }
                Rule::where_op => {
                    has_filter = true;
                    last_op_was_as = false;
                }
                Rule::type_name => {
                    let type_span = Span::from_pest(&element);
                    let parsed_type = Spanned::new(TypeName::from_pest(element)?, type_span);

                    if last_op_was_as {
                        as_typ = Some(parsed_type);
                        last_op_was_as = false;
                    }
                }
                Rule::expr => {
                    let expr_span = Span::from_pest(&element);
                    let parsed_expr = Spanned::new(Expr::from_pest(element)?, expr_span);

                    if has_filter && filter.is_none() {
                        filter = Some(parsed_expr);
                    } else if body.is_none() {
                        body = Some(parsed_expr);
                    }
                }
                _ => {}
            }
        }

        Ok(MatchBranch {
            ident,
            as_typ,
            filter,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_sum(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut var = None;
        let mut collection = None;
        let mut filter = None;
        let mut body = None;
        let mut has_filter = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let var_span = Span::from_pest(&inner);
                    var = Some(Spanned::new(inner.as_str().to_string(), var_span));
                }
                Rule::where_op => {
                    has_filter = true;
                }
                Rule::expr => {
                    let expr_span = Span::from_pest(&inner);
                    let expr = Box::new(Spanned::new(Expr::from_pest(inner)?, expr_span));
                    if collection.is_none() {
                        collection = Some(expr);
                    } else if has_filter && filter.is_none() {
                        filter = Some(expr);
                    } else if body.is_none() {
                        body = Some(expr);
                    }
                }
                _ => {}
            }
        }

        Ok(Expr::Sum {
            var: var.ok_or(AstError::MissingName(span.clone()))?,
            collection: collection.ok_or(AstError::MissingBody(span.clone()))?,
            filter,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_fold(pair: Pair<Rule>, reversed: bool) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut var = None;
        let mut accumulator = None;
        let mut init_value = None;
        let mut collection = None;
        let mut filter = None;
        let mut body = None;
        let mut has_filter = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let ident_span = Span::from_pest(&inner);
                    if var.is_none() {
                        var = Some(Spanned::new(inner.as_str().to_string(), ident_span));
                    } else {
                        accumulator = Some(Spanned::new(inner.as_str().to_string(), ident_span));
                    }
                }
                Rule::where_op => {
                    has_filter = true;
                }
                Rule::expr => {
                    let expr_span = Span::from_pest(&inner);
                    let expr = Box::new(Spanned::new(Expr::from_pest(inner)?, expr_span));
                    if collection.is_none() {
                        collection = Some(expr);
                    } else if init_value.is_none() {
                        init_value = Some(expr);
                    } else if has_filter && filter.is_none() {
                        filter = Some(expr);
                    } else if body.is_none() {
                        body = Some(expr);
                    }
                }
                _ => {}
            }
        }

        Ok(Expr::Fold {
            var: var.ok_or(AstError::MissingName(span.clone()))?,
            collection: collection.ok_or(AstError::MissingBody(span.clone()))?,
            accumulator: accumulator.ok_or(AstError::MissingName(span.clone()))?,
            init_value: init_value.ok_or(AstError::MissingBody(span.clone()))?,
            filter,
            body: body.ok_or(AstError::MissingBody(span))?,
            reversed,
        })
    }

    fn from_let_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut var = None;
        let mut value = None;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let var_span = Span::from_pest(&inner);
                    var = Some(Spanned::new(inner.as_str().to_string(), var_span));
                }
                Rule::expr => {
                    let expr_span = Span::from_pest(&inner);
                    let expr = Box::new(Spanned::new(Expr::from_pest(inner)?, expr_span));
                    if value.is_none() {
                        value = Some(expr);
                    } else if body.is_none() {
                        body = Some(expr);
                    }
                }
                _ => {}
            }
        }

        Ok(Expr::Let {
            var: var.ok_or(AstError::MissingName(span.clone()))?,
            value: value.ok_or(AstError::MissingBody(span.clone()))?,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_cardinality(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        // cardinality = { "|" ~ expr ~ "|" }
        let coll_pair = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        let coll_span = Span::from_pest(&coll_pair);
        let collection = Box::new(Spanned::new(Expr::from_pest(coll_pair)?, coll_span));

        Ok(Expr::Cardinality(collection))
    }

    fn from_neg(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let neg_pair = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        let neg_span = Span::from_pest(&neg_pair);
        let term = Box::new(Spanned::new(Expr::from_pest(neg_pair)?, neg_span));

        Ok(Expr::Neg(term))
    }

    fn from_panic(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let inner_pair = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        let inner_span = Span::from_pest(&inner_pair);
        let inner = Box::new(Spanned::new(Expr::from_pest(inner_pair)?, inner_span));

        Ok(Expr::Panic(inner))
    }

    fn from_tuple_literal(pair: Pair<Rule>) -> Result<Self, AstError> {
        // tuple_literal = { "(" ~ expr ~ "," ~ expr ~ ("," ~ expr)* ~ ","? ~ ")" }
        let elements: Result<Vec<_>, _> = pair
            .into_inner()
            .filter(|p| p.as_rule() == Rule::expr)
            .map(|expr_pair| {
                let expr_span = Span::from_pest(&expr_pair);
                Ok(Spanned::new(Expr::from_pest(expr_pair)?, expr_span))
            })
            .collect();

        Ok(Expr::TupleLiteral {
            elements: elements?,
        })
    }

    fn from_struct_literal(pair: Pair<Rule>) -> Result<Self, AstError> {
        // struct_literal = { "{" ~ (struct_field_expr ~ ("," ~ struct_field_expr)* ~ ","?)? ~ "}" }
        // struct_field_expr = { ident ~ ":" ~ expr }
        let mut fields = Vec::new();

        for field_pair in pair.into_inner() {
            if field_pair.as_rule() == Rule::struct_field_expr {
                let mut inner = field_pair.into_inner();

                let name_pair = inner.next().unwrap();
                let name_span = Span::from_pest(&name_pair);
                let name = Spanned::new(name_pair.as_str().to_string(), name_span);

                let expr_pair = inner.next().unwrap();
                let expr_span = Span::from_pest(&expr_pair);
                let expr = Spanned::new(Expr::from_pest(expr_pair)?, expr_span);

                fields.push((name, expr));
            }
        }

        Ok(Expr::StructLiteral { fields })
    }

    fn from_empty_typed_list(pair: Pair<Rule>) -> Result<Self, AstError> {
        // empty_typed_list = { "[" ~ "<" ~ type_name ">" ~ "]" }
        let span = Span::from_pest(&pair);
        let inner = pair.into_inner().next().unwrap();

        let typ_span = Span::from_pest(&inner);
        let inner_typ = Spanned::new(TypeName::from_pest(inner)?, typ_span.clone());

        let list_typ = SimpleTypeName::List(inner_typ);
        let maybe_typ = Spanned::new(
            MaybeTypeName {
                maybe_count: 0,
                inner: list_typ,
            },
            typ_span.clone(),
        );
        let typ = Spanned::new(
            TypeName {
                types: vec![maybe_typ],
            },
            typ_span,
        );

        Ok(Expr::ExplicitType {
            expr: Box::new(Spanned::new(Expr::ListLiteral { elements: vec![] }, span)),
            typ,
        })
    }

    fn from_list_literal(pair: Pair<Rule>) -> Result<Self, AstError> {
        // list_literal = { "[" ~ "]" | "[" ~ expr ~ ("," ~ expr)* ~ "]" }
        let mut elements = Vec::new();

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::expr {
                let expr_span = Span::from_pest(&inner);
                let expr = Spanned::new(Expr::from_pest(inner)?, expr_span);
                elements.push(expr);
            }
        }

        Ok(Expr::ListLiteral { elements })
    }

    fn from_list_range(pair: Pair<Rule>) -> Result<Self, AstError> {
        // list_range = { "[" ~ expr ~ ".." ~ expr "]" }
        let span = Span::from_pest(&pair);

        let mut inner = pair.into_inner();

        let first = inner.next().ok_or(AstError::MissingExpr(span.clone()))?;
        let expr_span = Span::from_pest(&first);
        let start = Box::new(Spanned::new(Expr::from_pest(first)?, expr_span));

        let second = inner.next().ok_or(AstError::MissingExpr(span))?;
        let expr_span = Span::from_pest(&second);
        let end = Box::new(Spanned::new(Expr::from_pest(second)?, expr_span));

        Ok(Expr::ListRange { start, end })
    }

    fn from_list_comprehension(pair: Pair<Rule>) -> Result<Self, AstError> {
        // list_comprehension = { "[" ~ expr ~ "for" ~ ident ~ "in" ~ expr ~ ("where" ~ expr)? ~ "]" }
        let span = Span::from_pest(&pair);
        let mut expr = None;
        let mut vars = vec![];
        let mut collections = vec![];
        let mut filter = None;
        let mut has_filter = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let var_span = Span::from_pest(&inner);
                    vars.push(Spanned::new(inner.as_str().to_string(), var_span));
                }
                Rule::where_op => {
                    has_filter = true;
                }
                Rule::expr => {
                    let expr_span = Span::from_pest(&inner);
                    let parsed_expr = Spanned::new(Expr::from_pest(inner)?, expr_span);

                    if has_filter {
                        filter = Some(Box::new(parsed_expr));
                    } else if expr.is_none() {
                        expr = Some(Box::new(parsed_expr));
                    } else {
                        collections.push(parsed_expr);
                    }
                }
                _ => {}
            }
        }

        if vars.len() < collections.len() {
            return Err(AstError::MissingName(span.clone()));
        }
        if vars.len() > collections.len() {
            return Err(AstError::MissingExpr(span.clone()));
        }
        let vars_and_collections = vars.into_iter().zip(collections.into_iter()).collect();

        Ok(Expr::ListComprehension {
            body: expr.ok_or(AstError::MissingBody(span.clone()))?,
            vars_and_collections,
            filter,
        })
    }

    fn from_global_collection(pair: Pair<Rule>) -> Result<Self, AstError> {
        // global_collection = { "@" ~ "[" ~ primitive_type ~ "]" }
        let span = Span::from_pest(&pair);

        // Find the primitive_type inside
        let type_pair = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingTypeName(span.clone()))?;

        let type_span = Span::from_pest(&type_pair);
        let type_name = TypeName::from_pest(type_pair)?;

        Ok(Expr::GlobalList(Spanned::new(type_name, type_span)))
    }

    fn from_var_call(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut name = None;
        let mut args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let name_span = Span::from_pest(&inner);
                    name = Some(Spanned::new(inner.as_str().to_string(), name_span));
                }
                Rule::args => {
                    args = parse_args(inner)?;
                }
                _ => {}
            }
        }

        Ok(Expr::VarCall {
            name: name.ok_or(AstError::MissingName(span))?,
            args,
        })
    }

    fn from_var_list_call(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut name = None;
        let mut args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let name_span = Span::from_pest(&inner);
                    name = Some(Spanned::new(inner.as_str().to_string(), name_span));
                }
                Rule::args => {
                    args = parse_args(inner)?;
                }
                _ => {}
            }
        }

        Ok(Expr::VarListCall {
            name: name.ok_or(AstError::MissingName(span))?,
            args,
        })
    }

    fn from_fn_call(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut name = None;
        let mut args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    let name_span = Span::from_pest(&inner);
                    name = Some(Spanned::new(inner.as_str().to_string(), name_span));
                }
                Rule::args => {
                    args = parse_args(inner)?; // Use the same parse_args as Constraint
                }
                _ => {}
            }
        }

        Ok(Expr::FnCall {
            name: name.ok_or(AstError::MissingName(span))?,
            args,
        })
    }

    fn from_qualified_type_cast(pair: Pair<Rule>) -> Result<Self, AstError> {
        // qualified_type_cast = { ident ~ "::" ~ variant_name ~ "(" ~ args? ~ ")" }
        let span = Span::from_pest(&pair);
        let mut idents = vec![];
        let mut args = Vec::new();

        for element in pair.into_inner() {
            match element.as_rule() {
                Rule::ident | Rule::variant_name => {
                    let ident_span = Span::from_pest(&element);
                    idents.push(Spanned::new(element.as_str().to_string(), ident_span));
                }
                Rule::args => {
                    args = parse_args(element)?;
                }
                _ => {}
            }
        }

        if idents.len() < 2 {
            return Err(AstError::MissingName(span));
        }

        Ok(Expr::QualifiedTypeCast {
            root: idents.remove(0),
            variant: idents.remove(0),
            args,
        })
    }

    fn from_qualified_unit_value(pair: Pair<Rule>) -> Result<Self, AstError> {
        // qualified_unit_value = { ident ~ "::" ~ variant_name }
        let span = Span::from_pest(&pair);
        let mut idents = vec![];

        for element in pair.into_inner() {
            if matches!(element.as_rule(), Rule::ident | Rule::variant_name) {
                let ident_span = Span::from_pest(&element);
                idents.push(Spanned::new(element.as_str().to_string(), ident_span));
            }
        }

        if idents.len() < 2 {
            return Err(AstError::MissingName(span));
        }

        // Unit value is represented as a QualifiedTypeCast with empty args
        Ok(Expr::QualifiedTypeCast {
            root: idents.remove(0),
            variant: idents.remove(0),
            args: vec![],
        })
    }

    fn from_qualified_struct_cast(pair: Pair<Rule>) -> Result<Self, AstError> {
        // qualified_struct_cast = { ident ~ "::" ~ variant_name ~ struct_literal }
        let span = Span::from_pest(&pair);
        let mut idents = vec![];
        let mut fields = Vec::new();

        for element in pair.into_inner() {
            match element.as_rule() {
                Rule::ident | Rule::variant_name => {
                    let ident_span = Span::from_pest(&element);
                    idents.push(Spanned::new(element.as_str().to_string(), ident_span));
                }
                Rule::struct_literal => {
                    // Parse struct_literal fields (same logic as from_struct_literal)
                    for field_pair in element.into_inner() {
                        if field_pair.as_rule() == Rule::struct_field_expr {
                            let mut field_inner = field_pair.into_inner();

                            let field_name_pair = field_inner.next().unwrap();
                            let field_name_span = Span::from_pest(&field_name_pair);
                            let name =
                                Spanned::new(field_name_pair.as_str().to_string(), field_name_span);

                            let expr_pair = field_inner.next().unwrap();
                            let expr_span = Span::from_pest(&expr_pair);
                            let expr = Spanned::new(Expr::from_pest(expr_pair)?, expr_span);

                            fields.push((name, expr));
                        }
                    }
                }
                _ => {}
            }
        }

        if idents.len() < 2 {
            return Err(AstError::MissingName(span));
        }

        Ok(Expr::QualifiedStructCast {
            root: idents.remove(0),
            variant: idents.remove(0),
            fields,
        })
    }

    fn from_complex_type_cast(pair: Pair<Rule>) -> Result<Self, AstError> {
        // complex_type_cast = { (list_type_brackets | tuple_type) ~ "(" ~ args? ~ ")" }
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First is the type (list_type_brackets or tuple_type)
        let type_pair = inner
            .next()
            .ok_or(AstError::MissingTypeName(span.clone()))?;
        let type_span = Span::from_pest(&type_pair);

        // Convert to SimpleTypeName based on rule
        let simple_type = match type_pair.as_rule() {
            Rule::list_type_brackets => {
                // list_type_brackets = { "[" ~ type_name? ~ "]" }
                let inner_type = type_pair.into_inner().next();
                match inner_type {
                    None => SimpleTypeName::EmptyList,
                    Some(inner_pair) => {
                        let inner_span = Span::from_pest(&inner_pair);
                        SimpleTypeName::List(Spanned::new(
                            TypeName::from_pest(inner_pair)?,
                            inner_span,
                        ))
                    }
                }
            }
            Rule::tuple_type => SimpleTypeName::from_tuple_type(type_pair)?,
            _ => {
                return Err(AstError::UnexpectedRule {
                    expected: "list_type_brackets or tuple_type",
                    found: type_pair.as_rule(),
                    span: type_span,
                })
            }
        };

        let maybe_type = MaybeTypeName {
            maybe_count: 0,
            inner: simple_type,
        };
        let typ = Spanned::new(
            TypeName {
                types: vec![Spanned::new(maybe_type, type_span.clone())],
            },
            type_span,
        );

        // Parse args
        let mut args = Vec::new();
        for element in inner {
            if element.as_rule() == Rule::args {
                args = parse_args(element)?;
            }
        }

        Ok(Expr::ComplexTypeCast { typ, args })
    }

    fn from_struct_type_cast(pair: Pair<Rule>) -> Result<Self, AstError> {
        // struct_type_cast = { ident ~ struct_literal }
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First is the type name (ident)
        let name_pair = inner.next().ok_or(AstError::MissingName(span.clone()))?;
        let name_span = Span::from_pest(&name_pair);
        let type_name = Spanned::new(name_pair.as_str().to_string(), name_span);

        // Second is the struct_literal
        let struct_pair = inner.next().ok_or(AstError::MissingBody(span))?;

        // Parse struct_literal fields (same logic as from_struct_literal)
        let mut fields = Vec::new();
        for field_pair in struct_pair.into_inner() {
            if field_pair.as_rule() == Rule::struct_field_expr {
                let mut field_inner = field_pair.into_inner();

                let field_name_pair = field_inner.next().unwrap();
                let field_name_span = Span::from_pest(&field_name_pair);
                let name = Spanned::new(field_name_pair.as_str().to_string(), field_name_span);

                let expr_pair = field_inner.next().unwrap();
                let expr_span = Span::from_pest(&expr_pair);
                let expr = Spanned::new(Expr::from_pest(expr_pair)?, expr_span);

                fields.push((name, expr));
            }
        }

        Ok(Expr::StructTypeCast { type_name, fields })
    }

    fn from_primitive_type_cast(pair: Pair<Rule>) -> Result<Self, AstError> {
        // primitive_type_cast = { primitive_type_keyword ~ "(" ~ args? ~ ")" }
        // primitive_type_keyword = { "LinExpr" | "Constraint" | "String" | "Bool" | "Int" }
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First is the primitive_type_keyword
        let keyword_pair = inner
            .next()
            .ok_or(AstError::MissingTypeName(span.clone()))?;
        let keyword_span = Span::from_pest(&keyword_pair);
        let keyword = keyword_pair.as_str();

        // Convert keyword to SimpleTypeName
        let simple_type = match keyword {
            "Int" => SimpleTypeName::Int,
            "Bool" => SimpleTypeName::Bool,
            "String" => SimpleTypeName::String,
            "LinExpr" => SimpleTypeName::LinExpr,
            "Constraint" => SimpleTypeName::Constraint,
            _ => {
                return Err(AstError::UnexpectedRule {
                    expected: "Int, Bool, String, LinExpr, or Constraint",
                    found: keyword_pair.as_rule(),
                    span: keyword_span,
                })
            }
        };

        let maybe_type = MaybeTypeName {
            maybe_count: 0,
            inner: simple_type,
        };
        let typ = Spanned::new(
            TypeName {
                types: vec![Spanned::new(maybe_type, keyword_span.clone())],
            },
            keyword_span,
        );

        // Parse args
        let mut args = Vec::new();
        for element in inner {
            if element.as_rule() == Rule::args {
                args = parse_args(element)?;
            }
        }

        Ok(Expr::ComplexTypeCast { typ, args })
    }

    fn from_boolean(pair: Pair<Rule>) -> Result<Self, AstError> {
        // boolean = { "true" | "false" }
        match pair.as_str() {
            "true" => Ok(Expr::Boolean(true)),
            "false" => Ok(Expr::Boolean(false)),
            _ => Err(AstError::UnexpectedRule {
                expected: "true or false",
                found: pair.as_rule(),
                span: Span::from_pest(&pair),
            }),
        }
    }

    fn from_none(pair: Pair<Rule>) -> Result<Self, AstError> {
        // boolean = { "true" | "false" }
        match pair.as_str() {
            "none" => Ok(Expr::None),
            _ => Err(AstError::UnexpectedRule {
                expected: "none",
                found: pair.as_rule(),
                span: Span::from_pest(&pair),
            }),
        }
    }

    fn from_string_literal(pair: Pair<Rule>) -> Result<Self, AstError> {
        // string_literal matches one of: raw_string_0 through raw_string_5
        // Each raw_string_N has the form: "N*~" ~ "\"" ~ content ~ "\"" ~ "N*~"
        // The pair.as_str() gives us the full matched text including delimiters
        // We need to strip the delimiters to get the actual string content

        let full_str = pair.as_str();

        // Count leading tildes
        let leading_tildes = full_str.chars().take_while(|&c| c == '~').count();

        // The string format is: ~*"content"~*
        // So we skip (leading_tildes + 1) chars at start for ~*"
        // and skip (leading_tildes + 1) chars at end for "~*
        let start = leading_tildes + 1; // skip ~* and opening "
        let end = full_str.len() - (leading_tildes + 1); // skip closing " and ~*

        let content = &full_str[start..end];
        Ok(Expr::StringLiteral(content.to_string()))
    }
}

fn parse_args(pair: Pair<Rule>) -> Result<Vec<Spanned<Expr>>, AstError> {
    let mut args = Vec::new();
    for arg_pair in pair.into_inner() {
        if arg_pair.as_rule() == Rule::arg {
            let arg_span = Span::from_pest(&arg_pair);
            // arg contains expr
            let comp_pair = arg_pair.into_inner().next().unwrap();
            args.push(Spanned::new(Expr::from_pest(comp_pair)?, arg_span));
        }
    }
    Ok(args)
}

#[cfg(test)]
mod tests;
