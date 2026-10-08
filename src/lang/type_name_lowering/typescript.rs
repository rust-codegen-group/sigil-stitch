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

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
fn precedence(type_name: &TypeName) -> Precedence {
    match type_name {
        TypeName::Function { .. } | TypeName::Callable { .. } => Precedence::Function,
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
            | TypeName::Parameter(_)
    )
}

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
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
        TypeName::Generic { params, .. } if params.is_empty() => {
            return Err(unsupported(
                "TypeScript type argument lists must not be empty",
            ));
        }
        TypeName::Application { arguments, .. } if arguments.is_empty() => {
            return Err(unsupported(
                "TypeScript type argument lists must not be empty",
            ));
        }
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
            "TypeScript generic application requires a named generic base",
        ))?,
        TypeName::Application { base, arguments } if supports_generic_application(base) => {
            let mut rendered = Vec::with_capacity(arguments.len());
            for argument in arguments {
                rendered.push(match argument {
                    crate::spec::where_spec::TypeArgument::Single(value) => {
                        lower_at(value, Precedence::Function)?
                    }
                    crate::spec::where_spec::TypeArgument::Expansion { .. } => {
                        return Err(unsupported(
                            "TypeScript has no expansion syntax in a generic argument list",
                        ));
                    }
                });
            }
            generic_delimited(lower_at(base, Precedence::Postfix)?, rendered, "<", ">")
        }
        TypeName::Application { .. } => {
            Err(unsupported("TypeScript application requires a named base"))?
        }
        TypeName::Parameter(name) => literal(name.clone()),
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
        TypeName::Callable {
            parameters,
            return_type,
        } => {
            use crate::spec::where_spec::{CallableParam, CallableParamPresence};
            let mut labels = std::collections::HashSet::new();
            for parameter in parameters {
                let label = match parameter {
                    CallableParam::Single { name, .. }
                    | CallableParam::Repeated { name, .. }
                    | CallableParam::Expansion { name, .. } => name,
                };
                if let Some(label) = label
                    && (!crate::lang::type_lowering::typescript::is_identifier(label)
                        || crate::lang::typescript::TS_RESERVED.contains(&label.as_str())
                        || !labels.insert(label.clone()))
                {
                    return Err(unsupported(
                        "TypeScript callable labels must be distinct identifiers",
                    ));
                }
            }
            let mut optional_seen = false;
            let mut rest_seen = false;
            for (index, parameter) in parameters.iter().enumerate() {
                match parameter {
                    CallableParam::Single { presence, .. } => {
                        if rest_seen
                            || (optional_seen && *presence == CallableParamPresence::Required)
                        {
                            return Err(unsupported(
                                "TypeScript callable parameter order is not representable",
                            ));
                        }
                        optional_seen |= *presence == CallableParamPresence::Optional;
                    }
                    CallableParam::Repeated { .. } | CallableParam::Expansion { .. } => {
                        if rest_seen || index + 1 != parameters.len() {
                            return Err(unsupported(
                                "TypeScript callable rest parameters must be final",
                            ));
                        }
                        rest_seen = true;
                    }
                }
            }
            let params = parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    let mut generated = format!("arg{index}");
                    while labels.contains(&generated) {
                        generated.push('_');
                    }
                    labels.insert(generated.clone());
                    match parameter {
                        crate::spec::where_spec::CallableParam::Single {
                            name: label,
                            type_name,
                            presence,
                        } => {
                            let label = label.clone().unwrap_or_else(|| generated.clone());
                            let optional = matches!(
                                presence,
                                crate::spec::where_spec::CallableParamPresence::Optional
                            );
                            Ok(concat([
                                name(label),
                                literal(if optional { "?: " } else { ": " }),
                                lower_at(type_name, Precedence::Function)?,
                            ]))
                        }
                        crate::spec::where_spec::CallableParam::Repeated {
                            name: label,
                            element_type,
                        } => {
                            let label = label.clone().unwrap_or_else(|| generated.clone());
                            Ok(concat([
                                literal("..."),
                                name(label),
                                literal(": "),
                                lower_at(element_type, Precedence::Postfix)?,
                                literal("[]"),
                            ]))
                        }
                        crate::spec::where_spec::CallableParam::Expansion {
                            name: label,
                            pattern,
                        } => {
                            let label = label.clone().unwrap_or_else(|| generated.clone());
                            Ok(concat([
                                literal("..."),
                                name(label),
                                literal(": "),
                                lower_at(pattern, Precedence::Function)?,
                            ]))
                        }
                    }
                })
                .collect::<Result<Vec<_>, SigilStitchError>>()?;
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
