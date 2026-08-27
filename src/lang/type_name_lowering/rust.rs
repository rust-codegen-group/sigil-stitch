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

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn reference_with_lifetime(inner: CodeBlock, mutable: bool, lifetime: &str) -> CodeBlock {
    prefix(
        &format!("&{lifetime} {}", if mutable { "mut " } else { "" }),
        inner,
    )
}

fn associated_qualified(base: CodeBlock, qualifier: Option<CodeBlock>, member: &str) -> CodeBlock {
    match qualifier {
        Some(qualifier) => concat([
            literal("<"),
            base,
            literal(" as "),
            qualifier,
            literal(">::"),
            name(member),
        ]),
        None => concat([base, literal("::"), name(member)]),
    }
}

fn bounds(keyword: &str, values: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    concat([literal(keyword), join(values, separator)])
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "rs".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn std_collections(name: &str) -> CodeBlock {
    terminal(&TypeName::importable("std::collections", name))
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some("::")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(inner) => generic_wrap("Vec", vec![lower(inner)?], "<", ">"),
        TypeName::ReadonlyArray(_) => {
            Err(unsupported("Rust has no readonly vector type expression"))?
        }
        TypeName::Generic { base, params } => generic_delimited(
            lower(base)?,
            params.iter().map(lower).collect::<Result<_, _>>()?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("Rust has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported(
            "Rust trait bounds are not standalone intersection types",
        ))?,
        TypeName::Pointer(inner) => prefix("*const ", lower(inner)?),
        TypeName::Slice(inner) => delimited("&[", vec![lower(inner)?], "", "]"),
        TypeName::Map { key, value } => concat([
            std_collections("HashMap"),
            delimited_soft("<", vec![lower(key)?, lower(value)?], ",", ">"),
        ]),
        TypeName::Optional(inner) => generic_wrap("Option", vec![lower(inner)?], "<", ">"),
        TypeName::Tuple(elements) if elements.len() == 1 => {
            concat([literal("("), lower(&elements[0])?, literal(",)")])
        }
        TypeName::Tuple(elements) => delimited(
            "(",
            elements.iter().map(lower).collect::<Result<_, _>>()?,
            ", ",
            ")",
        ),
        TypeName::Reference {
            inner,
            mutable,
            lifetime,
        } => match lifetime {
            Some(lifetime) => reference_with_lifetime(lower(inner)?, *mutable, lifetime),
            None if *mutable => prefix("&mut ", lower(inner)?),
            None => prefix("&", lower(inner)?),
        },
        TypeName::Function {
            params,
            return_type,
        } => concat([
            literal("fn"),
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
            qualifier,
            member,
        } => associated_qualified(
            lower(base)?,
            qualifier.as_deref().map(lower).transpose()?,
            member,
        ),
        TypeName::ImplTrait { bounds: values } => bounds(
            "impl ",
            values.iter().map(lower).collect::<Result<_, _>>()?,
            " + ",
        ),
        TypeName::DynTrait { bounds: values } => bounds(
            "dyn ",
            values.iter().map(lower).collect::<Result<_, _>>()?,
            " + ",
        ),
        TypeName::Wildcard {
            upper_bound: None,
            lower_bound: None,
        } => literal("_"),
        TypeName::Wildcard { .. } => Err(unsupported(
            "Rust bounded wildcards do not preserve upper or lower variance",
        ))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("Rust has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("rs", "Rust has no string singleton type expression");
