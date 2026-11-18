//! Python-owned structured layout construction.
//!
//! This module owns Python list/delimiter policy while returning the shared
//! `CodeBlock` carrier. The renderer only interprets the resulting layout
//! capsule; it does not infer Python grammar from text.

use crate::code_block::CodeBlock;
use crate::code_node::{CodeNode, LayoutBlock, LayoutBreakMode};

/// Build a Python delimiter group with Ruff-compatible edge and item breaks.
///
/// The inner group owns comma/item breaks while the outer group owns the
/// opening/closing edge breaks. Keeping those decisions in separate layout
/// groups lets an outer call break before a fitting nested generic.
pub(crate) fn delimited(
    open: &str,
    items: Vec<CodeBlock>,
    close: &str,
    flat_break: LayoutBreakMode,
) -> CodeBlock {
    if items.is_empty() {
        let mut nodes = Vec::new();
        if !open.is_empty() {
            nodes.push(CodeNode::Literal(open.to_string()));
        }
        if !close.is_empty() {
            nodes.push(CodeNode::Literal(close.to_string()));
        }
        return CodeBlock { nodes };
    }

    let item_count = items.len();
    if item_count == 1 {
        let item = items.into_iter().next().expect("singleton item exists");
        let mut inner_nodes = Vec::new();
        inner_nodes.extend(item.nodes);
        return edge_group(open, close, flat_break, false, inner_nodes);
    }

    let mut inner_nodes = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            inner_nodes.push(CodeNode::SoftBreak);
        }
        inner_nodes.extend(item.nodes);
        if index + 1 < item_count {
            inner_nodes.push(CodeNode::Literal(",".to_string()));
        }
    }

    edge_group(open, close, flat_break, item_count > 0, inner_nodes)
}

/// Build a Python type or expression infix list. A root broken list receives
/// parentheses so a physical line break remains legal Python syntax.
pub(crate) fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    let item_count = items.len();
    let mut nodes = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            nodes.push(CodeNode::SoftBreak);
            nodes.push(CodeNode::Literal(separator.trim_start().to_string()));
        }
        nodes.extend(item.nodes);
    }

    if item_count == 0 {
        return CodeBlock { nodes: Vec::new() };
    }

    edge_group_with_prefixes("", "", "(", ")", LayoutBreakMode::Space, false, nodes)
}

fn edge_group(
    open: &str,
    close: &str,
    inner_break: LayoutBreakMode,
    trailing_comma: bool,
    inner_nodes: Vec<CodeNode>,
) -> CodeBlock {
    edge_group_with_prefixes(
        open,
        close,
        open,
        close,
        inner_break,
        trailing_comma,
        inner_nodes,
    )
}

fn edge_group_with_prefixes(
    flat_prefix: &str,
    flat_suffix: &str,
    broken_prefix: &str,
    broken_suffix: &str,
    inner_break: LayoutBreakMode,
    trailing_comma: bool,
    inner_nodes: Vec<CodeNode>,
) -> CodeBlock {
    let child_layout = CodeNode::Layout(LayoutBlock::consistent(
        inner_break,
        "",
        "",
        "",
        "",
        inner_nodes,
    ));
    let edge_nodes = vec![
        CodeNode::Indent,
        CodeNode::SoftBreak,
        child_layout,
        CodeNode::Dedent,
        CodeNode::SoftBreak,
    ];
    let layout = LayoutBlock::consistent(
        LayoutBreakMode::Empty,
        flat_prefix,
        flat_suffix,
        broken_prefix,
        broken_suffix,
        edge_nodes,
    )
    .with_trailing_comma(trailing_comma);
    CodeBlock {
        nodes: vec![CodeNode::Layout(layout)],
    }
}

/// Build a standalone Python keyword argument fragment. The caller owns the
/// outer call group; this fragment carries no statement semantics and therefore
/// cannot be mistaken for an assignment by the outer renderer.
pub(crate) fn keyword_argument(label: CodeBlock, value: CodeBlock) -> CodeBlock {
    let label = label.for_inline_composition();
    let value = value.for_inline_composition();
    let mut nodes = label.nodes;
    nodes.push(CodeNode::Literal("=".to_string()));
    nodes.extend(value.nodes);
    CodeBlock { nodes }
}
