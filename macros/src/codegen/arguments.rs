use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

use crate::guard_plan::argument_is_fallible;
use crate::ir::{
    DelimitedLayout, FormattedCode, LayoutEdgesPlan, LayoutForceBreak, LayoutPlan, LoopSeparator,
    QuoteArg, SeparatorIntent, SingletonCommaPolicy, Statement, StringValue, TrailingCommaPolicy,
};

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
        QuoteArg::Type(expr) => quote! { #expr },
        QuoteArg::Code(expr) => quote! { (#expr).__sigil_trim_trailing_newline() },
        QuoteArg::Name(expr) => {
            // Move diagnostics without changing generated-token name resolution.
            let span = Span::call_site().located_at(expr.span());
            quote_spanned! {span=> ::sigil_stitch::code_block::NameArg((#expr).to_string()) }
        }
        QuoteArg::StringLit(expr) => {
            let span = Span::call_site().located_at(expr.span());
            quote_spanned! {span=> ::sigil_stitch::code_block::StringLitArg((#expr).to_string()) }
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
        QuoteArg::Layout { code, spec } => match spec {
            LayoutPlan::Inline => {
                let block = generate_formatted_result(code, context);
                quote! { #block.map(::sigil_stitch::code_block::CodeBlock::__sigil_inline_layout) }
            }
            LayoutPlan::Delimited(layout) => {
                if let Some(block) = generate_delimited_loop(code, layout, context) {
                    return block;
                }
                let block = generate_formatted_result(code, context);
                let layout_block = context.ident("layout_block");
                // An unclassified splice may already supply its own terminal separator.
                let trailing_comma = if code
                    .args()
                    .iter()
                    .any(|arg| matches!(arg, QuoteArg::ParsedSplice(_)))
                {
                    TrailingCommaPolicy::Never
                } else {
                    layout.trailing_comma
                };
                let wrap = generate_layout_call(
                    layout,
                    layout.edges,
                    &layout_block,
                    trailing_comma.runtime_value(),
                );
                quote! { #block.map(|#layout_block| #wrap) }
            }
        },
    }
}

fn generate_delimited_loop(
    code: &FormattedCode,
    spec: &DelimitedLayout,
    context: &mut GenerateContext,
) -> Option<TokenStream> {
    let plan = spec.delimited_loop?;
    if code.format() != "%L" {
        return None;
    }
    let [QuoteArg::ParsedSplice(statements)] = code.args() else {
        return None;
    };
    let [
        Statement::InlineFor {
            pat,
            iter_expr,
            separator:
                Some(LoopSeparator {
                    intent: SeparatorIntent::CommaSpace,
                    trailing,
                    ..
                }),
            body,
        },
    ] = statements.as_slice()
    else {
        return None;
    };

    let builder = context.ident("loop_builder");
    let item = context.ident("loop_item");
    let count = context.ident("loop_count");
    let comma = context.ident("loop_comma");
    let children = context.ident("loop_children");
    let item_code = generate_formatted_result(body, context);
    let request = trailing.as_ref().map_or_else(
        || quote! { false },
        |expr| quote! { #count != 0 && (#expr) },
    );
    let never = TrailingCommaPolicy::Never.runtime_value();
    let if_broken = TrailingCommaPolicy::IfBroken.runtime_value();
    let always = TrailingCommaPolicy::Always.runtime_value();
    let requested = match plan.singleton_comma {
        SingletonCommaPolicy::KeepInline => quote! {
            if #count == 1 {
                #builder.add(",", ());
                #never
            } else {
                #always
            }
        },
        SingletonCommaPolicy::ForceBroken => always.clone(),
    };
    let wrap = generate_layout_call(spec, plan.edges, &children, quote! { #comma });
    Some(quote! {
        (|| -> ::std::result::Result<
            ::sigil_stitch::code_block::CodeBlock,
            ::sigil_stitch::error::SigilStitchError,
        > {
            let mut #builder = ::sigil_stitch::code_block::CodeBlock::builder();
            let mut #count = 0u8;
            for #pat in #iter_expr {
                let #item = #item_code?;
                if !#item.is_empty() {
                    if #count != 0 {
                        #builder.add(",%W", ());
                    }
                    #builder.add_code(#item);
                    #count = (#count + 1).min(2);
                }
            }
            let #comma = if #request {
                #requested
            } else if #count > 1 {
                #if_broken
            } else {
                #never
            };
            let #children = #builder.build()?.__sigil_trim_trailing_newline();
            ::std::result::Result::Ok(#wrap)
        })()
    })
}

fn generate_layout_call(
    spec: &DelimitedLayout,
    edges_plan: LayoutEdgesPlan,
    children: &Ident,
    trailing_comma: TokenStream,
) -> TokenStream {
    let flat_prefix = Literal::string(&spec.flat_prefix);
    let flat_suffix = Literal::string(&spec.flat_suffix);
    let broken_prefix = Literal::string(&spec.broken_prefix);
    let broken_suffix = Literal::string(&spec.broken_suffix);
    let force_break = match spec.force_break {
        LayoutForceBreak::Automatic => {
            quote! { ::sigil_stitch::code_node::LayoutForceBreak::Automatic }
        }
        LayoutForceBreak::Forced => {
            quote! { ::sigil_stitch::code_node::LayoutForceBreak::Forced }
        }
    };
    let edges = match edges_plan {
        LayoutEdgesPlan::WithBreaks => {
            quote! { ::sigil_stitch::code_node::LayoutEdgeMode::WithBreaks }
        }
        LayoutEdgesPlan::Coupled => {
            quote! { ::sigil_stitch::code_node::LayoutEdgeMode::Coupled }
        }
    };
    quote! {
        ::sigil_stitch::code_block::CodeBlock::__sigil_layout_with_options(
            #children,
            ::sigil_stitch::code_block::LayoutOptions {
                group_mode: ::sigil_stitch::code_node::LayoutGroupMode::Consistent,
                break_mode: ::sigil_stitch::code_node::LayoutBreakMode::Space,
                edges: #edges,
                trailing_comma: #trailing_comma,
                force_break: #force_break,
                flat_prefix: #flat_prefix,
                flat_suffix: #flat_suffix,
                broken_prefix: #broken_prefix,
                broken_suffix: #broken_suffix,
            },
        )
    }
}

fn generate_formatted_result(
    formatted: &FormattedCode,
    context: &mut GenerateContext,
) -> TokenStream {
    let format = Literal::string(formatted.format());
    let values: Vec<Ident> = formatted
        .args()
        .iter()
        .map(|_| context.ident("layout_arg"))
        .collect();
    let bindings = formatted
        .args()
        .iter()
        .zip(values.iter())
        .map(|(arg, ident)| {
            let value = generate_arg(arg, context);
            if argument_is_fallible(arg) {
                quote! { let #ident = #value?; }
            } else {
                quote! { let #ident = #value; }
            }
        });
    let tuple = if values.is_empty() {
        quote! { () }
    } else {
        quote! { (#(#values,)*) }
    };
    quote! {
        (|| -> ::std::result::Result<
            ::sigil_stitch::code_block::CodeBlock,
            ::sigil_stitch::error::SigilStitchError,
        > {
            #(#bindings)*
            ::sigil_stitch::code_block::CodeBlock::of(#format, #tuple)
        })()
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
            span,
        } => {
            let mut format = Literal::string(format_string);
            format.set_span(*span);
            quote_spanned! {*span=> ::std::format!(#format, #(#expressions),*) }
        }
        StringValue::Dynamic(expr) => match mode {
            StringMode::Literal => quote! { #expr },
            StringMode::Owned => {
                let span = Span::call_site().located_at(expr.span());
                quote_spanned! {span=> (#expr).to_string() }
            }
        },
    }
}
