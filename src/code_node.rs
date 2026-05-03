//! Tree-based intermediate representation for code generation.
//!
//! `CodeNode` is the internal IR used by [`CodeBlock`](crate::code_block::CodeBlock).
//! Each node is self-contained — type references, names, and nested blocks are
//! stored inline rather than in a separate argument vector. This enables natural
//! tree traversal for import collection, structural transformation, and rendering.

use crate::code_block::{Arg, CodeBlock, FormatPart, Specifier};
use crate::type_name::TypeName;

/// A single node in the code generation tree.
///
/// Each variant is self-contained: a type reference is `CodeNode::TypeRef(TypeName)`,
/// not a separate format tag plus a positional argument.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CodeNode {
    /// Literal text (no interpolation).
    Literal(String),
    /// A type reference with import tracking (was `%T` + `Arg::TypeName`).
    TypeRef(TypeName),
    /// A name identifier (was `%N` + `Arg::Name`).
    NameRef(String),
    /// A string literal value, rendered with language-specific quoting
    /// (was `%S` + `Arg::StringLit`).
    StringLit(String),
    /// An inline literal string (was `%L` + `Arg::Literal`).
    InlineLiteral(String),
    /// A nested code block (was `%L` + `Arg::Code`).
    Nested(CodeBlock),
    /// A comment line. Rendered as `{prefix} {text}{suffix}` using the
    /// language's comment syntax.
    Comment(String),
    /// Soft line break point (`%W`). In direct mode emits a space; in pretty
    /// mode becomes `BoxDoc::softline()`.
    SoftBreak,
    /// Increase indent level (`%>`).
    Indent,
    /// Decrease indent level (`%<`).
    Dedent,
    /// Statement begin marker (`%[`). Triggers `ensure_indent()`.
    StatementBegin,
    /// Statement end marker (`%]`). Emits `;` if the language uses semicolons.
    StatementEnd,
    /// Hard newline.
    Newline,
    /// Block open delimiter, resolved at render time via `lang.block_syntax().block_open`.
    BlockOpen,
    /// Block open with an overridden delimiter string.
    BlockOpenOverride(String),
    /// Terminal block close delimiter, resolved via `lang.block_syntax().block_close`.
    BlockClose,
    /// Transitional block close delimiter (e.g. `} else`), resolved via
    /// `lang.block_syntax().block_close` + `" "`.
    BlockCloseTransition,
    /// A sequence of nodes (for grouping, e.g. a statement or control flow block).
    Sequence(Vec<CodeNode>),
}

/// Convert legacy `(FormatPart, Arg)` parallel vectors into `Vec<CodeNode>`.
///
/// Used by `CodeBlockBuilder::add()` which still calls `parse_format()` to get
/// `Vec<FormatPart>`, then zips with args into self-contained nodes.
pub(crate) fn parts_args_to_nodes(parts: &[FormatPart], args: &[Arg]) -> Vec<CodeNode> {
    let mut nodes = Vec::with_capacity(parts.len());
    let mut arg_index = 0;

    for part in parts {
        let node = match part {
            FormatPart::Literal(text) => CodeNode::Literal(text.clone()),
            FormatPart::Arg(spec) => {
                let arg = &args[arg_index];
                arg_index += 1;
                match (spec, arg) {
                    (Specifier::Type, Arg::TypeName(tn)) => CodeNode::TypeRef(tn.clone()),
                    (Specifier::Name, Arg::Name(n)) => CodeNode::NameRef(n.clone()),
                    (Specifier::StringLit, Arg::StringLit(s)) => CodeNode::StringLit(s.clone()),
                    (Specifier::Literal, Arg::Literal(s)) => CodeNode::InlineLiteral(s.clone()),
                    (Specifier::Literal, Arg::Code(block)) => CodeNode::Nested(block.clone()),
                    _ => CodeNode::Literal(String::new()),
                }
            }
            FormatPart::Wrap => CodeNode::SoftBreak,
            FormatPart::Indent => CodeNode::Indent,
            FormatPart::Dedent => CodeNode::Dedent,
            FormatPart::StatementBegin => CodeNode::StatementBegin,
            FormatPart::StatementEnd => CodeNode::StatementEnd,
            FormatPart::Newline => CodeNode::Newline,
            FormatPart::BlockOpen => CodeNode::BlockOpen,
            FormatPart::BlockOpenOverride(s) => CodeNode::BlockOpenOverride(s.clone()),
            FormatPart::BlockClose => CodeNode::BlockClose,
            FormatPart::BlockCloseTransition => CodeNode::BlockCloseTransition,
        };
        nodes.push(node);
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::type_name::TypeName;

    #[test]
    fn test_literal_conversion() {
        let parts = vec![FormatPart::Literal("hello".to_string())];
        let args = vec![];
        let nodes = parts_args_to_nodes(&parts, &args);
        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], CodeNode::Literal(s) if s == "hello"));
    }

    #[test]
    fn test_type_ref_conversion() {
        let tn = TypeName::primitive("string");
        let parts = vec![
            FormatPart::Literal("x: ".to_string()),
            FormatPart::Arg(Specifier::Type),
        ];
        let args = vec![Arg::TypeName(tn)];
        let nodes = parts_args_to_nodes(&parts, &args);
        assert_eq!(nodes.len(), 2);
        assert!(matches!(&nodes[0], CodeNode::Literal(s) if s == "x: "));
        assert!(matches!(&nodes[1], CodeNode::TypeRef(_)));
    }

    #[test]
    fn test_nested_block_conversion() {
        let inner = CodeBlock::of("inner()", ()).unwrap();
        let parts = vec![FormatPart::Arg(Specifier::Literal)];
        let args = vec![Arg::Code(inner)];
        let nodes = parts_args_to_nodes(&parts, &args);
        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], CodeNode::Nested(_)));
    }

    #[test]
    fn test_structural_nodes() {
        let parts = vec![
            FormatPart::Indent,
            FormatPart::StatementBegin,
            FormatPart::Literal("x".to_string()),
            FormatPart::StatementEnd,
            FormatPart::Newline,
            FormatPart::Dedent,
        ];
        let nodes = parts_args_to_nodes(&parts, &[]);
        assert_eq!(nodes.len(), 6);
        assert!(matches!(nodes[0], CodeNode::Indent));
        assert!(matches!(nodes[1], CodeNode::StatementBegin));
        assert!(matches!(nodes[3], CodeNode::StatementEnd));
        assert!(matches!(nodes[4], CodeNode::Newline));
        assert!(matches!(nodes[5], CodeNode::Dedent));
    }

    #[test]
    fn test_soft_break_conversion() {
        let parts = vec![
            FormatPart::Literal("a".to_string()),
            FormatPart::Wrap,
            FormatPart::Literal("b".to_string()),
        ];
        let nodes = parts_args_to_nodes(&parts, &[]);
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[1], CodeNode::SoftBreak));
    }

    #[test]
    fn test_block_open_close_conversion() {
        let parts = vec![
            FormatPart::BlockOpen,
            FormatPart::BlockClose,
            FormatPart::BlockOpenOverride("where".to_string()),
            FormatPart::BlockCloseTransition,
        ];
        let nodes = parts_args_to_nodes(&parts, &[]);
        assert_eq!(nodes.len(), 4);
        assert!(matches!(nodes[0], CodeNode::BlockOpen));
        assert!(matches!(nodes[1], CodeNode::BlockClose));
        assert!(matches!(&nodes[2], CodeNode::BlockOpenOverride(s) if s == "where"));
        assert!(matches!(nodes[3], CodeNode::BlockCloseTransition));
    }

    #[test]
    fn test_mixed_args_conversion() {
        let tn = TypeName::primitive("number");
        let parts = vec![
            FormatPart::Literal("let ".to_string()),
            FormatPart::Arg(Specifier::Name),
            FormatPart::Literal(": ".to_string()),
            FormatPart::Arg(Specifier::Type),
            FormatPart::Literal(" = ".to_string()),
            FormatPart::Arg(Specifier::StringLit),
        ];
        let args = vec![
            Arg::Name("x".to_string()),
            Arg::TypeName(tn),
            Arg::StringLit("hello".to_string()),
        ];
        let nodes = parts_args_to_nodes(&parts, &args);
        assert_eq!(nodes.len(), 6);
        assert!(matches!(&nodes[1], CodeNode::NameRef(s) if s == "x"));
        assert!(matches!(&nodes[3], CodeNode::TypeRef(_)));
        assert!(matches!(&nodes[5], CodeNode::StringLit(s) if s == "hello"));
    }
}
