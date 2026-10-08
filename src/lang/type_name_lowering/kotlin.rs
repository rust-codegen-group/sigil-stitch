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

fn prefix(prefix: &str, inner: CodeBlock) -> CodeBlock {
    concat([literal(prefix), inner])
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn associated_dot(base: CodeBlock, member: &str) -> CodeBlock {
    concat([base, literal("."), name(member)])
}

fn wildcard(
    upper: Option<CodeBlock>,
    lower: Option<CodeBlock>,
    unbounded: &str,
    upper_keyword: &str,
    lower_keyword: &str,
) -> CodeBlock {
    match (upper, lower) {
        (Some(bound), None) => prefix(upper_keyword, bound),
        (None, Some(bound)) => prefix(lower_keyword, bound),
        (None, None) => literal(unbounded),
        (Some(_), Some(_)) => unreachable!("intrinsic validation rejects dual bounds"),
    }
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "kt".to_string(),
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
        TypeName::Array(inner) => generic_wrap("Array", vec![lower(inner)?], "<", ">"),
        TypeName::ReadonlyArray(inner) => generic_wrap("List", vec![lower(inner)?], "<", ">"),
        TypeName::Generic { base, .. } | TypeName::Application { base, .. } => generic_delimited(
            lower(base)?,
            lower_application_arguments(type_name)?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("Kotlin has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported(
            "Kotlin has no general intersection type expression",
        ))?,
        TypeName::Pointer(_) => Err(unsupported("Kotlin has no pointer type expression"))?,
        TypeName::Slice(_) => Err(unsupported("Kotlin has no slice type expression"))?,
        TypeName::Map { key, value } => {
            generic_wrap("Map", vec![lower(key)?, lower(value)?], "<", ">")
        }
        TypeName::Optional(inner)
            if matches!(
                inner.as_ref(),
                TypeName::Function { .. } | TypeName::Callable { .. }
            ) =>
        {
            postfix(surround("(", lower(inner)?, ")"), "?")
        }
        TypeName::Optional(inner) => postfix(lower(inner)?, "?"),
        TypeName::Tuple(_) => Err(unsupported("Kotlin has no tuple type expression"))?,
        TypeName::Reference { .. } => Err(unsupported("Kotlin has no reference type modifier"))?,
        TypeName::Function { return_type, .. } | TypeName::Callable { return_type, .. } => {
            concat([
                delimited("(", lower_callable_parameters(type_name)?, ", ", ")"),
                literal(" -> "),
                lower(return_type)?,
            ])
        }
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => associated_dot(lower(base)?, member),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported(
            "Kotlin nested types do not use a trait qualifier",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported("Kotlin has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => {
            Err(unsupported("Kotlin has no dynamic-trait type expression"))?
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => wildcard(
            upper_bound.as_deref().map(lower).transpose()?,
            lower_bound.as_deref().map(lower).transpose()?,
            "*",
            "out ",
            "in ",
        ),
        TypeName::StringLiteral(_) => Err(unsupported(
            "Kotlin has no string singleton type expression",
        ))?,
        TypeName::Parameter(_) => Err(unsupported(
            "Kotlin modern type expressions are not supported in this position",
        ))?,
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("kt", "Kotlin has no string singleton type expression");

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
