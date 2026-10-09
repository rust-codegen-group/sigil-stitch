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
        TypeName::Parameter(value) => Some(literal(value.clone())),
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

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "go".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some(".")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(inner) => prefix("[]", lower(inner)?),
        TypeName::ReadonlyArray(_) => Err(unsupported("Go has no readonly slice type expression"))?,
        TypeName::Generic { base, .. } | TypeName::Application { base, .. } => generic_delimited(
            lower(base)?,
            lower_application_arguments(type_name)?,
            "[",
            "]",
        ),
        TypeName::Union(_) => Err(unsupported(
            "Go unions are constraint terms, not standalone type expressions",
        ))?,
        TypeName::Intersection(_) => Err(unsupported(
            "Go intersections require a complete interface constraint",
        ))?,
        TypeName::Pointer(inner) | TypeName::Optional(inner) => prefix("*", lower(inner)?),
        TypeName::Slice(inner) => prefix("[]", lower(inner)?),
        TypeName::Map { key, value } => {
            delimited("map[", vec![lower(key)?, lower(value)?], "]", "")
        }
        TypeName::Tuple(_) => Err(unsupported(
            "Go tuples are result-list syntax, not type expressions",
        ))?,
        TypeName::Reference { .. } => Err(unsupported(
            "Go pointers do not preserve reference mutability semantics",
        ))?,
        TypeName::Function { .. } | TypeName::Callable { .. } => concat([
            literal("func"),
            delimited("(", lower_callable_parameters(type_name)?, ", ", ")"),
            lower_callable_returns(type_name)?,
        ]),
        TypeName::AssociatedType { .. } => {
            Err(unsupported("Go has no associated-type expression"))?
        }
        TypeName::ImplTrait { .. } => Err(unsupported("Go has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("Go has no dynamic-trait type expression"))?,
        TypeName::Wildcard {
            upper_bound: None,
            lower_bound: None,
        } => literal("any"),
        TypeName::Wildcard { .. } => {
            Err(unsupported("Go has no bounded wildcard type expression"))?
        }
        TypeName::StringLiteral(_) => {
            Err(unsupported("Go has no string singleton type expression"))?
        }
        TypeName::Parameter(_) => Err(unsupported(
            "Go modern type expressions are not supported in this position",
        ))?,
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("go", "Go has no string singleton type expression");

#[expect(deprecated, reason = "preserve released scalar Function results")]
fn lower_callable_returns(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    match type_name {
        TypeName::Function { return_type, .. } => Ok(concat([literal(" "), lower(return_type)?])),
        TypeName::Callable { returns, .. } => match returns.as_slice() {
            [] => Ok(literal("")),
            [return_type] => Ok(concat([literal(" "), lower(return_type)?])),
            returns => Ok(concat([
                literal(" "),
                delimited_soft(
                    "(",
                    returns.iter().map(lower).collect::<Result<_, _>>()?,
                    ",",
                    ")",
                ),
            ])),
        },
        _ => unreachable!("called only for callable types"),
    }
}

#[expect(
    deprecated,
    reason = "borrow released and modern application inputs without conversion"
)]
fn lower_application_arguments(type_name: &TypeName) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let values = match type_name {
        TypeName::Generic { params, .. } => {
            params.iter().map(lower).collect::<Result<Vec<_>, _>>()?
        }
        TypeName::Application { arguments, .. } => arguments
            .iter()
            .map(|argument| match argument {
                crate::spec::where_spec::TypeArgument::Single(value) => lower(value),
                crate::spec::where_spec::TypeArgument::Expansion { .. } => Err(unsupported(
                    "argument-pack expansion is not supported in this type application",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => unreachable!("application helper receives only application inputs"),
    };
    if values.is_empty() {
        return Err(unsupported("this target has no empty type-argument syntax"));
    }
    Ok(values)
}

#[expect(
    deprecated,
    reason = "borrow released and modern callable inputs without conversion"
)]
fn lower_callable_parameters(type_name: &TypeName) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let values = match type_name {
        TypeName::Function { params, .. } => params.iter().map(lower).collect::<Result<Vec<_>, _>>()?,
        TypeName::Callable { parameters, .. } => parameters.iter().map(|parameter| match parameter {
            crate::spec::where_spec::CallableParam::Single { name: None, type_name, presence: crate::spec::where_spec::CallableParamPresence::Required } => lower(type_name),
            _ => Err(unsupported("this callable type cannot preserve labelled, optional, repeated, or expanded slots")),
        }).collect::<Result<Vec<_>, _>>()?,
        _ => unreachable!("callable helper receives only callable inputs"),
    };
    Ok(values)
}
