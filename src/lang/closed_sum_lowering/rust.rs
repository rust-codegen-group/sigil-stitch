//! Rust-owned ClosedSum declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::rust::Rust;
use crate::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use crate::spec::closed_sum_spec::{ClosedSumIntent, ValidatedClosedSumCase};
use crate::spec::modifiers::{DeclarationContext, Visibility};

use super::{emit_annotations, emit_case_annotations, emit_case_doc, emit_doc};

pub(crate) fn validate(lang: &Rust, sum: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
    for parameter in sum.generic_params() {
        if !parameter.has_kind_or_pack_domain() {
            continue;
        }
        if !matches!(
            parameter.domain().as_ref(),
            crate::spec::where_spec::GenericParamDomain::Single {
                kind: None | Some(crate::spec::where_spec::KindExpr::Type)
            } | crate::spec::where_spec::GenericParamDomain::Lifetime
        ) {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().into(),
                parameter_name: parameter.name().into(),
                reason: "this binding domain has no representation in this declaration".into(),
            });
        }
    }
    if !matches!(
        sum.visibility(),
        Visibility::Inherited
            | Visibility::Public
            | Visibility::Private
            | Visibility::PublicCrate
            | Visibility::PublicSuper
    ) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: format!(
                "Rust does not support {:?} visibility for a top-level closed sum",
                sum.visibility()
            ),
        });
    }
    if !crate::lang::type_lowering::common::is_identifier(sum.name())
        || matches!(sum.name(), "self" | "Self" | "super" | "crate")
        || lang.reserved_words().contains(&sum.name())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Rust closed-sum names require a non-keyword identifier".to_string(),
        });
    }
    for parameter in sum.type_params() {
        if parameter.is_lifetime() {
            if !crate::lang::rust::is_valid_lifetime_parameter_name(parameter.name()) {
                return Err(SigilStitchError::InvalidTypeParameter {
                    type_name: sum.name().to_string(),
                    parameter_name: parameter.name().to_string(),
                    reason: "Rust lifetime parameters require a valid non-keyword declared name"
                        .to_string(),
                });
            }
            if parameter.bounds().iter().any(|bound| {
                !crate::lang::rust::is_valid_generic_lifetime_bound(bound, &sum.type_params())
            }) {
                return Err(SigilStitchError::InvalidTypeParameter {
                    type_name: sum.name().to_string(),
                    parameter_name: parameter.name().to_string(),
                    reason:
                        "Rust lifetime parameters accept only declared lifetime or 'static bounds"
                            .to_string(),
                });
            }
        } else if !crate::lang::type_lowering::common::is_identifier(parameter.name())
            || parameter.name().starts_with('\'')
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Rust type parameters require an ordinary non-keyword identifier"
                    .to_string(),
            });
        }
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Rust type declarations do not support Scala-style context bounds"
                    .to_string(),
            });
        }
    }
    for constraint in sum.where_constraints() {
        let subject =
            match crate::lang::rust::lifetime_constraint_subject_name(constraint.subject()) {
                Ok(Some(subject)) => subject,
                Ok(None) => continue,
                Err(()) => {
                    return Err(SigilStitchError::InvalidTypeParameter {
                        type_name: sum.name().to_string(),
                        parameter_name: format!("{:?}", constraint.subject()),
                        reason:
                            "Rust lifetime constraints must use an unparameterized lifetime subject"
                                .to_string(),
                    });
                }
            };
        if !sum.type_params().iter().any(|parameter| {
            parameter.is_lifetime()
                && parameter.name() == subject
                && crate::lang::rust::is_valid_lifetime_parameter_name(parameter.name())
        }) {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: subject.to_string(),
                reason: "Rust lifetime constraints must target a declared lifetime".to_string(),
            });
        }
        if constraint.bounds().iter().any(|bound| {
            !crate::lang::rust::is_valid_generic_lifetime_bound(bound, &sum.type_params())
        }) {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: subject.to_string(),
                reason: "Rust lifetime constraints accept only declared lifetime or 'static bounds"
                    .to_string(),
            });
        }
    }
    for parameter in sum.type_params() {
        if !sum.cases().iter().any(|case| {
            case.positional_payload()
                .iter()
                .chain(case.record_payload().iter().map(|field| field.field_type()))
                .any(|payload| contains_parameter(payload, parameter.name()))
        }) {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Rust closed-sum type parameters must occur in at least one case payload"
                    .to_string(),
            });
        }
    }
    for case in sum.cases() {
        if !crate::lang::type_lowering::common::is_identifier(case.name())
            || matches!(case.name(), "self" | "Self" | "super" | "crate")
            || lang.reserved_words().contains(&case.name())
        {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "case name is not a valid non-keyword identifier".to_string(),
            });
        }
    }
    Ok(())
}

#[expect(
    deprecated,
    reason = "walk released compatibility variants alongside modern type expressions"
)]
fn contains_parameter(ty: &crate::type_name::TypeName, name: &str) -> bool {
    use crate::type_name::TypeName;
    match ty {
        TypeName::Primitive(value) | TypeName::Raw(value) => value == name,
        TypeName::Array(inner)
        | TypeName::ReadonlyArray(inner)
        | TypeName::Pointer(inner)
        | TypeName::Slice(inner)
        | TypeName::Optional(inner) => contains_parameter(inner, name),
        TypeName::Reference {
            inner, lifetime, ..
        } => lifetime.as_deref() == Some(name) || contains_parameter(inner, name),
        TypeName::Generic { base, params } => {
            contains_parameter(base, name)
                || params.iter().any(|param| contains_parameter(param, name))
        }
        TypeName::Application { base, arguments } => {
            contains_parameter(base, name)
                || arguments.iter().any(|argument| match argument {
                    crate::spec::where_spec::TypeArgument::Single(value)
                    | crate::spec::where_spec::TypeArgument::Expansion { pattern: value } => {
                        contains_parameter(value, name)
                    }
                })
        }
        TypeName::Parameter(value) => value == name,
        TypeName::Union(values)
        | TypeName::Intersection(values)
        | TypeName::Tuple(values)
        | TypeName::ImplTrait { bounds: values }
        | TypeName::DynTrait { bounds: values } => {
            values.iter().any(|value| contains_parameter(value, name))
        }
        TypeName::Map { key, value } => {
            contains_parameter(key, name) || contains_parameter(value, name)
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            params.iter().any(|param| contains_parameter(param, name))
                || contains_parameter(return_type, name)
        }
        TypeName::Callable {
            parameters,
            returns,
        } => {
            parameters.iter().any(|parameter| match parameter {
                crate::spec::where_spec::CallableParam::Single { type_name, .. } => {
                    contains_parameter(type_name, name)
                }
                crate::spec::where_spec::CallableParam::Repeated { element_type, .. } => {
                    contains_parameter(element_type, name)
                }
                crate::spec::where_spec::CallableParam::Expansion { pattern, .. } => {
                    contains_parameter(pattern, name)
                }
            }) || returns
                .iter()
                .any(|return_type| contains_parameter(return_type, name))
        }
        TypeName::AssociatedType {
            base, qualifier, ..
        } => {
            contains_parameter(base, name)
                || qualifier
                    .as_deref()
                    .is_some_and(|value| contains_parameter(value, name))
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            upper_bound
                .as_deref()
                .is_some_and(|value| contains_parameter(value, name))
                || lower_bound
                    .as_deref()
                    .is_some_and(|value| contains_parameter(value, name))
        }
        TypeName::Importable { .. } | TypeName::StringLiteral(_) => false,
    }
}

pub(crate) fn lower(
    lang: &Rust,
    sum: ValidatedClosedSum<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_doc(&mut block, lang, sum.intent());
    emit_annotations(&mut block, sum.intent(), "#[", "]")?;
    let mut args = Vec::new();
    let params = type_parameters(&sum, &mut args);
    block.add(
        &format!(
            "{}enum {}{params}",
            lang.render_visibility(sum.visibility(), DeclarationContext::TopLevel),
            lang.escape_reserved(sum.name())
        ),
        args,
    );
    if !sum.where_constraints().is_empty() {
        block.add(" where ", ());
        for (index, constraint) in sum.where_constraints().iter().enumerate() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T: ", constraint.subject().clone());
            for (bound_index, bound) in constraint.bounds().iter().enumerate() {
                if bound_index > 0 {
                    block.add(" + ", ());
                }
                block.add("%T", bound.clone());
            }
        }
    }
    block.add(" {", ());
    block.add_line();
    block.add("%>", ());
    let cases: Vec<_> = sum.cases().collect();
    for case in cases {
        emit_case(&mut block, lang, &case)?;
    }
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn type_parameters(sum: &ValidatedClosedSum<'_>, args: &mut Vec<Arg>) -> String {
    if sum.type_params().is_empty() {
        return String::new();
    }
    let mut text = String::from("<");
    let mut first = true;
    for parameter in sum
        .type_params()
        .iter()
        .filter(|p| p.is_lifetime())
        .chain(sum.type_params().iter().filter(|p| !p.is_lifetime()))
    {
        if !first {
            text.push_str(", ");
        }
        first = false;
        text.push_str(parameter.name());
        if !parameter.bounds().is_empty() {
            text.push_str(": ");
            for (index, bound) in parameter.bounds().iter().enumerate() {
                if index > 0 {
                    text.push_str(" + ");
                }
                text.push_str("%T");
                args.push(Arg::TypeName(bound.clone()));
            }
        }
    }
    text.push('>');
    text
}

fn emit_case(
    block: &mut crate::code_block::CodeBlockBuilder,
    lang: &Rust,
    case: &ValidatedClosedSumCase<'_>,
) -> Result<(), SigilStitchError> {
    emit_case_doc(block, lang, case.intent());
    emit_case_annotations(block, case.intent(), "#[", "]")?;
    block.add("%L", case.name());
    if !case.positional_payload().is_empty() {
        block.add("(", ());
        for (index, payload) in case.positional_payload().iter().enumerate() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T", payload.clone());
        }
        block.add(")", ());
    } else if let Some(fields) = case.record_payload() {
        block.add(" {", ());
        block.add_line();
        block.add("%>", ());
        block.add_code(lang.lower_fields(fields.clone())?);
        block.add("%<}", ());
    }
    block.add(",", ());
    block.add_line();
    Ok(())
}

#[cfg(test)]
mod occurrence_tests {
    use super::*;
    use crate::spec::where_spec::{CallableParam, CallableParamPresence, TypeArgument};
    use crate::type_name::TypeName;

    #[test]
    fn modern_occurrences_include_all_callable_segments_and_application_patterns() {
        for parameter in [
            CallableParam::Single {
                name: None,
                type_name: TypeName::parameter("T"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Repeated {
                name: None,
                element_type: TypeName::parameter("T"),
            },
            CallableParam::Expansion {
                name: None,
                pattern: TypeName::parameter("T"),
            },
        ] {
            let ty = TypeName::application(
                TypeName::parameter("F"),
                vec![TypeArgument::Single(TypeName::callable(
                    vec![parameter],
                    vec![TypeName::parameter("R")],
                ))],
            );
            for name in ["T", "F", "R"] {
                assert!(contains_parameter(&ty, name));
            }
            assert!(!contains_parameter(&ty, "Missing"));
        }
        let ty = TypeName::application(
            TypeName::parameter("F"),
            vec![TypeArgument::Expansion {
                pattern: TypeName::parameter("T"),
            }],
        );
        assert!(contains_parameter(&ty, "T"));
    }
}
