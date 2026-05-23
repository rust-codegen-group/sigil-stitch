//! Code generator for `sigil_quote!`.
//!
//! Converts parsed statements into a `TokenStream` of `CodeBlockBuilder` method calls.

use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::quote;

use crate::parse::verbatim_interpolation::{VerbatimResult, parse_verbatim_interpolation};
use crate::parse::{InterpolationKind, MetaBranch, ParsedInput, Statement, TypedArg};

/// Generate the output token stream from a parsed `sigil_quote!` input.
pub(crate) fn generate(input: ParsedInput) -> TokenStream {
    let builder_calls = generate_statements(&input.statements);

    quote! {
        {
            let mut __sigil_builder = ::sigil_stitch::code_block::CodeBlock::builder();
            #(#builder_calls)*
            __sigil_builder.build()
        }
    }
}

/// Generate builder calls for a list of statements.
fn generate_statements(statements: &[Statement]) -> Vec<TokenStream> {
    let mut calls = Vec::new();
    for stmt in statements {
        match stmt {
            Statement::BlankLine => {
                calls.push(quote! {
                    __sigil_builder.add_line();
                });
            }
            Statement::Indent => {
                calls.push(quote! {
                    __sigil_builder.add("%>", ());
                });
            }
            Statement::Dedent => {
                calls.push(quote! {
                    __sigil_builder.add("%<", ());
                });
            }
            Statement::Comment(text) => {
                calls.push(quote! {
                    __sigil_builder.add_comment(#text);
                });
            }
            Statement::Attr(text) => {
                calls.push(quote! {
                    __sigil_builder.add_attribute(#text);
                });
            }
            Statement::Statement { format, args } => {
                let args_tuple = build_args_tuple(args);
                calls.push(quote! {
                    __sigil_builder.add_statement(#format, #args_tuple);
                });
            }
            Statement::Line { format, args } => {
                let args_tuple = build_args_tuple(args);
                calls.push(quote! {
                    __sigil_builder.add(#format, #args_tuple);
                    __sigil_builder.add_line();
                });
            }
            Statement::ControlFlow { branches } => {
                for (i, branch) in branches.iter().enumerate() {
                    let fmt = &branch.condition_format;
                    let args_tuple = build_args_tuple(&branch.condition_args);
                    let body_calls = generate_statements(&branch.body);

                    if i == 0 {
                        calls.push(quote! {
                            __sigil_builder.begin_control_flow(#fmt, #args_tuple);
                        });
                    } else {
                        calls.push(quote! {
                            __sigil_builder.next_control_flow(#fmt, #args_tuple);
                        });
                    }

                    calls.extend(body_calls);
                }
                calls.push(quote! {
                    __sigil_builder.end_control_flow();
                });
            }
            Statement::SpliceEach { expr } => {
                calls.push(quote! {
                    for __sigil_item in #expr {
                        let __sigil_block: ::sigil_stitch::code_block::CodeBlock =
                            ::std::convert::Into::into(__sigil_item);
                        let __sigil_needs_nl =
                            !__sigil_block.ends_with_newline_or_block_close();
                        __sigil_builder.add_code(__sigil_block);
                        if __sigil_needs_nl {
                            __sigil_builder.add_line();
                        }
                    }
                });
            }
            Statement::MetaIf { branches } => {
                let meta_if = generate_meta_if(branches);
                calls.push(meta_if);
            }
            Statement::MetaFor {
                pat,
                iter_expr,
                body,
            } => {
                let body_calls = generate_statements(body);
                calls.push(quote! {
                    for #pat in #iter_expr {
                        #(#body_calls)*
                    }
                });
            }
            Statement::MetaLet { binding } => {
                calls.push(quote! {
                    let #binding;
                });
            }
        }
    }
    calls
}

/// Build the args tuple expression from typed args.
///
/// Wraps each arg according to its interpolation kind:
/// - `$T(expr)` -> bare expr (must be `TypeName`)
/// - `$N(expr)` -> `NameArg((expr).to_string())`
/// - `$S(expr)` -> `StringLitArg((expr).to_string())`
/// - `$V(expr)` -> `VerbatimStrArg((expr).to_string())`
/// - `$L(expr)` -> bare expr (via `Into<Arg>`)
/// - `$C(expr)` -> bare expr (must be `CodeBlock`)
fn build_args_tuple(args: &[TypedArg]) -> TokenStream {
    if args.is_empty() {
        quote! { () }
    } else {
        let arg_exprs: Vec<TokenStream> = args
            .iter()
            .map(|arg| {
                let expr = &arg.expr;
                match arg.kind {
                    InterpolationKind::Type | InterpolationKind::Code => {
                        quote! { #expr }
                    }
                    InterpolationKind::Literal => {
                        match extract_at_interpolation(expr) {
                            Some(VerbatimResult::Interpolated {
                                format_string,
                                expressions,
                            }) => {
                                let fmt_lit = Literal::string(&format_string);
                                let exprs: Vec<TokenStream> = expressions
                                    .iter()
                                    .map(|e| e.parse::<TokenStream>().unwrap())
                                    .collect();
                                quote! { ::std::format!(#fmt_lit, #(#exprs),*) }
                            }
                            Some(VerbatimResult::Literal(s)) => {
                                let lit = Literal::string(&s);
                                quote! { ::std::string::String::from(#lit) }
                            }
                            _ => {
                                quote! { #expr }
                            }
                        }
                    }
                    InterpolationKind::Name => {
                        quote! { ::sigil_stitch::code_block::NameArg((#expr).to_string()) }
                    }
                    InterpolationKind::StringLit => {
                        quote! { ::sigil_stitch::code_block::StringLitArg((#expr).to_string()) }
                    }
                    InterpolationKind::VerbatimStr => {
                        match extract_at_interpolation(expr) {
                            Some(VerbatimResult::Interpolated {
                                format_string,
                                expressions,
                            }) => {
                                let fmt_lit = Literal::string(&format_string);
                                let exprs: Vec<TokenStream> = expressions
                                    .iter()
                                    .map(|e| e.parse::<TokenStream>().unwrap())
                                    .collect();
                                quote! {
                                    ::sigil_stitch::code_block::VerbatimStrArg(
                                        ::std::format!(#fmt_lit, #(#exprs),*)
                                    )
                                }
                            }
                            Some(VerbatimResult::Literal(s)) => {
                                let lit = Literal::string(&s);
                                quote! {
                                    ::sigil_stitch::code_block::VerbatimStrArg(
                                        ::std::string::String::from(#lit)
                                    )
                                }
                            }
                            _ => {
                                quote! { ::sigil_stitch::code_block::VerbatimStrArg((#expr).to_string()) }
                            }
                        }
                    }
                }
            })
            .collect();
        quote! { (#(#arg_exprs,)*) }
    }
}

/// Generate a Rust if/else-if/else chain for meta-conditional branches.
fn generate_meta_if(branches: &[MetaBranch]) -> TokenStream {
    let mut result = TokenStream::new();

    for (i, branch) in branches.iter().enumerate() {
        let body_calls = generate_statements(&branch.body);
        let body_block = quote! { #(#body_calls)* };

        if i == 0 {
            // First branch: `if cond { ... }`
            let cond = branch.condition.as_ref().unwrap();
            result = quote! {
                if #cond {
                    #body_block
                }
            };
        } else if let Some(ref cond) = branch.condition {
            // Middle branch: `else if cond { ... }`
            result = quote! {
                #result else if #cond {
                    #body_block
                }
            };
        } else {
            // Final branch: `else { ... }`
            result = quote! {
                #result else {
                    #body_block
                }
            };
        }
    }

    result
}

/// Try to extract a string literal from a token stream and parse `@{expr}` interpolation.
///
/// Used by both `$V` (verbatim) and `$L` (literal). Returns `None` if the expression
/// is not a single string literal, in which case the expression is used as-is.
fn extract_at_interpolation(expr: &TokenStream) -> Option<VerbatimResult> {
    let tokens: Vec<TokenTree> = expr.clone().into_iter().collect();
    if tokens.len() != 1 {
        return None;
    }
    let lit = match &tokens[0] {
        TokenTree::Literal(lit) => lit,
        _ => return None,
    };
    let repr = lit.to_string();
    if !repr.starts_with('"') || !repr.ends_with('"') {
        return None;
    }
    let content = unescape_string_literal(&repr[1..repr.len() - 1])?;
    parse_verbatim_interpolation(&content).ok()
}

/// Unescape a Rust string literal body (content between the outer quotes).
fn unescape_string_literal(s: &str) -> Option<String> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next()? {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                '\'' => result.push('\''),
                '0' => result.push('\0'),
                'x' => {
                    let hi = chars.next()?.to_digit(16)?;
                    let lo = chars.next()?.to_digit(16)?;
                    result.push(char::from(((hi << 4) | lo) as u8));
                }
                'u' => {
                    if chars.next()? != '{' {
                        return None;
                    }
                    let mut val = 0u32;
                    loop {
                        let c = chars.next()?;
                        if c == '}' {
                            break;
                        }
                        val = val * 16 + c.to_digit(16)?;
                    }
                    result.push(char::from_u32(val)?);
                }
                _ => return None,
            }
        } else {
            result.push(ch);
        }
    }
    Some(result)
}
