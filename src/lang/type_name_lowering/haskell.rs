#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, join, literal, name, qualified, surround, terminal,
};

fn is_compound(type_name: &TypeName) -> bool {
    matches!(
        type_name,
        TypeName::Generic { .. }
            | TypeName::Union(_)
            | TypeName::Intersection(_)
            | TypeName::Function { .. }
            | TypeName::Tuple(_)
            | TypeName::Optional(_)
            | TypeName::Map { .. }
    )
}

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

fn generic_prefix(base: CodeBlock, params: Vec<(CodeBlock, bool)>) -> CodeBlock {
    let mut parts = vec![base];
    for (parameter, is_compound) in params {
        parts.push(literal(" "));
        parts.push(if is_compound {
            surround("(", parameter, ")")
        } else {
            parameter
        });
    }
    concat(parts)
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn curried(mut params: Vec<CodeBlock>, return_type: CodeBlock, arrow: &str) -> CodeBlock {
    params.push(return_type);
    join(params, arrow)
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "hs".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn data_map() -> CodeBlock {
    terminal(&TypeName::importable("Data.Map", "Map"))
}

fn function_parameter(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    let lowered = lower(type_name)?;
    if matches!(type_name, TypeName::Function { .. }) {
        Ok(surround("(", lowered, ")"))
    } else {
        Ok(lowered)
    }
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some(".")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(inner) | TypeName::ReadonlyArray(inner) => {
            delimited("[", vec![lower(inner)?], "", "]")
        }
        TypeName::Generic { base, params } => generic_prefix(
            lower(base)?,
            params
                .iter()
                .map(|parameter| Ok((lower(parameter)?, is_compound(parameter))))
                .collect::<Result<_, SigilStitchError>>()?,
        ),
        TypeName::Union(_) => Err(unsupported("Haskell has no union type expression"))?,
        TypeName::Intersection(_) => {
            Err(unsupported("Haskell has no intersection type expression"))?
        }
        TypeName::Pointer(_) => Err(unsupported(
            "Haskell pointer lowering requires an explicit pointer constructor",
        ))?,
        TypeName::Slice(_) => Err(unsupported("Haskell has no slice type expression"))?,
        TypeName::Map { key, value } => generic_prefix(
            data_map(),
            [key.as_ref(), value.as_ref()]
                .into_iter()
                .map(|parameter| Ok((lower(parameter)?, is_compound(parameter))))
                .collect::<Result<_, SigilStitchError>>()?,
        ),
        TypeName::Optional(inner) => {
            generic_prefix(literal("Maybe"), vec![(lower(inner)?, is_compound(inner))])
        }
        TypeName::Tuple(elements) if elements.len() == 1 => {
            Err(unsupported("Haskell has no single-element tuple syntax"))?
        }
        TypeName::Tuple(elements) => delimited(
            "(",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            ", ",
            ")",
        ),
        TypeName::Reference { .. } => Err(unsupported("Haskell has no reference type expression"))?,
        TypeName::Function {
            params,
            return_type: _,
        } if params.is_empty() => Err(unsupported(
            "Haskell has no nullary function type distinct from its result type",
        ))?,
        TypeName::Function {
            params,
            return_type,
        } => curried(
            params
                .iter()
                .map(function_parameter)
                .collect::<Result<_, _>>()?,
            lower(return_type)?,
            " -> ",
        ),
        TypeName::AssociatedType { .. } => Err(unsupported(
            "Haskell has no associated-type projection expression",
        ))?,
        TypeName::ImplTrait { .. } => {
            Err(unsupported("Haskell has no impl-trait type expression"))?
        }
        TypeName::DynTrait { .. } => {
            Err(unsupported("Haskell has no dynamic-trait type expression"))?
        }
        TypeName::Wildcard { .. } => Err(unsupported("Haskell has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => Err(unsupported(
            "Haskell has no string singleton type expression",
        ))?,
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("hs", "Haskell has no string singleton type expression");
