#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{concat, join_soft, literal, name, qualified, terminal};

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

fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    join_soft(items, separator.trim_start())
}

fn prefix(prefix: &str, inner: CodeBlock) -> CodeBlock {
    concat([literal(prefix), inner])
}

fn union_member(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    match type_name {
        TypeName::Optional(inner) if matches!(inner.as_ref(), TypeName::Optional(_)) => {
            union_member(inner)
        }
        TypeName::Optional(inner) => Ok(infix(vec![union_member(inner)?, literal("null")], " | ")),
        TypeName::Intersection(_) => Ok(concat([literal("("), lower(type_name)?, literal(")")])),
        _ => lower(type_name),
    }
}

fn intersection_member(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(type_name, TypeName::Union(_) | TypeName::Optional(_)) {
        return Err(unsupported(
            "PHP intersection types cannot contain a union member",
        ));
    }
    lower(type_name)
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "php".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some("\\")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(_) | TypeName::ReadonlyArray(_) => Err(unsupported(
            "PHP array element types require a documentation-type context",
        ))?,
        TypeName::Generic { .. } => Err(unsupported(
            "PHP has no native generic type application syntax",
        ))?,
        TypeName::Union(members) => infix(
            members.iter().map(union_member).collect::<Result<_, _>>()?,
            " | ",
        ),
        TypeName::Intersection(members) => infix(
            members
                .iter()
                .map(intersection_member)
                .collect::<Result<_, _>>()?,
            " & ",
        ),
        TypeName::Pointer(_) => Err(unsupported("PHP has no pointer type expression"))?,
        TypeName::Slice(_) => Err(unsupported("PHP has no slice type expression"))?,
        TypeName::Map { .. } => Err(unsupported(
            "PHP map key and value types require a documentation-type context",
        ))?,
        TypeName::Optional(inner) if matches!(inner.as_ref(), TypeName::Optional(_)) => {
            lower(inner)?
        }
        TypeName::Optional(inner)
            if matches!(
                inner.as_ref(),
                TypeName::Union(_) | TypeName::Intersection(_)
            ) =>
        {
            union_member(type_name)?
        }
        TypeName::Optional(inner) => prefix("?", lower(inner)?),
        TypeName::Tuple(_) => Err(unsupported("PHP has no tuple type expression"))?,
        TypeName::Reference { .. } => Err(unsupported(
            "PHP references are declaration syntax, not type expressions",
        ))?,
        TypeName::Function { .. } => Err(unsupported(
            "PHP has no native callable signature type expression",
        ))?,
        TypeName::AssociatedType { .. } => {
            Err(unsupported("PHP has no associated-type expression"))?
        }
        TypeName::ImplTrait { .. } => Err(unsupported("PHP has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("PHP has no dynamic-trait type expression"))?,
        TypeName::Wildcard { .. } => Err(unsupported("PHP has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("PHP has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("php", "PHP has no string singleton type expression");
