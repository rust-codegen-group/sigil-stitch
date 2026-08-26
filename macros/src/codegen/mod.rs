//! Infallible lowering from validated macro parse forms to generated Rust.

mod arguments;
mod context;
mod statements;

use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::ParsedInput;

use context::GenerateContext;

pub(crate) fn generate(input: ParsedInput) -> TokenStream {
    let mut context = GenerateContext::new();
    let builder = context.ident("builder");
    let helper_error = context.ident("helper_error");
    let body =
        statements::generate_sequence(&input.statements, &mut context, &builder, &helper_error);

    quote! {
        {
            let mut #builder = ::sigil_stitch::code_block::CodeBlock::builder();
            let mut #helper_error = ::std::option::Option::None;
            #body
            match #helper_error {
                ::std::option::Option::Some(__sigil_error) => {
                    ::std::result::Result::Err(__sigil_error)
                }
                ::std::option::Option::None => #builder.build(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FormattedCode, QuoteArg, Statement};

    #[test]
    fn lowers_large_infallible_sequences_without_recursive_tail_building() {
        let statements = (0..8_192).map(|_| Statement::BlankLine).collect();
        let generated = generate(ParsedInput { statements });

        assert_eq!(generated.to_string().matches("add_line").count(), 8_192);
    }

    #[test]
    fn lowers_large_fallible_sequences_without_recursive_tail_building() {
        let statements = (0..8_192)
            .map(|_| {
                let mut formatted = FormattedCode::new();
                formatted.push_argument(QuoteArg::TypeJoin {
                    separator: syn::parse_quote!("__fallible_stress__"),
                    iter: syn::parse_quote!(::std::iter::empty::<&str>()),
                });
                Statement::Line(formatted)
            })
            .collect();
        let generated = generate(ParsedInput { statements });

        assert_eq!(
            generated.to_string().matches("__fallible_stress__").count(),
            8_192
        );
    }
}
