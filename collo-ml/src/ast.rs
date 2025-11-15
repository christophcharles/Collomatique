use crate::parser::Rule;
use pest::iterators::Pair;

// ============= Span and Spanned =============

#[derive(Debug, Clone, PartialEq, Eq)]
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
        name: String,
        params: Vec<Spanned<Param>>,
        output_type: OutputType, // Declared type
        body: Spanned<Expr>,     // Body (can be LinExpr or Constraint)
    },
    Reify {
        docstring: Vec<String>,
        constraint_name: String,
        var_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub typ: InputType,
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
    Primitive(String),    // Student, Week, etc
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
        args: Vec<Computable>,
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
