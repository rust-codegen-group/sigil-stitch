//! Kotlin-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{
    SignatureBuilder, tupled_parameter_list, type_params_with_inline_constraints,
};
use crate::lang::kotlin::Kotlin;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::modifiers::Visibility;
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::where_spec::TypeParamSpec;
use crate::type_name::TypeName;

pub(crate) fn lower(
    lang: &Kotlin,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function)?;

    let mut signature = SignatureBuilder::new();
    if function.modifiers().visibility != Visibility::Inherited {
        signature.push_literal(lang.render_visibility(
            function.modifiers().visibility,
            function.declaration_context(),
        ));
    }
    if function.modifiers().is_abstract {
        signature.push_literal("abstract ");
    }
    if function.modifiers().is_override {
        signature.push_literal("override ");
    }
    if function.modifiers().is_async {
        signature.push_literal("suspend ");
    }

    if !function.modifiers().is_constructor {
        signature.push_literal("fun ");
    }
    let type_params = type_params_with_inline_constraints(function, lang.file_extension())?;
    let (declaration_type_params, where_bounds) = split_kotlin_bounds(type_params.as_ref());
    if signature.push_type_params(&declaration_type_params, lang) {
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

    if let Some(return_type) = function.return_type() {
        signature.push_literal(": ");
        signature.push_type(return_type);
    }
    if let Some(delegation) = function.delegation() {
        signature.push_literal(" : ");
        signature.push_code(delegation.clone());
    }
    append_where_bounds(&mut signature, &where_bounds);

    finish(&mut block, signature, function)?;
    block.build()
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &Kotlin,
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

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Kotlin, parameter: &ParameterSpec) {
    if parameter.is_variadic() {
        block.add("...", ());
    }
    let property = if parameter.is_property() {
        "val "
    } else if parameter.is_mutable_property() {
        "var "
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

fn split_kotlin_bounds(
    type_params: &[TypeParamSpec],
) -> (Vec<TypeParamSpec>, Vec<(String, TypeName)>) {
    let mut declaration_type_params = type_params.to_vec();
    let mut where_bounds = Vec::new();

    for type_param in &mut declaration_type_params {
        let mut bounds = std::mem::take(&mut type_param.bounds);
        bounds.append(&mut type_param.context_bounds);
        if bounds.len() <= 1 {
            type_param.bounds = bounds;
            continue;
        }
        where_bounds.extend(
            bounds
                .into_iter()
                .map(|bound| (type_param.name().to_string(), bound)),
        );
    }

    (declaration_type_params, where_bounds)
}

fn append_where_bounds(signature: &mut SignatureBuilder, where_bounds: &[(String, TypeName)]) {
    if where_bounds.is_empty() {
        return;
    }
    signature.push_literal(" where ");
    for (index, (subject, bound)) in where_bounds.iter().enumerate() {
        if index > 0 {
            signature.push_literal(", ");
        }
        signature.push_literal(subject);
        signature.push_literal(" : ");
        signature.push_type(bound);
    }
}

fn finish(
    block: &mut CodeBlockBuilder,
    mut signature: SignatureBuilder,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    let Some(body) = function.body() else {
        signature.append_to(block);
        block.add_line();
        return Ok(());
    };

    signature.push_literal(" {");
    signature.append_to(block);
    block.add_line();
    block.add("%>", ());
    block.add_code(body.clone());
    if !body.ends_with_newline_or_block_close() {
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
    Ok(())
}
