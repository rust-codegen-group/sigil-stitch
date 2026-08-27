#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{concat, literal, name, qualified, terminal};

fn terminal_type(type_name: &TypeName, qualified_separator: Option<&str>) -> Option<CodeBlock> {
    match type_name {
        TypeName::Importable {
            module,
            name: imported_name,
            qualified: true,
            ..
        } => Some(match qualified_separator {
            Some(separator) => qualified(module, separator, imported_name),
            None => name(imported_name.clone()),
        }),
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            Some(terminal(type_name))
        }
        _ => None,
    }
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn surround_type(prefix_text: &str, inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([literal(prefix_text), inner, literal(suffix)])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "c".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}
pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(
        type_name,
        TypeName::Importable {
            qualified: true,
            ..
        }
    ) {
        return Err(unsupported(
            "qualified type references have no C representation",
        ));
    }
    if let Some(terminal) = terminal_type(type_name, None) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(_) => Err(unsupported(
            "array declarators require an owning C declaration",
        ))?,
        TypeName::ReadonlyArray(_) => Err(unsupported(
            "C has no standalone readonly-array type expression",
        ))?,
        TypeName::Generic { .. } => Err(unsupported("C has no generic type application syntax"))?,
        TypeName::Union(_) => Err(unsupported("C has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported("C has no intersection type expression"))?,
        TypeName::Pointer(inner) => postfix(lower(inner)?, "*"),
        TypeName::Slice(_) => Err(unsupported("C has no slice type expression"))?,
        TypeName::Map { .. } => Err(unsupported("C has no map type expression"))?,
        TypeName::Optional(inner) => postfix(lower(inner)?, "*"),
        TypeName::Tuple(_) => Err(unsupported("C has no tuple type expression"))?,
        TypeName::Reference {
            inner,
            mutable,
            lifetime,
        } => match lifetime {
            Some(_) => Err(unsupported("C references cannot carry lifetimes"))?,
            None if *mutable => postfix(lower(inner)?, "*"),
            None => surround_type("const ", lower(inner)?, "*"),
        },
        TypeName::Function { .. } => Err(unsupported(
            "function declarators require an owning C declaration",
        ))?,
        TypeName::AssociatedType { .. } => Err(unsupported("C has no associated-type expression"))?,
        TypeName::ImplTrait { .. } => Err(unsupported("C has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("C has no dynamic-trait type expression"))?,
        TypeName::Wildcard { .. } => Err(unsupported("C has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("C has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("c", "C has no string singleton type expression");
