#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{
    concat, delimited_soft, literal, name, qualified, terminal,
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
        language: "java".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn java_util(name: &str) -> CodeBlock {
    terminal(&TypeName::importable("java.util", name))
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
        TypeName::Array(inner) => concat([
            java_util("List"),
            delimited_soft("<", vec![lower(inner)?], ",", ">"),
        ]),
        TypeName::ReadonlyArray(_) => {
            Err(unsupported("Java has no readonly list type expression"))?
        }
        TypeName::Generic { base, .. } | TypeName::Application { base, .. } => generic_delimited(
            lower(base)?,
            lower_application_arguments(type_name)?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("Java has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported(
            "Java intersections are bounds, not standalone type expressions",
        ))?,
        TypeName::Pointer(_) => Err(unsupported("Java has no pointer type expression"))?,
        TypeName::Slice(_) => Err(unsupported("Java has no slice type expression"))?,
        TypeName::Map { key, value } => concat([
            java_util("Map"),
            delimited_soft("<", vec![lower(key)?, lower(value)?], ",", ">"),
        ]),
        TypeName::Optional(inner) => concat([
            java_util("Optional"),
            delimited_soft("<", vec![lower(inner)?], ",", ">"),
        ]),
        TypeName::Tuple(_) => Err(unsupported("Java has no tuple type expression"))?,
        TypeName::Reference { .. } => Err(unsupported("Java has no reference type modifier"))?,
        TypeName::Function { .. } | TypeName::Callable { .. } => Err(unsupported(
            "Java function types require a selected functional interface",
        ))?,
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => associated_dot(lower(base)?, member),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported(
            "Java nested types do not use a trait qualifier",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported("Java has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("Java has no dynamic-trait type expression"))?,
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => wildcard(
            upper_bound.as_deref().map(lower).transpose()?,
            lower_bound.as_deref().map(lower).transpose()?,
            "?",
            "? extends ",
            "? super ",
        ),
        TypeName::StringLiteral(_) => {
            Err(unsupported("Java has no string singleton type expression"))?
        }
        TypeName::Parameter(_) => Err(unsupported(
            "Java modern type expressions are not supported in this position",
        ))?,
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("java", "Java has no string singleton type expression");

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
