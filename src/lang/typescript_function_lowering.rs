//! TypeScript-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::FunctionForm;
use crate::lang::function_lowering::{
    SignatureBuilder, tupled_parameter_list, type_params_with_inline_constraints,
};
use crate::lang::typescript::TypeScript;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::modifiers::DeclarationContext;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &TypeScript,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function)?;

    let mut signature = SignatureBuilder::new();
    if function.declaration_context() != DeclarationContext::InterfaceMember {
        signature.push_literal(lang.render_visibility(
            function.modifiers().visibility,
            function.declaration_context(),
        ));
    }
    if function.modifiers().is_abstract {
        signature.push_literal("abstract ");
    }
    if function.modifiers().is_static {
        signature.push_literal("static ");
    }
    if function.modifiers().is_override {
        signature.push_literal("override ");
    }
    if function.modifiers().is_async {
        signature.push_literal("async ");
    }

    if function.form() != FunctionForm::Constructor
        && function.declaration_context() == DeclarationContext::TopLevel
    {
        signature.push_literal("function ");
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
    if let Some(return_type) = function.return_type() {
        signature.push_literal(": ");
        signature.push_type(return_type);
    }

    finish(&mut block, signature, lang, function)?;
    block.build()
}

fn append_type_parameters(
    signature: &mut SignatureBuilder,
    type_params: &[crate::spec::where_spec::TypeParamSpec],
) {
    if type_params.is_empty() {
        return;
    }
    signature.push_literal("<");
    for (index, parameter) in type_params.iter().enumerate() {
        if index > 0 {
            signature.push_literal(", ");
        }
        signature.push_literal(parameter.name());
        if !parameter.bounds().is_empty() {
            signature.push_literal(" extends ");
            for (bound_index, bound) in parameter.bounds().iter().enumerate() {
                if bound_index > 0 {
                    signature.push_literal(" & ");
                }
                signature.push_type(bound);
            }
        }
    }
    signature.push_literal(">");
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &TypeScript,
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

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &TypeScript, parameter: &ParameterSpec) {
    if parameter.is_variadic() {
        block.add("...", ());
    }
    let property = if parameter.is_property() {
        "readonly "
    } else if parameter.is_mutable_property() {
        "public "
    } else {
        ""
    };
    block.add(
        "%L",
        format!("{property}{}", lang.escape_reserved(parameter.name())),
    );
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

#[expect(deprecated, reason = "0.6.8 renderer compatibility bridge")]
fn finish(
    block: &mut CodeBlockBuilder,
    mut signature: SignatureBuilder,
    lang: &TypeScript,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    let Some(body) = function.body() else {
        if lang.block_syntax().uses_semicolons {
            signature.push_literal(";");
        }
        signature.append_to(block);
        block.add_line();
        return Ok(());
    };

    signature.push_literal(" {");
    signature.append_to(block);
    block.add_line();
    block.add("%>", ());
    if let Some(delegation) = function.delegation() {
        block.add_statement("%L", delegation.clone());
    }
    block.add_code(body.clone());
    if !body.ends_with_newline_or_block_close() {
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
    Ok(())
}
