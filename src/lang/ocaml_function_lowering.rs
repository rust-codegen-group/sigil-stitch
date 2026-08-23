//! OCaml-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{SignatureBuilder, curried_parameter_list};
use crate::lang::ocaml::OCaml;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &OCaml,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function);

    let mut signature = SignatureBuilder::new();
    signature.push_literal("let ");
    signature.push_literal(function.name());
    if !function.parameters().is_empty() {
        signature.push_literal(" ");
        signature.push_code(curried_parameter_list(
            function.parameters(),
            |parameters, parameter| emit_parameter(parameters, lang, parameter),
        )?);
    }
    append_suffixes(&mut signature, function);
    if let Some(return_type) = function.return_type() {
        signature.push_literal(" : ");
        signature.push_type(return_type);
    }
    signature.push_literal(" =");
    signature.append_to(&mut block);
    block.add_line();
    block.add("%>", ());
    let body = function
        .body()
        .expect("OCaml function validation requires a body");
    block.add_code(body.clone());
    if !body.ends_with_newline_or_block_close() {
        block.add_line();
    }
    block.add("%<", ());
    block.build()
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &OCaml, parameter: &ParameterSpec) {
    block.add("%L", lang.escape_reserved(parameter.name()));
    if !parameter.param_type().is_empty() {
        block.add(" : %T", parameter.param_type().clone());
    }
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
}

fn emit_preamble(block: &mut CodeBlockBuilder, lang: &OCaml, function: ValidatedFunction<'_>) {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
}
