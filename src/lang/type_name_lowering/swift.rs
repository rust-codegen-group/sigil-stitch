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

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
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

fn bounds(keyword: &str, values: Vec<CodeBlock>) -> CodeBlock {
    concat([literal(keyword), join(values, " & ")])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "swift".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn parenthesized(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    Ok(surround("(", lower(type_name)?, ")"))
}

fn optional_inner(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(
        type_name,
        TypeName::Intersection(_) | TypeName::Function { .. }
    ) {
        parenthesized(type_name)
    } else {
        lower(type_name)
    }
}

fn intersection_member(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(type_name, TypeName::Function { .. }) {
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
        TypeName::Array(inner) => delimited("[", vec![lower(inner)?], "", "]"),
        TypeName::ReadonlyArray(_) => Err(unsupported(
            "Swift has no readonly-array type distinct from Array",
        ))?,
        TypeName::Generic { base, params } => generic_delimited(
            lower(base)?,
            params.iter().map(lower).collect::<Result<_, _>>()?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("Swift has no union type expression"))?,
        TypeName::Intersection(members) => infix(
            members
                .iter()
                .map(intersection_member)
                .collect::<Result<_, _>>()?,
            " & ",
        ),
        TypeName::Pointer(_) => Err(unsupported(
            "Swift pointer types require a selected pointer ownership and mutability",
        ))?,
        TypeName::Slice(_) => Err(unsupported(
            "Swift has no slice type that preserves slice semantics",
        ))?,
        TypeName::Map { key, value } => delimited("[", vec![lower(key)?, lower(value)?], ": ", "]"),
        TypeName::Optional(inner) => postfix(optional_inner(inner)?, "?"),
        TypeName::Tuple(elements) if elements.len() == 1 => {
            Err(unsupported("Swift has no single-element tuple type"))?
        }
        TypeName::Tuple(elements) => delimited(
            "(",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            ", ",
            ")",
        ),
        TypeName::Reference { .. } => Err(unsupported(
            "Swift has no reference modifier that preserves shared, mutable, and lifetime intent",
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
            literal(" -> "),
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
            "Swift member types do not use a separate qualifier",
        ))?,
        TypeName::ImplTrait { bounds: values } => {
            bounds("some ", values.iter().map(lower).collect::<Result<_, _>>()?)
        }
        TypeName::DynTrait { bounds: values } => {
            bounds("any ", values.iter().map(lower).collect::<Result<_, _>>()?)
        }
        TypeName::Wildcard { .. } => Err(unsupported("Swift has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("Swift has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("swift", "Swift has no string singleton type expression");
