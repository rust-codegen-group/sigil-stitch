#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, delimited_soft, join, join_soft, literal, name, qualified, surround, terminal,
};

fn terminal_type(type_name: &TypeName) -> Option<CodeBlock> {
    match type_name {
        TypeName::Importable {
            module,
            name: imported_name,
            qualified: true,
            ..
        } => Some(qualified(module, ".", imported_name)),
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

fn generic_wrap(wrapper: &str, params: Vec<CodeBlock>) -> CodeBlock {
    concat([literal(wrapper), delimited_soft("[", params, ",", "]")])
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    join_soft(items, separator.trim_start())
}

fn associated_dot(base: CodeBlock, member: &str) -> CodeBlock {
    concat([base, literal("."), name(member)])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "scala".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn parenthesized(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    Ok(surround("(", lower(type_name)?, ")"))
}

fn union_member(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(type_name, TypeName::Function { .. }) {
        parenthesized(type_name)
    } else {
        lower(type_name)
    }
}

fn intersection_member(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(type_name, TypeName::Union(_) | TypeName::Function { .. }) {
        parenthesized(type_name)
    } else {
        lower(type_name)
    }
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name) {
        return Ok(terminal);
    }

    Ok(match type_name {
        TypeName::Array(inner) => generic_wrap("Array", vec![lower(inner)?]),
        TypeName::ReadonlyArray(inner) => generic_wrap("IArray", vec![lower(inner)?]),
        TypeName::Generic { base, params } => generic_delimited(
            lower(base)?,
            params.iter().map(lower).collect::<Result<_, _>>()?,
            "[",
            "]",
        ),
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
        TypeName::Pointer(_) => Err(unsupported("Scala has no pointer type expression"))?,
        TypeName::Slice(_) => Err(unsupported(
            "Scala has no slice type that preserves slice semantics",
        ))?,
        TypeName::Map { key, value } => generic_wrap("Map", vec![lower(key)?, lower(value)?]),
        TypeName::Optional(inner) => generic_wrap("Option", vec![lower(inner)?]),
        TypeName::Tuple(elements) if elements.is_empty() => literal("Unit"),
        TypeName::Tuple(elements) if elements.len() == 1 => {
            generic_wrap("Tuple1", vec![lower(&elements[0])?])
        }
        TypeName::Tuple(elements) => delimited(
            "(",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            ", ",
            ")",
        ),
        TypeName::Reference { .. } => Err(unsupported(
            "Scala has no reference modifier that preserves shared, mutable, and lifetime intent",
        ))?,
        TypeName::Function {
            params,
            return_type,
        } => concat([
            delimited(
                "(",
                params.iter().map(lower).collect::<Result<_, _>>()?,
                ", ",
                ")",
            ),
            literal(" => "),
            lower(return_type)?,
        ]),
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => associated_dot(lower(base)?, member),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported(
            "Scala path-dependent types do not use a separate qualifier",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported("Scala has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => {
            Err(unsupported("Scala has no dynamic-trait type expression"))?
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => match (upper_bound, lower_bound) {
            (Some(bound), None) => concat([literal("? <: "), lower(bound)?]),
            (None, Some(bound)) => concat([literal("? >: "), lower(bound)?]),
            (None, None) => literal("?"),
            (Some(_), Some(_)) => unreachable!("intrinsic validation rejects dual bounds"),
        },
        TypeName::StringLiteral(_) => {
            Err(unsupported("Scala has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("scala", "Scala has no string singleton type expression");
