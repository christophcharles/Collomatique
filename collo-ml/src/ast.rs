use crate::parser::Rule;
use pest::iterators::Pair;

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
        docstring: Vec<String>,
        public: bool,
        name: Spanned<String>,
        params: Vec<Param>,
        output_type: Spanned<TypeName>, // Declared type
        body: Spanned<Expr>,            // Body
    },
    Reify {
        docstring: Vec<String>,
        constraint_name: Spanned<String>,
        var_list: bool,
        name: Spanned<String>,
    },
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
    LinExpr,
    Constraint,
    None,
    Int,
    Bool,
    Object(String), // Student, Week, etc
    EmptyList,
    List(Spanned<TypeName>), // [Student], [[Int]], etc.
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
    Ident(Spanned<String>),
    Path {
        object: Box<Spanned<Expr>>, // first segment might be an expression - for "get_group().student.age" this is "get_group()"
        segments: Vec<Spanned<String>>, // and this is ["student", "age"]
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
    TypeConversion {
        expr: Box<Spanned<Expr>>,
        typ: Spanned<TypeName>,
    },
    ExplicitType {
        expr: Box<Spanned<Expr>>,
        typ: Spanned<TypeName>,
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
                    // statement is a wrapper containing let_statement or reify_statement
                    let span = Span::from_pest(&inner_pair);
                    let actual_stmt = inner_pair.into_inner().next().unwrap();
                    let stmt = Statement::from_pest(actual_stmt)?;
                    statements.push(Spanned::new(stmt, span));
                }
                Rule::EOI => {}
                _ => {
                    return Err(AstError::UnexpectedRule {
                        expected: "let_statement, reify_statement, or EOI",
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
            _ => Err(AstError::UnexpectedRule {
                expected: "let_statement or reify_statement",
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
                    let content = inner_pair
                        .into_inner()
                        .next()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_default();
                    docstring.push(content);
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
        let mut constraint_name = None;
        let mut name = None;
        let mut var_list = false;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::docstring => {
                    let content = inner_pair
                        .into_inner()
                        .next()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_default();
                    docstring.push(content);
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
            constraint_name: constraint_name.ok_or(AstError::MissingName(span.clone()))?,
            name: name.ok_or(AstError::MissingName(span))?,
            var_list,
        })
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
                    _ => Err(AstError::UnexpectedRule {
                        expected: "primitive_type or type_name",
                        found: inner_pair.as_rule(),
                        span: Span::from_pest(&inner_pair),
                    }),
                }
            }
        }
    }

    fn from_primitive_type(pair: Pair<Rule>) -> Result<Self, AstError> {
        match pair.as_rule() {
            Rule::primitive_type => {
                let type_name = pair.as_str();
                match type_name {
                    "None" => Ok(SimpleTypeName::None),
                    "Int" => Ok(SimpleTypeName::Int),
                    "Bool" => Ok(SimpleTypeName::Bool),
                    "LinExpr" => Ok(SimpleTypeName::LinExpr),
                    "Constraint" => Ok(SimpleTypeName::Constraint),
                    _ => Ok(SimpleTypeName::Object(type_name.to_string())),
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

        // expr -> and_expr
        let and_expr = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;
        Self::from_or_expr(and_expr)
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
        let mut result = Self::from_type_conversion(first)?;

        while let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_type_conversion(right_pair)?;

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

    fn from_type_conversion(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::type_conversion {
            return Err(AstError::UnexpectedRule {
                expected: "type_conversion",
                found: pair.as_rule(),
                span,
            });
        }

        let mut inner = pair.into_inner();

        // First is always path
        let expr_pair = inner.next().unwrap();
        let expr_span = Span::from_pest(&expr_pair);
        let expr = Self::from_explicit_type(expr_pair)?;

        // Check if there's a type annotation
        if let Some(type_pair) = inner.next() {
            // This is the type_name after "as"
            let type_span = Span::from_pest(&type_pair);
            let typ = TypeName::from_pest(type_pair)?;

            Ok(Expr::TypeConversion {
                expr: Box::new(Spanned::new(expr, expr_span)),
                typ: Spanned::new(typ, type_span),
            })
        } else {
            // No type annotation, just return the expression
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
            if segment.as_rule() == Rule::ident {
                let segment_span = Span::from_pest(&segment);
                segments.push(Spanned::new(segment.as_str().to_string(), segment_span));
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
            Rule::fn_call => Self::from_fn_call(inner),
            Rule::boolean => Self::from_boolean(inner),
            Rule::none => Self::from_none(inner),
            Rule::neg => Self::from_neg(inner),
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
                expected: "if_expr sum cardinality list_comprehension list_literal global_collection var_call fn_call boolean number path expr",
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
