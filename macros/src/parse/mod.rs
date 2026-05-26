//! Token stream parser for `sigil_quote!`.
//!
//! Parses the macro input into a structured `ParsedInput` containing the
//! language type and a list of statements.

mod annotate;
mod brace_classifier;
mod format;
mod spacing;
mod statements;
mod stmt_rewrite;
mod types;
mod util;
pub(crate) mod verbatim_interpolation;

pub(crate) use types::*;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use statements::parse_one_statement;

fn parse_macro_lang(ts: &TokenStream) -> MacroLang {
    let first = ts.clone().into_iter().next();
    if let Some(TokenTree::Ident(id)) = first {
        match id.to_string().as_str() {
            "Bash" => MacroLang::Bash,
            "Zsh" => MacroLang::Zsh,
            "Go" => MacroLang::Go,
            "Haskell" => MacroLang::Haskell,
            "OCaml" => MacroLang::OCaml,
            "Php" => MacroLang::Php,
            _ => MacroLang::Unaware,
        }
    } else {
        MacroLang::Unaware
    }
}

/// Parse the full `sigil_quote!` input.
///
/// Expected form: `LangType { body }`
pub(crate) fn parse_input(input: TokenStream) -> Result<ParsedInput, CompileError> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Err(CompileError::new(
            Span::call_site(),
            "sigil_quote! requires a language type and body",
        ));
    }

    // Find the body group (last token must be a brace group).
    let last = &tokens[tokens.len() - 1];
    let body_group = match last {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(CompileError::new(
                last.span(),
                "sigil_quote! body must be enclosed in braces: sigil_quote!(Type { ... })",
            ));
        }
    };

    // Everything before the body group is the language type.
    let lang_tokens: TokenStream = tokens[..tokens.len() - 1].iter().cloned().collect();
    if lang_tokens.is_empty() {
        return Err(CompileError::new(
            Span::call_site(),
            "sigil_quote! requires a language type before the body: sigil_quote!(Type { ... })",
        ));
    }

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let lang = parse_macro_lang(&lang_tokens);
    let statements = parse_body(&body_tokens, lang)?;

    Ok(ParsedInput {
        lang,
        lang_type: lang_tokens,
        statements,
    })
}

/// Parse the body tokens into a list of statements.
pub(super) fn parse_body(
    tokens: &[TokenTree],
    lang: MacroLang,
) -> Result<Vec<Statement>, CompileError> {
    let mut statements = Vec::new();
    let mut pos = 0;

    // Track the line of the last consumed token for blank-line detection.
    let mut prev_line: Option<usize> = None;

    while pos < tokens.len() {
        // Detect blank lines via span-location gaps.
        let current_line = tokens[pos].span().start().line;
        if let Some(pl) = prev_line {
            let gap = current_line.saturating_sub(pl).saturating_sub(1);
            if gap > 0 {
                // Suppress blank lines after comments — doc comments must
                // attach to the following declaration without a separator.
                // This mirrors the spec-level behavior where FunSpec/TypeSpec
                // render doc comments and declarations together.
                let suppress = matches!(
                    statements.last(),
                    Some(Statement::Comment(_) | Statement::Attr(_))
                );
                if !suppress {
                    for _ in 0..gap {
                        statements.push(Statement::BlankLine);
                    }
                }
            }
        }

        let (stmt, next_pos) = parse_one_statement(tokens, pos, lang)?;
        // Track the line of the last token consumed.
        if next_pos > pos {
            prev_line = Some(tokens[next_pos - 1].span().end().line);
        }
        statements.push(stmt);
        pos = next_pos;
    }

    Ok(stmt_rewrite::rewrite_statements(statements, lang))
}
