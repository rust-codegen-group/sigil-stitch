#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::{qualified, terminal};

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "rb".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}

pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    match type_name {
        TypeName::Importable {
            module,
            name: imported_name,
            qualified: true,
            ..
        } => Ok(qualified(module, "::", imported_name)),
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            Ok(terminal(type_name))
        }
        TypeName::StringLiteral(_) => {
            Err(unsupported("Ruby has no string singleton type expression"))
        }
        _ => Err(unsupported(
            "compound type references have no Ruby representation",
        )),
    }
}

#[cfg(test)]
assert_string_literal_rejection!("rb", "Ruby has no string singleton type expression");
