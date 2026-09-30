//! Complete language-owned lowering for the dedicated ClosedSum declaration.

pub(crate) mod dart;
pub(crate) mod haskell;
pub(crate) mod java;
pub(crate) mod kotlin;
pub(crate) mod ocaml;
pub(crate) mod rust;
pub(crate) mod scala;
pub(crate) mod swift;

use crate::code_block::CodeBlockBuilder;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::closed_sum_spec::{ClosedSumCaseIntent, ClosedSumIntent};

pub(crate) fn emit_doc<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    intent: ClosedSumIntent<'_>,
) {
    if !intent.doc().is_empty() {
        let lines: Vec<&str> = intent.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
}

pub(crate) fn emit_annotations(
    block: &mut CodeBlockBuilder,
    intent: ClosedSumIntent<'_>,
    prefix: &str,
    suffix: &str,
) -> Result<(), SigilStitchError> {
    for annotation in intent.annotation_specs() {
        block.add_code(annotation.emit_with_syntax(prefix, suffix)?);
        block.add_line();
    }
    for annotation in intent.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}

pub(crate) fn emit_case_doc<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    case: ClosedSumCaseIntent<'_>,
) {
    if !case.doc().is_empty() {
        let lines: Vec<&str> = case.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
}

pub(crate) fn emit_case_annotations(
    block: &mut CodeBlockBuilder,
    case: ClosedSumCaseIntent<'_>,
    prefix: &str,
    suffix: &str,
) -> Result<(), SigilStitchError> {
    for annotation in case.annotation_specs() {
        block.add_code(annotation.emit_with_syntax(prefix, suffix)?);
        block.add_line();
    }
    for annotation in case.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}
