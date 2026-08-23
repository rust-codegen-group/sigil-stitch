//! Python-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{SignatureBuilder, tupled_parameter_list};
use crate::lang::python::Python;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &Python,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_decorators(&mut block, function)?;

    let mut signature = SignatureBuilder::new();
    if function.modifiers().is_async {
        signature.push_literal("async ");
    }
    signature.push_literal("def ");
    signature.push_literal(function.name());
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    if let Some(return_type) = function.return_type() {
        signature.push_literal(" -> ");
        signature.push_type(return_type);
    }
    signature.push_literal(":");
    signature.append_to(&mut block);
    block.add_line();
    block.add("%>", ());

    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    if let Some(delegation) = function.delegation() {
        block.add_statement("%L", delegation.clone());
    }
    if let Some(body) = function.body() {
        block.add_code(body.clone());
        if !body.ends_with_newline_or_block_close() {
            block.add_line();
        }
    } else {
        block.add_statement("...", ());
    }
    block.add("%<", ());
    block.build()
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Python, parameter: &ParameterSpec) {
    block.add("%L", lang.escape_reserved(parameter.name()));
    if !parameter.param_type().is_empty() {
        block.add(": %T", parameter.param_type().clone());
    }
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

fn emit_decorators(
    block: &mut CodeBlockBuilder,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    for annotation in function.annotation_specs() {
        block.add_code(annotation.emit_with_syntax("@", "")?);
        block.add_line();
    }
    for annotation in function.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}
