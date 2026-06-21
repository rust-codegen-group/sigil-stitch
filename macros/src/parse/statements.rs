use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::brace_classifier::{self, BraceKind};
use super::directives::{parse_for_components, parse_if_components};
use super::format::tokens_to_format;
use super::parse_body;
use super::types::{Branch, CompileError, InterpolationKind, MacroLang, Statement, TypedArg};
use super::util::{is_ident, is_semicolon};

/// Parse a single statement starting at `pos`.
/// Returns the statement and the position after the consumed tokens.
pub(super) fn parse_one_statement(
    tokens: &[TokenTree],
    start: usize,
    lang: MacroLang,
) -> Result<(Statement, usize), CompileError> {
    // Check for $comment(...) at current position.
    if let Some((comment_text, next)) = try_parse_comment(tokens, start)? {
        return Ok((Statement::Comment(comment_text), next));
    }

    // Check for $attr(...) at current position.
    if let Some((attr_text, next)) = try_parse_attr(tokens, start)? {
        return Ok((Statement::Attr(attr_text), next));
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
    if let Some((stmt, next)) = try_parse_meta_if(tokens, start, lang)? {
        return Ok((stmt, next));
    }

    // Check for $for(pat in expr) { ... }
    if let Some((stmt, next)) = try_parse_meta_for(tokens, start, lang)? {
        return Ok((stmt, next));
    }

    // Check for $let(binding);
    if let Some((stmt, next)) = try_parse_meta_let(tokens, start)? {
        return Ok((stmt, next));
    }

    // Collect tokens for this statement, looking for `;` or a brace group.
    let mut pos = start;
    let mut collected: Vec<TokenTree> = Vec::new();
    let mut prev_end_line: Option<usize> = None;

    // Track whether we're inside a control-flow header that allows embedded
    // semicolons (Go: `if init; cond {`, `for init; cond; post {`,
    // `switch init; expr {`). Semicolons before the opening `{` are part of
    // the header, not statement terminators.
    let mut in_cf_header = false;

    while pos < tokens.len() {
        let tt = &tokens[pos];

        if collected.is_empty()
            && let TokenTree::Ident(id) = tt
        {
            let s = id.to_string();
            if matches!(s.as_str(), "if" | "for" | "switch") {
                in_cf_header = true;
            }
        }

        // Check for `;` — statement terminator, unless inside a CF header.
        // (Any trailing `$+` in collected is handled by tokens_to_format_inner.)
        if is_semicolon(tt) && !in_cf_header {
            let (format, args) = tokens_to_format(&collected, lang)?;
            return Ok((Statement::Statement { format, args }, pos + 1));
        }

        // Check for brace group — potential control flow.
        if let TokenTree::Group(g) = tt
            && g.delimiter() == Delimiter::Brace
        {
            // Look ahead: if next token is `;`, this is NOT control flow
            // (it's an object literal or struct init in a statement),
            // UNLESS the preceding tokens indicate control flow AND the
            // body looks like a multi-statement block.
            let next = pos + 1;
            if next < tokens.len()
                && is_semicolon(&tokens[next])
                && (!brace_classifier::looks_like_control_flow_header(&collected)
                    || !brace_classifier::should_be_block_or_multiline(g))
            {
                // When the body contains statement-level markers, the brace
                // must be recursively parsed so $C_each/$for/$if are recognized.
                // Fall through to the classify() path instead of inlining.
                if brace_classifier::has_statement_marker(g) {
                    // Expression brace with statement markers:
                    // e.g., `return { $C_each(items); };`
                    // Recursively parse the body and inline as %L + ParsedBlock.
                    // The `;` at pos+1 is the statement terminator (consumed below
                    // by the normal semicolon path — we skip it via pos+2).
                    let (format, args) = handle_expression_brace_with_markers(&collected, g, lang)?;
                    return Ok((Statement::Statement { format, args }, pos + 2));
                } else {
                    // Part of a statement: `const x = { ... };`
                    collected.push(tt.clone());
                    prev_end_line = Some(tt.span().end().line);
                    pos += 1;
                    continue;
                }
            }

            // Look ahead: if next token is `=` (alone), this is a destructuring
            // pattern, not control flow (e.g. `const { name, age } = person;`).
            if next < tokens.len()
                && let TokenTree::Punct(eq_p) = &tokens[next]
                && eq_p.as_char() == '='
                && eq_p.spacing() == Spacing::Alone
            {
                collected.push(tt.clone());
                prev_end_line = Some(tt.span().end().line);
                pos += 1;
                continue;
            }

            // (Any trailing `$+` in collected is handled by tokens_to_format_inner.)

            // Distinguish control-flow `{` from literal `{` (e.g., Lua tables).
            // Brace languages always use `{` for blocks, but end-delimited
            // languages (Lua, Ruby, Elixir) use `{` for table/hash literals
            // and can only detect control flow from surrounding keywords.
            // Delegated to `brace_classifier` for the heuristic logic.
            match brace_classifier::classify(&collected, g) {
                BraceKind::Literal => {
                    collected.push(tt.clone());
                    prev_end_line = Some(tt.span().end().line);
                    pos += 1;
                    continue;
                }
                BraceKind::ControlFlow => {}
            }

            // Control flow detected (if/for/while/function/class).
            let (stmt, mut next_pos) = parse_control_flow(tokens, &collected, g, pos, lang)?;
            // Expression-level control flow (e.g., `match` in PHP/Rust)
            // should emit a trailing `;` when one is present in the source.
            let is_expression_cf = collected
                .iter()
                .any(|tt| is_ident(tt, "match") || is_ident(tt, "switch"));
            let trailing_semicolon =
                is_expression_cf && next_pos < tokens.len() && is_semicolon(&tokens[next_pos]);
            // Always consume trailing `;` — it's a DSL statement terminator,
            // not target-language syntax.
            if next_pos < tokens.len() && is_semicolon(&tokens[next_pos]) {
                next_pos += 1;
            }
            let stmt = if let Statement::ControlFlow { branches, .. } = stmt {
                Statement::ControlFlow {
                    branches,
                    trailing_semicolon,
                }
            } else {
                stmt
            };
            return Ok((stmt, next_pos));
        }

        // Check for paren group — potential declaration block (Go: const, var, import, type).
        if let TokenTree::Group(g) = tt
            && g.delimiter() == Delimiter::Parenthesis
            && is_paren_block_start(&collected, lang)
        {
            let (stmt, next_pos) = parse_paren_block(&collected, g, pos, lang)?;
            return Ok((stmt, next_pos));
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
                // Don't split if next line starts with `.` (method chaining),
                // or if an incomplete statement continues with inline `$for`/`$if`.
                let starts_with_dot = matches!(tt, TokenTree::Punct(p) if p.as_char() == '.');
                let continues_with_inline_meta = starts_with_inline_meta(tokens, pos)
                    && can_continue_before_inline_meta(&collected);
                if !starts_with_dot && !continues_with_inline_meta {
                    let (format, args) = tokens_to_format(&collected, lang)?;
                    return Ok((Statement::Line { format, args }, pos));
                }
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
        let (format, args) = tokens_to_format(&collected, lang)?;
        Ok((Statement::Line { format, args }, pos))
    }
}

fn starts_with_inline_meta(tokens: &[TokenTree], pos: usize) -> bool {
    pos + 1 < tokens.len()
        && matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '$')
        && (is_ident(&tokens[pos + 1], "for") || is_ident(&tokens[pos + 1], "if"))
}

fn can_continue_before_inline_meta(tokens: &[TokenTree]) -> bool {
    matches!(
        tokens.last(),
        Some(TokenTree::Punct(p))
            if matches!(p.as_char(), '=' | '|')
    )
}

/// Parse a control flow chain starting from tokens that lead into a brace group.
fn parse_control_flow(
    tokens: &[TokenTree],
    condition_tokens: &[TokenTree],
    first_brace: &proc_macro2::Group,
    brace_pos: usize,
    lang: MacroLang,
) -> Result<(Statement, usize), CompileError> {
    let (cond_format, cond_args) = tokens_to_format(condition_tokens, lang)?;
    let body_tokens: Vec<TokenTree> = first_brace.stream().into_iter().collect();
    let body = parse_body(&body_tokens, lang)?;

    let mut branches = vec![Branch {
        condition_format: cond_format,
        condition_args: cond_args,
        body,
    }];

    let mut pos = brace_pos + 1;

    // Check for else chain.
    while pos < tokens.len() {
        let is_else = is_ident(&tokens[pos], "else");
        let is_elseif = is_ident(&tokens[pos], "elseif");
        let is_elif = is_ident(&tokens[pos], "elif");

        if is_else || is_elseif || is_elif {
            let kw_span = tokens[pos].span();
            let keyword: String = tokens[pos].to_string();
            pos += 1; // consume keyword

            // For bare `else` with no condition tokens, use keyword as-is.
            // For `elseif`/`elif`, collect tokens until `{` as condition.
            let is_bare_else = is_else;

            // Collect tokens until we find a brace group.
            let mut else_condition_tokens: Vec<TokenTree> = Vec::new();
            let mut found_brace = false;

            while pos < tokens.len() {
                if let TokenTree::Group(g) = &tokens[pos]
                    && g.delimiter() == Delimiter::Brace
                {
                    let body_toks: Vec<TokenTree> = g.stream().into_iter().collect();
                    let body = parse_body(&body_toks, lang)?;

                    let (cond_format, cond_args) =
                        if is_bare_else && else_condition_tokens.is_empty() {
                            ("else".to_string(), Vec::new())
                        } else if is_bare_else {
                            let (fmt, args) = tokens_to_format(&else_condition_tokens, lang)?;
                            (format!("else {fmt}"), args)
                        } else if else_condition_tokens.is_empty() {
                            (keyword.clone(), Vec::new())
                        } else {
                            let (fmt, args) = tokens_to_format(&else_condition_tokens, lang)?;
                            (format!("{keyword} {fmt}"), args)
                        };

                    branches.push(Branch {
                        condition_format: cond_format,
                        condition_args: cond_args,
                        body,
                    });
                    pos += 1;
                    found_brace = true;
                    break;
                }
                else_condition_tokens.push(tokens[pos].clone());
                pos += 1;
            }

            if !found_brace {
                return Err(CompileError::new(
                    kw_span,
                    "expected `{` after `else`/`elseif`/`elif`",
                ));
            }
        } else {
            break;
        }
    }

    Ok((
        Statement::ControlFlow {
            branches,
            trailing_semicolon: false,
        },
        pos,
    ))
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
    lang: MacroLang,
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

    let (next_pos, branches) = parse_if_components(tokens, start + 2, lang)?;

    Ok(Some((Statement::MetaIf { branches }, next_pos)))
}

/// Try to parse `$for(pat in expr) { ... }` at position `start`.
/// Produces `Statement::MetaFor` which expands to a Rust `for` loop at compile time.
fn try_parse_meta_for(
    tokens: &[TokenTree],
    start: usize,
    lang: MacroLang,
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

    let (next_pos, pat, iter_expr, separator, trailing, body) =
        parse_for_components(tokens, start + 2, lang)?;

    Ok(Some((
        Statement::MetaFor {
            pat,
            iter_expr,
            separator,
            trailing,
            body,
        },
        next_pos, // helper returns paren_pos + 2 = start + 4
    )))
}

/// Try to parse `$let(binding);` at position `start`.
///
/// Emits a Rust-level `let` binding in the generated code, allowing
/// intermediate variable bindings (including fallible `?`) inside
/// `$for` and `$if` bodies.
fn try_parse_meta_let(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(Statement, usize)>, CompileError> {
    // Need at least 4 tokens: `$`, `let`, `(binding)`, `;`
    if start + 3 >= tokens.len() {
        return Ok(None);
    }

    let is_dollar = matches!(&tokens[start], TokenTree::Punct(p) if p.as_char() == '$');
    if !is_dollar {
        return Ok(None);
    }

    if !is_ident(&tokens[start + 1], "let") {
        return Ok(None);
    }

    let paren_group = match &tokens[start + 2] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[start + 2].span(),
                "$let requires a parenthesized binding: $let(var = expr);",
            ));
        }
    };

    let binding = paren_group.stream();
    if binding.is_empty() {
        return Err(CompileError::new(
            paren_group.span(),
            "$let binding cannot be empty: $let(var = expr);",
        ));
    }

    if !is_semicolon(&tokens[start + 3]) {
        return Err(CompileError::new(
            tokens[start + 3].span(),
            "$let must be followed by `;`: $let(var = expr);",
        ));
    }

    Ok(Some((Statement::MetaLet { binding }, start + 4)))
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

/// Try to parse `$comment(expr)` at position `start`.
fn try_parse_comment(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(TokenStream, usize)>, CompileError> {
    // Need at least 3 tokens: `$`, `comment`, `(...)`.
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

    // Check for parenthesized expression.
    let group = match &tokens[start + 2] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
        _ => {
            return Err(CompileError::new(
                tokens[start + 2].span(),
                "$comment requires parenthesized expression: $comment(expr)",
            ));
        }
    };

    let expr = group.stream();

    // Skip optional semicolon after $comment(expr);
    let mut next = start + 3;
    if next < tokens.len() && is_semicolon(&tokens[next]) {
        next += 1;
    }

    Ok(Some((expr, next)))
}

/// Try to parse `$attr(expr)` at position `start`.
fn try_parse_attr(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(TokenStream, usize)>, CompileError> {
    // Need at least 3 tokens: `$`, `attr`, `(...)`.
    if start + 2 >= tokens.len() {
        return Ok(None);
    }

    // Check for `$` punct.
    let _dollar = match &tokens[start] {
        TokenTree::Punct(p) if p.as_char() == '$' => p,
        _ => return Ok(None),
    };

    // Check for `attr` ident.
    if !is_ident(&tokens[start + 1], "attr") {
        return Ok(None);
    }

    // Check for parenthesized expression.
    let group = match &tokens[start + 2] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
        _ => {
            return Err(CompileError::new(
                tokens[start + 2].span(),
                "$attr requires a parenthesized expression: $attr(expr)",
            ));
        }
    };

    let expr = group.stream();

    // Skip optional semicolon after $attr(expr);
    let mut next = start + 3;
    if next < tokens.len() && is_semicolon(&tokens[next]) {
        next += 1;
    }

    Ok(Some((expr, next)))
}

/// Handle a brace group containing statement-level markers in an expression
/// position (e.g., `return { $C_each(items); };`).
///
/// Recursively parses the body inside the brace group, formats the prefix
/// tokens, and produces a `(format, args)` pair with `ParsedBlock` for the body.
fn handle_expression_brace_with_markers(
    prefix_tokens: &[TokenTree],
    brace_group: &proc_macro2::Group,
    lang: MacroLang,
) -> Result<(String, Vec<TypedArg>), CompileError> {
    let body_tokens: Vec<TokenTree> = brace_group.stream().into_iter().collect();
    let body_stmts = parse_body(&body_tokens, lang)?;

    let (prefix_format, mut prefix_args) = tokens_to_format(prefix_tokens, lang)?;

    let format = if prefix_format.is_empty() {
        "%L".to_string()
    } else {
        format!("{prefix_format}%L")
    };

    prefix_args.push(TypedArg {
        kind: InterpolationKind::ParsedBlock,
        expr: TokenStream::new(),
        parsed_body: Some(body_stmts),
    });

    Ok((format, prefix_args))
}

/// Check whether the collected prefix tokens and language indicate a
/// paren-delimited declaration block (Go: `const`, `var`, `import`, `type`).
fn is_paren_block_start(collected: &[TokenTree], lang: MacroLang) -> bool {
    if lang != MacroLang::Go {
        return false;
    }
    collected.last().is_some_and(|tt| {
        let s = tt.to_string();
        matches!(s.as_str(), "const" | "var" | "import" | "type")
    })
}

/// Parse a parenthesized declaration block.
///
/// Formats the header tokens (e.g., `"const"`) as `"const ("`, then recursively
/// parses the body inside the paren group.
fn parse_paren_block(
    header_tokens: &[TokenTree],
    paren_group: &proc_macro2::Group,
    _paren_pos: usize,
    lang: MacroLang,
) -> Result<(Statement, usize), CompileError> {
    let (header_format, header_args) = tokens_to_format(header_tokens, lang)?;
    let header_format = format!("{header_format} (");

    let body_tokens: Vec<TokenTree> = paren_group.stream().into_iter().collect();
    let body = parse_body(&body_tokens, lang)?;

    Ok((
        Statement::ParenBlock {
            header_format,
            header_args,
            body,
        },
        _paren_pos + 1,
    ))
}

#[cfg(test)]
#[path = "statements_tests.rs"]
mod tests;
