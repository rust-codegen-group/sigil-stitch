//! OCaml-owned ClosedSum declaration grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::ocaml::OCaml;
use crate::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use crate::spec::closed_sum_spec::ClosedSumIntent;
use crate::spec::modifiers::Visibility;

use super::emit_doc;

pub(crate) fn validate(lang: &OCaml, sum: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
    if sum.visibility() != Visibility::Inherited {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "OCaml type declarations do not support explicit visibility".to_string(),
        });
    }
    if sum.name() == "_"
        || !crate::lang::type_lowering::ocaml::is_lowercase_identifier(sum.name())
        || lang.reserved_words().contains(&sum.name())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "OCaml closed-sum names require a lowercase non-keyword identifier".to_string(),
        });
    }
    let mut parameter_names = std::collections::HashSet::new();
    for parameter in sum.type_params() {
        let name = parameter
            .name()
            .strip_prefix('\'')
            .unwrap_or(parameter.name());
        if parameter.is_lifetime()
            || name == "_"
            || !crate::lang::type_lowering::ocaml::is_lowercase_identifier(name)
            || lang.reserved_words().contains(&name)
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "OCaml type parameters require a lowercase type-variable identifier"
                    .to_string(),
            });
        }
        if !parameter_names.insert(name) {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "OCaml type parameters must have distinct normalized names".to_string(),
            });
        }
    }
    for case in sum.cases() {
        let mut chars = case.name().chars();
        let valid = chars.next().is_some_and(char::is_uppercase)
            && chars.all(|c| c == '_' || c == '\'' || c.is_alphanumeric());
        if !valid || lang.reserved_words().contains(&case.name()) {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "case name is not a valid constructor name".to_string(),
            });
        }
        if !case.annotations().is_empty() || !case.annotation_specs().is_empty() {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "OCaml cases do not support annotations".to_string(),
            });
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &OCaml,
    sum: ValidatedClosedSum<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_doc(&mut block, lang, sum.intent());
    block.add("type ", ());
    match sum.type_params() {
        [] => {}
        [parameter] => {
            block.add("%L ", ocaml_parameter(parameter.name()));
        }
        parameters => {
            block.add("(", ());
            for (index, parameter) in parameters.iter().enumerate() {
                if index > 0 {
                    block.add(", ", ());
                }
                block.add("%L", ocaml_parameter(parameter.name()));
            }
            block.add(") ", ());
        }
    }
    block.add("%L =", sum.name());
    if sum.cases().next().is_none() {
        block.add(" |", ());
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    block.add_line();
    block.add("%>", ());
    for (index, case) in sum.cases().enumerate() {
        if index > 0 {
            block.add("| ", ());
        }
        if !case.doc().is_empty() {
            let lines: Vec<&str> = case.doc().iter().map(String::as_str).collect();
            block.add("%L", lang.render_doc_comment(&lines));
            block.add_line();
        }
        block.add("%L", case.name());
        if !case.positional_payload().is_empty() {
            block.add(" of ", ());
            for (payload_index, payload) in case.positional_payload().iter().enumerate() {
                if payload_index > 0 {
                    block.add(" * ", ());
                }
                if crate::type_name_render::is_compound_type(payload) {
                    block.add("(%T)", payload.clone());
                } else {
                    block.add("%T", payload.clone());
                }
            }
        } else if let Some(fields) = case.record_payload() {
            block.add(" of { ", ());
            block.add_code(lang.lower_fields(fields.clone())?);
            block.add(" }", ());
        }
        block.add_line();
    }
    block.add("%<", ());
    Ok(vec![block.build()?])
}

fn ocaml_parameter(name: &str) -> String {
    if name.starts_with('\'') {
        name.to_string()
    } else {
        format!("'{name}")
    }
}
