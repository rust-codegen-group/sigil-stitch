//! C++-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::cpp::Cpp;
use crate::lang::function_lowering::{SignatureBuilder, tupled_parameter_list};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn validate_returns(
    lang: &Cpp,
    function: crate::lang::FunctionIntent<'_>,
) -> Result<(), SigilStitchError> {
    if let Some(returns) = function.returns()
        && returns.len() > 1
    {
        return Err(SigilStitchError::UnsupportedFunctionReturns {
            language: lang.file_extension().into(),
            function_name: function.name().into(),
            context: function.function_context(),
            form: function.form(),
            actual: returns.len(),
            reason: "this language supports at most one return slot".into(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Cpp,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    validate_returns(lang, *function)?;
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function)?;
    if function.generic_params().next().is_some() {
        block.add("template<", ());
        for (index, parameter) in function.generic_params().enumerate() {
            if index > 0 {
                block.add(", ", ());
            }
            let pack = matches!(
                parameter.domain().as_ref(),
                crate::spec::where_spec::GenericParamDomain::Pack { .. }
            );
            block.add(
                if pack { "class... %L" } else { "class %L" },
                parameter.name(),
            );
        }
        block.add(">", ());
        block.add_line();
    }

    let mut signature = SignatureBuilder::new();
    if function.modifiers().is_abstract {
        signature.push_literal("virtual ");
    }
    if function.modifiers().is_static {
        signature.push_literal("static ");
    }
    if function.modifiers().is_async {
        signature.push_literal("async ");
    }
    if let Some(returns) = function.returns() {
        let native_empty = crate::type_name::TypeName::primitive("void");
        let return_type = returns.first().unwrap_or(&native_empty);
        signature.push_type(return_type);
        signature.push_literal(" ");
    }

    signature.push_literal(function.name());
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

    finish(&mut block, signature, function)?;
    block.build()
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &Cpp,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    for annotation in function.annotation_specs() {
        block.add_code(annotation.emit_with_syntax("[[", "]]")?);
        block.add_line();
    }
    for annotation in function.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Cpp, parameter: &ParameterSpec) {
    if !parameter.param_type().is_empty() {
        block.add("%T ", parameter.param_type().clone());
    }
    block.add("%L", lang.escape_reserved(parameter.name()));
    if let Some(default) = parameter.default_value() {
        block.add(" = %L", default.clone());
    }
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    let mut override_pending = function.modifiers().is_override
        && !function
            .suffixes()
            .iter()
            .any(|suffix| suffix.trim() == "override");
    for suffix in function.suffixes() {
        if override_pending && suffix.trim_start().starts_with('=') {
            signature.push_literal(" override");
            override_pending = false;
        }
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
    if override_pending {
        signature.push_literal(" override");
    }
}

fn finish(
    block: &mut CodeBlockBuilder,
    mut signature: SignatureBuilder,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    let needs_body = function.body().is_some() || function.delegation().is_some();
    if !needs_body {
        signature.push_literal(";");
        signature.append_to(block);
        block.add_line();
        return Ok(());
    }

    signature.push_literal(" {");
    signature.append_to(block);
    block.add_line();
    block.add("%>", ());
    if let Some(body) = function.body() {
        block.add_code(body.clone());
        if !body.ends_with_newline_or_block_close() {
            block.add_line();
        }
    }
    block.add("%<}", ());
    block.add_line();
    Ok(())
}
