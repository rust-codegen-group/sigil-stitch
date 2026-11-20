//! Python-owned structured layout construction.
//!
//! This module owns Python list/delimiter policy while returning the shared
//! `CodeBlock` carrier. The renderer only interprets the resulting layout
//! capsule; it does not infer Python grammar from text.

use crate::code_block::CodeBlock;
use crate::code_node::{CodeNode, LayoutBlock, LayoutBreakMode, TrailingCommaPolicy};

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
        return edge_group(
            open,
            close,
            flat_break,
            TrailingCommaPolicy::Never,
            inner_nodes,
        );
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

    edge_group(
        open,
        close,
        flat_break,
        TrailingCommaPolicy::IfBroken,
        inner_nodes,
    )
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

    edge_group_with_prefixes(
        "",
        "",
        "(",
        ")",
        LayoutBreakMode::Space,
        TrailingCommaPolicy::Never,
        nodes,
    )
}

fn edge_group(
    open: &str,
    close: &str,
    inner_break: LayoutBreakMode,
    trailing_comma: TrailingCommaPolicy,
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
    trailing_comma: TrailingCommaPolicy,
    inner_nodes: Vec<CodeNode>,
) -> CodeBlock {
    let child_layout = CodeNode::Layout(
        LayoutBlock::consistent(inner_break, "", "", "", "", inner_nodes)
            .with_trailing_comma(trailing_comma),
    );
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
    );
    CodeBlock {
        nodes: vec![CodeNode::Layout(layout)],
    }
}

/// Normalize structural blank lines in a function suite. Python's closing
/// block markers render no token, so a close that already ends its line would
/// otherwise contribute a redundant blank line. Only that closer break is
/// dropped, and only once: explicit blank lines after a close and closers that
/// still own the line's only break are preserved. Opaque source and the
/// interiors of layouts are left alone.
pub(crate) fn function_body(body: &CodeBlock) -> CodeBlock {
    if body.contains_opaque_multiline() {
        return body.clone();
    }

    #[allow(deprecated)]
    fn is_block_close(node: &CodeNode) -> bool {
        matches!(
            node,
            CodeNode::BlockCloseIntent { .. } | CodeNode::BlockClose(_)
        )
    }

    fn ends_with_close(nodes: &[CodeNode]) -> bool {
        match nodes.last() {
            Some(node) if is_block_close(node) => true,
            Some(CodeNode::Newline) => ends_with_close(&nodes[..nodes.len() - 1]),
            Some(CodeNode::Nested(block)) => ends_with_close(&block.nodes),
            Some(CodeNode::Sequence(children)) => ends_with_close(children),
            _ => false,
        }
    }

    fn ends_with_terminated_close(nodes: &[CodeNode]) -> bool {
        match nodes.last() {
            Some(CodeNode::Newline) => ends_with_close(&nodes[..nodes.len() - 1]),
            Some(CodeNode::Nested(block)) => ends_with_terminated_close(&block.nodes),
            Some(CodeNode::Sequence(children)) => ends_with_terminated_close(children),
            _ => false,
        }
    }

    fn tail_breaks_after_closes(nodes: &[CodeNode]) -> usize {
        let mut breaks = 0;
        let mut saw_close = false;
        for node in nodes.iter().rev() {
            match node {
                CodeNode::Newline if saw_close => {
                    breaks += 1;
                    if breaks == 2 {
                        return breaks;
                    }
                }
                CodeNode::Newline | CodeNode::Indent | CodeNode::Dedent => {}
                CodeNode::Nested(block) => return tail_breaks_after_closes(&block.nodes),
                CodeNode::Sequence(children) => return tail_breaks_after_closes(children),
                node if is_block_close(node) => {
                    saw_close = true;
                }
                _ => break,
            }
        }
        breaks
    }

    fn line_break_precedes_close(nodes: &[CodeNode]) -> bool {
        let mut terminated = false;
        for node in nodes.iter().rev() {
            match node {
                CodeNode::Indent
                | CodeNode::Dedent
                | CodeNode::StatementBegin
                | CodeNode::StatementEnd => {}
                CodeNode::Newline => {
                    terminated = true;
                    break;
                }
                node if is_block_close(node) => {}
                _ => break,
            }
        }
        terminated
    }

    fn normalize(nodes: &mut Vec<CodeNode>) {
        let mut normalized = Vec::with_capacity(nodes.len());
        let mut skip_next_newline = false;
        for mut node in std::mem::take(nodes) {
            let skip_newline = std::mem::take(&mut skip_next_newline);
            match &mut node {
                CodeNode::Nested(block) => normalize(&mut block.nodes),
                CodeNode::Sequence(children) => normalize(children),
                CodeNode::Newline if skip_newline => continue,
                CodeNode::Newline
                    if normalized.last().is_some_and(|node| {
                        matches!(node, CodeNode::Nested(block)
                            if ends_with_terminated_close(&block.nodes)
                                || tail_breaks_after_closes(&block.nodes) >= 2)
                    }) =>
                {
                    continue;
                }
                node if is_block_close(node) => {
                    if matches!(normalized.last(), Some(CodeNode::Newline)) {
                        normalized.pop();
                    }
                    skip_next_newline = line_break_precedes_close(&normalized);
                }
                _ => {}
            }
            normalized.push(node);
        }
        *nodes = normalized;
    }

    fn trim_terminal_breaks(nodes: &mut Vec<CodeNode>) -> bool {
        let mut index = nodes.len();
        while index > 0 {
            index -= 1;
            match &mut nodes[index] {
                CodeNode::Newline => {
                    nodes.remove(index);
                }
                CodeNode::Nested(block) => {
                    if !trim_terminal_breaks(&mut block.nodes) {
                        return false;
                    }
                }
                CodeNode::Sequence(children) => {
                    if !trim_terminal_breaks(children) {
                        return false;
                    }
                }
                node if invisible(node) => {}
                _ => return false,
            }
        }
        true
    }

    fn invisible(node: &CodeNode) -> bool {
        matches!(
            node,
            CodeNode::Indent | CodeNode::Dedent | CodeNode::StatementBegin | CodeNode::StatementEnd
        ) || is_block_close(node)
    }

    let mut body = body.clone();
    normalize(&mut body.nodes);
    trim_terminal_breaks(&mut body.nodes);
    body.nodes.push(CodeNode::Newline);
    body
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
