//! Complete field-sequence lowering.
//!
//! The compatibility module is the sole interpreter of the pre-0.6.8 shared
//! field grammar. Built-in adapters use language-local modules alongside it.

mod compatibility;

pub(crate) mod c;
pub(crate) mod cpp;
pub(crate) mod csharp;
pub(crate) mod dart;
pub(crate) mod go;
pub(crate) mod haskell;
pub(crate) mod java;
pub(crate) mod javascript;
pub(crate) mod kotlin;
pub(crate) mod ocaml;
pub(crate) mod php;
pub(crate) mod python;
pub(crate) mod rust;
pub(crate) mod scala;
pub(crate) mod swift;
pub(crate) mod typescript;

pub(crate) use compatibility::{
    lower as lower_compatibility, lower_one as lower_compatibility_one,
};

use crate::code_block::CodeBlockBuilder;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::field_spec::FieldSequenceIntent;
use crate::spec::field_spec::FieldSpec;

pub(crate) fn validation_result(
    collect: impl FnOnce(&mut Vec<SigilStitchError>),
) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect(&mut errors);
    match errors.into_iter().next() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub(crate) fn emit_doc<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    field: &FieldSpec,
) {
    if field.doc().is_empty() {
        return;
    }
    let lines: Vec<&str> = field.doc().iter().map(String::as_str).collect();
    block.add("%L", lang.render_doc_comment(&lines));
    block.add_line();
}

pub(crate) fn emit_annotations(
    block: &mut CodeBlockBuilder,
    field: &FieldSpec,
    prefix: &str,
    suffix: &str,
) -> Result<(), crate::error::SigilStitchError> {
    for annotation in field.annotation_specs() {
        block.add_code(annotation.emit_with_syntax(prefix, suffix)?);
        block.add_line();
    }
    for annotation in field.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}

pub(crate) fn collect_escaped_name_collisions<L: CodeLang + ?Sized>(
    lang: &L,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<crate::error::SigilStitchError>,
) {
    collect_name_collisions_by(lang, fields, |name| lang.escape_field_name(name), errors);
}

pub(crate) fn collect_name_collisions_by<L, F>(
    lang: &L,
    fields: FieldSequenceIntent<'_>,
    emitted_name: F,
    errors: &mut Vec<crate::error::SigilStitchError>,
) where
    L: CodeLang + ?Sized,
    F: Fn(&str) -> String,
{
    let mut escaped_names = std::collections::HashMap::new();
    for field in fields.fields() {
        let escaped = emitted_name(field.name());
        if let Some(previous) = escaped_names
            .insert(escaped.clone(), field.name())
            .filter(|previous| *previous != field.name())
        {
            errors.push(crate::error::SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: format!(
                    "field name collides with {previous:?} after both escape as {escaped:?}"
                ),
            });
        }
    }
}

pub(crate) fn collect_invalid_identifiers<L: CodeLang + ?Sized>(
    lang: &L,
    fields: FieldSequenceIntent<'_>,
    is_valid: fn(&str) -> bool,
    errors: &mut Vec<crate::error::SigilStitchError>,
) {
    for field in fields.fields() {
        if !is_valid(field.name()) {
            errors.push(crate::error::SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "field name is not a valid identifier in the selected language".to_string(),
            });
        }
    }
}
