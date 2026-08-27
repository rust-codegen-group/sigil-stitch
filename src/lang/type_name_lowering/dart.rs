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

fn generic_wrap(wrapper: &str, params: Vec<CodeBlock>, open: &str, close: &str) -> CodeBlock {
    concat([
        literal(wrapper),
        delimited_soft(
            if open.is_empty() && !wrapper.is_empty() {
                " "
            } else {
                open
            },
            params,
            ",",
            close,
        ),
    ])
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "dart".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some(".")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(inner) => generic_wrap("List", vec![lower(inner)?], "<", ">"),
        TypeName::ReadonlyArray(_) => {
            Err(unsupported("Dart has no readonly list type expression"))?
        }
        TypeName::Generic { base, params } => generic_delimited(
            lower(base)?,
            params.iter().map(lower).collect::<Result<_, _>>()?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("Dart has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported("Dart has no intersection type expression"))?,
        TypeName::Pointer(_) => Err(unsupported(
            "Dart pointer lowering requires an explicit FFI pointer type",
        ))?,
        TypeName::Slice(_) => Err(unsupported("Dart has no slice type expression"))?,
        TypeName::Map { key, value } => {
            generic_wrap("Map", vec![lower(key)?, lower(value)?], "<", ">")
        }
        TypeName::Optional(inner) => postfix(lower(inner)?, "?"),
        TypeName::Tuple(elements) if elements.len() == 1 => {
            concat([literal("("), lower(&elements[0])?, literal(",)")])
        }
        TypeName::Tuple(elements) => delimited(
            "(",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            ", ",
            ")",
        ),
        TypeName::Reference { .. } => Err(unsupported("Dart has no reference type expression"))?,
        TypeName::Function {
            params,
            return_type,
        } => concat([
            lower(return_type)?,
            literal(" Function"),
            delimited(
                "(",
                params.iter().map(lower).collect::<Result<_, _>>()?,
                ", ",
                ")",
            ),
        ]),
        TypeName::AssociatedType { .. } => {
            Err(unsupported("Dart has no associated-type expression"))?
        }
        TypeName::ImplTrait { .. } => Err(unsupported("Dart has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("Dart has no dynamic-trait type expression"))?,
        TypeName::Wildcard { .. } => Err(unsupported("Dart has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("Dart has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("dart", "Dart has no string singleton type expression");
