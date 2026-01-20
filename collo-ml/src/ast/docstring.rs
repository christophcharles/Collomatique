use super::{
    AstError, DocstringLine, DocstringPart, Expr, MaybeTypeName, NamespacePath, SimpleTypeName,
    Span, Spanned, TypeName,
};
use crate::parser::{ColloMLParser, Rule};
use pest::Parser;

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
            let dummy_span = Span {
                start: expr_span_start,
                end: expr_span_start,
            };
            let string_type = TypeName {
                types: vec![Spanned::new(
                    MaybeTypeName {
                        maybe_count: 0,
                        inner: SimpleTypeName::Path(Spanned::new(
                            NamespacePath {
                                segments: vec![Spanned::new(
                                    "String".to_string(),
                                    dummy_span.clone(),
                                )],
                            },
                            dummy_span.clone(),
                        )),
                    },
                    dummy_span,
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
