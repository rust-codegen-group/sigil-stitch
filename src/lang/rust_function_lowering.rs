//! Rust-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{SignatureBuilder, tupled_parameter_list};
use crate::lang::rust::Rust;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::where_spec::WhereConstraint;

pub(crate) fn lower(
    lang: &Rust,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, function)?;

    let mut signature = SignatureBuilder::new();
    signature.push_literal(lang.render_visibility(
        function.modifiers().visibility,
        function.declaration_context(),
    ));
    if function.modifiers().is_async {
        signature.push_literal("async ");
    }
    signature.push_literal("fn ");
    signature.push_literal(function.name());
    append_type_parameters(&mut signature, function.type_params());
    signature.push_literal("(");
    signature.push_code(tupled_parameter_list(
        function.parameters(),
        |parameters, parameter| emit_parameter(parameters, lang, parameter),
    )?);
    signature.push_literal(")");
    append_suffixes(&mut signature, function);
    if let Some(return_type) = function.return_type() {
        signature.push_literal(" -> ");
        signature.push_type(return_type);
    }
    signature.append_to(&mut block);
    append_where_constraints(&mut block, function.where_constraints());

    if let Some(body) = function.body() {
        if function.where_constraints().is_empty() {
            block.add(" {", ());
        } else {
            block.add("{", ());
        }
        block.add_line();
        block.add("%>", ());
        block.add_code(body.clone());
        if !body.ends_with_newline_or_block_close() {
            block.add_line();
        }
        block.add("%<}", ());
        block.add_line();
    } else {
        block.add(";", ());
        block.add_line();
    }
    block.build()
}

fn append_type_parameters(
    signature: &mut SignatureBuilder,
    type_params: &[crate::spec::where_spec::TypeParamSpec],
) {
    if type_params.is_empty() {
        return;
    }
    signature.push_literal("<");
    let mut first = true;
    for parameter in type_params
        .iter()
        .filter(|parameter| parameter.is_lifetime())
        .chain(
            type_params
                .iter()
                .filter(|parameter| !parameter.is_lifetime()),
        )
    {
        if !first {
            signature.push_literal(", ");
        }
        first = false;
        signature.push_literal(parameter.name());
        if !parameter.bounds().is_empty() {
            signature.push_literal(": ");
            for (index, bound) in parameter.bounds().iter().enumerate() {
                if index > 0 {
                    signature.push_literal(" + ");
                }
                signature.push_type(bound);
            }
        }
    }
    signature.push_literal(">");
}

fn emit_parameter(block: &mut CodeBlockBuilder, lang: &Rust, parameter: &ParameterSpec) {
    block.add("%L", lang.escape_reserved(parameter.name()));
    if !parameter.param_type().is_empty() {
        block.add(": %T", parameter.param_type().clone());
    }
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
}

fn append_where_constraints(block: &mut CodeBlockBuilder, constraints: &[WhereConstraint]) {
    if constraints.is_empty() {
        return;
    }
    block.add_line();
    block.add("where", ());
    block.add_line();
    block.add("%>", ());
    for constraint in constraints {
        block.add("%T: ", constraint.subject().clone());
        for (index, bound) in constraint.bounds().iter().enumerate() {
            if index > 0 {
                block.add(" + ", ());
            }
            block.add("%T", bound.clone());
        }
        block.add(",", ());
        block.add_line();
    }
    block.add("%<", ());
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &Rust,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    for annotation in function.annotation_specs() {
        block.add_code(annotation.emit_with_syntax("#[", "]")?);
        block.add_line();
    }
    for annotation in function.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_node::CodeNode;
    use crate::lang::rust::Rust;
    use crate::spec::fun_spec::FunSpec;
    use crate::spec::modifiers::DeclarationContext;
    use crate::spec::where_spec::TypeParamSpec;
    use crate::type_name::TypeName;

    #[test]
    fn where_continuations_use_structural_indentation() {
        let function = FunSpec::builder("work")
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Clone")])
            .body(CodeBlock::of("todo!()", ()).unwrap())
            .build()
            .unwrap();
        let block = function
            .emit(
                &Rust::with_indent(Rust::new(), "<indent>"),
                DeclarationContext::TopLevel,
            )
            .unwrap();

        assert!(block.nodes.windows(3).any(|nodes| {
            matches!(
                nodes,
                [CodeNode::Literal(where_keyword), CodeNode::Newline, CodeNode::Indent]
                    if where_keyword == "where"
            )
        }));
        assert!(
            !block
                .nodes
                .iter()
                .any(|node| matches!(node, CodeNode::Literal(text) if text.contains("<indent>")))
        );
    }
}
