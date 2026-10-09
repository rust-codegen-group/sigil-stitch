//! Go-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{
    SignatureBuilder, tupled_parameter_list, type_params_with_inline_constraints,
};
use crate::lang::go::Go;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::modifiers::DeclarationContext;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn validate_generic_parameters(
    lang: &Go,
    function: crate::lang::FunctionIntent<'_>,
) -> Result<(), SigilStitchError> {
    for parameter in function.generic_params() {
        if !parameter.has_kind_or_pack_domain() {
            continue;
        }
        if !matches!(
            parameter.domain().as_ref(),
            crate::spec::where_spec::GenericParamDomain::Single {
                kind: None | Some(crate::spec::where_spec::KindExpr::Type)
            }
        ) {
            return Err(SigilStitchError::UnsupportedTypeName {
                language: lang.file_extension().into(),
                context: format!("generic_param.{}", parameter.name()),
                reason: "this binding domain has no representation in this function declaration"
                    .into(),
            });
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Go,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function);

    let mut signature = SignatureBuilder::new();
    if function.receiver().is_some()
        || function.declaration_context() == DeclarationContext::TopLevel
    {
        signature.push_literal("func ");
    }
    if let Some(receiver) = function.receiver() {
        signature.push_literal("(");
        signature.push_literal(&lang.escape_reserved(receiver.name()));
        signature.push_literal(" ");
        signature.push_type(receiver.param_type());
        signature.push_literal(") ");
    }
    signature.push_literal(function.name());
    let type_params = type_params_with_inline_constraints(function, lang.file_extension())?;
    append_type_parameters(&mut signature, type_params.as_ref());
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    if let Some(returns) = function.returns()
        && !returns.is_empty()
    {
        signature.push_literal(" ");
        if returns.len() > 1 {
            signature.push_literal("(");
        }
        for (index, return_type) in returns.iter().enumerate() {
            if index > 0 {
                signature.push_literal(", ");
            }
            signature.push_type(return_type);
        }
        if returns.len() > 1 {
            signature.push_literal(")");
        }
    }

    if let Some(body) = function.body() {
        signature.push_literal(" {");
        signature.append_to(&mut block);
        block.add_line();
        block.add("%>", ());
        block.add_code(body.clone());
        if !body.ends_with_newline_or_block_close() {
            block.add_line();
        }
        block.add("%<}", ());
        block.add_line();
    } else {
        signature.append_to(&mut block);
        block.add_line();
    }
    block.build()
}

fn append_type_parameters(
    signature: &mut SignatureBuilder,
    type_params: &[crate::spec::where_spec::GenericParamView<'_>],
) {
    if type_params.is_empty() {
        return;
    }
    signature.push_literal("[");
    for (index, parameter) in type_params.iter().enumerate() {
        if index > 0 {
            signature.push_literal(", ");
        }
        signature.push_literal(parameter.name());
        signature.push_literal(" ");
        match parameter.bounds() {
            [] => signature.push_literal("any"),
            [bound] => signature.push_type(bound),
            bounds => {
                signature.push_literal("interface { ");
                for (bound_index, bound) in bounds.iter().enumerate() {
                    if bound_index > 0 {
                        signature.push_literal("; ");
                    }
                    signature.push_type(bound);
                }
                signature.push_literal(" }");
            }
        }
    }
    signature.push_literal("]");
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Go, parameter: &ParameterSpec) {
    block.add("%L", lang.escape_reserved(parameter.name()));
    if !parameter.param_type().is_empty() {
        block.add(" %T", parameter.param_type().clone());
    }
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
}

fn emit_preamble(block: &mut CodeBlockBuilder, lang: &Go, function: ValidatedFunction<'_>) {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
}
