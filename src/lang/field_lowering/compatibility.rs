//! Frozen pre-0.6.8 field lowering for permissive external adapters.

#![allow(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::config::OptionalFieldStyle;
use crate::spec::field_spec::{FieldSpec, ValidatedFields};
use crate::spec::modifiers::DeclarationContext;

pub(crate) fn lower<L: CodeLang + ?Sized>(
    lang: &L,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let declaration_context = match fields.context() {
        crate::lang::capability::FieldContext::Direct(context) => context,
        crate::lang::capability::FieldContext::TypeMember(kind)
        | crate::lang::capability::FieldContext::VariantRecordPayload(kind) => {
            lang.type_member_declaration_context(kind)
        }
    };
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        lower_one(&mut block, lang, field, declaration_context)?;
    }
    block.build()
}

pub(crate) fn lower_one<L: CodeLang + ?Sized>(
    block: &mut crate::code_block::CodeBlockBuilder,
    lang: &L,
    field: &FieldSpec,
    context: DeclarationContext,
) -> Result<(), SigilStitchError> {
    let emit_doc = || -> Option<String> {
        if field.doc.is_empty() {
            return None;
        }
        let lines: Vec<&str> = field.doc.iter().map(String::as_str).collect();
        Some(lang.render_doc_comment(&lines))
    };

    if lang.doc_before_annotations()
        && let Some(doc) = emit_doc()
    {
        block.add("%L", doc);
        block.add_line();
    }
    for annotation in &field.annotation_specs {
        block.add_code(annotation.emit_with(lang)?);
        block.add_line();
    }
    for annotation in &field.annotations {
        block.add_code(annotation.clone());
        block.add_line();
    }
    if !lang.doc_before_annotations()
        && let Some(doc) = emit_doc()
    {
        block.add("%L", doc);
        block.add_line();
    }

    let visibility = lang.render_visibility(field.modifiers.visibility, context);
    let terminator = lang.block_syntax().field_terminator;
    let mut format = visibility.to_string();
    let mut arguments = Vec::new();
    if field.modifiers.is_static {
        format.push_str(lang.function_syntax().static_keyword);
    }

    let optional_style = if field.is_optional {
        lang.optional_field_style()
    } else {
        OptionalFieldStyle::Ignored
    };
    let type_before_name = lang.type_decl_syntax().type_before_name;
    let name_suffix = match optional_style {
        OptionalFieldStyle::NameSuffix(suffix) => suffix,
        _ => "",
    };
    let name_prefix = match optional_style {
        OptionalFieldStyle::TypePrefix(prefix) if type_before_name => prefix,
        _ => "",
    };
    let (type_prefix, type_suffix) = match optional_style {
        OptionalFieldStyle::TypeSuffix(suffix) => (String::new(), suffix.to_string()),
        OptionalFieldStyle::TypeWrap { open, close } => (open.to_string(), close.to_string()),
        OptionalFieldStyle::TypePrefix(prefix) if !type_before_name => {
            (prefix.to_string(), String::new())
        }
        OptionalFieldStyle::UnionWithNone(separator) => (String::new(), format!("{separator}None")),
        _ => (String::new(), String::new()),
    };

    if type_before_name {
        if field.modifiers.is_readonly {
            format.push_str(lang.enum_and_annotation().readonly_keyword);
        }
        if !field.field_type.is_empty() {
            format.push_str(&type_prefix);
            format.push_str("%T");
            format.push_str(&type_suffix);
            arguments.push(Arg::TypeName(field.field_type.clone()));
            format.push(' ');
        }
        format.push_str(name_prefix);
        format.push_str(lang.variable_prefix());
        format.push_str(&lang.escape_field_name(&field.name));
        format.push_str(name_suffix);
    } else {
        if field.modifiers.is_readonly {
            format.push_str(lang.enum_and_annotation().readonly_keyword);
        } else {
            format.push_str(lang.enum_and_annotation().mutable_field_keyword);
        }
        format.push_str(lang.variable_prefix());
        format.push_str(&lang.escape_field_name(&field.name));
        format.push_str(name_suffix);
        if !field.field_type.is_empty() {
            format.push_str(lang.type_decl_syntax().type_annotation_separator);
            format.push_str(&type_prefix);
            format.push_str("%T");
            format.push_str(&type_suffix);
            arguments.push(Arg::TypeName(field.field_type.clone()));
        }
    }

    if let Some(initializer) = &field.initializer {
        format.push_str(" = %L");
        arguments.push(Arg::Code(initializer.clone()));
    }
    if let Some(tag) = &field.tag {
        format.push_str(" `");
        format.push_str(tag);
        format.push('`');
    }
    format.push_str(terminator);
    block.add(&format, arguments);
    block.add_line();
    Ok(())
}
