#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, delimited_soft, join_soft, literal, name, string_literal, surround, terminal,
};

fn terminal_type(type_name: &TypeName) -> Option<CodeBlock> {
    match type_name {
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

fn prefix(prefix: &str, inner: CodeBlock) -> CodeBlock {
    concat([literal(prefix), inner])
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    join_soft(items, separator.trim_start())
}

fn associated_index(base: CodeBlock, member: &str) -> CodeBlock {
    concat([base, literal("["), string_literal(member), literal("]")])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "ts".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Function,
    Union,
    Intersection,
    Literal,
    Postfix,
    Primary,
}

fn precedence(type_name: &TypeName) -> Precedence {
    match type_name {
        TypeName::Function { .. } => Precedence::Function,
        TypeName::Union(_) | TypeName::Optional(_) => Precedence::Union,
        TypeName::Intersection(_) => Precedence::Intersection,
        TypeName::StringLiteral(_) => Precedence::Literal,
        TypeName::Array(_) | TypeName::ReadonlyArray(_) | TypeName::AssociatedType { .. } => {
            Precedence::Postfix
        }
        _ => Precedence::Primary,
    }
}

fn lower_at(
    type_name: &TypeName,
    minimum_precedence: Precedence,
) -> Result<CodeBlock, SigilStitchError> {
    let block = lower_unparenthesized(type_name)?;
    if precedence(type_name) < minimum_precedence {
        Ok(surround("(", block, ")"))
    } else {
        Ok(block)
    }
}

fn supports_generic_application(type_name: &TypeName) -> bool {
    matches!(
        type_name,
        TypeName::Importable { .. }
            | TypeName::Primitive(_)
            | TypeName::Raw(_)
            | TypeName::Generic { .. }
            | TypeName::AssociatedType {
                qualifier: None,
                ..
            }
    )
}

fn lower_unparenthesized(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if matches!(
        type_name,
        TypeName::Importable {
            qualified: true,
            ..
        }
    ) {
        return Err(unsupported(
            "qualified type references have no TypeScript representation",
        ));
    }
    if let Some(terminal) = terminal_type(type_name) {
        return Ok(terminal);
    }

    Ok(match type_name {
        TypeName::Array(inner) => postfix(lower_at(inner, Precedence::Postfix)?, "[]"),
        TypeName::ReadonlyArray(inner) => prefix(
            "readonly ",
            postfix(lower_at(inner, Precedence::Postfix)?, "[]"),
        ),
        TypeName::Generic { base, params } if supports_generic_application(base) => {
            generic_delimited(
                lower_at(base, Precedence::Postfix)?,
                params
                    .iter()
                    .map(|parameter| lower_at(parameter, Precedence::Function))
                    .collect::<Result<_, _>>()?,
                "<",
                ">",
            )
        }
        TypeName::Generic { .. } => Err(unsupported(
            "TypeScript generic application requires a named or projected generic base",
        ))?,
        TypeName::Union(members) => infix(
            members
                .iter()
                .map(|member| lower_at(member, Precedence::Union))
                .collect::<Result<_, _>>()?,
            " | ",
        ),
        TypeName::Intersection(members) => infix(
            members
                .iter()
                .map(|member| lower_at(member, Precedence::Intersection))
                .collect::<Result<_, _>>()?,
            " & ",
        ),
        TypeName::Pointer(_) => Err(unsupported("TypeScript has no pointer type expression"))?,
        TypeName::Slice(_) => Err(unsupported(
            "TypeScript has no slice type distinct from an array",
        ))?,
        TypeName::Map { key, value } => concat([
            literal("Record"),
            delimited_soft(
                "<",
                vec![
                    lower_at(key, Precedence::Function)?,
                    lower_at(value, Precedence::Function)?,
                ],
                ",",
                ">",
            ),
        ]),
        TypeName::Optional(inner) => infix(
            vec![lower_at(inner, Precedence::Union)?, literal("null")],
            " | ",
        ),
        TypeName::Tuple(elements) => delimited_soft(
            "[",
            elements
                .iter()
                .map(|element| lower_at(element, Precedence::Function))
                .collect::<Result<_, _>>()?,
            ",",
            "]",
        ),
        TypeName::Reference { .. } => {
            Err(unsupported("TypeScript has no reference type modifier"))?
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            let params = params
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    Ok(concat([
                        name(format!("arg{index}")),
                        literal(": "),
                        lower_at(parameter, Precedence::Function)?,
                    ]))
                })
                .collect::<Result<_, SigilStitchError>>()?;
            concat([
                delimited_soft("(", params, ",", ")"),
                literal(" => "),
                lower_at(return_type, Precedence::Function)?,
            ])
        }
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => associated_index(lower_at(base, Precedence::Postfix)?, member),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported(
            "TypeScript indexed-access types do not preserve a separate qualifier",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported(
            "TypeScript has no opaque impl-trait type expression",
        ))?,
        TypeName::DynTrait { .. } => Err(unsupported(
            "TypeScript has no dynamic-trait type expression",
        ))?,
        TypeName::Wildcard { .. } => {
            Err(unsupported("TypeScript has no wildcard type expression"))?
        }
        TypeName::StringLiteral(value) => string_literal(value.clone()),
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    lower_at(type_name, Precedence::Function)
}
