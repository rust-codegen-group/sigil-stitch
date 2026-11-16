//! Python-owned structured layout construction.
//!
//! This module owns Python list/delimiter policy while returning the shared
//! `CodeBlock` carrier. The renderer only interprets the resulting layout
//! capsule; it does not infer Python grammar from text.

use crate::code_block::CodeBlock;
use crate::code_node::{CodeNode, LayoutBlock, LayoutBreakMode};

/// Build one Python delimiter group with a consistent item-break decision.
///
/// The continuation depth applies only to broken soft breaks. Flat output does
/// not acquire leading indentation, and nested item layouts retain their own
/// semantic structure.
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
    let mut nodes = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            if index == 1 {
                nodes.push(CodeNode::Indent);
            }
            nodes.push(CodeNode::SoftBreak);
        }
        nodes.extend(item.nodes);
        if index + 1 < item_count {
            nodes.push(CodeNode::Literal(",".to_string()));
        }
    }

    if item_count > 1 {
        nodes.push(CodeNode::Dedent);
    }

    CodeBlock {
        nodes: vec![CodeNode::Layout(LayoutBlock::consistent(
            flat_break, open, close, open, close, nodes,
        ))],
    }
}

/// Build a Python type or expression infix list. A root broken list receives
/// parentheses so a physical line break remains legal Python syntax.
pub(crate) fn infix(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    let item_count = items.len();
    let mut nodes = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            if index == 1 {
                nodes.push(CodeNode::Indent);
            }
            nodes.push(CodeNode::SoftBreak);
            nodes.push(CodeNode::Literal(separator.trim_start().to_string()));
        }
        nodes.extend(item.nodes);
    }
    if item_count > 1 {
        nodes.push(CodeNode::Dedent);
    }

    CodeBlock {
        nodes: vec![CodeNode::Layout(LayoutBlock::consistent(
            LayoutBreakMode::Space,
            "",
            "",
            "(",
            ")",
            nodes,
        ))],
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
