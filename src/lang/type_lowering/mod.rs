//! Complete language-owned type-declaration lowering.

mod common;
mod compatibility;

pub(crate) mod c;
pub(crate) mod cpp;
pub(crate) mod csharp;
pub(crate) mod dart;
pub(crate) mod go;
pub(crate) mod haskell;
pub(crate) mod java;
pub(crate) mod javascript;
pub(crate) mod kotlin;
pub(crate) mod ocaml;
pub(crate) mod php;
pub(crate) mod python;
pub(crate) mod ruby;
pub(crate) mod rust;
pub(crate) mod scala;
pub(crate) mod swift;
pub(crate) mod typescript;

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::type_spec::ValidatedType;

pub(crate) fn is_identifier(name: &str) -> bool {
    common::is_identifier(name)
}

pub(crate) fn lower_compatibility<L: CodeLang + ?Sized>(
    lang: &L,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    compatibility::lower(lang, type_)
}
