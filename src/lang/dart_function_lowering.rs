//! Dart-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::dart::Dart;
use crate::lang::function_lowering::{
    SignatureBuilder, tupled_parameter_list, type_params_with_inline_constraints,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &Dart,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function)?;

    let mut signature = SignatureBuilder::new();
    if function.modifiers().is_static {
        signature.push_literal("static ");
    }
    if let Some(return_type) = function.return_type() {
        signature.push_type(return_type);
        signature.push_literal(" ");
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
    if let Some(delegation) = function.delegation() {
        signature.push_literal(" : ");
        signature.push_code(delegation.clone());
    }
    if function.modifiers().is_async {
        signature.push_literal(" async");
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
        signature.push_literal(";");
        signature.append_to(&mut block);
        block.add_line();
    }
    block.build()
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Dart, parameter: &ParameterSpec) {
    if !parameter.param_type().is_empty() {
        block.add("%T ", parameter.param_type().clone());
    }
    block.add("%L", lang.escape_reserved(parameter.name()));
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &Dart,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
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
