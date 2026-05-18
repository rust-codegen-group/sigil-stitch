use proc_macro2::{Delimiter, Spacing, TokenTree};

use super::format::tokens_to_format;
use super::parse_body;
use super::types::{Branch, CompileError, MacroLang, MetaBranch, Statement};
use super::util::{is_ident, is_semicolon, unescape_string};

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
                && (!looks_like_control_flow_header(&collected) || !should_be_block_or_multiline(g))
            {
                // Part of a statement: `const x = { ... };`
                collected.push(tt.clone());
                prev_end_line = Some(tt.span().end().line);
                pos += 1;
                continue;
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
            if !looks_like_control_flow_header(&collected) {
                // Exception: `= { multi-statement }` after a function signature
                // is a function body, not a literal. Treat as control flow if
                // the body has statements AND there's a paren group in the prefix
                // (indicating a function declaration).
                let last_is_eq = collected
                    .last()
                    .is_some_and(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '='));
                let has_paren_group = collected.iter().any(
                    |t| matches!(t, TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis),
                );
                if !last_is_eq || !has_paren_group || !should_be_block(g) {
                    // Treat as a literal brace group — not control flow.
                    collected.push(tt.clone());
                    prev_end_line = Some(tt.span().end().line);
                    pos += 1;
                    continue;
                }
            }

            // Control flow detected.
            let (stmt, mut next_pos) = parse_control_flow(tokens, &collected, g, pos, lang)?;
            // Consume optional trailing `;` after the control flow block.
            if next_pos < tokens.len() && is_semicolon(&tokens[next_pos]) {
                next_pos += 1;
            }
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
                // Don't split if next line starts with `.` (method chaining)
                let starts_with_dot = matches!(tt, TokenTree::Punct(p) if p.as_char() == '.');
                if !starts_with_dot {
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

/// Check whether the tokens before a `{` brace group look like a control-flow
/// header (e.g. `if ... {`, `for ... {`, `function ... {`) rather than a
/// literal brace expression (e.g. Lua table constructor `local t = { ... }`).
///
/// Returns `false` (→ literal) only for clear literal patterns: tokens ending
/// with `=`, `,`, `return`, or a single identifier before `()` (function call
/// with table argument, e.g. `foo(...) {`). Everything else defaults to
/// `true` (→ control flow), which is correct for all brace languages.
fn looks_like_control_flow_header(tokens: &[TokenTree]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    let n = tokens.len();
    let last = &tokens[n - 1];

    // Control-flow keywords that always precede `{`
    if is_ident(last, "then") || is_ident(last, "do") || is_ident(last, "else") {
        return true;
    }

    // `=` → assignment of a table/object literal
    if matches!(last, TokenTree::Punct(p) if p.as_char() == '=') {
        return false;
    }
    // `,` → table entry separator (unlikely at statement level, but safe)
    if matches!(last, TokenTree::Punct(p) if p.as_char() == ',') {
        return false;
    }
    // `return` → returning a table
    if is_ident(last, "return") {
        return false;
    }

    // `(...)` group — check if it's a function call with table argument.
    if matches!(last, TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis) {
        // Only one token before `()` and it's an ident → function call
        // (e.g. `foo(...) {` with table arg). This is a literal.
        if n == 2 && matches!(&tokens[0], TokenTree::Ident(_)) {
            let s = tokens[0].to_string();
            // Known control-flow keywords that appear as a single ident
            // before `()` are NOT function calls.
            if matches!(
                s.as_str(),
                "if" | "for"
                    | "while"
                    | "catch"
                    | "switch"
                    | "foreach"
                    | "for_each"
                    | "unless"
                    | "until"
                    | "match"
                    | "try"
                    | "synchronized"
                    | "when"
                    | "guard"
                    | "function"
            ) {
                return true;
            }
            // Single ident (not a keyword) → function call → literal
            return false;
        }
        // Multiple tokens before `()` or starts with keyword → control flow
        return true;
    }

    // `repeat` starts a control-flow block
    if is_ident(&tokens[0], "repeat") {
        return true;
    }

    // Default: assume control flow — backward compatible with brace languages
    // where `{` at statement level always denotes a block.
    true
}

/// Determine if a brace group contains multiple statements (semicolons)
/// and thus should be treated as a block rather than inlined.
fn should_be_block(g: &proc_macro2::Group) -> bool {
    let stream: Vec<TokenTree> = g.stream().into_iter().collect();
    for tt in &stream {
        if is_semicolon(tt) {
            return true;
        }
    }
    false
}

/// Like `should_be_block`, but also returns true for multi-line bodies.
/// Used for `{...};` with control-flow headers where semicolons may be
/// absent (e.g. Kotlin `when`, Haskell `do`).
fn should_be_block_or_multiline(g: &proc_macro2::Group) -> bool {
    let stream: Vec<TokenTree> = g.stream().into_iter().collect();
    if stream.is_empty() {
        return false;
    }
    for tt in &stream {
        if is_semicolon(tt) {
            return true;
        }
    }
    let first_line = stream.first().unwrap().span().start().line;
    let last_line = stream.last().unwrap().span().end().line;
    first_line != last_line
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
    let body = parse_body(&body_tokens, lang)?;
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
            let body = parse_body(&body_tokens, lang)?;
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
            let body = parse_body(&body_tokens, lang)?;
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
    let body = parse_body(&body_tokens, lang)?;

    Ok(Some((
        Statement::MetaFor {
            pat,
            iter_expr,
            body,
        },
        start + 4,
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
