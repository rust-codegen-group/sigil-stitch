//! JavaScript-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{SignatureBuilder, tupled_parameter_list};
use crate::lang::javascript::JavaScript;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::modifiers::DeclarationContext;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &JavaScript,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function);

    let mut signature = SignatureBuilder::new();
    signature.push_literal(lang.render_visibility(
        function.modifiers().visibility,
        function.declaration_context(),
    ));
    if function.modifiers().is_static {
        signature.push_literal("static ");
    }
    if function.modifiers().is_async {
        signature.push_literal("async ");
    }
    if !function.modifiers().is_constructor
        && function.declaration_context() == DeclarationContext::TopLevel
    {
        signature.push_literal("function ");
    }
    signature.push_literal(function.name());
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    signature.push_literal(" {");
    signature.append_to(&mut block);
    block.add_line();
    block.add("%>", ());
    if let Some(delegation) = function.delegation() {
        block.add_statement("%L", delegation.clone());
    }
    let body = function
        .body()
        .expect("JavaScript function validation requires a body");
    block.add_code(body.clone());
    if !body.ends_with_newline_or_block_close() {
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
    block.build()
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &JavaScript, parameter: &ParameterSpec) {
    if parameter.is_variadic() {
        block.add("...", ());
    }
    block.add("%L", lang.escape_reserved(parameter.name()));
    if let Some(default) = parameter.default_value() {
        block.add(" = %L", default.clone());
    }
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
}

fn emit_preamble(block: &mut CodeBlockBuilder, lang: &JavaScript, function: ValidatedFunction<'_>) {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
}
