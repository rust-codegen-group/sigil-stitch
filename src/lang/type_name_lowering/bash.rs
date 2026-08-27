#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::type_name::TypeName;
use crate::type_name_lowering::structure::terminal;

fn unsupported(reason: &str) -> SigilStitchError {
    SigilStitchError::UnsupportedTypeName {
        language: "sh".to_string(),
        context: "root".to_string(),
        reason: reason.to_string(),
    }
}
pub(crate) fn lower(type_name: &TypeName) -> Result<CodeBlock, SigilStitchError> {
    match type_name {
        TypeName::Importable {
            qualified: true, ..
        } => Err(unsupported(
            "qualified type references have no Bash representation",
        )),
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            Ok(terminal(type_name))
        }
        TypeName::StringLiteral(_) => {
            Err(unsupported("Bash has no string singleton type expression"))
        }
        _ => Err(unsupported(
            "compound type references have no Bash representation",
        )),
    }
}

#[cfg(test)]
assert_string_literal_rejection!("sh", "Bash has no string singleton type expression");
