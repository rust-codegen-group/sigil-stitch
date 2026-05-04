//! Code generator for `sigil_quote!`.
//!
//! Converts parsed statements into a `TokenStream` of `CodeBlockBuilder` method calls.

use proc_macro2::TokenStream;
use quote::quote;

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
                        if let Some(ref custom_open) = branch.block_open_override {
                            calls.push(quote! {
                                __sigil_builder.begin_control_flow_with_open(#fmt, #args_tuple, #custom_open);
                            });
                        } else {
                            calls.push(quote! {
                                __sigil_builder.begin_control_flow(#fmt, #args_tuple);
                            });
                        }
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
                    InterpolationKind::Type
                    | InterpolationKind::Literal
                    | InterpolationKind::Code => {
                        quote! { #expr }
                    }
                    InterpolationKind::Name => {
                        quote! { ::sigil_stitch::code_block::NameArg((#expr).to_string()) }
                    }
                    InterpolationKind::StringLit => {
                        quote! { ::sigil_stitch::code_block::StringLitArg((#expr).to_string()) }
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
