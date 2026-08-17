use proc_macro2::{Delimiter, TokenTree};

use super::util::{is_ident, is_semicolon};

pub(super) struct Recovered<T> {
    pub(super) next_pos: usize,
    pub(super) value: syn::Result<T>,
}

pub(super) enum LineBoundary {
    Continue,
    ContinueWithoutMarker,
    Split,
}

pub(super) fn line_boundary(
    tokens: &[TokenTree],
    pos: usize,
    collected: &[TokenTree],
    prev_end_line: Option<usize>,
) -> LineBoundary {
    if collected.is_empty()
        || !prev_end_line.is_some_and(|line| tokens[pos].span().start().line > line)
    {
        return LineBoundary::Continue;
    }

    let count = collected.len();
    if count >= 2
        && matches!(&collected[count - 2], TokenTree::Punct(p) if p.as_char() == '$')
        && matches!(&collected[count - 1], TokenTree::Punct(p) if p.as_char() == '+')
    {
        return LineBoundary::ContinueWithoutMarker;
    }

    let starts_with_dot = matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '.');
    let continues_with_inline_meta = (starts_with_inline_meta(tokens, pos)
        && can_continue_before_inline_meta(collected))
        || (starts_with_inline_if_tail(tokens, pos) && contains_inline_if(collected));

    if starts_with_dot || continues_with_inline_meta {
        LineBoundary::Continue
    } else {
        LineBoundary::Split
    }
}

pub(super) fn next_statement_boundary(tokens: &[TokenTree], start: usize) -> usize {
    if let Some(boundary) = directive_boundary(tokens, start) {
        return boundary;
    }

    let mut collected = Vec::new();
    let mut prev_end_line = None;
    let mut pos = start;
    let mut in_control_flow_header = false;

    while pos < tokens.len() {
        let token = &tokens[pos];
        if collected.is_empty()
            && let TokenTree::Ident(ident) = token
            && matches!(ident.to_string().as_str(), "if" | "for" | "switch")
        {
            in_control_flow_header = true;
        }

        if is_semicolon(token) && !in_control_flow_header {
            return pos + 1;
        }

        match line_boundary(tokens, pos, &collected, prev_end_line) {
            LineBoundary::Continue => {}
            LineBoundary::ContinueWithoutMarker => {
                collected.pop();
                collected.pop();
            }
            LineBoundary::Split => return pos,
        }

        collected.push(token.clone());
        prev_end_line = Some(token.span().end().line);
        pos += 1;
    }

    pos
}

fn directive_boundary(tokens: &[TokenTree], start: usize) -> Option<usize> {
    if !is_marker(tokens, start, "if")
        && !is_marker(tokens, start, "for")
        && !is_marker(tokens, start, "let")
        && !is_marker(tokens, start, "comment")
        && !is_marker(tokens, start, "attr")
        && !is_marker(tokens, start, "C_each")
    {
        return None;
    }

    if is_marker(tokens, start, "if") {
        if !is_group(tokens, start + 2, Delimiter::Parenthesis)
            || !is_group(tokens, start + 3, Delimiter::Brace)
        {
            return None;
        }

        let mut pos = start + 4;
        while is_marker(tokens, pos, "else_if") {
            if !is_group(tokens, pos + 2, Delimiter::Parenthesis)
                || !is_group(tokens, pos + 3, Delimiter::Brace)
            {
                break;
            }
            pos += 4;
        }
        if is_marker(tokens, pos, "else") && is_group(tokens, pos + 2, Delimiter::Brace) {
            pos += 3;
        }
        return Some(after_optional_semicolon(tokens, pos));
    }

    if is_marker(tokens, start, "for") {
        if !is_group(tokens, start + 2, Delimiter::Parenthesis)
            || !is_group(tokens, start + 3, Delimiter::Brace)
        {
            return None;
        }
        return Some(after_optional_semicolon(tokens, start + 4));
    }

    if !is_group(tokens, start + 2, Delimiter::Parenthesis) {
        return None;
    }
    Some(after_optional_semicolon(tokens, start + 3))
}

fn is_marker(tokens: &[TokenTree], pos: usize, name: &str) -> bool {
    matches!(tokens.get(pos), Some(TokenTree::Punct(p)) if p.as_char() == '$')
        && tokens
            .get(pos + 1)
            .is_some_and(|token| is_ident(token, name))
}

fn is_group(tokens: &[TokenTree], pos: usize, delimiter: Delimiter) -> bool {
    matches!(tokens.get(pos), Some(TokenTree::Group(group)) if group.delimiter() == delimiter)
}

fn after_optional_semicolon(tokens: &[TokenTree], pos: usize) -> usize {
    if tokens.get(pos).is_some_and(is_semicolon) {
        pos + 1
    } else {
        pos
    }
}

pub(super) fn combine(slot: &mut Option<syn::Error>, error: syn::Error) {
    if let Some(existing) = slot {
        existing.combine(error);
    } else {
        *slot = Some(error);
    }
}

fn starts_with_inline_meta(tokens: &[TokenTree], pos: usize) -> bool {
    pos + 1 < tokens.len()
        && matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '$')
        && (is_ident(&tokens[pos + 1], "for") || is_ident(&tokens[pos + 1], "if"))
}

fn starts_with_inline_if_tail(tokens: &[TokenTree], pos: usize) -> bool {
    pos + 1 < tokens.len()
        && matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '$')
        && (is_ident(&tokens[pos + 1], "else_if") || is_ident(&tokens[pos + 1], "else"))
}

fn contains_inline_if(tokens: &[TokenTree]) -> bool {
    tokens.windows(2).any(|pair| {
        matches!(&pair[0], TokenTree::Punct(p) if p.as_char() == '$') && is_ident(&pair[1], "if")
    })
}

fn can_continue_before_inline_meta(tokens: &[TokenTree]) -> bool {
    matches!(
        tokens.last(),
        Some(TokenTree::Punct(p)) if matches!(p.as_char(), '=' | '|')
    )
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;

    use super::*;

    #[test]
    fn recovers_after_balanced_directive_on_the_same_line() {
        let stream: TokenStream = "$if(let) { value; } $for(item in items) { value; }"
            .parse()
            .unwrap();
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        let boundary = next_statement_boundary(&tokens, 0);

        assert!(is_marker(&tokens, boundary, "for"));
    }

    #[test]
    fn recovery_consumes_a_complete_if_chain() {
        let stream: TokenStream = "$if(let) {} $else_if(other) {} $else {} $for(item in items) {}"
            .parse()
            .unwrap();
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        let boundary = next_statement_boundary(&tokens, 0);

        assert!(is_marker(&tokens, boundary, "for"));
    }
}
