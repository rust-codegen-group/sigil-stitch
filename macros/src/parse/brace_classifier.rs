//! Brace classification for `sigil_quote!` control-flow detection.
//!
//! Determines whether a `{` brace group at the start of a new statement
//! introduces control flow (`if`, `for`, `while`, function body, etc.) or
//! is a literal brace expression (object literal, struct init, destructuring).

use proc_macro2::{Delimiter, TokenTree};

use super::util::{is_ident, is_semicolon};

/// Classification result for a brace group at the statement level.
pub(super) enum BraceKind {
    /// The brace group is control flow — the caller should proceed with
    /// `parse_control_flow`.
    ControlFlow,
    /// The brace group is literal — the caller should keep it as part of
    /// the current statement.
    Literal,
}

/// Classify a brace group given the tokens that precede it.
///
/// Returns `BraceKind::ControlFlow` when the prefix tokens indicate a
/// control-flow header (e.g., `if ... {`, `for ... {`) or an exception
/// applies (function body, method shorthand with sigil markers).
/// Returns `BraceKind::Literal` for clear non-control-flow patterns
/// (assignment, return, single-ident function call).
///
/// Use this as the primary decision point; the two short-circuits in
/// `parse_one_statement` (next-token-is-`;` and next-token-is-`=`) are
/// handled inline because they depend on outer loop context.
pub(super) fn classify(prefix_tokens: &[TokenTree], group: &proc_macro2::Group) -> BraceKind {
    if looks_like_control_flow_header(prefix_tokens) {
        // Don't intercept `$if(cond) { ... }` or `$for(pat in expr) { ... }`
        // — these are inline meta directives, not target-language control flow.
        if ends_with_sigil_directive(prefix_tokens) {
            return BraceKind::Literal;
        }
        return BraceKind::ControlFlow;
    }

    // Exception: `= { multi-statement }` after a function signature
    // is a function body, not a literal.
    let last_is_eq = prefix_tokens
        .last()
        .is_some_and(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '='));
    let has_paren_group = prefix_tokens.iter().enumerate().any(|(i, t)| {
        if matches!(t, TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis) {
            // Exclude paren groups that belong to sigil-stitch interpolation
            // markers ($N(...), $V(...), $C_each(...), etc.). These are NOT
            // function-signature parens; they only appear as interpolation args.
            // Pattern: Punct('$'), Ident(_), Group(Paren)
            !is_interpolation_paren(prefix_tokens, i)
        } else {
            false
        }
    });

    // Exception: `foo() { $... }` where a paren group precedes the brace
    // (method shorthand / function declaration) and the body contains
    // sigil-stitch markers. Without this, `foo() { $C_each(items); }` is
    // misclassified as a function call with an object-literal argument.
    let body_has_meta = has_meta_marker(group);
    let body_has_stmt_marker = has_statement_marker(group);
    let is_function_body = last_is_eq && has_paren_group && should_be_block(group);
    let is_method_with_meta = has_paren_group && body_has_meta;
    // `= { $C_each(...) }` — object literal containing statement-level markers.
    // Use `has_statement_marker` (not `has_meta_marker`) to avoid triggering on
    // inline specifiers like `$S("x")` inside Lua table constructors.
    let is_eq_object_with_stmt = last_is_eq && body_has_stmt_marker;
    // Any brace body with statement-level markers needs recursive parsing
    // so $C_each/$for/$if/$let inside are recognized. Even without `=` or
    // a paren group, `return { $C_each(fields); }` must not be inlined.
    let is_body_with_stmt_marker = body_has_stmt_marker;

    if is_function_body || is_method_with_meta || is_eq_object_with_stmt || is_body_with_stmt_marker
    {
        BraceKind::ControlFlow
    } else {
        BraceKind::Literal
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
pub(super) fn looks_like_control_flow_header(tokens: &[TokenTree]) -> bool {
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

/// Check whether the paren group at `pos` in `tokens` is part of a
/// sigil-stitch interpolation marker (pattern: `$Ident(...)`).
fn is_interpolation_paren(tokens: &[TokenTree], pos: usize) -> bool {
    if pos < 2 {
        return false;
    }
    matches!(&tokens[pos - 2], TokenTree::Punct(p) if p.as_char() == '$')
        && matches!(&tokens[pos - 1], TokenTree::Ident(_))
}

/// Check whether the prefix tokens end with a sigil-stitch inline directive
/// pattern. Recognizes:
/// - `$if(expr)` / `$for(expr)` / `$else_if(expr)` → brace is directive body
/// - `$else` → brace is `$else { ... }` directive body
fn ends_with_sigil_directive(tokens: &[TokenTree]) -> bool {
    let n = tokens.len();
    // Pattern: `$`, `else`, (no paren group needed — $else has no condition)
    if n >= 2
        && let TokenTree::Punct(dollar) = &tokens[n - 2]
        && dollar.as_char() == '$'
        && let TokenTree::Ident(id) = &tokens[n - 1]
        && id.to_string().as_str() == "else"
    {
        return true;
    }
    // Pattern: `$`, `if`/`for`/`else_if`, `(...)`
    if n >= 3
        && let TokenTree::Punct(dollar) = &tokens[n - 3]
        && dollar.as_char() == '$'
        && let TokenTree::Ident(id) = &tokens[n - 2]
        && matches!(id.to_string().as_str(), "if" | "for" | "else_if")
        && let TokenTree::Group(g) = &tokens[n - 1]
        && g.delimiter() == Delimiter::Parenthesis
    {
        return true;
    }
    false
}

/// Check if a brace group contains a statement-level sigil marker
/// (`$C_each`, `$let`) that requires a control-flow context.
/// Note: `$for`/`$if`/`$else_if`/`$else` are now handled inline by
/// `tokens_to_format_inner` and do NOT need statement-level interception.
/// Unlike `has_meta_marker`, this skips inline specifiers (`$S`, `$V`, `$L`,
/// `$N`, `$T`, `$join`) which work fine inside literal brace groups.
pub(super) fn has_statement_marker(g: &proc_macro2::Group) -> bool {
    let stream: Vec<TokenTree> = g.stream().into_iter().collect();
    let mut i = 0;
    while i < stream.len() {
        if i + 1 < stream.len()
            && let TokenTree::Punct(p) = &stream[i]
            && p.as_char() == '$'
            && let TokenTree::Ident(id) = &stream[i + 1]
        {
            let s = id.to_string();
            if matches!(s.as_str(), "C_each" | "let") {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Check if a brace group contains any `$` sigil at the top level,
/// indicating it's code using sigil-stitch markers (not an object literal).
/// Used to disambiguate `foo() { $C_each(...) }` (method body) from
/// `func({ key: value })` (call with object-literal argument).
fn has_meta_marker(g: &proc_macro2::Group) -> bool {
    let stream: Vec<TokenTree> = g.stream().into_iter().collect();
    for tt in &stream {
        match tt {
            TokenTree::Punct(p) if p.as_char() == '$' => return true,
            TokenTree::Group(g) if has_meta_marker(g) => return true,
            _ => {}
        }
    }
    false
}

/// Determine if a brace group contains multiple statements (semicolons)
/// and thus should be treated as a block rather than inlined.
pub(super) fn should_be_block(g: &proc_macro2::Group) -> bool {
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
pub(super) fn should_be_block_or_multiline(g: &proc_macro2::Group) -> bool {
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
