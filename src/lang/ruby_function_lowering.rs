//! Ruby-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{SignatureBuilder, tupled_parameter_list};
use crate::lang::ruby::Ruby;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &Ruby,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function);

    let visibility = lang.render_visibility(
        function.modifiers().visibility,
        function.declaration_context(),
    );
    if !visibility.is_empty() {
        block.add("%L", visibility.to_string());
    }
    let mut signature = SignatureBuilder::new();
    signature.push_literal("def ");
    signature.push_literal(function.name());
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    signature.append_to(&mut block);
    block.add_line();
    block.add("%>", ());
    let body = function
        .body()
        .expect("Ruby function validation requires a body");
    block.add_code(body.clone());
    if !body.ends_with_newline_or_block_close() {
        block.add_line();
    }
    block.add("%<end", ());
    block.add_line();
    block.build()
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Ruby, parameter: &ParameterSpec) {
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

fn emit_preamble(block: &mut CodeBlockBuilder, lang: &Ruby, function: ValidatedFunction<'_>) {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        let doc = lang.render_doc_comment(&lines);
        block.add("%L", doc);
    }
}
