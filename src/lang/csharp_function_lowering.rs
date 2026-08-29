//! C#-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::FunctionForm;
use crate::lang::csharp::CSharp;
use crate::lang::csharp_constraints;
use crate::lang::function_lowering::{SignatureBuilder, tupled_parameter_list};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::modifiers::{DeclarationContext, Visibility};
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn lower(
    lang: &CSharp,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function)?;

    let mut signature = SignatureBuilder::new();
    let is_static_constructor = function.form() == FunctionForm::Constructor
        && function.modifiers().is_static
        && function.declaration_context() == DeclarationContext::Member;
    if !is_static_constructor && function.modifiers().visibility != Visibility::Inherited {
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
    if function.modifiers().is_async
        && function.declaration_context() != DeclarationContext::InterfaceMember
    {
        signature.push_literal("async ");
    }
    if let Some(return_type) = function.return_type() {
        signature.push_type(return_type);
        signature.push_literal(" ");
    }

    signature.push_literal(function.name());
    append_type_parameters(&mut signature, function.type_params());
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    let has_where_constraints = append_where_constraints(
        &mut signature,
        lang,
        function.type_params(),
        function.where_constraints(),
    );

    finish(&mut block, signature, function, has_where_constraints)?;
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
    }
    signature.push_literal(">");
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &CSharp,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    for annotation in function.annotation_specs() {
        block.add_code(annotation.emit_with_syntax("[", "]")?);
        block.add_line();
    }
    for annotation in function.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &CSharp, parameter: &ParameterSpec) {
    if !parameter.param_type().is_empty() {
        block.add("%T ", parameter.param_type().clone());
    }
    let property = if parameter.is_property() {
        "readonly "
    } else {
        ""
    };
    block.add(
        "%L",
        format!("{property}{}", lang.escape_reserved(parameter.name())),
    );
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

fn append_where_constraints(
    signature: &mut SignatureBuilder,
    lang: &CSharp,
    type_params: &[crate::spec::where_spec::TypeParamSpec],
    constraints: &[crate::spec::where_spec::WhereConstraint],
) -> bool {
    let mut emitted = false;
    for type_param in type_params {
        let bounds = csharp_constraints::merged_constraint_bounds(type_param, constraints);
        if bounds.is_empty() {
            continue;
        }
        emitted = true;
        signature.push_literal("\n");
        signature.push_literal(lang.indent_unit());
        signature.push_literal("where ");
        signature.push_literal(type_param.name());
        signature.push_literal(" : ");
        for (index, bound) in bounds.into_iter().enumerate() {
            if index > 0 {
                signature.push_literal(", ");
            }
            signature.push_type(&bound);
        }
    }
    emitted
}

fn finish(
    block: &mut CodeBlockBuilder,
    mut signature: SignatureBuilder,
    function: ValidatedFunction<'_>,
    has_where_constraints: bool,
) -> Result<(), SigilStitchError> {
    let Some(body) = function.body() else {
        signature.push_literal(";");
        signature.append_to(block);
        block.add_line();
        return Ok(());
    };

    if has_where_constraints {
        signature.push_literal("\n{");
    } else {
        signature.push_literal(" {");
    }
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
