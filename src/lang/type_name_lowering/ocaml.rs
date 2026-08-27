#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, delimited_soft, join, join_soft, literal, name, qualified, surround, terminal,
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

fn application_parameter(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    let lowered = lower(type_name)?;
    if matches!(type_name, TypeName::Tuple(_) | TypeName::Function { .. }) {
        Ok(surround("(", lowered, ")"))
    } else {
        Ok(lowered)
    }
}

fn generic_postfix(base: CodeBlock, params: Vec<CodeBlock>) -> CodeBlock {
    if params.len() == 1 {
        concat([params.into_iter().next().unwrap(), literal(" "), base])
    } else {
        concat([delimited_soft("(", params, ",", ")"), literal(" "), base])
    }
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    join_soft(items, separator.trim_start())
}

fn curried(mut params: Vec<CodeBlock>, return_type: CodeBlock, arrow: &str) -> CodeBlock {
    params.push(return_type);
    join(params, arrow)
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "ml".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn postfix_type(type_name: &TypeName, suffix: &str) -> Result<CodeBlock, SigilStitchError> {
    let inner = application_parameter(type_name)?;
    Ok(postfix(inner, suffix))
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
        TypeName::Array(inner) => postfix_type(inner, " array")?,
        TypeName::ReadonlyArray(inner) => postfix_type(inner, " list")?,
        TypeName::Generic { base, params } => generic_postfix(
            lower(base)?,
            params
                .iter()
                .map(application_parameter)
                .collect::<Result<_, _>>()?,
        ),
        TypeName::Union(_) => Err(unsupported("OCaml has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported("OCaml has no intersection type expression"))?,
        TypeName::Pointer(_) => Err(unsupported(
            "OCaml pointer lowering requires an explicit pointer constructor",
        ))?,
        TypeName::Slice(_) => Err(unsupported("OCaml has no slice type expression"))?,
        TypeName::Map { key, value } => {
            delimited("(", vec![lower(key)?, lower(value)?], ", ", ") Hashtbl.t")
        }
        TypeName::Optional(inner) => postfix_type(inner, " option")?,
        TypeName::Tuple(elements) if elements.is_empty() => literal("unit"),
        TypeName::Tuple(elements) if elements.len() == 1 => {
            Err(unsupported("OCaml has no single-element tuple syntax"))?
        }
        TypeName::Tuple(elements) => {
            infix(elements.iter().map(lower).collect::<Result<_, _>>()?, " * ")
        }
        TypeName::Reference {
            inner,
            mutable: true,
            lifetime: None,
        } => postfix_type(inner, " ref")?,
        TypeName::Reference {
            lifetime: Some(_), ..
        } => Err(unsupported("OCaml references cannot carry lifetimes"))?,
        TypeName::Reference { .. } => Err(unsupported(
            "OCaml has no immutable reference type modifier",
        ))?,
        TypeName::Function {
            params,
            return_type: _,
        } if params.is_empty() => Err(unsupported(
            "OCaml has no nullary function type distinct from its result type",
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
            "OCaml has no associated-type projection expression",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported("OCaml has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => {
            Err(unsupported("OCaml has no dynamic-trait type expression"))?
        }
        TypeName::Wildcard { .. } => Err(unsupported("OCaml has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("OCaml has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("ml", "OCaml has no string singleton type expression");
