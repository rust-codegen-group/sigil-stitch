//! Token stream parser for `sigil_quote!`.
//!
//! Parses the macro input into a structured `ParsedInput` containing the
//! language type and a list of statements.

mod annotate;
mod brace_classifier;
mod directives;
mod format;
mod lang;
mod recovery;
mod rust_interpolation;
mod spacing;
mod statements;
mod stmt_rewrite;
mod util;

pub(crate) use crate::ir::ParsedInput;
pub(crate) use lang::MacroLang;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use crate::ir::Statement;
use recovery::{combine, next_statement_boundary};
use statements::parse_one_statement;

/// Parse the full `sigil_quote!` input.
///
/// Expected form: `LangType { body }`
pub(crate) fn parse_input(input: TokenStream) -> syn::Result<ParsedInput> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "sigil_quote! requires a language type and body",
        ));
    }

    // Find the body group (last token must be a brace group).
    let last = &tokens[tokens.len() - 1];
    let body_group = match last {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(syn::Error::new(
                last.span(),
                "sigil_quote! body must be enclosed in braces: sigil_quote!(Type { ... })",
            ));
        }
    };

    // Everything before the body group is the language type.
    let lang_tokens: TokenStream = tokens[..tokens.len() - 1].iter().cloned().collect();
    if lang_tokens.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "sigil_quote! requires a language type before the body: sigil_quote!(Type { ... })",
        ));
    }

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let lang = MacroLang::parse(lang_tokens)?;
    let statements = parse_body(&body_tokens, lang)?;

    Ok(ParsedInput { statements })
}

/// Parse the body tokens into a list of statements.
pub(super) fn parse_body(tokens: &[TokenTree], lang: MacroLang) -> syn::Result<Vec<Statement>> {
    let mut statements = Vec::new();
    let mut pos = 0;
    let mut errors = None;

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

        match parse_one_statement(tokens, pos, lang) {
            Ok((stmt, next_pos)) => {
                if next_pos > pos {
                    prev_line = Some(tokens[next_pos - 1].span().end().line);
                }
                statements.push(stmt);
                pos = next_pos;
            }
            Err(recovered) => {
                combine(&mut errors, recovered.error);
                let next_pos = recovered
                    .next_pos
                    .unwrap_or_else(|| next_statement_boundary(tokens, pos));
                if next_pos > pos {
                    prev_line = Some(tokens[next_pos - 1].span().end().line);
                    pos = next_pos;
                } else {
                    break;
                }
            }
        }
    }

    if let Some(error) = errors {
        Err(error)
    } else {
        Ok(stmt_rewrite::rewrite_statements(statements, lang))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_language_uses_the_final_path_segment() {
        let input: TokenStream = "crate::lang::TypeScript { const x = 1; }".parse().unwrap();
        assert!(parse_input(input).is_ok());
    }

    #[test]
    fn independent_statement_errors_are_combined() {
        let input: TokenStream = "TypeScript { const a = $L(let); const b = $N(struct); }"
            .parse()
            .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected two invalid expressions"),
            Err(error) => error,
        };
        assert_eq!(error.into_iter().count(), 2);
    }

    #[test]
    fn no_semicolon_line_recovery_reaches_the_next_error() {
        let input: TokenStream = "TypeScript {\nconst a = $L(let)\nconst b = $N(struct)\n}"
            .parse()
            .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected two invalid expressions"),
            Err(error) => error,
        };
        assert_eq!(error.into_iter().count(), 2);
    }

    #[test]
    fn recovery_preserves_explicit_and_dot_continuations() {
        let input: TokenStream =
            "TypeScript {\nconst a = source $+\n.method($L(let))\nconst b = $N(struct)\n}"
                .parse()
                .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected two invalid expressions"),
            Err(error) => error,
        };
        assert_eq!(error.into_iter().count(), 2);
    }

    #[test]
    fn recovery_preserves_inline_meta_continuations() {
        let input: TokenStream =
            "TypeScript {\nconst a =\n$if(let) { value }\nconst b = $N(struct)\n}"
                .parse()
                .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected two invalid expressions"),
            Err(error) => error,
        };
        assert_eq!(error.into_iter().count(), 2);
    }

    #[test]
    fn same_line_directives_and_invalid_for_body_are_combined() {
        let input: TokenStream = "TypeScript { \
            $if(let) { value; } \
            $for(item + 1 in items) { const nested = $N(struct); } \
            const inline = [$for(entry + 1 in entries) { $N(enum) }]; \
        }"
        .parse()
        .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected five directive errors"),
            Err(error) => error,
        };

        assert_eq!(error.into_iter().count(), 5);
    }

    #[test]
    fn malformed_else_if_keeps_prior_branch_errors_without_duplication() {
        let input: TokenStream = "TypeScript {
            $if(let) { const first = $N(struct); } $else_if true { value; }
        }"
        .parse()
        .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected malformed $if errors"),
            Err(error) => error,
        };
        let errors: Vec<String> = error.into_iter().map(|error| error.to_string()).collect();

        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("$if condition"), "{errors:?}");
        assert!(errors[1].contains("invalid $N expression"), "{errors:?}");
        assert!(
            errors[2].contains("$else_if requires a parenthesized condition"),
            "{errors:?}"
        );
    }

    #[test]
    fn malformed_else_if_recovery_reaches_the_next_statement() {
        let input: TokenStream = "TypeScript {
            $if(let) { const first = $N(struct); } $else_if true { value; }
            const after = $L(enum);
        }"
        .parse()
        .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected malformed $if errors"),
            Err(error) => error,
        };
        let errors: Vec<String> = error.into_iter().map(|error| error.to_string()).collect();

        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors[3].contains("invalid $L expression"), "{errors:?}");
    }

    #[test]
    fn malformed_else_keeps_prior_branch_errors_without_duplication() {
        let input: TokenStream = "TypeScript {
            $if(let) { const first = $N(struct); } $else value;
        }"
        .parse()
        .unwrap();
        let error = match parse_input(input) {
            Ok(_) => panic!("expected malformed $if errors"),
            Err(error) => error,
        };
        let errors: Vec<String> = error.into_iter().map(|error| error.to_string()).collect();

        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("$if condition"), "{errors:?}");
        assert!(errors[1].contains("invalid $N expression"), "{errors:?}");
        assert!(
            errors[2].contains("$else requires a brace body"),
            "{errors:?}"
        );
    }
}
