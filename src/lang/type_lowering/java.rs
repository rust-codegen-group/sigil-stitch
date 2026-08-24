//! Java-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::java::Java;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

fn is_identifier(name: &str) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};

    fn is_start(ch: char) -> bool {
        use GeneralCategory::*;
        matches!(
            get_general_category(ch),
            UppercaseLetter
                | LowercaseLetter
                | TitlecaseLetter
                | ModifierLetter
                | OtherLetter
                | LetterNumber
                | CurrencySymbol
                | ConnectorPunctuation
        )
    }

    fn is_continue(ch: char) -> bool {
        use GeneralCategory::*;
        is_start(ch)
            || matches!(
                get_general_category(ch),
                NonspacingMark | SpacingMark | DecimalNumber | Format
            )
    }

    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch == '$' || is_start(ch))
        && chars.all(|ch| ch == '$' || is_continue(ch))
}

pub(crate) fn validate(lang: &Java, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[Visibility::Inherited, Visibility::Public],
        &[
            TypeKind::Class,
            TypeKind::Struct,
            TypeKind::Interface,
            TypeKind::Trait,
        ],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Java reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        is_identifier,
        lang.reserved_words(),
    )?;
    for parameter in type_.type_params() {
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Java type declarations do not support context bounds".to_string(),
            });
        }
    }
    if !type_.where_constraints().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Java type constraints must be attached directly to a declared type parameter"
                .to_string(),
        });
    }
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && type_.nominal_super_types().len() > 1
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Java classes may extend at most one superclass".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Java,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "@", "")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Class | TypeKind::Struct => "class ",
        TypeKind::Interface | TypeKind::Trait => "interface ",
        TypeKind::Enum => "enum ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&common::type_params(lang, &type_, &mut arguments));
    if !type_.nominal_super_types().is_empty() {
        format.push_str(" extends ");
        for (index, base) in type_.nominal_super_types().iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(base.clone()));
        }
    }
    if !type_.implemented_types().is_empty() {
        format.push_str(" implements ");
        for (index, implemented) in type_.implemented_types().iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(implemented.clone()));
        }
    }
    format.push_str(" {");
    block.add(&format, arguments);
    block.add_line();
    block.add("%>", ());
    let variants = common::emit_variants(&mut block, lang, &type_)?;
    if variants
        && (type_.fields().is_some()
            || !type_.properties().is_empty()
            || !type_.methods().is_empty()
            || !type_.extra_members().is_empty())
    {
        block.add_line();
    }
    let fields = common::emit_fields(&mut block, lang, &type_)?;
    if (variants || fields) && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, &type_)?;
    common::emit_extra_members(&mut block, &type_);
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}
