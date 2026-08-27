#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, delimited_soft, join, literal, name, qualified, surround, terminal,
};

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

fn generic_delimited(
    base: CodeBlock,
    params: Vec<CodeBlock>,
    open: &str,
    close: &str,
) -> CodeBlock {
    concat([base, delimited_soft(open, params, ",", close)])
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn associated_dot(base: CodeBlock, member: &str) -> CodeBlock {
    concat([base, literal("."), name(member)])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "cs".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn collections_generic(name: &str) -> CodeBlock {
    terminal(&TypeName::importable("System.Collections.Generic", name))
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some(".")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(inner) => concat([
            collections_generic("List"),
            delimited_soft("<", vec![lower(inner)?], ",", ">"),
        ]),
        TypeName::ReadonlyArray(inner) => concat([
            collections_generic("IReadOnlyList"),
            delimited_soft("<", vec![lower(inner)?], ",", ">"),
        ]),
        TypeName::Generic { base, params } => generic_delimited(
            lower(base)?,
            params.iter().map(lower).collect::<Result<_, _>>()?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("C# has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported("C# has no intersection type expression"))?,
        TypeName::Pointer(inner) => postfix(lower(inner)?, "*"),
        TypeName::Slice(_) => Err(unsupported(
            "C# slice lowering requires a selected span type",
        ))?,
        TypeName::Map { .. } => Err(unsupported(
            "C# map lowering requires a selected dictionary type",
        ))?,
        TypeName::Optional(inner) => postfix(lower(inner)?, "?"),
        TypeName::Tuple(elements) if elements.is_empty() => {
            Err(unsupported("C# has no empty tuple type expression"))?
        }
        TypeName::Tuple(elements) if elements.len() == 1 => {
            Err(unsupported("C# has no single-element tuple syntax"))?
        }
        TypeName::Tuple(elements) => delimited(
            "(",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            ", ",
            ")",
        ),
        TypeName::Reference { .. } => Err(unsupported(
            "C# reference modifiers are declaration syntax, not type expressions",
        ))?,
        TypeName::Function { .. } => Err(unsupported(
            "C# function types require a selected delegate type",
        ))?,
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => associated_dot(lower(base)?, member),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported("C# nested types do not use a trait qualifier"))?,
        TypeName::ImplTrait { .. } => Err(unsupported("C# has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("C# has no dynamic-trait type expression"))?,
        TypeName::Wildcard { .. } => Err(unsupported("C# has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("C# has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("cs", "C# has no string singleton type expression");
