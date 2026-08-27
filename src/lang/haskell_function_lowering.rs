//! Haskell-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::type_params_with_inline_constraints;
use crate::lang::haskell::Haskell;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;

pub(crate) fn lower(
    lang: &Haskell,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function);

    if let Some(return_type) = function.return_type() {
        let type_params = type_params_with_inline_constraints(function, lang.file_extension())?;
        block.add("%L :: ", function.name());
        emit_type_context(&mut block, type_params.as_ref());
        for parameter in function.parameters() {
            block.add("%T -> ", parameter.param_type().clone());
        }
        block.add("%T", return_type.clone());
        append_suffixes(&mut block, function);
        block.add_line();
    }

    if let Some(body) = function.body() {
        block.add("%L", function.name());
        for parameter in function.parameters() {
            block.add(" %L", lang.escape_reserved(parameter.name()));
        }
        block.add(" =", ());
        if function.return_type().is_none() {
            append_suffixes(&mut block, function);
        }
        block.add_line();
        block.add("%>", ());
        block.add_code(body.clone());
        if !body.ends_with_newline_or_block_close() {
            block.add_line();
        }
        block.add("%<", ());
    }
    block.build()
}

fn emit_type_context(
    block: &mut CodeBlockBuilder,
    type_params: &[crate::spec::where_spec::TypeParamSpec],
) {
    let constraint_count = type_params
        .iter()
        .map(|parameter| parameter.bounds().len() + parameter.context_bounds().len())
        .sum::<usize>();
    if constraint_count == 0 {
        return;
    }
    if constraint_count > 1 {
        block.add("(", ());
    }
    let mut index = 0;
    for parameter in type_params {
        for bound in parameter.bounds().iter().chain(parameter.context_bounds()) {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T %L", (bound.clone(), parameter.name()));
            index += 1;
        }
    }
    if constraint_count > 1 {
        block.add(")", ());
    }
    block.add(" => ", ());
}

fn append_suffixes(block: &mut CodeBlockBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        block.add(" %L", suffix.as_str());
    }
}

fn emit_preamble(block: &mut CodeBlockBuilder, lang: &Haskell, function: ValidatedFunction<'_>) {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
}
