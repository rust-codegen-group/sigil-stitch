use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use syn::parse::{ParseStream, Parser};

use crate::ir::StringValue;

pub(super) fn parse_expr(
    mut tokens: TokenStream,
    context: &str,
    fallback_span: Span,
) -> syn::Result<syn::Expr> {
    let mut sentinel = Punct::new('@', Spacing::Alone);
    sentinel.set_span(fallback_span);
    tokens.extend([TokenTree::Punct(sentinel)]);

    let parser = |input: ParseStream<'_>| {
        let expr = input.parse::<syn::Expr>()?;
        if !input.peek(syn::Token![@]) {
            return Err(input.error("unexpected token after expression"));
        }
        input.parse::<syn::Token![@]>()?;
        if !input.is_empty() {
            return Err(input.error("unexpected token after expression"));
        }
        Ok(expr)
    };

    parser.parse2(tokens).map_err(|error| {
        syn::Error::new(
            error.span(),
            format!("invalid {context} expression: {error}"),
        )
    })
}

pub(super) fn parse_string_expr(
    tokens: TokenStream,
    marker: &str,
    fallback_span: Span,
) -> syn::Result<StringValue> {
    let expr = parse_expr(tokens, marker, fallback_span)?;
    match &expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(literal),
            ..
        }) => parse_literal(literal, marker),
        _ => Ok(StringValue::Dynamic(expr)),
    }
}

fn parse_literal(literal: &syn::LitStr, marker: &str) -> syn::Result<StringValue> {
    let input = literal.value();
    let mut format_string = String::with_capacity(input.len());
    let mut literal_string = String::with_capacity(input.len());
    let mut expressions = Vec::new();
    let mut errors: Option<syn::Error> = None;
    let mut saw_escape = false;
    let mut saw_interpolation = false;
    let mut offset = 0;

    while offset < input.len() {
        if input[offset..].starts_with("@@") {
            saw_escape = true;
            format_string.push('@');
            literal_string.push('@');
            offset += 2;
            continue;
        }

        if input[offset..].starts_with("@{") {
            saw_interpolation = true;
            let marker_offset = offset;
            let expr_start = offset + 2;
            let Some(candidate) = interpolation_end(&input, expr_start) else {
                combine_error(
                    &mut errors,
                    syn::Error::new(
                        literal.span(),
                        format!(
                            "{marker} has an unclosed `@{{` at decoded byte offset {marker_offset}"
                        ),
                    ),
                );
                break;
            };

            let expression = &input[expr_start..candidate];
            if expression.trim().is_empty() {
                combine_error(
                    &mut errors,
                    syn::Error::new(
                        literal.span(),
                        format!(
                            "{marker} has an empty `@{{}}` at decoded byte offset {marker_offset}"
                        ),
                    ),
                );
            } else {
                match syn::parse_str::<syn::Expr>(expression) {
                    Ok(expr) => expressions.push(expr),
                    Err(error) => combine_error(
                        &mut errors,
                        syn::Error::new(
                            literal.span(),
                            format!(
                                "{marker} has invalid `@{{...}}` syntax at decoded byte offset \
                                 {marker_offset}: {error}"
                            ),
                        ),
                    ),
                }
            }

            format_string.push_str("{}");
            offset = candidate + 1;
            continue;
        }

        let Some(ch) = input[offset..].chars().next() else {
            break;
        };
        match ch {
            '{' => format_string.push_str("{{"),
            '}' => format_string.push_str("}}"),
            _ => format_string.push(ch),
        }
        literal_string.push(ch);
        offset += ch.len_utf8();
    }

    if let Some(error) = errors {
        return Err(error);
    }

    if saw_interpolation {
        Ok(StringValue::Interpolated {
            format_string,
            expressions,
        })
    } else {
        let value = if saw_escape { literal_string } else { input };
        Ok(StringValue::Literal(value))
    }
}

/// Find the top-level closing brace in one pass while skipping Rust literals
/// and comments that may contain brace characters.
fn interpolation_end(input: &str, start: usize) -> Option<usize> {
    let mut pos = start;
    let mut brace_depth = 0usize;

    while pos < input.len() {
        if input[pos..].starts_with("//") {
            pos = input[pos..]
                .find('\n')
                .map_or(input.len(), |newline| pos + newline + 1);
            continue;
        }
        if input[pos..].starts_with("/*") {
            pos = block_comment_end(input, pos);
            continue;
        }
        if let Some((content_start, hashes)) = raw_string_start(input, pos) {
            pos = raw_string_end(input, content_start, hashes).unwrap_or(input.len());
            continue;
        }
        if input[pos..].starts_with('"') {
            pos = cooked_literal_end(input, pos, '"').unwrap_or(input.len());
            continue;
        }
        if input[pos..].starts_with('\'')
            && !looks_like_lifetime(input, pos)
            && let Some(end) = cooked_literal_end(input, pos, '\'')
        {
            pos = end;
            continue;
        }

        let ch = input[pos..].chars().next()?;
        match ch {
            '{' => brace_depth += 1,
            '}' if brace_depth == 0 => return Some(pos),
            '}' => brace_depth -= 1,
            _ => {}
        }
        pos += ch.len_utf8();
    }

    None
}

fn block_comment_end(input: &str, start: usize) -> usize {
    let mut pos = start + 2;
    let mut depth = 1usize;

    while pos < input.len() {
        if input[pos..].starts_with("/*") {
            depth += 1;
            pos += 2;
        } else if input[pos..].starts_with("*/") {
            depth -= 1;
            pos += 2;
            if depth == 0 {
                return pos;
            }
        } else {
            let Some(ch) = input[pos..].chars().next() else {
                break;
            };
            pos += ch.len_utf8();
        }
    }

    input.len()
}

fn raw_string_start(input: &str, start: usize) -> Option<(usize, usize)> {
    if start > 0
        && input[..start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch == '_' || ch.is_alphanumeric())
    {
        return None;
    }

    let rest = &input[start..];
    let prefix_len = if rest.starts_with("br") || rest.starts_with("cr") {
        2
    } else if rest.starts_with('r') {
        1
    } else {
        return None;
    };
    let after_prefix = start + prefix_len;
    let hashes = input[after_prefix..]
        .bytes()
        .take_while(|byte| *byte == b'#')
        .count();
    let quote = after_prefix + hashes;
    if input.as_bytes().get(quote) != Some(&b'"') {
        return None;
    }

    Some((quote + 1, hashes))
}

fn raw_string_end(input: &str, mut pos: usize, hashes: usize) -> Option<usize> {
    while pos < input.len() {
        let relative = input[pos..].find('"')?;
        let quote = pos + relative;
        let end = quote + 1 + hashes;
        if end <= input.len() && input[quote + 1..end].bytes().all(|byte| byte == b'#') {
            return Some(end);
        }
        pos = quote + 1;
    }
    None
}

fn cooked_literal_end(input: &str, start: usize, delimiter: char) -> Option<usize> {
    let mut pos = start + delimiter.len_utf8();

    while pos < input.len() {
        let ch = input[pos..].chars().next()?;
        if ch == '\\' {
            pos += ch.len_utf8();
            if let Some(escaped) = input[pos..].chars().next() {
                pos += escaped.len_utf8();
            }
            continue;
        }
        if ch == delimiter {
            return Some(pos + ch.len_utf8());
        }
        if delimiter == '\'' && ch == '\n' {
            return None;
        }
        pos += ch.len_utf8();
    }

    None
}

fn looks_like_lifetime(input: &str, start: usize) -> bool {
    let mut chars = input[start + 1..].chars().peekable();
    let Some(first) = chars.peek().copied() else {
        return false;
    };
    if first != '_' && !first.is_alphabetic() {
        return false;
    }

    chars.next();
    while chars
        .peek()
        .is_some_and(|ch| *ch == '_' || ch.is_alphanumeric())
    {
        chars.next();
    }

    !matches!(chars.peek(), Some('\''))
}

fn combine_error(slot: &mut Option<syn::Error>, error: syn::Error) {
    if let Some(existing) = slot {
        existing.combine(error);
    } else {
        *slot = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> syn::Result<StringValue> {
        let literal = syn::parse_str::<syn::LitStr>(src)?;
        parse_literal(&literal, "$V")
    }

    #[test]
    fn ordinary_and_raw_literals_decode_through_lit_str() {
        assert!(matches!(
            parse(r#""line\n@{name}""#).unwrap(),
            StringValue::Interpolated { .. }
        ));
        assert!(matches!(
            parse(r##"r#"raw @{name}"#"##).unwrap(),
            StringValue::Interpolated { .. }
        ));
    }

    #[test]
    fn braces_inside_rust_syntax_do_not_close_the_wrapper() {
        let parsed = parse(r##"r#"@{format!("{}", value)}"#"##).unwrap();
        let StringValue::Interpolated { expressions, .. } = parsed else {
            panic!("expected interpolation");
        };
        assert_eq!(expressions.len(), 1);
    }

    #[test]
    fn line_comment_brace_does_not_close_the_wrapper() {
        let parsed = parse("r#\"@{value // }\n}\"#").unwrap();
        let StringValue::Interpolated { expressions, .. } = parsed else {
            panic!("expected interpolation");
        };
        assert_eq!(expressions.len(), 1);
    }

    #[test]
    fn malformed_group_and_later_group_both_report() {
        let error = match parse(r#""@{let} and @{also let}""#) {
            Ok(_) => panic!("expected malformed interpolation"),
            Err(error) => error,
        };
        assert_eq!(error.into_iter().count(), 2);
    }

    #[test]
    fn large_brace_heavy_expression_finds_the_real_close() {
        let braces = "}".repeat(16_384);
        let source = format!("r#\"@{{\"{braces}\".len()}}\"#");
        assert!(matches!(
            parse(&source).unwrap(),
            StringValue::Interpolated { .. }
        ));
    }

    #[test]
    fn boundary_scanner_skips_rust_literals_comments_and_lifetimes() {
        let source = r###"
            let value: &'static str = "}";
            let raw = r##"}"##;
            let byte_raw = br#"}"#;
            let character = '}';
            /* } /* } */ } */
            value // }
        } tail"###;
        let end = interpolation_end(source, 0).unwrap();

        assert_eq!(&source[end..], "} tail");
    }

    #[test]
    fn empty_and_unclosed_include_decoded_offsets() {
        let empty = match parse(r#""prefix @{}""#) {
            Ok(_) => panic!("expected empty interpolation error"),
            Err(error) => error.to_string(),
        };
        assert!(empty.contains("decoded byte offset 7"), "{empty}");
        let unclosed = match parse(r#""prefix @{value""#) {
            Ok(_) => panic!("expected unclosed interpolation error"),
            Err(error) => error.to_string(),
        };
        assert!(unclosed.contains("decoded byte offset 7"), "{unclosed}");
    }

    #[test]
    fn expression_parser_does_not_invent_parentheses() {
        assert!(parse_expr(TokenStream::new(), "$L", Span::call_site()).is_err());
        assert!(parse_expr(r#""a", "b""#.parse().unwrap(), "$L", Span::call_site(),).is_err());
        assert!(parse_expr(r#"("a", "b")"#.parse().unwrap(), "$L", Span::call_site(),).is_ok());
    }
}
