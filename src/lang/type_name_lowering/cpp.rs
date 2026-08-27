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

fn prefix(prefix: &str, inner: CodeBlock) -> CodeBlock {
    concat([literal(prefix), inner])
}

fn postfix(inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([inner, literal(suffix)])
}

fn surround_type(prefix_text: &str, inner: CodeBlock, suffix: &str) -> CodeBlock {
    concat([literal(prefix_text), inner, literal(suffix)])
}

fn delimited(open: &str, items: Vec<CodeBlock>, separator: &str, close: &str) -> CodeBlock {
    surround(open, join(items, separator), close)
}

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "cpp".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

fn standard_library_type(header: &str, name: &str) -> CodeBlock {
    terminal(&TypeName::importable(header, &format!("std::{name}")))
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    if let Some(terminal) = terminal_type(type_name, Some("::")) {
        return Ok(terminal);
    }
    Ok(match type_name {
        TypeName::Array(inner) => concat([
            standard_library_type("vector", "vector"),
            delimited_soft("<", vec![lower(inner)?], ",", ">"),
        ]),
        TypeName::ReadonlyArray(inner) => prefix(
            "const ",
            concat([
                standard_library_type("vector", "vector"),
                delimited_soft("<", vec![lower(inner)?], ",", ">"),
            ]),
        ),
        TypeName::Generic { base, params } => generic_delimited(
            lower(base)?,
            params.iter().map(lower).collect::<Result<_, _>>()?,
            "<",
            ">",
        ),
        TypeName::Union(_) => Err(unsupported("C++ has no union type expression"))?,
        TypeName::Intersection(_) => Err(unsupported("C++ has no intersection type expression"))?,
        TypeName::Pointer(inner) => postfix(lower(inner)?, "*"),
        TypeName::Slice(_) => Err(unsupported(
            "C++ slice lowering requires a selected span type",
        ))?,
        TypeName::Map { key, value } => concat([
            standard_library_type("map", "map"),
            delimited_soft("<", vec![lower(key)?, lower(value)?], ",", ">"),
        ]),
        TypeName::Optional(inner) => concat([
            standard_library_type("optional", "optional"),
            delimited_soft("<", vec![lower(inner)?], ",", ">"),
        ]),
        TypeName::Tuple(elements) => concat([
            standard_library_type("tuple", "tuple"),
            delimited_soft(
                "<",
                elements.iter().map(lower).collect::<Result<_, _>>()?,
                ",",
                ">",
            ),
        ]),
        TypeName::Reference {
            inner,
            mutable,
            lifetime,
        } => match lifetime {
            Some(_) => Err(unsupported("C++ references cannot carry lifetimes"))?,
            None if *mutable => postfix(lower(inner)?, "&"),
            None => surround_type("const ", lower(inner)?, "&"),
        },
        TypeName::Function {
            params,
            return_type,
        } => concat([
            standard_library_type("functional", "function"),
            literal("<"),
            lower(return_type)?,
            delimited(
                "(",
                params.iter().map(lower).collect::<Result<_, _>>()?,
                ", ",
                ")",
            ),
            literal(">"),
        ]),
        TypeName::AssociatedType {
            base,
            qualifier: None,
            member,
        } => concat([lower(base)?, literal("::"), name(member)]),
        TypeName::AssociatedType {
            qualifier: Some(_), ..
        } => Err(unsupported(
            "C++ associated types do not use a trait qualifier",
        ))?,
        TypeName::ImplTrait { .. } => Err(unsupported("C++ has no impl-trait type expression"))?,
        TypeName::DynTrait { .. } => Err(unsupported("C++ has no dynamic-trait type expression"))?,
        TypeName::Wildcard { .. } => Err(unsupported("C++ has no wildcard type expression"))?,
        TypeName::StringLiteral(_) => {
            Err(unsupported("C++ has no string singleton type expression"))?
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            unreachable!("terminal variants returned above")
        }
    })
}

#[cfg(test)]
assert_string_literal_rejection!("cpp", "C++ has no string singleton type expression");
