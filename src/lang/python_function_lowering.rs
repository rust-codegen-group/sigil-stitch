//! Python-owned function declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::code_node::{CodeNode, LayoutBlock, LayoutBreakMode, TrailingCommaPolicy};
use crate::error::SigilStitchError;
use crate::lang::function_lowering::{SignatureBuilder, inline_default_value};
use crate::lang::python::Python;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;

pub(crate) fn validate_returns(
    lang: &Python,
    function: crate::lang::FunctionIntent<'_>,
) -> Result<(), SigilStitchError> {
    if let Some(returns) = function.returns()
        && returns.len() > 1
    {
        return Err(SigilStitchError::UnsupportedFunctionReturns {
            language: lang.file_extension().into(),
            function_name: function.name().into(),
            context: function.function_context(),
            form: function.form(),
            actual: returns.len(),
            reason: "this language supports at most one return slot".into(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Python,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    validate_returns(lang, *function)?;
    let mut block = CodeBlock::builder();
    emit_decorators(&mut block, function)?;

    let inline_ellipsis =
        function.body().is_none() && function.doc().is_empty() && function.delegation().is_none();
    emit_signature(&mut block, lang, function, inline_ellipsis)?;
    block.add_line();
    if inline_ellipsis {
        return block.build();
    }
    block.add("%>", ());

    if !function.doc().is_empty() {
        let lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    if let Some(delegation) = function.delegation() {
        block.add_statement("%L", delegation.clone());
    }
    if let Some(body) = function.body() {
        block.add_code(super::python_layout::function_body(body));
    } else {
        block.add_statement("...", ());
    }
    block.add("%<", ());
    block.build()
}

fn emit_signature(
    block: &mut CodeBlockBuilder,
    lang: &Python,
    function: ValidatedFunction<'_>,
    inline_ellipsis: bool,
) -> Result<(), SigilStitchError> {
    if function.modifiers().is_async {
        block.add("async ", ());
    }
    block.add("def %L(", function.name());

    let mut suffix = SignatureBuilder::new();
    suffix.push_literal(")");
    append_suffixes(&mut suffix, function);
    if let Some(returns) = function.returns() {
        let native_empty = crate::type_name::TypeName::primitive("None");
        let return_type = returns.first().unwrap_or(&native_empty);
        suffix.push_literal(" -> ");
        suffix.push_type(return_type);
    }
    suffix.push_literal(if inline_ellipsis { ": ..." } else { ":" });
    let mut tail = CodeBlock::builder();
    suffix.append_to(&mut tail);

    let parameters = function
        .parameters()
        .iter()
        .map(|parameter| emit_parameter(lang, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    block.add_code(signature_layout(parameters, tail.build()?));
    Ok(())
}

fn signature_layout(parameters: Vec<CodeBlock>, tail: CodeBlock) -> CodeBlock {
    let parameter_count = parameters.len();
    if parameter_count == 0 {
        return tail;
    }

    let mut items = Vec::new();
    for (index, parameter) in parameters.into_iter().enumerate() {
        if index > 0 {
            items.push(CodeNode::Literal(",".into()));
            items.push(CodeNode::SoftBreak);
        }
        items.extend(parameter.nodes);
    }

    let signature = if parameter_count == 1 {
        // A singleton always gets a comma if its parameter list breaks. End
        // the capsule at the closing edge (before the return annotation), so
        // its conditional comma is emitted before that edge's pending break.
        let mut edges = vec![CodeNode::Indent, CodeNode::SoftBreak];
        edges.extend(items);
        edges.extend([CodeNode::Dedent, CodeNode::SoftBreak]);
        let mut signature = vec![CodeNode::Layout(
            LayoutBlock::consistent(LayoutBreakMode::Empty, "", "", "", "", edges)
                .with_trailing_comma(TrailingCommaPolicy::IfBroken),
        )];
        signature.extend(tail.nodes);
        signature
    } else {
        // Test the parameter edges against the whole signature so that a
        // fitting return annotation does not break just to keep the
        // parameters inline. Once the edges break, the list can still pack
        // parameters that fit on one line without a trailing comma.
        let mut edges = vec![
            CodeNode::Indent,
            CodeNode::SoftBreak,
            CodeNode::Layout(
                LayoutBlock::consistent(LayoutBreakMode::Space, "", "", "", "", items)
                    .with_trailing_comma(TrailingCommaPolicy::IfBroken),
            ),
            CodeNode::Dedent,
            CodeNode::SoftBreak,
        ];
        edges.extend(tail.nodes);
        vec![CodeNode::Layout(LayoutBlock::consistent(
            LayoutBreakMode::Empty,
            "",
            "",
            "",
            "",
            edges,
        ))]
    };
    CodeBlock { nodes: signature }
}

fn emit_parameter(lang: &Python, parameter: &ParameterSpec) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    block.add("%L", lang.escape_reserved(parameter.name()));
    if !parameter.param_type().is_empty() {
        block.add(": %T", parameter.param_type().clone());
    }
    if let Some(default) = parameter.default_value() {
        block.add(
            if parameter.param_type().is_empty() {
                "=%L"
            } else {
                " = %L"
            },
            inline_default_value(default),
        );
    }
    block.build()
}

fn append_suffixes(signature: &mut SignatureBuilder, function: ValidatedFunction<'_>) {
    for suffix in function.suffixes() {
        signature.push_literal(" ");
        signature.push_literal(suffix);
    }
}

fn emit_decorators(
    block: &mut CodeBlockBuilder,
    function: ValidatedFunction<'_>,
) -> Result<(), SigilStitchError> {
    for annotation in function.annotation_specs() {
        block.add_code(annotation.emit_with_syntax("@", "")?);
        block.add_line();
    }
    for annotation in function.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}
