use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::guard_plan::statement_is_fallible;
use crate::ir::{Branch, ConditionalBranch, FormattedCode, LoopSeparator, MetaIf, Statement};

use super::arguments::{generate_call, render_owned_string};
use super::context::GenerateContext;

pub(super) fn generate_sequence(
    statements: &[Statement],
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    generate_sequence_with_guard(statements, context, builder, helper_error, false)
}

fn generate_sequence_with_guard(
    statements: &[Statement],
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
    mut guarded: bool,
) -> TokenStream {
    let mut generated = TokenStream::new();

    for (index, statement) in statements.iter().enumerate() {
        if guarded && matches!(statement, Statement::MetaLet { .. }) {
            let binding = generate_statement(statement, context, builder, helper_error);
            let tail = generate_sequence_with_guard(
                &statements[index + 1..],
                context,
                builder,
                helper_error,
                false,
            );
            generated.extend(quote! {
                if #helper_error.is_none() {
                    #binding
                    #tail
                }
            });
            return generated;
        }

        let statement = generate_statement(statement, context, builder, helper_error);
        if guarded {
            generated.extend(quote! {
                if #helper_error.is_none() {
                    #statement
                }
            });
        } else {
            generated.extend(statement);
        }
        guarded |= statement_is_fallible(&statements[index]);
    }

    generated
}

fn generate_statement(
    statement: &Statement,
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    match statement {
        Statement::BlankLine => quote! { #builder.add_line(); },
        Statement::Indent => quote! { #builder.add("%>", ()); },
        Statement::Dedent => quote! { #builder.add("%<", ()); },
        Statement::Comment(value) => {
            let expr = render_owned_string(value);
            quote! { #builder.add_comment(&#expr); }
        }
        Statement::Attr(value) => {
            let expr = render_owned_string(value);
            quote! { #builder.add_attribute(&#expr); }
        }
        Statement::Terminated(formatted) => generate_formatted(
            formatted,
            context,
            builder,
            helper_error,
            FormattedCall::Statement,
        ),
        Statement::Line(formatted) => generate_formatted(
            formatted,
            context,
            builder,
            helper_error,
            FormattedCall::Line,
        ),
        Statement::ControlFlow {
            branches,
            trailing_semicolon,
        } => generate_control_flow(
            branches,
            *trailing_semicolon,
            context,
            builder,
            helper_error,
        ),
        Statement::SpliceEach { expr } => {
            let item = context.ident("splice_item");
            let block = context.ident("splice_block");
            let needs_newline = context.ident("splice_needs_newline");
            quote! {
                for #item in #expr {
                    let #block: ::sigil_stitch::code_block::CodeBlock =
                        ::std::convert::Into::into(#item);
                    let #needs_newline = !#block.ends_with_newline_or_block_close();
                    #builder.add_code(#block);
                    if #needs_newline {
                        #builder.add_line();
                    }
                }
            }
        }
        Statement::MetaIf(meta_if) => generate_meta_if(meta_if, context, builder, helper_error),
        Statement::MetaFor {
            pat,
            iter_expr,
            separator,
            body,
        } => generate_meta_for(
            pat,
            iter_expr,
            separator.as_ref(),
            body,
            context,
            builder,
            helper_error,
        ),
        Statement::InlineFor {
            pat,
            iter_expr,
            separator,
            body,
        } => generate_inline_for(
            pat,
            iter_expr,
            separator.as_ref(),
            body,
            context,
            builder,
            helper_error,
        ),
        Statement::MetaLet { local, .. } => quote! { #local },
        Statement::ParenBlock { header, body } => {
            let header = generate_call(header, context, helper_error, |args| {
                let format = header.format();
                quote! {
                    #builder.add(#format, #args);
                    #builder.add_line();
                    #builder.add("%>", ());
                }
            });
            let body = generate_sequence(body, context, builder, helper_error);
            quote! {
                #header
                if #helper_error.is_none() {
                    #body
                    if #helper_error.is_none() {
                        #builder.add("%<", ());
                        #builder.add(")", ());
                    }
                }
            }
        }
    }
}

enum FormattedCall {
    Statement,
    Line,
}

fn generate_formatted(
    formatted: &FormattedCode,
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
    call: FormattedCall,
) -> TokenStream {
    let format = formatted.format();
    generate_call(formatted, context, helper_error, |args| match call {
        FormattedCall::Statement => quote! { #builder.add_statement(#format, #args); },
        FormattedCall::Line => quote! {
            #builder.add(#format, #args);
            #builder.add_line();
        },
    })
}

fn generate_control_flow(
    branches: &[Branch],
    trailing_semicolon: bool,
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    let end = if trailing_semicolon {
        quote! { #builder.end_control_flow_with_semicolon(); }
    } else {
        quote! { #builder.end_control_flow(); }
    };
    let mut generated = TokenStream::new();

    for (index, branch) in branches.iter().enumerate() {
        let format = branch.condition.format();
        let intent = branch.intent.runtime_path();
        let branch_start = generate_call(&branch.condition, context, helper_error, |args| {
            if index == 0 {
                quote! { #builder.begin_control_flow_with_intent(#intent, #format, #args); }
            } else {
                quote! { #builder.next_control_flow_with_intent(#intent, #format, #args); }
            }
        });
        let body = generate_sequence(&branch.body, context, builder, helper_error);
        generated.extend(quote! {
            if #helper_error.is_none() {
                #branch_start
                if #helper_error.is_none() {
                    #body
                }
            }
        });
    }

    generated.extend(quote! {
        if #helper_error.is_none() {
            #end
        }
    });
    generated
}

fn generate_meta_if(
    meta_if: &MetaIf,
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    let first = generate_conditional_branch(&meta_if.first, context, builder, helper_error);
    let first_condition = &meta_if.first.condition;
    let mut chain = quote! { if #first_condition { #first } };

    for branch in &meta_if.else_if {
        let body = generate_conditional_branch(branch, context, builder, helper_error);
        let condition = &branch.condition;
        chain = quote! { #chain else if #condition { #body } };
    }

    if let Some(body) = &meta_if.otherwise {
        let body = generate_sequence(body, context, builder, helper_error);
        chain = quote! { #chain else { #body } };
    }

    chain
}

fn generate_conditional_branch(
    branch: &ConditionalBranch,
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    generate_sequence(&branch.body, context, builder, helper_error)
}

#[allow(clippy::too_many_arguments)]
fn generate_meta_for(
    pat: &syn::Pat,
    iter_expr: &syn::Expr,
    separator: Option<&LoopSeparator>,
    body: &[Statement],
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    let emitted = context.ident("for_emitted");
    let body = generate_sequence(body, context, builder, helper_error);

    if let Some(separator) = separator {
        let separator_expr = &separator.expr;
        let trailing = separator
            .trailing
            .as_ref()
            .map_or_else(|| quote! { false }, |expr| quote! { #expr });
        quote! {
            let mut #emitted = false;
            for #pat in #iter_expr {
                if #emitted {
                    #builder.add(
                        "%L",
                        ::std::string::ToString::to_string(&(#separator_expr)),
                    );
                }
                #emitted = true;
                #body
                if #helper_error.is_some() {
                    break;
                }
            }
            if #helper_error.is_none() && #emitted && (#trailing) {
                #builder.add(
                    "%L",
                    ::std::string::ToString::to_string(&(#separator_expr)),
                );
            }
        }
    } else {
        quote! {
            for #pat in #iter_expr {
                #body
                if #helper_error.is_some() {
                    break;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_inline_for(
    pat: &syn::Pat,
    iter_expr: &syn::Expr,
    separator: Option<&LoopSeparator>,
    body: &FormattedCode,
    context: &mut GenerateContext,
    builder: &Ident,
    helper_error: &Ident,
) -> TokenStream {
    let emitted = context.ident("inline_for_emitted");
    let format = body.format();
    let body = generate_call(
        body,
        context,
        helper_error,
        |args| quote! { #builder.add(#format, #args); },
    );

    if let Some(separator) = separator {
        let separator_expr = &separator.expr;
        let trailing = separator
            .trailing
            .as_ref()
            .map_or_else(|| quote! { false }, |expr| quote! { #expr });
        quote! {
            let mut #emitted = false;
            for #pat in #iter_expr {
                if #emitted {
                    #builder.add(
                        "%L",
                        ::std::string::ToString::to_string(&(#separator_expr)),
                    );
                }
                #emitted = true;
                #body
                if #helper_error.is_some() {
                    break;
                }
            }
            if #helper_error.is_none() && #emitted && (#trailing) {
                #builder.add(
                    "%L",
                    ::std::string::ToString::to_string(&(#separator_expr)),
                );
            }
        }
    } else {
        quote! {
            for #pat in #iter_expr {
                #body
                if #helper_error.is_some() {
                    break;
                }
            }
        }
    }
}
