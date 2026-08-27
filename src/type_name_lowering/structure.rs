//! Syntax-neutral construction mechanics used by language-local type lowerers.

use crate::code_block::CodeBlock;
use crate::code_node::CodeNode;
use crate::type_name::TypeName;

pub(crate) fn block(nodes: Vec<CodeNode>) -> CodeBlock {
    CodeBlock { nodes }
}

pub(crate) fn terminal(type_name: &TypeName) -> CodeBlock {
    block(vec![CodeNode::TypeRef(type_name.clone())])
}

pub(crate) fn literal(value: impl Into<String>) -> CodeBlock {
    block(vec![CodeNode::Literal(value.into())])
}

pub(crate) fn name(value: impl Into<String>) -> CodeBlock {
    block(vec![CodeNode::NameRef(value.into())])
}

pub(crate) fn string_literal(value: impl Into<String>) -> CodeBlock {
    block(vec![CodeNode::StringLit(value.into())])
}

pub(crate) fn concat(parts: impl IntoIterator<Item = CodeBlock>) -> CodeBlock {
    let mut nodes = Vec::new();
    for part in parts {
        nodes.extend(part.nodes);
    }
    block(nodes)
}

pub(crate) fn surround(open: &str, inner: CodeBlock, close: &str) -> CodeBlock {
    concat([literal(open), inner, literal(close)])
}

pub(crate) fn join(items: Vec<CodeBlock>, separator: &str) -> CodeBlock {
    let mut nodes = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            nodes.push(CodeNode::Literal(separator.to_string()));
        }
        nodes.extend(item.nodes);
    }
    if nodes.is_empty() {
        block(nodes)
    } else {
        block(vec![CodeNode::Sequence(nodes)])
    }
}

pub(crate) fn join_soft(items: Vec<CodeBlock>, separator_after_break: &str) -> CodeBlock {
    let mut nodes = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            nodes.push(CodeNode::SoftBreak);
            nodes.push(CodeNode::Literal(separator_after_break.to_string()));
        }
        nodes.extend(item.nodes);
    }
    if nodes.is_empty() {
        block(nodes)
    } else {
        block(vec![CodeNode::Sequence(nodes)])
    }
}

pub(crate) fn join_trailing_soft(items: Vec<CodeBlock>, separator_before_break: &str) -> CodeBlock {
    let mut nodes = Vec::new();
    let item_count = items.len();
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            nodes.push(CodeNode::SoftBreak);
        }
        nodes.extend(item.nodes);
        if index + 1 < item_count {
            nodes.push(CodeNode::Literal(separator_before_break.to_string()));
        }
    }
    if nodes.is_empty() {
        block(nodes)
    } else {
        block(vec![CodeNode::Sequence(nodes)])
    }
}

pub(crate) fn delimited_soft(
    open: &str,
    items: Vec<CodeBlock>,
    separator_after_break: &str,
    close: &str,
) -> CodeBlock {
    surround(
        open,
        join_trailing_soft(items, separator_after_break),
        close,
    )
}

pub(crate) fn qualified(module: &str, separator: &str, name: &str) -> CodeBlock {
    self::name(format!("{module}{separator}{name}"))
}
