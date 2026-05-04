use proc_macro2::{Delimiter, TokenTree};

use super::format::tokens_to_format;
use super::parse_body;
use super::types::{Branch, CompileError, MetaBranch, Statement};
use super::util::{is_ident, is_semicolon, unescape_string};

/// Parse a single statement starting at `pos`.
/// Returns the statement and the position after the consumed tokens.
pub(super) fn parse_one_statement(
    tokens: &[TokenTree],
    start: usize,
) -> Result<(Statement, usize), CompileError> {
    // Check for $comment(...) at current position.
    if let Some((comment_text, next)) = try_parse_comment(tokens, start)? {
        return Ok((Statement::Comment(comment_text), next));
    }

    // Check for $> or $< at current position.
    if let Some((stmt, next)) = try_parse_indent_directive(tokens, start) {
        return Ok((stmt, next));
    }

    // Check for $C_each(expr) at current position.
    if let Some((stmt, next)) = try_parse_splice_each(tokens, start)? {
        return Ok((stmt, next));
    }

    // Check for $if(cond) { ... } [$else_if(cond) { ... }] [$else { ... }]
    if let Some((stmt, next)) = try_parse_meta_if(tokens, start)? {
        return Ok((stmt, next));
    }

    // Check for $for(pat in expr) { ... }
    if let Some((stmt, next)) = try_parse_meta_for(tokens, start)? {
        return Ok((stmt, next));
    }

    // Collect tokens for this statement, looking for `;` or a brace group.
    let mut pos = start;
    let mut collected: Vec<TokenTree> = Vec::new();
    let mut prev_end_line: Option<usize> = None;

    while pos < tokens.len() {
        let tt = &tokens[pos];

        // Check for `;` — statement terminator.
        // (Any trailing `$+` in collected is handled by tokens_to_format_inner.)
        if is_semicolon(tt) {
            let (format, args) = tokens_to_format(&collected)?;
            return Ok((Statement::Statement { format, args }, pos + 1));
        }

        // Check for brace group — potential control flow.
        if let TokenTree::Group(g) = tt
            && g.delimiter() == Delimiter::Brace
        {
            // Look ahead: if next token is `;`, this is NOT control flow
            // (it's an object literal or struct init in a statement).
            let next = pos + 1;
            if next < tokens.len() && is_semicolon(&tokens[next]) {
                // Part of a statement: `const x = { ... };`
                collected.push(tt.clone());
                prev_end_line = Some(tt.span().end().line);
                pos += 1;
                continue;
            }

            // (Any trailing `$+` in collected is handled by tokens_to_format_inner.)

            // Check for $open("...") at end of collected tokens.
            let (condition_tokens, block_open_override) = try_extract_open_override(&collected)?;

            // Control flow detected.
            return parse_control_flow(tokens, &condition_tokens, g, pos, block_open_override);
        }

        // Line-break detection: split statement when tokens span multiple lines.
        if !collected.is_empty()
            && let Some(pel) = prev_end_line
            && tt.span().start().line > pel
        {
            let n = collected.len();
            if n >= 2
                && matches!(&collected[n - 2], TokenTree::Punct(p) if p.as_char() == '$')
                && matches!(&collected[n - 1], TokenTree::Punct(p) if p.as_char() == '+')
            {
                collected.pop();
                collected.pop();
            } else {
                let (format, args) = tokens_to_format(&collected)?;
                return Ok((Statement::Line { format, args }, pos));
            }
        }

        collected.push(tt.clone());
        prev_end_line = Some(tt.span().end().line);
        pos += 1;
    }

    // End of input without `;` — emit as a Line.
    // Strip here (not just in tokens_to_format_inner) so a bare `$+` yields
    // an empty `collected` → `BlankLine` instead of `Line { format: "" }`.
    strip_trailing_continuation(&mut collected);
    if collected.is_empty() {
        Ok((Statement::BlankLine, pos))
    } else {
        let (format, args) = tokens_to_format(&collected)?;
        Ok((Statement::Line { format, args }, pos))
    }
}

/// Check if the last tokens in `collected` form `$open("text")`.
/// Returns the remaining condition tokens and the optional override string.
fn try_extract_open_override(
    collected: &[TokenTree],
) -> Result<(Vec<TokenTree>, Option<String>), CompileError> {
    let n = collected.len();
    if n < 3 {
        return Ok((collected.to_vec(), None));
    }

    // Check for pattern: Punct($) Ident(open) Group(Paren containing string literal)
    let dollar = &collected[n - 3];
    let ident = &collected[n - 2];
    let group = &collected[n - 1];

    let is_dollar = matches!(dollar, TokenTree::Punct(p) if p.as_char() == '$');
    let is_open = is_ident(ident, "open");
    let paren_group = if let TokenTree::Group(g) = group
        && g.delimiter() == Delimiter::Parenthesis
    {
        Some(g)
    } else {
        None
    };

    if !is_dollar || !is_open || paren_group.is_none() {
        return Ok((collected.to_vec(), None));
    }

    let g = paren_group.unwrap();
    let inner: Vec<TokenTree> = g.stream().into_iter().collect();
    if inner.len() != 1 {
        return Err(CompileError::new(
            g.span(),
            "$open requires a single string literal: $open(\"text\")",
        ));
    }

    let text = match &inner[0] {
        TokenTree::Literal(lit) => {
            let s = lit.to_string();
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                let raw = &s[1..s.len() - 1];
                match unescape_string(raw) {
                    Ok(text) => text,
                    Err(msg) => return Err(CompileError::new(lit.span(), &msg)),
                }
            } else {
                return Err(CompileError::new(
                    lit.span(),
                    "$open requires a string literal",
                ));
            }
        }
        _ => {
            return Err(CompileError::new(
                inner[0].span(),
                "$open requires a string literal",
            ));
        }
    };

    let condition_tokens = collected[..n - 3].to_vec();
    Ok((condition_tokens, Some(text)))
}

/// Parse a control flow chain starting from tokens that lead into a brace group.
fn parse_control_flow(
    tokens: &[TokenTree],
    condition_tokens: &[TokenTree],
    first_brace: &proc_macro2::Group,
    brace_pos: usize,
    block_open_override: Option<String>,
) -> Result<(Statement, usize), CompileError> {
    let (cond_format, cond_args) = tokens_to_format(condition_tokens)?;
    let body_tokens: Vec<TokenTree> = first_brace.stream().into_iter().collect();
    let body = parse_body(&body_tokens)?;

    let mut branches = vec![Branch {
        condition_format: cond_format,
        condition_args: cond_args,
        body,
        block_open_override,
    }];

    let mut pos = brace_pos + 1;

    // Check for else chain.
    while pos < tokens.len() {
        if is_ident(&tokens[pos], "else") {
            let else_span = tokens[pos].span();
            pos += 1; // consume `else`

            // Collect tokens until we find a brace group (handles `else if (...) {`).
            let mut else_condition_tokens: Vec<TokenTree> = Vec::new();
            let mut found_brace = false;

            while pos < tokens.len() {
                if let TokenTree::Group(g) = &tokens[pos]
                    && g.delimiter() == Delimiter::Brace
                {
                    let body_toks: Vec<TokenTree> = g.stream().into_iter().collect();
                    let body = parse_body(&body_toks)?;

                    let (cond_format, cond_args) = if else_condition_tokens.is_empty() {
                        ("else".to_string(), Vec::new())
                    } else {
                        let (fmt, args) = tokens_to_format(&else_condition_tokens)?;
                        (format!("else {fmt}"), args)
                    };

                    branches.push(Branch {
                        condition_format: cond_format,
                        condition_args: cond_args,
                        body,
                        block_open_override: None,
                    });
                    pos += 1;
                    found_brace = true;
                    break;
                }
                else_condition_tokens.push(tokens[pos].clone());
                pos += 1;
            }

            if !found_brace {
                return Err(CompileError::new(else_span, "expected `{` after `else`"));
            }
        } else {
            break;
        }
    }

    Ok((Statement::ControlFlow { branches }, pos))
}

/// Try to parse `$C_each(expr)` at position `start`.
/// Produces `Statement::SpliceEach` which splices each code block from an iterable.
fn try_parse_splice_each(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(Statement, usize)>, CompileError> {
    // Need at least 3 tokens: `$`, `C_each`, `(expr)`
    if start + 2 >= tokens.len() {
        return Ok(None);
    }

    let is_dollar = matches!(&tokens[start], TokenTree::Punct(p) if p.as_char() == '$');
    if !is_dollar {
        return Ok(None);
    }

    if !is_ident(&tokens[start + 1], "C_each") {
        return Ok(None);
    }

    let group = match &tokens[start + 2] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
        _ => {
            return Err(CompileError::new(
                tokens[start + 2].span(),
                "$C_each requires a parenthesized expression: $C_each(expr)",
            ));
        }
    };

    let mut next = start + 3;
    // Skip optional trailing semicolon.
    if next < tokens.len() && is_semicolon(&tokens[next]) {
        next += 1;
    }

    Ok(Some((
        Statement::SpliceEach {
            expr: group.stream(),
        },
        next,
    )))
}

/// Try to parse `$if(cond) { ... } [$else_if(cond) { ... }] [$else { ... }]`
/// at position `start`. These are meta-conditionals that control which builder
/// calls are emitted at runtime, NOT target-language control flow.
fn try_parse_meta_if(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(Statement, usize)>, CompileError> {
    // Need at least 4 tokens: `$`, `if`, `(cond)`, `{ body }`
    if start + 3 >= tokens.len() {
        return Ok(None);
    }

    let is_dollar = matches!(&tokens[start], TokenTree::Punct(p) if p.as_char() == '$');
    if !is_dollar {
        return Ok(None);
    }

    if !is_ident(&tokens[start + 1], "if") {
        return Ok(None);
    }

    let mut pos = start + 2;
    let mut branches = Vec::new();

    // Parse `(cond) { body }` for the $if branch.
    let cond_group = match &tokens[pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[pos].span(),
                "$if requires a parenthesized condition: $if(condition) { ... }",
            ));
        }
    };
    pos += 1;

    let body_group = match &tokens[pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[pos].span(),
                "$if requires a brace body: $if(condition) { ... }",
            ));
        }
    };
    pos += 1;

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let body = parse_body(&body_tokens)?;
    branches.push(MetaBranch {
        condition: Some(cond_group.stream()),
        body,
    });

    // Parse optional $else_if / $else continuations.
    loop {
        // Look for `$` `else_if` or `$` `else`.
        if pos + 1 >= tokens.len() {
            break;
        }
        let is_dollar = matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '$');
        if !is_dollar {
            break;
        }

        if is_ident(&tokens[pos + 1], "else_if") {
            // $else_if(cond) { body }
            pos += 2;
            if pos >= tokens.len() {
                return Err(CompileError::new(
                    tokens[pos - 1].span(),
                    "$else_if requires a parenthesized condition",
                ));
            }
            let cond_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
                _ => {
                    return Err(CompileError::new(
                        tokens[pos].span(),
                        "$else_if requires a parenthesized condition: $else_if(condition) { ... }",
                    ));
                }
            };
            pos += 1;
            if pos >= tokens.len() {
                return Err(CompileError::new(
                    tokens[pos - 1].span(),
                    "$else_if requires a brace body",
                ));
            }
            let body_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
                _ => {
                    return Err(CompileError::new(
                        tokens[pos].span(),
                        "$else_if requires a brace body: $else_if(condition) { ... }",
                    ));
                }
            };
            pos += 1;

            let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
            let body = parse_body(&body_tokens)?;
            branches.push(MetaBranch {
                condition: Some(cond_group.stream()),
                body,
            });
        } else if is_ident(&tokens[pos + 1], "else") {
            // $else { body }
            pos += 2;
            if pos >= tokens.len() {
                return Err(CompileError::new(
                    tokens[pos - 1].span(),
                    "$else requires a brace body",
                ));
            }
            let body_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
                _ => {
                    return Err(CompileError::new(
                        tokens[pos].span(),
                        "$else requires a brace body: $else { ... }",
                    ));
                }
            };
            pos += 1;

            let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
            let body = parse_body(&body_tokens)?;
            branches.push(MetaBranch {
                condition: None,
                body,
            });
            break; // $else is always last.
        } else {
            break;
        }
    }

    Ok(Some((Statement::MetaIf { branches }, pos)))
}

/// Try to parse `$for(pat in expr) { ... }` at position `start`.
/// Produces `Statement::MetaFor` which expands to a Rust `for` loop at compile time.
fn try_parse_meta_for(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(Statement, usize)>, CompileError> {
    // Need at least 4 tokens: `$`, `for`, `(pat in expr)`, `{ body }`
    if start + 3 >= tokens.len() {
        return Ok(None);
    }

    let is_dollar = matches!(&tokens[start], TokenTree::Punct(p) if p.as_char() == '$');
    if !is_dollar {
        return Ok(None);
    }

    if !is_ident(&tokens[start + 1], "for") {
        return Ok(None);
    }

    let paren_group = match &tokens[start + 2] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[start + 2].span(),
                "$for requires a parenthesized pattern: $for(pat in expr) { ... }",
            ));
        }
    };

    let body_group = match &tokens[start + 3] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[start + 3].span(),
                "$for requires a brace body: $for(pat in expr) { ... }",
            ));
        }
    };

    // Split paren contents on the first `in` keyword.
    let paren_tokens: Vec<TokenTree> = paren_group.stream().into_iter().collect();
    let in_pos = paren_tokens.iter().position(|tt| is_ident(tt, "in"));
    let in_pos = match in_pos {
        Some(p) => p,
        None => {
            return Err(CompileError::new(
                paren_group.span(),
                "$for requires `in` keyword: $for(pat in expr) { ... }",
            ));
        }
    };

    if in_pos == 0 {
        return Err(CompileError::new(
            paren_group.span(),
            "$for pattern cannot be empty: $for(pat in expr) { ... }",
        ));
    }
    if in_pos + 1 >= paren_tokens.len() {
        return Err(CompileError::new(
            paren_group.span(),
            "$for iterator expression cannot be empty: $for(pat in expr) { ... }",
        ));
    }

    let pat: proc_macro2::TokenStream = paren_tokens[..in_pos].iter().cloned().collect();
    let iter_expr: proc_macro2::TokenStream = paren_tokens[in_pos + 1..].iter().cloned().collect();

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let body = parse_body(&body_tokens)?;

    Ok(Some((
        Statement::MetaFor {
            pat,
            iter_expr,
            body,
        },
        start + 4,
    )))
}

/// Strip a trailing `$+` continuation marker from collected tokens.
fn strip_trailing_continuation(collected: &mut Vec<TokenTree>) {
    let n = collected.len();
    if n >= 2
        && matches!(&collected[n - 2], TokenTree::Punct(p) if p.as_char() == '$')
        && matches!(&collected[n - 1], TokenTree::Punct(p) if p.as_char() == '+')
    {
        collected.pop();
        collected.pop();
    }
}

/// Check for `$>` or `$<` at position `start`.
fn try_parse_indent_directive(tokens: &[TokenTree], start: usize) -> Option<(Statement, usize)> {
    if start + 1 >= tokens.len() {
        return None;
    }
    let is_dollar = matches!(&tokens[start], TokenTree::Punct(p) if p.as_char() == '$');
    if !is_dollar {
        return None;
    }
    if let TokenTree::Punct(p2) = &tokens[start + 1] {
        match p2.as_char() {
            '>' => return Some((Statement::Indent, start + 2)),
            '<' => return Some((Statement::Dedent, start + 2)),
            _ => {}
        }
    }
    None
}

/// Try to parse `$comment("text")` at position `start`.
fn try_parse_comment(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(String, usize)>, CompileError> {
    // Need at least 3 tokens: `$`, `comment`, `("text")`
    if start + 2 >= tokens.len() {
        return Ok(None);
    }

    // Check for `$` punct.
    let _dollar = match &tokens[start] {
        TokenTree::Punct(p) if p.as_char() == '$' => p,
        _ => return Ok(None),
    };

    // Check for `comment` ident.
    if !is_ident(&tokens[start + 1], "comment") {
        return Ok(None);
    }

    // Check for parenthesized string literal.
    let group = match &tokens[start + 2] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
        _ => {
            return Err(CompileError::new(
                tokens[start + 2].span(),
                "$comment requires parenthesized string: $comment(\"text\")",
            ));
        }
    };

    let inner: Vec<TokenTree> = group.stream().into_iter().collect();
    if inner.len() != 1 {
        return Err(CompileError::new(
            group.span(),
            "$comment requires a single string literal: $comment(\"text\")",
        ));
    }

    let text = match &inner[0] {
        TokenTree::Literal(lit) => {
            let s = lit.to_string();
            // Strip surrounding quotes and unescape.
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                let raw = &s[1..s.len() - 1];
                match unescape_string(raw) {
                    Ok(text) => text,
                    Err(msg) => {
                        return Err(CompileError::new(lit.span(), &msg));
                    }
                }
            } else {
                return Err(CompileError::new(
                    lit.span(),
                    "$comment requires a string literal",
                ));
            }
        }
        _ => {
            return Err(CompileError::new(
                inner[0].span(),
                "$comment requires a string literal",
            ));
        }
    };

    // Skip optional semicolon after $comment("text");
    let mut next = start + 3;
    if next < tokens.len() && is_semicolon(&tokens[next]) {
        next += 1;
    }

    Ok(Some((text, next)))
}
