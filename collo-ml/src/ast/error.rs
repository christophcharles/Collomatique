use super::Span;
use crate::parser::Rule;
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
