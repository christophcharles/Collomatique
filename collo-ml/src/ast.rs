use crate::parser::Rule;
use pest::iterators::Pair;

// ============= Span and Spanned =============

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Let {
        docstring: Vec<String>,
        public: bool,
        name: Spanned<String>,
        params: Vec<Param>,
        output_type: OutputType, // Declared type
        body: Spanned<Expr>,     // Body (can be LinExpr or Constraint)
    },
    Reify {
        docstring: Vec<String>,
        constraint_name: Spanned<String>,
        var_name: Spanned<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Spanned<String>,
    pub typ: Spanned<InputType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputType {
    LinExpr,
    Constraint,
}

// ============= Input Types (for parameters) =============

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputType {
    Int,
    Bool,
    Object(String),       // Student, Week, etc
    List(Box<InputType>), // [Student], [[Int]], etc.
}

// ============= Expressions =============

// Top-level expression (can be LinExpr, Constraint)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    LinExpr(LinExpr),
    Constraint(Constraint),
}

// ============= Linear Expressions (runtime, contains ILP variables) =============

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinExpr {
    Var {
        name: String,
        args: Vec<Spanned<Computable>>,
    },
    Constant(Spanned<Computable>),
    Add(Box<Spanned<LinExpr>>, Box<Spanned<LinExpr>>),
    Sub(Box<Spanned<LinExpr>>, Box<Spanned<LinExpr>>),
    Mul {
        coeff: Spanned<Computable>,
        expr: Box<Spanned<LinExpr>>,
    },
    Sum {
        var: String,
        collection: Spanned<Collection>,
        filter: Option<Spanned<Computable>>,
        body: Box<Spanned<LinExpr>>,
    },
    If {
        condition: Spanned<Computable>,
        then_expr: Box<Spanned<LinExpr>>,
        else_expr: Box<Spanned<LinExpr>>,
    },
    FnCall {
        name: String,
        args: Vec<Spanned<Computable>>,
    },
}

// ============= Constraints =============

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint {
    Comparison {
        left: Spanned<LinExpr>,
        op: ComparisonOp,
        right: Spanned<LinExpr>,
    },
    And(Box<Spanned<Constraint>>, Box<Spanned<Constraint>>),
    Forall {
        var: String,
        collection: Spanned<Collection>,
        filter: Option<Spanned<Computable>>,
        body: Box<Spanned<Constraint>>,
    },
    If {
        condition: Spanned<Computable>,
        then_expr: Box<Spanned<Constraint>>,
        else_expr: Box<Spanned<Constraint>>,
    },
    FnCall {
        name: String,
        args: Vec<Spanned<Computable>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonOp {
    LessEq,    // <=
    GreaterEq, // >=
    Equal,     // ==
}

// ============= Computable (compile-time values) =============

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Computable {
    Number(i32),
    Path(Path),

    // Arithmetic
    Add(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Sub(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Mul(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Div(Box<Spanned<Computable>>, Box<Spanned<Computable>>), // //
    Mod(Box<Spanned<Computable>>, Box<Spanned<Computable>>), // %

    // Comparisons
    Eq(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Ne(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Lt(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Le(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Gt(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Ge(Box<Spanned<Computable>>, Box<Spanned<Computable>>),

    // Boolean operations
    And(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Or(Box<Spanned<Computable>>, Box<Spanned<Computable>>),
    Not(Box<Spanned<Computable>>),

    // Collection membership
    In {
        item: Box<Spanned<Computable>>,
        collection: Spanned<Collection>,
    },

    // Other
    Cardinality(Spanned<Collection>),
    If {
        condition: Box<Spanned<Computable>>,
        then_expr: Box<Spanned<Computable>>,
        else_expr: Box<Spanned<Computable>>,
    },
}

// ============= Collections =============

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Collection {
    Global(String),
    Path(Path),
    Union(Box<Spanned<Collection>>, Box<Spanned<Collection>>),
    Inter(Box<Spanned<Collection>>, Box<Spanned<Collection>>),
    Diff(Box<Spanned<Collection>>, Box<Spanned<Collection>>),
}

// ============= Path (field access) =============

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub segments: Vec<String>, // ["student", "age"] for student.age
}

// ============= Error Type =============

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AstError {
    #[error("Expected {expected}, found {found:?} at {span:?}")]
    UnexpectedRule {
        expected: &'static str,
        found: Rule,
        span: Span,
    },
    #[error("Missing name at {0:?}")]
    MissingName(Span),
    #[error("Missing output type at {0:?}")]
    MissingOutputType(Span),
    #[error("Missing statement body at {0:?}")]
    MissingBody(Span),
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
                Rule::output_type => {
                    output_type = Some(parse_output_type(&inner_pair)?);
                }
                Rule::output => {
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
            output_type: output_type.ok_or(AstError::MissingOutputType(span.clone()))?,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_reify_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut docstring = Vec::new();
        let mut constraint_name = None;
        let mut var_name = None;

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
                    var_name = Some(Spanned::new(
                        inner_pair.as_str().trim_start_matches('$').to_string(),
                        Span::from_pest(&inner_pair),
                    ));
                }
                _ => {}
            }
        }

        Ok(Statement::Reify {
            docstring,
            constraint_name: constraint_name.ok_or(AstError::MissingName(span.clone()))?,
            var_name: var_name.ok_or(AstError::MissingName(span))?,
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
                    Rule::input_type_name => {
                        let inner_span = Span::from_pest(&inner);
                        typ = Some(Spanned::new(InputType::from_pest(inner)?, inner_span));
                    }
                    _ => {}
                }
            }

            let param = Param {
                name: name.ok_or(AstError::MissingName(span.clone()))?,
                typ: typ.ok_or(AstError::MissingOutputType(span.clone()))?,
            };
            params.push(param);
        }
    }
    Ok(params)
}

fn parse_output_type(pair: &Pair<Rule>) -> Result<OutputType, AstError> {
    match pair.as_str() {
        "LinExpr" => Ok(OutputType::LinExpr),
        "Constraint" => Ok(OutputType::Constraint),
        _ => Err(AstError::UnexpectedRule {
            expected: "LinExpr or Constraint",
            found: pair.as_rule(),
            span: Span::from_pest(pair),
        }),
    }
}

impl InputType {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::input_type_name {
            return Err(AstError::UnexpectedRule {
                expected: "input_type_name",
                found: pair.as_rule(),
                span,
            });
        }

        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingOutputType(span))?;

        match inner.as_rule() {
            Rule::primitive_input_type => {
                let type_name = inner.as_str();
                match type_name {
                    "Int" => Ok(InputType::Int),
                    "Bool" => Ok(InputType::Bool),
                    _ => Ok(InputType::Object(type_name.to_string())),
                }
            }
            Rule::input_type_name => {
                // It's a list type: [...]
                Ok(InputType::List(Box::new(Self::from_pest(inner)?)))
            }
            _ => Err(AstError::UnexpectedRule {
                expected: "primitive_input_type or input_type_name",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }
}

impl Expr {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::output {
            return Err(AstError::UnexpectedRule {
                expected: "output",
                found: pair.as_rule(),
                span,
            });
        }

        // output contains either constraint_expr or lin_expr
        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::constraint_expr => Ok(Expr::Constraint(Constraint::from_pest(inner)?)),
            Rule::lin_expr => Ok(Expr::LinExpr(LinExpr::from_pest(inner)?)),
            _ => Err(AstError::UnexpectedRule {
                expected: "constraint_expr or lin_expr",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }
}

impl Constraint {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::constraint_expr {
            return Err(AstError::UnexpectedRule {
                expected: "constraint_expr",
                found: pair.as_rule(),
                span,
            });
        }

        // constraint_expr -> and_expr
        let and_expr = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;
        Self::from_and_expr(and_expr)
    }

    fn from_and_expr(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First quantified_expr
        let first = inner.next().unwrap();
        let mut result = Self::from_quantified(first)?;

        // Chain together with 'and'
        while let Some(_and_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_quantified(right_pair)?;

            let result_span = span.clone();
            result = Constraint::And(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_quantified(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::forall => Self::from_forall(inner),
            Rule::primary_constraint_expr => Self::from_primary(inner),
            _ => Err(AstError::UnexpectedRule {
                expected: "forall or primary_constraint_expr",
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

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    if var.is_none() {
                        var = Some(inner.as_str().to_string());
                    }
                }
                Rule::collection_expr => {
                    let coll_span = Span::from_pest(&inner);
                    collection = Some(Spanned::new(Collection::from_pest(inner)?, coll_span));
                }
                Rule::computable => {
                    // This is the filter (where clause)
                    let comp_span = Span::from_pest(&inner);
                    filter = Some(Spanned::new(Computable::from_pest(inner)?, comp_span));
                }
                Rule::constraint_expr => {
                    let body_span = Span::from_pest(&inner);
                    body = Some(Box::new(Spanned::new(Self::from_pest(inner)?, body_span)));
                }
                _ => {}
            }
        }

        Ok(Constraint::Forall {
            var: var.ok_or(AstError::MissingName(span.clone()))?,
            collection: collection.ok_or(AstError::MissingBody(span.clone()))?,
            filter,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_primary(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::if_constraint_expr => Self::from_if(inner),
            Rule::from_lin_expr => Self::from_comparison(inner),
            Rule::fn_call => Self::from_fn_call(inner),
            Rule::constraint_expr => Self::from_pest(inner),
            _ => Err(AstError::UnexpectedRule {
                expected: "if_constraint_expr, from_lin_expr, fn_call, or constraint_expr",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }

    fn from_if(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let condition_pair = inner.next().unwrap();
        let condition_span = Span::from_pest(&condition_pair);
        let condition = Spanned::new(Computable::from_pest(condition_pair)?, condition_span);

        let then_pair = inner.next().unwrap();
        let then_span = Span::from_pest(&then_pair);
        let then_expr = Box::new(Spanned::new(Self::from_pest(then_pair)?, then_span));

        let else_pair = inner.next().unwrap();
        let else_span = Span::from_pest(&else_pair);
        let else_expr = Box::new(Spanned::new(Self::from_pest(else_pair)?, else_span));

        Ok(Constraint::If {
            condition,
            then_expr,
            else_expr,
        })
    }

    fn from_comparison(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let left_pair = inner.next().unwrap();
        let left_span = Span::from_pest(&left_pair);
        let left = Spanned::new(LinExpr::from_pest(left_pair)?, left_span);

        let op_pair = inner.next().unwrap();
        let op = match op_pair.as_str() {
            "<=" => ComparisonOp::LessEq,
            ">=" => ComparisonOp::GreaterEq,
            "==" => ComparisonOp::Equal,
            _ => {
                return Err(AstError::UnexpectedRule {
                    expected: "<=, >=, or ==",
                    found: op_pair.as_rule(),
                    span: Span::from_pest(&op_pair),
                })
            }
        };

        let right_pair = inner.next().unwrap();
        let right_span = Span::from_pest(&right_pair);
        let right = Spanned::new(LinExpr::from_pest(right_pair)?, right_span);

        Ok(Constraint::Comparison { left, op, right })
    }

    fn from_fn_call(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut name = None;
        let mut args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    name = Some(inner.as_str().to_string());
                }
                Rule::args => {
                    args = parse_args(inner)?;
                }
                _ => {}
            }
        }

        Ok(Constraint::FnCall {
            name: name.ok_or(AstError::MissingName(span))?,
            args,
        })
    }
}

fn parse_args(pair: Pair<Rule>) -> Result<Vec<Spanned<Computable>>, AstError> {
    let mut args = Vec::new();
    for arg_pair in pair.into_inner() {
        if arg_pair.as_rule() == Rule::arg {
            let arg_span = Span::from_pest(&arg_pair);
            // arg contains computable
            let comp_pair = arg_pair.into_inner().next().unwrap();
            args.push(Spanned::new(Computable::from_pest(comp_pair)?, arg_span));
        }
    }
    Ok(args)
}

impl LinExpr {
    fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::lin_expr {
            return Err(AstError::UnexpectedRule {
                expected: "lin_expr",
                found: pair.as_rule(),
                span,
            });
        }

        // lin_expr -> lin_add_sub_expr
        let add_sub = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;
        Self::from_add_sub(add_sub)
    }

    fn from_add_sub(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        // First term
        let first = inner.next().unwrap();
        let mut result = Self::from_term(first)?;

        // Chain together with +/-
        while let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_term(right_pair)?;

            let result_span = span.clone();
            result = match op_pair.as_rule() {
                Rule::add_op => LinExpr::Add(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::sub_op => LinExpr::Sub(
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

    fn from_term(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner().peekable();

        let first = inner.next().unwrap();

        // Check if there's a multiplication pattern (coeff * variable)
        if let Some(second) = inner.peek() {
            if second.as_rule() == Rule::mul_op {
                // ... existing multiplication handling ...
                inner.next(); // consume mul_op
                let atom = inner.next().unwrap();

                let coeff_span = Span::from_pest(&first);
                let coeff = if first.as_rule() == Rule::computable_primary {
                    Spanned::new(Computable::from_primary(first)?, coeff_span)
                } else {
                    let comp_inner = first.into_inner().next().unwrap();
                    Spanned::new(Computable::from_pest(comp_inner)?, coeff_span)
                };

                let expr_span = Span::from_pest(&atom);
                let expr = Box::new(Spanned::new(Self::from_atom(atom)?, expr_span));

                return Ok(LinExpr::Mul { coeff, expr });
            }
        }

        // Otherwise, it's just a lin_atom
        match first.as_rule() {
            Rule::lin_atom => Self::from_atom(first),
            _ => Err(AstError::UnexpectedRule {
                expected: "lin_atom",
                found: first.as_rule(),
                span: Span::from_pest(&first),
            }),
        }
    }

    fn from_atom(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::sum_expr => Self::from_sum(inner),
            Rule::if_lin_expr => Self::from_if(inner),
            Rule::var_call => Self::from_var_call(inner),
            Rule::fn_call => Self::from_fn_call(inner),
            Rule::lin_expr => Self::from_pest(inner),
            Rule::computable_mul_div_mod => {
                let comp_span = Span::from_pest(&inner);
                Ok(LinExpr::Constant(Spanned::new(
                    Computable::from_mul_div_mod(inner)?,
                    comp_span,
                )))
            }
            _ => Err(AstError::UnexpectedRule {
                expected:
                    "sum_expr, if_lin_expr, var_call, fn_call, lin_expr, or computable_mul_div_mod",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }

    fn from_sum(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut var = None;
        let mut collection = None;
        let mut filter = None;
        let mut body = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    if var.is_none() {
                        var = Some(inner.as_str().to_string());
                    }
                }
                Rule::collection_expr => {
                    let coll_span = Span::from_pest(&inner);
                    collection = Some(Spanned::new(Collection::from_pest(inner)?, coll_span));
                }
                Rule::computable => {
                    // This is the filter (where clause)
                    let comp_span = Span::from_pest(&inner);
                    filter = Some(Spanned::new(Computable::from_pest(inner)?, comp_span));
                }
                Rule::lin_expr => {
                    let body_span = Span::from_pest(&inner);
                    body = Some(Box::new(Spanned::new(Self::from_pest(inner)?, body_span)));
                }
                _ => {}
            }
        }

        Ok(LinExpr::Sum {
            var: var.ok_or(AstError::MissingName(span.clone()))?,
            collection: collection.ok_or(AstError::MissingBody(span.clone()))?,
            filter,
            body: body.ok_or(AstError::MissingBody(span))?,
        })
    }

    fn from_if(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let condition_pair = inner.next().unwrap();
        let condition_span = Span::from_pest(&condition_pair);
        let condition = Spanned::new(Computable::from_pest(condition_pair)?, condition_span);

        let then_pair = inner.next().unwrap();
        let then_span = Span::from_pest(&then_pair);
        let then_expr = Box::new(Spanned::new(Self::from_pest(then_pair)?, then_span));

        let else_pair = inner.next().unwrap();
        let else_span = Span::from_pest(&else_pair);
        let else_expr = Box::new(Spanned::new(Self::from_pest(else_pair)?, else_span));

        Ok(LinExpr::If {
            condition,
            then_expr,
            else_expr,
        })
    }

    fn from_var_call(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut name = None;
        let mut args = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::ident => {
                    name = Some(inner.as_str().to_string());
                }
                Rule::args => {
                    args = parse_args(inner)?;
                }
                _ => {}
            }
        }

        Ok(LinExpr::Var {
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
                    name = Some(inner.as_str().to_string());
                }
                Rule::args => {
                    args = parse_args(inner)?; // Use the same parse_args as Constraint
                }
                _ => {}
            }
        }

        Ok(LinExpr::FnCall {
            name: name.ok_or(AstError::MissingName(span))?,
            args,
        })
    }
}

impl Computable {
    pub fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::computable {
            return Err(AstError::UnexpectedRule {
                expected: "computable",
                found: pair.as_rule(),
                span,
            });
        }

        // computable -> computable_or
        let or_expr = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;
        Self::from_or(or_expr)
    }

    fn from_or(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_and(first)?;

        while let Some(_or_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_and(right_pair)?;

            let result_span = span.clone();
            result = Computable::Or(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_and(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_not(first)?;

        while let Some(_and_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_not(right_pair)?;

            let result_span = span.clone();
            result = Computable::And(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_not(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::cond_not_op => {
                // It's a not expression
                let expr_pair = inner.next().unwrap();
                let expr_span = Span::from_pest(&expr_pair);
                let expr = Self::from_not(expr_pair)?;
                Ok(Computable::Not(Box::new(Spanned::new(expr, expr_span))))
            }
            Rule::computable_comparison => Self::from_comparison(first),
            _ => Err(AstError::UnexpectedRule {
                expected: "cond_not_op or computable_comparison",
                found: first.as_rule(),
                span: Span::from_pest(&first),
            }),
        }
    }

    fn from_comparison(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::computable_in => Self::from_in(first),
            Rule::computable_relational => Self::from_relational(first),
            _ => Err(AstError::UnexpectedRule {
                expected: "computable_in or computable_relational",
                found: first.as_rule(),
                span: Span::from_pest(&first),
            }),
        }
    }

    fn from_in(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut inner = pair.into_inner();

        let item_pair = inner.next().unwrap();
        let item_span = Span::from_pest(&item_pair);
        let item = Box::new(Spanned::new(Self::from_add_sub(item_pair)?, item_span));

        let _in_op = inner.next().unwrap(); // consume "in"

        let coll_pair = inner.next().unwrap();
        let coll_span = Span::from_pest(&coll_pair);
        let collection = Spanned::new(Collection::from_pest(coll_pair)?, coll_span);

        Ok(Computable::In { item, collection })
    }

    fn from_relational(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let left_pair = inner.next().unwrap();
        let left = Self::from_add_sub(left_pair)?;

        // Check if there's a comparison operator
        if let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_add_sub(right_pair)?;

            let result_span = span.clone();
            let left_spanned = Box::new(Spanned::new(left, result_span.clone()));
            let right_spanned = Box::new(Spanned::new(right, result_span));

            match op_pair.as_str() {
                "==" => Ok(Computable::Eq(left_spanned, right_spanned)),
                "!=" => Ok(Computable::Ne(left_spanned, right_spanned)),
                "<" => Ok(Computable::Lt(left_spanned, right_spanned)),
                "<=" => Ok(Computable::Le(left_spanned, right_spanned)),
                ">" => Ok(Computable::Gt(left_spanned, right_spanned)),
                ">=" => Ok(Computable::Ge(left_spanned, right_spanned)),
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

    fn from_add_sub(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_mul_div_mod(first)?;

        while let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_mul_div_mod(right_pair)?;

            let result_span = span.clone();
            result = match op_pair.as_rule() {
                Rule::add_op => Computable::Add(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::sub_op => Computable::Sub(
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

    fn from_mul_div_mod(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_primary(first)?;

        while let Some(op_pair) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_primary(right_pair)?;

            let result_span = span.clone();
            result = match op_pair.as_rule() {
                Rule::mul_op => Computable::Mul(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::div_op => Computable::Div(
                    Box::new(Spanned::new(result, result_span.clone())),
                    Box::new(Spanned::new(right, result_span)),
                ),
                Rule::mod_op => Computable::Mod(
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

    pub fn from_primary(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::computable_primary {
            return Err(AstError::UnexpectedRule {
                expected: "computable_primary",
                found: pair.as_rule(),
                span,
            });
        }

        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::if_computable => Self::from_if(inner),
            Rule::cardinality => Self::from_cardinality(inner),
            Rule::number => {
                let num_str = inner.as_str();
                let value = num_str
                    .parse::<i32>()
                    .map_err(|e| AstError::ParseIntError {
                        span: Span::from_pest(&inner),
                        error: e,
                    })?;
                Ok(Computable::Number(value))
            }
            Rule::path => Ok(Computable::Path(Path::from_pest(inner)?)),
            Rule::computable => {
                // Parenthesized computable
                Self::from_pest(inner)
            }
            _ => Err(AstError::UnexpectedRule {
                expected: "if_computable, cardinality, number, path, or computable",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }

    fn from_if(pair: Pair<Rule>) -> Result<Self, AstError> {
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

        Ok(Computable::If {
            condition,
            then_expr,
            else_expr,
        })
    }

    fn from_cardinality(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        // cardinality = { "|" ~ collection_expr ~ "|" }
        let coll_pair = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        let coll_span = Span::from_pest(&coll_pair);
        let collection = Spanned::new(Collection::from_pest(coll_pair)?, coll_span);

        Ok(Computable::Cardinality(collection))
    }
}

impl Collection {
    pub fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::collection_expr {
            return Err(AstError::UnexpectedRule {
                expected: "collection_expr",
                found: pair.as_rule(),
                span,
            });
        }

        // collection_expr -> union_expr
        let union_expr = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;
        Self::from_union(union_expr)
    }

    fn from_union(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_inter(first)?;

        while let Some(_union_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_inter(right_pair)?;

            let result_span = span.clone();
            result = Collection::Union(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_inter(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_diff(first)?;

        while let Some(_inter_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_diff(right_pair)?;

            let result_span = span.clone();
            result = Collection::Inter(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_diff(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        let mut inner = pair.into_inner();

        let first = inner.next().unwrap();
        let mut result = Self::from_collection(first)?;

        // diff can only appear once (based on grammar: diff_op ~ collection)? )
        if let Some(_diff_op) = inner.next() {
            let right_pair = inner.next().unwrap();
            let right = Self::from_collection(right_pair)?;

            let result_span = span.clone();
            result = Collection::Diff(
                Box::new(Spanned::new(result, result_span.clone())),
                Box::new(Spanned::new(right, result_span)),
            );
        }

        Ok(result)
    }

    fn from_collection(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::collection {
            return Err(AstError::UnexpectedRule {
                expected: "collection",
                found: pair.as_rule(),
                span,
            });
        }

        let inner = pair
            .into_inner()
            .next()
            .ok_or(AstError::MissingBody(span))?;

        match inner.as_rule() {
            Rule::primitive_input_type => {
                // Global collection: @[Type]
                let type_name = inner.as_str().to_string();
                Ok(Collection::Global(type_name))
            }
            Rule::path => Ok(Collection::Path(Path::from_pest(inner)?)),
            Rule::collection_expr => {
                // Parenthesized collection expression
                Self::from_pest(inner)
            }
            _ => Err(AstError::UnexpectedRule {
                expected: "primitive_input_type, path, or collection_expr",
                found: inner.as_rule(),
                span: Span::from_pest(&inner),
            }),
        }
    }
}

impl Path {
    pub fn from_pest(pair: Pair<Rule>) -> Result<Self, AstError> {
        let span = Span::from_pest(&pair);
        if pair.as_rule() != Rule::path {
            return Err(AstError::UnexpectedRule {
                expected: "path",
                found: pair.as_rule(),
                span,
            });
        }

        let mut segments = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::ident {
                segments.push(inner.as_str().to_string());
            }
        }

        Ok(Path { segments })
    }
}

#[cfg(test)]
mod tests;
