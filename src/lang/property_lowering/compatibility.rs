//! Frozen pre-0.6.8 computed-property lowering for external adapters.

#![allow(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::PropertyContext;
use crate::lang::{CodeLang, ValidatedProperty};
use crate::spec::modifiers::{DeclarationContext, PropertyStyle};
use crate::spec::property_spec::PropertySpec;

pub(crate) fn lower<L: CodeLang + ?Sized>(
    lang: &L,
    property: ValidatedProperty<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let context = match property.context() {
        PropertyContext::Direct(context) => context,
        PropertyContext::TypeMember(kind) if lang.methods_inside_type_body(kind) => {
            lang.type_member_declaration_context(kind)
        }
        PropertyContext::TypeMember(_) => DeclarationContext::Member,
    };
    match lang.property_style() {
        PropertyStyle::Accessor => lower_accessor(lang, property.property(), context),
        PropertyStyle::Field => lower_field(lang, property.property(), context),
    }
}

fn lower_accessor<L: CodeLang + ?Sized>(
    lang: &L,
    property: &PropertySpec,
    context: DeclarationContext,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut blocks = Vec::new();
    if let Some(body) = property.getter() {
        let mut block = CodeBlock::builder();
        emit_preamble(&mut block, lang, property)?;
        let mut signature = String::new();
        let mut arguments = Vec::new();
        signature.push_str(lang.render_visibility(property.modifiers().visibility, context));
        if property.modifiers().is_static {
            signature.push_str(lang.function_syntax().static_keyword);
        }
        signature.push_str("get ");
        signature.push_str(property.name());
        signature.push_str("()");
        if !property.property_type().is_empty() {
            signature.push_str(lang.function_syntax().return_type_separator);
            signature.push_str("%T");
            arguments.push(Arg::TypeName(property.property_type().clone()));
        }
        signature.push_str(lang.block_syntax().block_open);
        let inside_body_doc = render_inside_body_doc(lang, property);
        emit_body(
            &mut block,
            &signature,
            arguments,
            body,
            inside_body_doc.as_deref(),
            lang,
        )?;
        blocks.push(block.build()?);
    }

    if let Some(setter) = property.setter() {
        let mut block = CodeBlock::builder();
        if property.getter().is_none() {
            emit_preamble(&mut block, lang, property)?;
        }
        let mut signature = String::new();
        let mut arguments = Vec::new();
        signature.push_str(lang.render_visibility(property.modifiers().visibility, context));
        if property.modifiers().is_static {
            signature.push_str(lang.function_syntax().static_keyword);
        }
        signature.push_str("set ");
        signature.push_str(property.name());
        signature.push('(');
        signature.push_str(lang.variable_prefix());
        signature.push_str(&lang.escape_reserved(setter.param_name()));
        if !property.property_type().is_empty() {
            signature.push_str(lang.type_decl_syntax().type_annotation_separator);
            signature.push_str("%T");
            arguments.push(Arg::TypeName(property.property_type().clone()));
        }
        signature.push(')');
        signature.push_str(lang.block_syntax().block_open);
        let inside_body_doc = property
            .getter()
            .is_none()
            .then(|| render_inside_body_doc(lang, property))
            .flatten();
        emit_body(
            &mut block,
            &signature,
            arguments,
            setter.body(),
            inside_body_doc.as_deref(),
            lang,
        )?;
        blocks.push(block.build()?);
    }
    Ok(blocks)
}

fn lower_field<L: CodeLang + ?Sized>(
    lang: &L,
    property: &PropertySpec,
    context: DeclarationContext,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, property)?;
    let mut signature = String::new();
    let mut arguments = Vec::new();
    signature.push_str(lang.render_visibility(property.modifiers().visibility, context));
    if property.modifiers().is_static {
        signature.push_str(lang.function_syntax().static_keyword);
    }
    if property.setter().is_some() {
        signature.push_str(lang.enum_and_annotation().mutable_field_keyword);
    } else {
        signature.push_str(lang.enum_and_annotation().readonly_keyword);
    }
    signature.push_str(lang.variable_prefix());
    signature.push_str(&lang.escape_field_name(property.name()));
    if !property.property_type().is_empty() {
        signature.push_str(lang.type_decl_syntax().type_annotation_separator);
        signature.push_str("%T");
        arguments.push(Arg::TypeName(property.property_type().clone()));
    }
    signature.push_str(lang.block_syntax().block_open);
    block.add(&signature, arguments);
    block.add_line();
    block.add("%>", ());
    if let Some(doc) = render_inside_body_doc(lang, property) {
        block.add("%L", doc);
        block.add_line();
    }

    if let Some(body) = property.getter() {
        let getter = format!(
            "{}{}",
            lang.property_getter_keyword(),
            lang.block_syntax().block_open
        );
        emit_body(&mut block, &getter, Vec::new(), body, None, lang)?;
    }
    if let Some(setter) = property.setter() {
        let setter_signature = format!(
            "set({}){}",
            setter.param_name(),
            lang.block_syntax().block_open
        );
        emit_body(
            &mut block,
            &setter_signature,
            Vec::new(),
            setter.body(),
            None,
            lang,
        )?;
    }
    block.add("%<", ());
    if !lang.block_syntax().block_close.is_empty() {
        block.add(lang.block_syntax().block_close, ());
        block.add_line();
    }
    Ok(vec![block.build()?])
}

fn emit_body<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    signature: &str,
    arguments: Vec<Arg>,
    body: &CodeBlock,
    inside_body_doc: Option<&str>,
    lang: &L,
) -> Result<(), SigilStitchError> {
    block.add(signature, arguments);
    block.add_line();
    block.add("%>", ());
    if let Some(doc) = inside_body_doc {
        block.add("%L", doc);
        block.add_line();
    }
    block.add_code(body.clone());
    block.add_line();
    block.add("%<", ());
    if !lang.block_syntax().block_close.is_empty() {
        block.add(lang.block_syntax().block_close, ());
        block.add_line();
    }
    Ok(())
}

fn render_inside_body_doc<L: CodeLang + ?Sized>(
    lang: &L,
    property: &PropertySpec,
) -> Option<String> {
    if property.doc().is_empty() || !lang.doc_comment_inside_body() {
        return None;
    }
    let lines: Vec<&str> = property.doc().iter().map(String::as_str).collect();
    Some(lang.render_doc_comment(&lines))
}

fn emit_preamble<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    property: &PropertySpec,
) -> Result<(), SigilStitchError> {
    let doc = || -> Option<String> {
        if property.doc().is_empty() || lang.doc_comment_inside_body() {
            return None;
        }
        let lines: Vec<&str> = property.doc().iter().map(String::as_str).collect();
        Some(lang.render_doc_comment(&lines))
    };
    if lang.doc_before_annotations()
        && let Some(doc) = doc()
    {
        block.add("%L", doc);
        block.add_line();
    }
    for annotation in property.annotation_specs() {
        block.add_code(annotation.emit_with(lang)?);
        block.add_line();
    }
    for annotation in property.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    if !lang.doc_before_annotations()
        && let Some(doc) = doc()
    {
        block.add("%L", doc);
        block.add_line();
    }
    Ok(())
}
