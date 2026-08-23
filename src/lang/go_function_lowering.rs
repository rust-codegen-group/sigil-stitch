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
    signature.push_type_params(type_params.as_ref(), lang);
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    if let Some(return_type) = function.return_type() {
        signature.push_literal(" ");
        signature.push_type(return_type);
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
