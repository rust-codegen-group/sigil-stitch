#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, join, literal, name, qualified, surround, terminal,
};

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
fn is_compound(type_name: &TypeName) -> bool {
    matches!(
        type_name,
        TypeName::Generic { .. }
            | TypeName::Application { .. }
            | TypeName::Union(_)
            | TypeName::Intersection(_)
            | TypeName::Function { .. }
            | TypeName::Callable { .. }
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

fn application_base(base: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    let lowered = lower(base)?;
    if is_compound(base) {
        Ok(surround("(", lowered, ")"))
    } else {
        Ok(lowered)
    }
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

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
fn function_parameter(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    let lowered = lower(type_name)?;
    if matches!(
        type_name,
        TypeName::Function { .. } | TypeName::Callable { .. }
    ) {
        Ok(surround("(", lowered, ")"))
    } else {
        Ok(lowered)
    }
}

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some(".")) {
        return Ok(
            if type_name
                .simple_name()
                .is_some_and(crate::lang::haskell::is_symbolic_name)
            {
                surround("(", terminal, ")")
            } else {
                terminal
            },
        );
    }
    Ok(match type_name {
        TypeName::Array(inner) | TypeName::ReadonlyArray(inner) => {
            delimited("[", vec![lower(inner)?], "", "]")
        }
        TypeName::Generic { base, params } => generic_prefix(
            application_base(base)?,
            params
                .iter()
                .map(|parameter| Ok((lower(parameter)?, is_compound(parameter))))
                .collect::<Result<_, SigilStitchError>>()?,
        ),
        TypeName::Application { base, arguments } => {
            let mut params = Vec::with_capacity(arguments.len());
            for argument in arguments {
                match argument {
                    crate::spec::where_spec::TypeArgument::Single(value) => {
                        params.push((lower(value)?, is_compound(value)));
                    }
                    crate::spec::where_spec::TypeArgument::Expansion { .. } => {
                        return Err(unsupported(
                            "Haskell does not support C++-style type argument expansion",
                        ));
                    }
                }
            }
            generic_prefix(application_base(base)?, params)
        }
        TypeName::Parameter(name) => literal(name.clone()),
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
        TypeName::Callable {
            parameters,
            returns,
        } => {
            let mut lowered = Vec::with_capacity(parameters.len());
            for parameter in parameters {
                match parameter {
                    crate::spec::where_spec::CallableParam::Single { name: Some(_), .. }
                    | crate::spec::where_spec::CallableParam::Single {
                        presence: crate::spec::where_spec::CallableParamPresence::Optional,
                        ..
                    }
                    | crate::spec::where_spec::CallableParam::Repeated { .. }
                    | crate::spec::where_spec::CallableParam::Expansion { .. } => {
                        return Err(unsupported(
                            "Haskell callable types cannot preserve labelled or variadic segments",
                        ));
                    }
                    crate::spec::where_spec::CallableParam::Single {
                        name: None,
                        type_name,
                        ..
                    } => lowered.push(function_parameter(type_name)?),
                }
            }
            if lowered.is_empty() {
                return Err(unsupported("Haskell has no nullary callable type"));
            }
            let return_type = match returns.as_slice() {
                [] => literal("()"),
                [return_type] => lower(return_type)?,
                _ => {
                    return Err(unsupported(
                        "Haskell supports at most one callable return slot",
                    ));
                }
            };
            curried(lowered, return_type, " -> ")
        }
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
