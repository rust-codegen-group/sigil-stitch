//! Structured source-tree nodes used by [`CodeBlock`](crate::code_block::CodeBlock).
//!
//! `CodeNode` is the structured node model used by
//! [`CodeBlock`](crate::code_block::CodeBlock).
//! Each node is self-contained — type references, names, and nested blocks are
//! stored inline rather than in a separate argument vector. This enables natural
//! tree traversal for import collection, structural transformation, and rendering.

use crate::code_block::{Arg, CodeBlock, FormatPart, Specifier};
use crate::error::SigilStitchError;
use crate::type_name::TypeName;

/// Structural role of a control-flow block.
///
/// This is a language-neutral label only. It carries **what** a block is, not
/// how any language renders it. Language adapters map the label to their own
/// openers and closers locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BlockIntent {
    /// A block without a recognized language-specific role.
    Generic,
    /// An `if` block.
    If,
    /// An `elif` / `elseif` / `else if` branch.
    ElseIf,
    /// A bare `else` branch.
    Else,
    /// A `for` loop.
    For,
    /// A `while` loop.
    While,
    /// An `until` loop.
    Until,
    /// A `case` block.
    Case,
    /// A `match` expression/statement block.
    Match,
    /// A `try` block.
    Try,
    /// A `class` declaration body.
    Class,
    /// An `instance` declaration body.
    Instance,
    /// A `module` declaration body.
    Module,
    /// A `module type` / signature declaration body.
    ModuleType,
    /// A `do` block.
    Do,
    /// A function or method body.
    Function,
    /// A lambda expression body.
    Lambda,
}

impl BlockIntent {
    /// Classify a builder control-flow condition from its raw format string.
    ///
    /// This is deliberately language-neutral and policy-free. It recognizes
    /// static leading tokens only; language adapters decide what to do with
    /// the resulting label.
    pub(crate) fn classify_condition(condition: &str) -> Self {
        let trimmed = condition.trim();
        if trimmed.is_empty() {
            return Self::Generic;
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        let first = words.first().copied().unwrap_or_default();
        match first {
            "if" => Self::If,
            "elif" | "elseif" => Self::ElseIf,
            "else" => {
                if words.get(1) == Some(&"if") {
                    Self::ElseIf
                } else {
                    Self::Else
                }
            }
            "for" => Self::For,
            "while" => Self::While,
            "until" => Self::Until,
            "case" => Self::Case,
            "match" => Self::Match,
            "try" => Self::Try,
            "class" => Self::Class,
            "instance" => Self::Instance,
            "do" => Self::Do,
            "module" => {
                if words.get(1) == Some(&"type") {
                    Self::ModuleType
                } else {
                    Self::Module
                }
            }
            "go" => {
                if words
                    .get(1)
                    .is_some_and(|word| *word == "func" || word.starts_with("func("))
                {
                    Self::Function
                } else {
                    Self::Generic
                }
            }
            "function" | "func" | "fn" | "def" | "fun" => Self::Function,
            _ => {
                if trimmed.ends_with(" do") {
                    Self::Do
                } else if let Some(intent) = embedded_control_intent(trimmed) {
                    intent
                } else if looks_like_lambda_condition(trimmed) {
                    Self::Lambda
                } else {
                    Self::Generic
                }
            }
        }
    }
}

/// Recognize control-flow roles embedded after a leading binding.
///
/// OCaml's idiomatic `let describe x = match v with` starts with `let`, so
/// the leading-token classifier above cannot see the `match`. Keep this probe
/// conservative: it only fires when the condition ends in the OCaml `with`
/// marker and contains a standalone `match` or `try` word.
fn embedded_control_intent(condition: &str) -> Option<BlockIntent> {
    if !condition.ends_with(" with") {
        return None;
    }
    if condition.contains(" match ") {
        Some(BlockIntent::Match)
    } else if condition.contains(" try ") {
        Some(BlockIntent::Try)
    } else {
        None
    }
}

/// Returns true when a raw builder condition has a conservative C++-style
/// lambda capture shape. Control keywords are classified before this probe.
fn looks_like_lambda_condition(condition: &str) -> bool {
    if condition.starts_with('[') {
        return true;
    }

    let Some(rest) = condition.strip_prefix("return ") else {
        let Some((_, after_eq)) = condition.split_once('=') else {
            return false;
        };
        let after_eq = after_eq.trim_start();
        if !after_eq.starts_with('[') {
            return false;
        }
        return capture_followed_by_parens(after_eq);
    };

    if !rest.starts_with('[') {
        return false;
    }
    capture_followed_by_parens(rest)
}

fn capture_followed_by_parens(after_capture_start: &str) -> bool {
    let Some(close) = after_capture_start.find(']') else {
        return false;
    };
    after_capture_start[close + 1..].contains('(')
}

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
    /// A verbatim string literal, rendered with minimal escaping that preserves
    /// interpolation sigils (was `%V` + `Arg::VerbatimStr`).
    VerbatimStr(String),
    /// An inline literal string (was `%L` + `Arg::Literal`).
    InlineLiteral(String),
    /// A nested code block (was `%L` + `Arg::Code`).
    Nested(CodeBlock),
    /// A comment line. Rendered as `{prefix} {text}{suffix}` using the
    /// language's comment syntax.
    Comment(String),
    /// An attribute / annotation line. Rendered with the language's annotation
    /// prefix and suffix (Rust: `#[text]`, Java/Python: `@text`, C++: `[[text]]`).
    Attribute(String),
    /// Soft line break point (`%W`). Emits a space when the enclosing layout
    /// group fits, otherwise a newline followed by the configured indentation.
    SoftBreak,
    /// Increase indent level (`%>`).
    Indent,
    /// Decrease indent level (`%<`).
    Dedent,
    /// Statement begin marker (`%[`). Triggers `ensure_indent()`.
    StatementBegin,
    /// Statement end marker (`%]`). Requests the complete language-owned
    /// statement-end suffix, which may be `;`, another suffix, or empty.
    StatementEnd,
    /// Hard newline.
    Newline,
    /// Legacy block open delimiter carrying only condition text.
    ///
    /// Retained for source construction and rendering compatibility, including
    /// unchanged external adapters. Its Serde representation is not versioned.
    /// New code should use [`CodeNode::BlockOpenIntent`].
    #[deprecated(note = "use CodeNode::BlockOpenIntent")]
    BlockOpen(String),
    /// Legacy terminal block close delimiter carrying only condition text.
    ///
    /// Retained for source construction and rendering compatibility, including
    /// unchanged external adapters. Its Serde representation is not versioned.
    /// New code should use [`CodeNode::BlockCloseIntent`].
    #[deprecated(note = "use CodeNode::BlockCloseIntent")]
    BlockClose(String),
    /// Legacy non-terminal block close before a branch keyword.
    ///
    /// Retained for source construction and rendering compatibility, including
    /// unchanged external adapters. Its Serde representation is not versioned.
    /// New code should use [`CodeNode::BranchCloseIntent`].
    #[deprecated(note = "use CodeNode::BranchCloseIntent")]
    BranchClose(String),
    /// Block open delimiter carrying the condition text and its structural
    /// intent. At render time the renderer calls
    /// [`crate::lang::RendererLang::render_block_open`] for the complete
    /// target-language suffix.
    BlockOpenIntent {
        /// Raw condition format text from the matching builder call.
        condition: String,
        /// Structural role of the block.
        intent: BlockIntent,
    },
    /// Terminal block close delimiter carrying the condition text and its
    /// structural intent.
    ///
    /// Emits: the closer only (no statement-end suffix or newline — those come
    /// from `StatementEnd` and `Newline` nodes that follow).
    BlockCloseIntent {
        /// Raw condition format text from the matching builder call.
        condition: String,
        /// Structural role of the block.
        intent: BlockIntent,
    },
    /// Non-terminal block close before a branch keyword (`else`, `elif`,
    /// `catch`), carrying structural intent.
    ///
    /// Like [`CodeNode::BlockCloseIntent`] but asks
    /// [`crate::lang::RendererLang::render_branch_transition`] for the complete
    /// outgoing closer and connector whitespace before the branch keyword.
    BranchCloseIntent {
        /// Raw condition format text from the matching builder call.
        condition: String,
        /// Structural role of the block being transitioned.
        intent: BlockIntent,
    },
    /// A sequence of nodes (for grouping, e.g. a statement or control flow block).
    Sequence(Vec<CodeNode>),
}

/// Convert legacy `(FormatPart, Arg)` parallel vectors into `Vec<CodeNode>`.
///
/// Used by `CodeBlockBuilder::add()` which still calls `parse_format()` to get
/// `Vec<FormatPart>`, then zips with args into self-contained nodes.
#[allow(deprecated)]
pub(crate) fn parts_args_to_nodes(
    format: &str,
    parts: &[FormatPart],
    args: &[Arg],
) -> Result<Vec<CodeNode>, SigilStitchError> {
    let expected_specifiers: Vec<String> = parts
        .iter()
        .filter_map(|part| match part {
            FormatPart::Arg(spec) => Some(format!("%{}", spec.format_char())),
            _ => None,
        })
        .collect();
    if expected_specifiers.len() != args.len() {
        return Err(SigilStitchError::FormatArgCount {
            format: format.to_string(),
            expected: expected_specifiers.len(),
            actual: args.len(),
            expected_specifiers,
            actual_arg_kinds: args.iter().map(|arg| arg.kind_name().to_string()).collect(),
        });
    }

    let mut nodes = Vec::with_capacity(parts.len());
    let mut arg_index = 0;

    for part in parts {
        let node = match part {
            FormatPart::Literal(text) => CodeNode::Literal(text.clone()),
            FormatPart::Arg(spec) => {
                let arg = &args[arg_index];
                if !spec.matches_arg(arg) {
                    return Err(SigilStitchError::FormatArgKind {
                        format: format.to_string(),
                        index: arg_index,
                        expected: format!("%{} ({})", spec.format_char(), spec.expected_arg_kind()),
                        actual: arg.kind_name().to_string(),
                    });
                }
                arg_index += 1;
                match (spec, arg) {
                    (Specifier::Type, Arg::TypeName(tn)) => CodeNode::TypeRef(tn.clone()),
                    (Specifier::Name, Arg::Name(n)) => CodeNode::NameRef(n.clone()),
                    (Specifier::StringLit, Arg::StringLit(s)) => CodeNode::StringLit(s.clone()),
                    (Specifier::VerbatimStr, Arg::VerbatimStr(s)) => {
                        CodeNode::VerbatimStr(s.clone())
                    }
                    (Specifier::Literal, Arg::Literal(s)) => CodeNode::InlineLiteral(s.clone()),
                    (Specifier::Literal, Arg::Code(block)) => CodeNode::Nested(block.clone()),
                    (Specifier::Comment, Arg::Comment(s)) => CodeNode::Comment(s.clone()),
                    _ => unreachable!("format argument compatibility checked above"),
                }
            }
            FormatPart::Wrap => CodeNode::SoftBreak,
            FormatPart::Indent => CodeNode::Indent,
            FormatPart::Dedent => CodeNode::Dedent,
            FormatPart::StatementBegin => CodeNode::StatementBegin,
            FormatPart::StatementEnd => CodeNode::StatementEnd,
            FormatPart::Newline => CodeNode::Newline,
            FormatPart::BlockOpen(s) => CodeNode::BlockOpen(s.clone()),
            FormatPart::BlockClose(s) => CodeNode::BlockClose(s.clone()),
            FormatPart::BranchClose(s) => CodeNode::BranchClose(s.clone()),
        };
        nodes.push(node);
    }

    Ok(nodes)
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
        let nodes = parts_args_to_nodes("hello", &parts, &args).unwrap();
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
        let nodes = parts_args_to_nodes("x: %T", &parts, &args).unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(matches!(&nodes[0], CodeNode::Literal(s) if s == "x: "));
        assert!(matches!(&nodes[1], CodeNode::TypeRef(_)));
    }

    #[test]
    fn test_nested_block_conversion() {
        let inner = CodeBlock::of("inner()", ()).unwrap();
        let parts = vec![FormatPart::Arg(Specifier::Literal)];
        let args = vec![Arg::Code(inner)];
        let nodes = parts_args_to_nodes("%L", &parts, &args).unwrap();
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
        let nodes = parts_args_to_nodes("%>%[x%]\n%<", &parts, &[]).unwrap();
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
        let nodes = parts_args_to_nodes("a%Wb", &parts, &[]).unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[1], CodeNode::SoftBreak));
    }

    #[test]
    fn test_block_open_close_conversion() {
        let parts = vec![
            FormatPart::BlockOpen("if x".to_string()),
            FormatPart::BlockClose("if x".to_string()),
            FormatPart::BranchClose("if x".to_string()),
        ];
        let nodes = parts_args_to_nodes("block delimiters", &parts, &[]).unwrap();
        assert_eq!(nodes.len(), 3);
        #[allow(deprecated)]
        {
            assert!(matches!(&nodes[0], CodeNode::BlockOpen(s) if s == "if x"));
            assert!(matches!(&nodes[1], CodeNode::BlockClose(s) if s == "if x"));
            assert!(matches!(&nodes[2], CodeNode::BranchClose(s) if s == "if x"));
        }
    }

    #[test]
    fn classify_condition_uses_static_leading_tokens() {
        let cases = [
            ("if (x > 0)", BlockIntent::If),
            ("if (matrix[0] > 0)", BlockIntent::If),
            ("elif x", BlockIntent::ElseIf),
            ("elseif x", BlockIntent::ElseIf),
            ("else if x", BlockIntent::ElseIf),
            ("else", BlockIntent::Else),
            ("for i in x", BlockIntent::For),
            ("while x", BlockIntent::While),
            ("until x", BlockIntent::Until),
            ("case $x in", BlockIntent::Case),
            ("match x with", BlockIntent::Match),
            ("try", BlockIntent::Try),
            ("let describe x = match v with", BlockIntent::Match),
            ("let x = try f x with", BlockIntent::Try),
            ("let x = matching v with", BlockIntent::Generic),
            ("class Eq a", BlockIntent::Class),
            ("instance Show T", BlockIntent::Instance),
            ("module Foo", BlockIntent::Module),
            ("module type S", BlockIntent::ModuleType),
            ("do", BlockIntent::Do),
            ("main = do", BlockIntent::Do),
            ("go func()", BlockIntent::Function),
            ("func f()", BlockIntent::Function),
            ("function f()", BlockIntent::Function),
            ("auto fn = [&](int x)", BlockIntent::Lambda),
            ("return [&]()", BlockIntent::Lambda),
            ("auto fn = arr[i](x)", BlockIntent::Generic),
            ("interface User", BlockIntent::Generic),
            ("", BlockIntent::Generic),
        ];

        for (condition, expected) in cases {
            assert_eq!(
                BlockIntent::classify_condition(condition),
                expected,
                "condition: {condition:?}"
            );
        }
    }

    #[test]
    fn block_intent_nodes_round_trip_through_serde() {
        let nodes = vec![
            CodeNode::BlockOpenIntent {
                condition: "if x".to_string(),
                intent: BlockIntent::If,
            },
            CodeNode::BlockCloseIntent {
                condition: "if x".to_string(),
                intent: BlockIntent::If,
            },
        ];
        let json = serde_json::to_string(&nodes).unwrap();
        let decoded: Vec<CodeNode> = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            &decoded[0],
            CodeNode::BlockOpenIntent {
                condition,
                intent: BlockIntent::If,
            } if condition == "if x"
        ));
        assert!(matches!(
            &decoded[1],
            CodeNode::BlockCloseIntent {
                condition,
                intent: BlockIntent::If,
            } if condition == "if x"
        ));
    }

    #[test]
    #[allow(deprecated)]
    fn legacy_block_nodes_round_trip_through_current_serde() {
        // This verifies the current derives, not a versioned representation.
        let nodes = vec![
            CodeNode::BlockOpen("if x".to_string()),
            CodeNode::BranchClose("if x".to_string()),
            CodeNode::BlockClose("if x".to_string()),
        ];
        let json = serde_json::to_string(&nodes).unwrap();
        let decoded: Vec<CodeNode> = serde_json::from_str(&json).unwrap();
        assert!(matches!(&decoded[0], CodeNode::BlockOpen(s) if s == "if x"));
        assert!(matches!(&decoded[1], CodeNode::BranchClose(s) if s == "if x"));
        assert!(matches!(&decoded[2], CodeNode::BlockClose(s) if s == "if x"));
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
        let nodes = parts_args_to_nodes("let %N: %T = %S", &parts, &args).unwrap();
        assert_eq!(nodes.len(), 6);
        assert!(matches!(&nodes[1], CodeNode::NameRef(s) if s == "x"));
        assert!(matches!(&nodes[3], CodeNode::TypeRef(_)));
        assert!(matches!(&nodes[5], CodeNode::StringLit(s) if s == "hello"));
    }

    #[test]
    fn test_wrong_arg_kind_returns_error_instead_of_empty_literal() {
        let parts = vec![FormatPart::Arg(Specifier::Name)];
        let args = vec![Arg::Literal("wrong".to_string())];

        let error = parts_args_to_nodes("%N", &parts, &args).unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::FormatArgKind {
                index: 0,
                ref expected,
                ref actual,
                ..
            } if expected == "%N (Name)" && actual == "Literal"
        ));
    }

    #[test]
    fn every_specifier_accepts_only_its_documented_argument_kinds() {
        let args = vec![
            Arg::TypeName(TypeName::primitive("Value")),
            Arg::Name("value".to_string()),
            Arg::StringLit("value".to_string()),
            Arg::VerbatimStr("value".to_string()),
            Arg::Literal("value".to_string()),
            Arg::Code(CodeBlock::of("", ()).unwrap()),
            Arg::Comment("value".to_string()),
        ];

        for &specifier in Specifier::all() {
            let expected_kinds: &[&str] = match specifier {
                Specifier::Type => &["TypeName"],
                Specifier::Name => &["Name"],
                Specifier::StringLit => &["StringLit"],
                Specifier::VerbatimStr => &["VerbatimStr"],
                Specifier::Literal => &["Literal", "Code"],
                Specifier::Comment => &["Comment"],
            };
            let parts = [FormatPart::Arg(specifier)];
            for arg in &args {
                let result = parts_args_to_nodes(
                    &format!("%{}", specifier.format_char()),
                    &parts,
                    std::slice::from_ref(arg),
                );
                assert_eq!(result.is_ok(), expected_kinds.contains(&arg.kind_name()));
            }
        }
    }
}
