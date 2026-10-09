//! Kotlin-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::FunctionForm;
use crate::lang::function_lowering::{
    SignatureBuilder, tupled_parameter_list, type_params_with_inline_constraints,
};
use crate::lang::kotlin::Kotlin;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::modifiers::Visibility;
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::where_spec::GenericParamView;
use crate::type_name::TypeName;

pub(crate) fn validate_generic_parameters(
    lang: &Kotlin,
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

pub(crate) fn validate_returns(
    lang: &Kotlin,
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
    lang: &Kotlin,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    validate_returns(lang, *function)?;
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

    if function.form() != FunctionForm::Constructor {
        signature.push_literal("fun ");
    }
    let type_params = type_params_with_inline_constraints(function, lang.file_extension())?;
    let (declaration_type_params, where_bounds) = split_kotlin_bounds(type_params.as_ref());
    if append_type_parameters(&mut signature, &declaration_type_params) {
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

    if let Some(returns) = function.returns() {
        let native_empty = crate::type_name::TypeName::primitive("Unit");
        let return_type = returns.first().unwrap_or(&native_empty);
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

fn append_type_parameters(
    signature: &mut SignatureBuilder,
    type_params: &[GenericParamView<'_>],
) -> bool {
    if type_params.is_empty() {
        return false;
    }
    signature.push_literal("<");
    for (index, parameter) in type_params.iter().enumerate() {
        if index > 0 {
            signature.push_literal(", ");
        }
        signature.push_literal(parameter.name());
        if let Some(bound) = parameter.bounds().first() {
            signature.push_literal(" : ");
            signature.push_type(bound);
        }
    }
    signature.push_literal(">");
    true
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

fn split_kotlin_bounds<'a>(
    type_params: &[GenericParamView<'a>],
) -> (Vec<GenericParamView<'a>>, Vec<(String, TypeName)>) {
    let mut declaration_type_params = type_params.to_vec();
    let mut where_bounds = Vec::new();

    for type_param in &mut declaration_type_params {
        let bounds = std::mem::take(&mut type_param.bounds);
        if bounds.len() <= 1 {
            type_param.bounds = bounds;
            continue;
        }
        where_bounds.extend(
            bounds
                .iter()
                .cloned()
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
