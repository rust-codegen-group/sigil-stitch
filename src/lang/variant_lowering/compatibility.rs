//! Frozen pre-0.6.8 enum-variant lowering for permissive external adapters.

#![allow(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::enum_variant_spec::{EnumVariantSpec, ValidatedVariants, VariantContext};
use crate::spec::modifiers::DeclarationContext;

pub(crate) fn variants_before_fields<L: CodeLang + ?Sized>(lang: &L) -> bool {
    lang.enum_and_annotation().variants_before_fields
}

pub(crate) fn lower<L: CodeLang + ?Sized>(
    lang: &L,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let count = variants.variants().len();
    for (index, variant) in variants.variants().iter().enumerate() {
        lower_legacy_into(
            lang,
            variant,
            VariantContext {
                is_first: index == 0,
                is_last: index + 1 == count,
                has_trailing_members: variants.has_following_members(),
            },
            &mut block,
        )?;
    }
    block.build()
}

pub(crate) fn lower_legacy_into<L: CodeLang + ?Sized>(
    lang: &L,
    variant: &EnumVariantSpec,
    context: VariantContext,
    block: &mut CodeBlockBuilder,
) -> Result<(), SigilStitchError> {
    let syntax = lang.enum_and_annotation();
    let separator = syntax.variant_separator;

    let emit_doc = || -> Option<String> {
        if variant.doc.is_empty() || lang.doc_comment_inside_body() {
            return None;
        }
        let lines: Vec<&str> = variant.doc.iter().map(String::as_str).collect();
        Some(lang.render_doc_comment(&lines))
    };

    if lang.doc_before_annotations()
        && let Some(doc) = emit_doc()
    {
        block.add("%L", doc);
        block.add_line();
    }
    for annotation in &variant.annotation_specs {
        block.add_code(annotation.emit_with(lang)?);
        block.add_line();
    }
    for annotation in &variant.annotations {
        block.add_code(annotation.clone());
        block.add_line();
    }
    if !lang.doc_before_annotations()
        && let Some(doc) = emit_doc()
    {
        block.add("%L", doc);
        block.add_line();
    }

    let prefix = if context.is_first {
        syntax.variant_prefix_first.unwrap_or(syntax.variant_prefix)
    } else {
        syntax.variant_prefix
    };
    let mut format = format!("{prefix}{}", variant.name);
    let mut arguments = Vec::new();

    if !variant.associated_types.is_empty() {
        format.push('(');
        for (index, payload) in variant.associated_types.iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(payload.clone()));
        }
        format.push(')');
    }

    if !variant.fields.is_empty() {
        format.push_str(" {");
        block.add(&format, arguments);
        block.add_line();
        block.add("%>", ());
        for field in &variant.fields {
            block.add_code(field.emit_with(lang, DeclarationContext::Member)?);
        }
        block.add("%<", ());
        if context.is_last
            && context.has_trailing_members
            && !syntax.variant_section_terminator.is_empty()
        {
            block.add(&format!("}}{}", syntax.variant_section_terminator), ());
        } else if !separator.is_empty() && (!context.is_last || syntax.variant_trailing_separator) {
            block.add(&format!("}}{separator}"), ());
        } else {
            block.add("}", ());
        }
        block.add_line();
        return Ok(());
    }

    if let Some(discriminant) = &variant.discriminant {
        format.push_str(" = %L");
        arguments.push(Arg::Code(discriminant.clone()));
    } else if !variant.constructor_arguments.is_empty() {
        format.push('(');
        for (index, argument) in variant.constructor_arguments.iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%L");
            arguments.push(Arg::Code(argument.clone()));
        }
        format.push(')');
    } else if let Some(value) = &variant.value {
        match syntax.variant_value_format {
            crate::lang::config::VariantValueFormat::Assignment => format.push_str(" = %L"),
            crate::lang::config::VariantValueFormat::ConstructorArg => format.push_str("(%L)"),
        }
        arguments.push(Arg::Code(value.clone()));
    }

    if context.is_last
        && context.has_trailing_members
        && !syntax.variant_section_terminator.is_empty()
    {
        format.push_str(syntax.variant_section_terminator);
    } else if !separator.is_empty() && (!context.is_last || syntax.variant_trailing_separator) {
        format.push_str(separator);
    }
    block.add(&format, arguments);
    block.add_line();
    Ok(())
}
