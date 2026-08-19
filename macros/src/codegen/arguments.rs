use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use crate::guard_plan::argument_is_fallible;
use crate::ir::{FormattedCode, QuoteArg, Statement, StringValue};

use super::context::GenerateContext;
use super::statements::generate_sequence;

pub(super) fn generate_call<F>(
    formatted: &FormattedCode,
    context: &mut GenerateContext,
    helper_error: &Ident,
    emit: F,
) -> TokenStream
where
    F: FnOnce(TokenStream) -> TokenStream,
{
    if !formatted.args().iter().any(argument_is_fallible) {
        let tuple = inline_tuple(formatted.args(), context);
        return emit(tuple);
    }

    let prepared: Vec<Ident> = formatted
        .args()
        .iter()
        .map(|_| context.ident("arg"))
        .collect();
    let tuple = tuple_from_idents(&prepared);
    let call = emit(tuple);
    let mut lowered = call;

    for (arg, ident) in formatted.args().iter().zip(prepared.iter()).rev() {
        let value = generate_arg(arg, context);
        if argument_is_fallible(arg) {
            let captured = context.ident("captured_error");
            lowered = quote! {
                match #value {
                    ::std::result::Result::Ok(#ident) => {
                        #lowered
                    }
                    ::std::result::Result::Err(#captured) => {
                        #helper_error = ::std::option::Option::Some(#captured);
                    }
                }
            };
        } else {
            lowered = quote! {
                match #value {
                    #ident => {
                        #lowered
                    }
                }
            };
        }
    }

    lowered
}

fn inline_tuple(args: &[QuoteArg], context: &mut GenerateContext) -> TokenStream {
    if args.is_empty() {
        quote! { () }
    } else {
        let values = args.iter().map(|arg| generate_arg(arg, context));
        quote! { (#(#values,)*) }
    }
}

fn tuple_from_idents(idents: &[Ident]) -> TokenStream {
    if idents.is_empty() {
        quote! { () }
    } else {
        quote! { (#(#idents,)*) }
    }
}

fn generate_arg(arg: &QuoteArg, context: &mut GenerateContext) -> TokenStream {
    match arg {
        QuoteArg::Type(expr) | QuoteArg::Code(expr) => quote! { #expr },
        QuoteArg::Name(expr) => {
            quote! { ::sigil_stitch::code_block::NameArg((#expr).to_string()) }
        }
        QuoteArg::StringLit(expr) => {
            quote! { ::sigil_stitch::code_block::StringLitArg((#expr).to_string()) }
        }
        QuoteArg::VerbatimStr(value) => {
            let rendered = render_owned_string(value);
            quote! { ::sigil_stitch::code_block::VerbatimStrArg(#rendered) }
        }
        QuoteArg::Literal(value) => render_string(value, StringMode::Literal),
        QuoteArg::Comment(value) => {
            let rendered = render_owned_string(value);
            quote! { ::sigil_stitch::code_block::CommentArg(#rendered) }
        }
        QuoteArg::Join { separator, iter } => {
            let items = context.ident("join_items");
            let item = context.ident("join_item");
            quote! {
                {
                    let #items: ::std::vec::Vec<::std::string::String> = (#iter)
                        .into_iter()
                        .map(|#item| ::std::string::ToString::to_string(&#item))
                        .collect();
                    #items.join(#separator)
                }
            }
        }
        QuoteArg::TypeJoin { separator, iter } => {
            let nested_builder = context.ident("type_join_builder");
            let index = context.ident("type_join_index");
            let item = context.ident("type_join_item");
            quote! {
                {
                    let mut #nested_builder =
                        ::sigil_stitch::code_block::CodeBlock::builder();
                    for (#index, #item) in (#iter).into_iter().enumerate() {
                        if #index > 0 {
                            #nested_builder.add(
                                "%L",
                                ::std::string::ToString::to_string(&(#separator)),
                            );
                        }
                        #nested_builder.add("%T", #item.clone());
                    }
                    #nested_builder.build()
                }
            }
        }
        QuoteArg::ParsedBlock(statements) => {
            generate_nested(statements, context, NestedKind::Block)
        }
        QuoteArg::ParsedSplice(statements) => {
            generate_nested(statements, context, NestedKind::Splice)
        }
    }
}

enum NestedKind {
    Block,
    Splice,
}

fn generate_nested(
    statements: &[Statement],
    context: &mut GenerateContext,
    kind: NestedKind,
) -> TokenStream {
    let builder = context.ident("nested_builder");
    let helper_error = context.ident("nested_helper_error");
    let success = context.ident("nested_block");
    let captured = context.ident("nested_build_error");
    let finish = match kind {
        NestedKind::Block => quote! {
            #builder.end_control_flow_no_newline();
        },
        NestedKind::Splice => TokenStream::new(),
    };
    let body = generate_sequence(statements, context, &builder, &helper_error);
    let begin = match kind {
        NestedKind::Block => quote! {
            #builder.begin_control_flow_with_intent(
                ::sigil_stitch::code_node::BlockIntent::Generic,
                "",
                (),
            );
        },
        NestedKind::Splice => TokenStream::new(),
    };
    let on_success = match kind {
        NestedKind::Block => quote! { ::std::result::Result::Ok(#success) },
        NestedKind::Splice => quote! {
            ::std::result::Result::Ok(#success.__sigil_trim_trailing_newline())
        },
    };

    quote! {
        {
            let mut #builder = ::sigil_stitch::code_block::CodeBlock::builder();
            let mut #helper_error = ::std::option::Option::None;
            #begin
            #body
            if #helper_error.is_none() {
                #finish
            }
            match #helper_error {
                ::std::option::Option::Some(#captured) => {
                    ::std::result::Result::Err(#captured)
                }
                ::std::option::Option::None => {
                    match #builder.build() {
                        ::std::result::Result::Ok(#success) => #on_success,
                        ::std::result::Result::Err(#captured) => {
                            ::std::result::Result::Err(#captured)
                        }
                    }
                }
            }
        }
    }
}

enum StringMode {
    Literal,
    Owned,
}

pub(super) fn render_owned_string(value: &StringValue) -> TokenStream {
    render_string(value, StringMode::Owned)
}

fn render_string(value: &StringValue, mode: StringMode) -> TokenStream {
    match value {
        StringValue::Literal(value) => {
            let literal = Literal::string(value);
            quote! { ::std::string::String::from(#literal) }
        }
        StringValue::Interpolated {
            format_string,
            expressions,
        } => {
            let format = Literal::string(format_string);
            quote! { ::std::format!(#format, #(#expressions),*) }
        }
        StringValue::Dynamic(expr) => match mode {
            StringMode::Literal => quote! { #expr },
            StringMode::Owned => quote! { (#expr).to_string() },
        },
    }
}
