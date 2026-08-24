//! Kotlin-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::FunctionForm;
use crate::lang::kotlin::Kotlin;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Kotlin, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[
            Visibility::Inherited,
            Visibility::Public,
            Visibility::Private,
        ],
        &[TypeKind::Class],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Kotlin reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        common::is_identifier,
        lang.reserved_words(),
    )?;
    for parameter in type_.type_params() {
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Kotlin type declarations do not support Scala-style context bounds"
                    .to_string(),
            });
        }
    }
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && type_.nominal_super_types().len() > 1
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Kotlin classes may inherit from at most one superclass".to_string(),
        });
    }
    let mut constructor_names = std::collections::HashSet::new();
    for parameter in type_.primary_constructor_parameters() {
        if !common::is_identifier(parameter.name())
            || parameter.name() == "_"
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "Kotlin primary-constructor parameter {:?} must contain only an identifier; use property flags for val or var",
                    parameter.name()
                ),
            });
        }
        if !constructor_names.insert(parameter.name()) {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "duplicate Kotlin primary-constructor parameter {:?}",
                    parameter.name()
                ),
            });
        }
        if parameter.param_type().is_empty() || parameter.is_variadic() {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "invalid Kotlin primary-constructor parameter {:?}",
                    parameter.name()
                ),
            });
        }
        if parameter.is_property() && parameter.is_mutable_property() {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "primary-constructor parameter {:?} cannot be both val and var",
                    parameter.name()
                ),
            });
        }
        if parameter.default_value().is_some_and(CodeBlock::is_empty) {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "Kotlin primary-constructor parameter {:?} has an empty default value",
                    parameter.name()
                ),
            });
        }
    }
    if type_.kind() == TypeKind::Struct {
        if type_.primary_constructor_parameters().is_empty() {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: "Kotlin data classes require at least one primary-constructor property"
                    .to_string(),
            });
        }
        if let Some(parameter) = type_
            .primary_constructor_parameters()
            .iter()
            .find(|parameter| !parameter.is_property() && !parameter.is_mutable_property())
        {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "Kotlin data-class parameter {:?} must declare a val or var property",
                    parameter.name()
                ),
            });
        }
    }
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && !type_.nominal_super_types().is_empty()
        && type_.primary_constructor_parameters().is_empty()
    {
        for constructor in type_
            .methods()
            .iter()
            .filter(|method| method.name() == "constructor")
        {
            if constructor
                .delegation
                .as_ref()
                .is_none_or(CodeBlock::is_empty)
            {
                return Err(SigilStitchError::InvalidTypeDeclaration {
                    type_name: type_.name().to_string(),
                    reason: "Kotlin secondary constructors in a subclass without a primary constructor must delegate to this or super"
                        .to_string(),
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Kotlin,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::TypeAlias {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        let mut arguments = Vec::new();
        let params = common::type_params(lang, &type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        block.add(
            &format!(
                "{}typealias {}{params} = %T",
                lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
                type_.name()
            ),
            arguments,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    if type_.kind() == TypeKind::Newtype {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        let mut arguments = Vec::new();
        let (params, deferred_bounds) = type_parameters(&type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        let mut format = format!(
            "{}value class {}{params}(val value: %T)",
            lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
            type_.name()
        );
        append_where_constraints(&mut format, &mut arguments, &type_, deferred_bounds);
        block.add(&format, arguments);
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, &type_)?;
    let mut format = String::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Class => "class ",
        TypeKind::Struct => "data class ",
        TypeKind::Interface | TypeKind::Trait => "interface ",
        TypeKind::Enum => "enum class ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    let (parameters, deferred_bounds) = type_parameters(&type_, &mut arguments);
    format.push_str(&parameters);
    if !type_.primary_constructor_parameters().is_empty() {
        format.push_str("(%L)");
        arguments.push(Arg::Code(primary_constructor(
            type_.primary_constructor_parameters(),
        )?));
    }
    if !type_.nominal_super_types().is_empty() || !type_.implemented_types().is_empty() {
        format.push_str(" : ");
        let secondary_constructors_own_super_initialization =
            type_.primary_constructor_parameters().is_empty()
                && type_
                    .methods()
                    .iter()
                    .any(|method| method.form() == FunctionForm::Constructor);
        let mut index = 0;
        for base in type_.nominal_super_types() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(base.clone()));
            if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
                && !secondary_constructors_own_super_initialization
            {
                format.push_str("()");
            }
            index += 1;
        }
        for implemented in type_.implemented_types() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(implemented.clone()));
            index += 1;
        }
    }
    append_where_constraints(&mut format, &mut arguments, &type_, deferred_bounds);
    format.push_str(" {");
    block.add(&format, arguments);
    block.add_line();
    block.add("%>", ());
    let variants = common::emit_variants(&mut block, lang, &type_)?;
    let fields = common::emit_fields(&mut block, lang, &type_)?;
    let properties = if (variants || fields) && !type_.properties().is_empty() {
        block.add_line();
        common::emit_properties(&mut block, lang, &type_)?
    } else {
        common::emit_properties(&mut block, lang, &type_)?
    };
    if (variants || fields || properties) && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, &type_)?;
    common::emit_extra_members(&mut block, &type_);
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn type_parameters(
    type_: &ValidatedType<'_>,
    arguments: &mut Vec<Arg>,
) -> (String, Vec<(String, crate::type_name::TypeName)>) {
    if type_.type_params().is_empty() {
        return (String::new(), Vec::new());
    }

    let mut format = String::from("<");
    let mut deferred = Vec::new();
    for (index, parameter) in type_.type_params().iter().enumerate() {
        if index > 0 {
            format.push_str(", ");
        }
        format.push_str(parameter.name());
        if let Some((first, rest)) = parameter.bounds().split_first() {
            format.push_str(" : %T");
            arguments.push(Arg::TypeName(first.clone()));
            deferred.extend(
                rest.iter()
                    .cloned()
                    .map(|bound| (parameter.name().to_string(), bound)),
            );
        }
    }
    format.push('>');
    (format, deferred)
}

fn append_where_constraints(
    format: &mut String,
    arguments: &mut Vec<Arg>,
    type_: &ValidatedType<'_>,
    deferred_bounds: Vec<(String, crate::type_name::TypeName)>,
) {
    let explicit = type_.where_constraints().iter().flat_map(|constraint| {
        let subject = constraint
            .subject()
            .simple_name()
            .expect("validated Kotlin constraints target declared parameters")
            .to_string();
        constraint
            .bounds()
            .iter()
            .cloned()
            .map(move |bound| (subject.clone(), bound))
    });
    let constraints = deferred_bounds
        .into_iter()
        .chain(explicit)
        .collect::<Vec<_>>();
    if constraints.is_empty() {
        return;
    }
    format.push_str(" where ");
    for (index, (subject, bound)) in constraints.into_iter().enumerate() {
        if index > 0 {
            format.push_str(", ");
        }
        format.push_str(&subject);
        format.push_str(" : %T");
        arguments.push(Arg::TypeName(bound));
    }
}

fn preamble(
    block: &mut CodeBlockBuilder,
    lang: &Kotlin,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_doc(block, lang, type_);
    common::emit_structured_annotations(block, type_, "@", "")?;
    common::emit_raw_annotations(block, type_);
    Ok(())
}

fn primary_constructor(parameters: &[ParameterSpec]) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    block.add("%>", ());
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            block.add(",%W", ());
        }
        if parameter.is_mutable_property() {
            block.add("var ", ());
        } else if parameter.is_property() {
            block.add("val ", ());
        }
        block.add(
            &format!("{}: %T", parameter.name()),
            parameter.param_type().clone(),
        );
        if let Some(default) = parameter.default_value() {
            block.add(" = %L", default.clone());
        }
    }
    block.add("%<", ());
    block.build()
}
