#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, delimited_soft, join, join_soft, literal, name, qualified, string_literal, surround,
    terminal,
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

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    join_soft(items, separator.trim_start())
}

fn optional_infix(inner: CodeBlock, separator: &str, absent: &str) -> CodeBlock {
    infix(vec![inner, literal(absent)], separator)
}

fn associated_dot(base: CodeBlock, member: &str) -> CodeBlock {
    concat([base, literal("."), name(member)])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "py".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn typing(name: &str) -> CodeBlock {
    terminal(&TypeName::importable("typing", name))
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
        TypeName::Array(inner) => generic_wrap("list", vec![lower(inner)?], "[", "]"),
        TypeName::ReadonlyArray(_) => Err(unsupported(
            "Python has no builtin readonly-list type expression",
        ))?,
        TypeName::Generic { base, .. } | TypeName::Application { base, .. } => generic_delimited(
            lower(base)?,
            lower_application_arguments(type_name)?,
            "[",
            "]",
        ),
        TypeName::Union(members)
            if members
                .iter()
                .all(|member| matches!(member, TypeName::StringLiteral(_))) =>
        {
            concat([
                typing("Literal"),
                delimited_soft(
                    "[",
                    members
                        .iter()
                        .map(|member| match member {
                            TypeName::StringLiteral(value) => string_literal(value.clone()),
                            _ => unreachable!("guard accepts only direct string literals"),
                        })
                        .collect(),
                    ",",
                    "]",
                ),
            ])
        }
        TypeName::Union(members) => {
            infix(members.iter().map(lower).collect::<Result<_, _>>()?, " | ")
        }
        TypeName::Intersection(_) => {
            Err(unsupported("Python has no intersection type expression"))?
        }
        TypeName::Pointer(_) => Err(unsupported("Python has no pointer type expression"))?,
        TypeName::Slice(_) => Err(unsupported("Python has no slice type expression"))?,
        TypeName::Map { key, value } => {
            delimited("dict[", vec![lower(key)?, lower(value)?], ", ", "]")
        }
        TypeName::Optional(inner) => optional_infix(lower(inner)?, " | ", "None"),
        TypeName::Tuple(elements) if elements.is_empty() => literal("tuple[()]"),
        TypeName::Tuple(elements) => generic_wrap(
            "tuple",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            "[",
            "]",
        ),
        TypeName::Reference { .. } => Err(unsupported("Python has no reference type modifier"))?,
        TypeName::Function { .. } | TypeName::Callable { .. } => concat([
            typing("Callable"),
            delimited("[[", lower_callable_parameters(type_name)?, ", ", "]"),
            literal(", "),
            lower_callable_returns(type_name)?,
            literal("]"),
        ]),
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => associated_dot(lower(base)?, member),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported(
            "Python member types do not use a trait qualifier",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported("Python has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => {
            Err(unsupported("Python has no dynamic-trait type expression"))?
        }
        TypeName::Wildcard { .. } => Err(unsupported("Python has no wildcard type expression"))?,
        TypeName::StringLiteral(value) => concat([
            typing("Literal"),
            delimited_soft("[", vec![string_literal(value.clone())], ",", "]"),
        ]),
        TypeName::Parameter(_) => Err(unsupported(
            "Python modern type expressions are not supported in this position",
        ))?,
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
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

#[expect(deprecated, reason = "preserve released scalar Function results")]
fn lower_callable_returns(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    match type_name {
        TypeName::Function { return_type, .. } => lower(return_type),
        TypeName::Callable { returns, .. } => match returns.as_slice() {
            [] => Ok(literal("None")),
            [return_type] => lower(return_type),
            _ => Err(unsupported(
                "this language supports at most one callable return slot",
            )),
        },
        _ => unreachable!("called only for callable types"),
    }
}
