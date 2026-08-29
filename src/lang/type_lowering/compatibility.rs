//! Frozen pre-0.6.8 type-declaration lowering for permissive adapters.

#![allow(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantContext};
use crate::spec::modifiers::{DeclarationContext, TypeKind};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::type_spec::ValidatedType;
use crate::spec::where_spec::{
    WhereClauseStyle, WhereConstraint, emit_separate_where_block, emit_where_block,
    render_type_params_for,
};
use crate::type_name::TypeName;

pub(crate) fn lower<L: CodeLang + ?Sized>(
    lang: &L,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.is_closed_sum() {
        return Err(SigilStitchError::UnsupportedTypeCapabilities {
            language: lang.file_extension().to_string(),
            type_name: type_.name().to_string(),
            capabilities: vec![crate::lang::capability::TypeCapability::ClosedSum],
        });
    }
    match type_.kind() {
        TypeKind::TypeAlias => return Ok(vec![lower_alias(lang, &type_)?]),
        TypeKind::Newtype => return Ok(vec![lower_newtype(lang, &type_)?]),
        _ => {}
    }
    if lang.methods_inside_type_body(type_.kind()) {
        Ok(vec![lower_inline(lang, &type_)?])
    } else {
        lower_split(lang, &type_)
    }
}

fn lower_inline<L: CodeLang + ?Sized>(
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, type_)?;
    emit_header(&mut block, lang, type_)?;
    block.add("%>", ());

    let body_prefix = lang.type_body_prefix(type_.name(), type_.kind());
    let has_body_prefix = !body_prefix.is_empty();
    if has_body_prefix {
        block.add("%L", body_prefix);
        block.add_line();
        block.add("%>", ());
    }
    if !type_.doc().is_empty() && lang.doc_comment_inside_body() {
        let lines: Vec<&str> = type_.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    emit_embedded(&mut block, lang, type_);

    let variants_first = super::super::variant_lowering::variants_precede_fields(lang, true);
    if variants_first {
        if let Some(variants) = type_.variants() {
            emit_variants(
                &mut block,
                lang,
                variants,
                type_.fields().is_some()
                    || !type_.properties().is_empty()
                    || !type_.methods().is_empty()
                    || !type_.extra_members().is_empty(),
            )?;
        }
        if let Some(fields) = type_.fields() {
            if type_.variants().is_some() {
                block.add_line();
            }
            block.add_code(lang.lower_fields(fields.clone())?);
        }
    } else {
        if let Some(fields) = type_.fields() {
            block.add_code(lang.lower_fields(fields.clone())?);
        }
        if let Some(variants) = type_.variants() {
            if type_.fields().is_some() {
                block.add_line();
            }
            emit_variants(
                &mut block,
                lang,
                variants,
                !type_.properties().is_empty()
                    || !type_.methods().is_empty()
                    || !type_.extra_members().is_empty(),
            )?;
        }
    }

    let has_body_above = !type_.embedded_types().is_empty()
        || type_.fields().is_some()
        || type_.variants().is_some();
    if !type_.properties().is_empty() {
        if has_body_above {
            block.add_line();
        }
        emit_properties(&mut block, lang, type_)?;
    }
    if (has_body_above || !type_.properties().is_empty()) && !type_.methods().is_empty() {
        block.add_line();
    }
    emit_methods(&mut block, lang, type_)?;
    for member in type_.extra_members() {
        block.add_code(member.clone());
    }
    if has_body_prefix {
        block.add("%<", ());
    }
    let body_suffix = lang.type_body_suffix(type_.name(), type_.kind());
    if !body_suffix.is_empty() {
        block.add("%L", body_suffix);
        block.add_line();
    }
    emit_type_close(&mut block, lang, type_)?;
    block.build()
}

fn lower_split<L: CodeLang + ?Sized>(
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut blocks = Vec::new();
    let mut declaration = CodeBlock::builder();
    emit_preamble(&mut declaration, lang, type_)?;
    emit_header(&mut declaration, lang, type_)?;
    declaration.add("%>", ());

    let body_prefix = lang.type_body_prefix(type_.name(), type_.kind());
    let has_body_prefix = !body_prefix.is_empty();
    if has_body_prefix {
        declaration.add("%L", body_prefix);
        declaration.add_line();
        declaration.add("%>", ());
    }
    emit_embedded(&mut declaration, lang, type_);
    let variants_first = super::super::variant_lowering::variants_precede_fields(lang, false);
    if variants_first {
        if let Some(variants) = type_.variants() {
            emit_variants(
                &mut declaration,
                lang,
                variants,
                type_.fields().is_some() || !type_.extra_members().is_empty(),
            )?;
        }
        if let Some(fields) = type_.fields() {
            if type_.variants().is_some() {
                declaration.add_line();
            }
            declaration.add_code(lang.lower_fields(fields.clone())?);
        }
    } else {
        if let Some(fields) = type_.fields() {
            declaration.add_code(lang.lower_fields(fields.clone())?);
        }
        if let Some(variants) = type_.variants() {
            if type_.fields().is_some() {
                declaration.add_line();
            }
            emit_variants(
                &mut declaration,
                lang,
                variants,
                !type_.extra_members().is_empty(),
            )?;
        }
    }
    for member in type_.extra_members() {
        declaration.add_code(member.clone());
    }
    if has_body_prefix {
        declaration.add("%<", ());
    }
    let body_suffix = lang.type_body_suffix(type_.name(), type_.kind());
    if !body_suffix.is_empty() {
        declaration.add("%L", body_suffix);
        declaration.add_line();
    }
    emit_type_close(&mut declaration, lang, type_)?;
    blocks.push(declaration.build()?);

    if !type_.methods().is_empty() || !type_.properties().is_empty() {
        let mut implementation = CodeBlock::builder();
        let mut format = String::from("impl");
        let mut arguments = Vec::new();
        format.push_str(&render_type_params_for(
            type_.type_params(),
            lang,
            &mut arguments,
        ));
        format.push(' ');
        format.push_str(type_.name());
        if !type_.type_params().is_empty() {
            let syntax = lang.generic_syntax();
            format.push_str(syntax.open);
            for (index, parameter) in type_.type_params().iter().enumerate() {
                if index > 0 {
                    format.push_str(", ");
                }
                format.push_str(parameter.name());
            }
            format.push_str(syntax.close);
        }
        append_where_and_open(
            &mut format,
            &mut arguments,
            lang,
            type_.kind(),
            type_.where_constraints(),
        );
        implementation.add(&format, arguments);
        implementation.add_line();
        implementation.add("%>", ());
        emit_properties(&mut implementation, lang, type_)?;
        if !type_.properties().is_empty() && !type_.methods().is_empty() {
            implementation.add_line();
        }
        emit_methods(&mut implementation, lang, type_)?;
        implementation.add("%<", ());
        let close = lang.block_syntax().block_close;
        if !close.is_empty() {
            implementation.add(close, ());
            implementation.add_line();
        }
        blocks.push(implementation.build()?);
    }
    Ok(blocks)
}

fn emit_embedded<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) {
    for embedded in type_.embedded_types() {
        block.add(
            &format!("%T{}", lang.block_syntax().field_terminator),
            embedded.clone(),
        );
        block.add_line();
    }
}

fn emit_properties<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    for (index, property) in type_.properties().iter().enumerate() {
        if index > 0 {
            block.add_line();
        }
        for property_block in lang.lower_property(property.clone())? {
            block.add_code(property_block);
        }
    }
    Ok(())
}

fn emit_methods<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    for (index, method) in type_.methods().iter().enumerate() {
        if index > 0 {
            block.add_line();
        }
        block.add_code(lang.lower_function(*method)?);
    }
    Ok(())
}

fn lower_alias<L: CodeLang + ?Sized>(
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let mut arguments = Vec::new();
    emit_preamble(&mut block, lang, type_)?;
    let visibility =
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel);
    let keyword = lang.type_keyword(type_.kind());
    let parameters = render_type_params_for(type_.type_params(), lang, &mut arguments);
    let target = type_
        .target_type()
        .cloned()
        .unwrap_or_else(|| TypeName::primitive(""));
    let terminator = if lang.block_syntax().uses_semicolons {
        ";"
    } else {
        ""
    };
    let format = if lang.type_decl_syntax().type_alias_target_first {
        if let TypeName::Function {
            params,
            return_type,
        } = &target
        {
            arguments.push(Arg::TypeName((**return_type).clone()));
            arguments.extend(params.iter().cloned().map(Arg::TypeName));
            let parameter_format = std::iter::repeat_n("%T", params.len())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{keyword} %T (*{}{parameters})({parameter_format}){terminator}",
                type_.name()
            )
        } else {
            arguments.push(Arg::TypeName(target));
            format!("{keyword} %T {}{parameters}{terminator}", type_.name())
        }
    } else {
        arguments.push(Arg::TypeName(target));
        format!(
            "{visibility}{keyword} {}{parameters} = %T{terminator}",
            type_.name()
        )
    };
    block.add(&format, arguments);
    block.add_line();
    block.build()
}

fn lower_newtype<L: CodeLang + ?Sized>(
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, type_)?;
    let visibility =
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel);
    let target = type_
        .target_type()
        .cloned()
        .unwrap_or_else(|| TypeName::primitive(""));
    block.add_code(lang.emit_newtype_decl(
        visibility,
        type_.name(),
        type_.type_params(),
        &target,
    )?);
    block.add_line();
    if let Some(suffix) = lang.emit_type_close_suffix(type_.kind(), type_.implemented_types())? {
        block.add("%>", ());
        block.add("%>", ());
        block.add_code(suffix);
        block.add_line();
        block.add("%<", ());
        block.add("%<", ());
    }
    block.build()
}

fn emit_preamble<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    let emit_doc = || {
        (!type_.doc().is_empty() && !lang.doc_comment_inside_body()).then(|| {
            let lines: Vec<&str> = type_.doc().iter().map(String::as_str).collect();
            lang.render_doc_comment(&lines)
        })
    };
    if lang.doc_before_annotations()
        && let Some(doc) = emit_doc()
    {
        block.add("%L", doc);
        block.add_line();
    }
    for annotation in type_.annotation_specs() {
        block.add_code(annotation.emit_with(lang)?);
        block.add_line();
    }
    for annotation in type_.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    if !lang.doc_before_annotations()
        && let Some(doc) = emit_doc()
    {
        block.add("%L", doc);
        block.add_line();
    }
    Ok(())
}

fn emit_header<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    let mut format = String::new();
    let mut arguments = Vec::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(lang.type_keyword(type_.kind()));
    format.push(' ');
    format.push_str(type_.name());
    format.push_str(&render_type_params_for(
        type_.type_params(),
        lang,
        &mut arguments,
    ));

    let syntax = lang.type_decl_syntax();
    if !type_.primary_constructor_parameters().is_empty() && syntax.supports_primary_constructor {
        format.push_str("(%L)");
        arguments.push(Arg::Code(primary_constructor(
            lang,
            type_.primary_constructor_parameters(),
        )?));
    }
    if !type_.nominal_super_types().is_empty() && !syntax.super_type_keyword.is_empty() {
        format.push_str(syntax.super_type_keyword);
        for (index, super_type) in type_.nominal_super_types().iter().enumerate() {
            if index > 0 {
                format.push_str(
                    syntax
                        .super_type_subsequent_separator
                        .unwrap_or(syntax.super_type_separator),
                );
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(super_type.clone()));
        }
    }
    if !type_.implemented_types().is_empty() && !syntax.implements_keyword.is_empty() {
        format.push_str(syntax.implements_keyword);
        for (index, implemented) in type_.implemented_types().iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(implemented.clone()));
        }
    }
    let suffix = lang.type_kind_suffix(type_.kind());
    if !suffix.is_empty() {
        format.push(' ');
        format.push_str(suffix);
    }
    if !type_.nominal_super_types().is_empty() || !type_.implemented_types().is_empty() {
        format.push_str(lang.block_syntax().bases_close);
    }
    append_where_and_open(
        &mut format,
        &mut arguments,
        lang,
        type_.kind(),
        type_.where_constraints(),
    );
    block.add(&format, arguments);
    block.add_line();
    Ok(())
}

fn append_where_and_open<L: CodeLang + ?Sized>(
    format: &mut String,
    arguments: &mut Vec<Arg>,
    lang: &L,
    kind: TypeKind,
    constraints: &[WhereConstraint],
) {
    if constraints.is_empty() {
        format.push_str(lang.type_header_block_open(kind));
        return;
    }
    match lang.function_syntax().where_clause_style {
        WhereClauseStyle::WhereBlock => {
            emit_where_block(format, arguments, constraints, lang);
            format.push_str("\n{");
        }
        WhereClauseStyle::SeparateWhere => {
            emit_separate_where_block(format, arguments, constraints, lang);
            format.push_str("\n{");
        }
        WhereClauseStyle::Inline => format.push_str(lang.type_header_block_open(kind)),
    }
}

fn primary_constructor<L: CodeLang + ?Sized>(
    lang: &L,
    parameters: &[ParameterSpec],
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    block.add("%>", ());
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            block.add(",%W", ());
        }
        emit_parameter(&mut block, lang, parameter);
    }
    block.add("%<", ());
    block.build()
}

fn emit_parameter<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    parameter: &ParameterSpec,
) {
    let mut format = String::new();
    let mut arguments = Vec::new();
    if lang.type_decl_syntax().type_before_name {
        if !parameter.param_type().is_empty() {
            format.push_str("%T ");
            arguments.push(Arg::TypeName(parameter.param_type().clone()));
        }
        if parameter.is_property() {
            format.push_str(lang.enum_and_annotation().readonly_keyword);
        } else if parameter.is_mutable_property() {
            format.push_str(lang.enum_and_annotation().mutable_field_keyword);
        }
        format.push_str(lang.variable_prefix());
        format.push_str(&lang.escape_reserved(parameter.name()));
    } else {
        if parameter.is_variadic() {
            format.push_str("...");
        }
        if parameter.is_property() {
            format.push_str(lang.enum_and_annotation().readonly_keyword);
        } else if parameter.is_mutable_property() {
            format.push_str(lang.enum_and_annotation().mutable_field_keyword);
        }
        format.push_str(lang.variable_prefix());
        format.push_str(&lang.escape_reserved(parameter.name()));
        if !parameter.param_type().is_empty() {
            format.push_str(lang.type_decl_syntax().type_annotation_separator);
            format.push_str("%T");
            arguments.push(Arg::TypeName(parameter.param_type().clone()));
        }
    }
    if let Some(default) = parameter.default_value() {
        format.push_str(" = %L");
        arguments.push(Arg::Code(default.clone()));
    }
    block.add(&format, arguments);
}

fn emit_variants<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    variants: &ValidatedVariants<'_>,
    has_trailing_members: bool,
) -> Result<(), SigilStitchError> {
    let count = variants.variants().len();
    for (index, variant) in variants.variants().iter().enumerate() {
        super::super::variant_lowering::lower_legacy_into(
            lang,
            variant,
            VariantContext {
                is_first: index == 0,
                is_last: index + 1 == count,
                has_trailing_members,
            },
            block,
        )?;
    }
    Ok(())
}

fn emit_type_close<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    block.add("%<", ());
    let syntax = lang.block_syntax();
    let suffix = lang.emit_type_close_suffix(type_.kind(), type_.implemented_types())?;
    if !syntax.block_close.is_empty() {
        block.add(
            &format!("{}{}", syntax.block_close, syntax.type_close_terminator),
            (),
        );
        if let Some(suffix) = suffix {
            block.add(" ", ());
            block.add_code(suffix);
        }
        block.add_line();
    } else if let Some(suffix) = suffix {
        block.add("%>", ());
        block.add_code(suffix);
        block.add_line();
        block.add("%<", ());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust::Rust;
    use crate::spec::enum_variant_spec::EnumVariantSpec;
    use crate::spec::type_spec::TypeSpec;

    #[test]
    fn compatibility_lowering_defensively_rejects_closed_sum_intent() {
        let type_ = TypeSpec::closed_sum("Outcome")
            .add_variant(EnumVariantSpec::new("Value").unwrap())
            .build()
            .unwrap();
        let lang = Rust::new();
        let validated = type_.validate_complete(&lang).unwrap();
        let error = lower(&lang, validated).unwrap_err();

        assert!(matches!(
            error,
            SigilStitchError::UnsupportedTypeCapabilities { capabilities, .. }
                if capabilities == vec![crate::lang::capability::TypeCapability::ClosedSum]
        ));
    }
}
