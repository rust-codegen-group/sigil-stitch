use super::{CodeBlock, validate_balanced_indent_markers, validate_no_unresolved_indent_markers};
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::type_name_lowering::DiagnosticPath;
use crate::type_name_lowering::validation::validate_type_name;

impl CodeBlock {
    pub(crate) fn rewrite_source(
        &self,
        path: &DiagnosticPath,
        rewrite: impl FnOnce(&mut Vec<CodeNode>),
    ) -> Result<Self, SigilStitchError> {
        let mut rewritten = self.clone();
        rewrite(&mut rewritten.nodes);
        // Indentation may span node vectors, so only the complete rewritten
        // source tree can be checked; validating each callback would reject it.
        validate_balanced_indent_markers(&rewritten.nodes).map_err(|error| {
            SigilStitchError::InvalidRewrittenSource {
                context: path.to_string(),
                reason: error.to_string(),
            }
        })?;
        validate_no_unresolved_indent_markers(&rewritten.nodes).map_err(|error| {
            SigilStitchError::InvalidRewrittenSource {
                context: path.to_string(),
                reason: error.to_string(),
            }
        })?;
        validate_source_nodes(&rewritten.nodes, path)?;
        Ok(rewritten)
    }
}

fn validate_source_nodes(
    nodes: &[CodeNode],
    path: &DiagnosticPath,
) -> Result<(), SigilStitchError> {
    for (index, node) in nodes.iter().enumerate() {
        let node_path = path.node(index);
        match node {
            CodeNode::TypeRef(type_name) => validate_type_name(type_name, &node_path)?,
            CodeNode::Nested(block) => validate_source_nodes(&block.nodes, &path.nested(index))?,
            CodeNode::Sequence(children) => validate_source_nodes(children, &path.sequence(index))?,
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rewrite::walk_nodes_mut;
    use crate::lang::typescript::TypeScript;
    use crate::type_name::TypeName;
    use std::cell::Cell;

    fn positions(node: CodeNode) -> [(CodeBlock, &'static str); 3] {
        [
            (
                CodeBlock {
                    nodes: vec![node.clone()],
                },
                "source.node[0]",
            ),
            (
                CodeBlock {
                    nodes: vec![CodeNode::Nested(CodeBlock {
                        nodes: vec![node.clone()],
                    })],
                },
                "source.nested[0].node[0]",
            ),
            (
                CodeBlock {
                    nodes: vec![CodeNode::Sequence(vec![node])],
                },
                "source.sequence[0].node[0]",
            ),
        ]
    }

    #[test]
    fn rewrite_calls_the_hook_once_and_preserves_the_source() {
        let source = CodeBlock {
            nodes: vec![
                CodeNode::Literal("root".to_string()),
                CodeNode::Nested(CodeBlock::of("nested", ()).unwrap()),
                CodeNode::Sequence(vec![CodeNode::Literal("sequence".to_string())]),
            ],
        };
        let hook_calls = Cell::new(0);
        let visits = Cell::new(0);
        let rewritten = source
            .rewrite_source(&DiagnosticPath::root("source"), |nodes| {
                hook_calls.set(hook_calls.get() + 1);
                walk_nodes_mut(nodes, &|nodes| {
                    visits.set(visits.get() + 1);
                    for node in nodes {
                        if let CodeNode::Literal(text) = node {
                            text.push('!');
                        }
                    }
                });
            })
            .unwrap();

        assert_eq!(hook_calls.get(), 1);
        assert_eq!(visits.get(), 3);
        assert_eq!(
            rewritten.render_standalone(&TypeScript::new(), 80).unwrap(),
            "root!nested!sequence!"
        );
        assert_eq!(
            source.render_standalone(&TypeScript::new(), 80).unwrap(),
            "rootnestedsequence"
        );
    }

    #[test]
    fn rewrite_validates_only_after_the_complete_traversal() {
        let source = CodeBlock::of("%L", CodeBlock::of("value", ()).unwrap()).unwrap();
        let rewritten = source
            .rewrite_source(&DiagnosticPath::root("source"), |nodes| {
                walk_nodes_mut(nodes, &|nodes| {
                    if matches!(nodes.first(), Some(CodeNode::Nested(_))) {
                        nodes.insert(0, CodeNode::Indent);
                    } else {
                        nodes.push(CodeNode::Dedent);
                    }
                });
            })
            .unwrap();

        assert_eq!(
            rewritten.render_standalone(&TypeScript::new(), 80).unwrap(),
            "  value"
        );
    }

    #[test]
    fn rewrite_can_repair_an_unchecked_source_before_validation() {
        let source = CodeBlock {
            nodes: vec![CodeNode::Dedent],
        };
        let rewritten = source
            .rewrite_source(&DiagnosticPath::root("source"), |nodes| {
                *nodes = vec![CodeNode::Literal("repaired".to_string())];
            })
            .unwrap();

        assert_eq!(
            rewritten.render_standalone(&TypeScript::new(), 80).unwrap(),
            "repaired"
        );
        assert!(matches!(source.nodes.as_slice(), [CodeNode::Dedent]));
    }

    #[test]
    fn rewrite_rejects_invalid_structure_at_every_position_without_mutating_the_source() {
        let source = CodeBlock::of("original", ()).unwrap();
        for (node, expected_reason) in [
            (
                CodeNode::Dedent,
                SigilStitchError::UnbalancedIndent { depth: -1 }.to_string(),
            ),
            (
                CodeNode::Indent,
                SigilStitchError::UnbalancedIndent { depth: 1 }.to_string(),
            ),
            (
                CodeNode::InlineLiteral("%>".to_string()),
                SigilStitchError::UnresolvedIndentMarker {
                    marker: "%>".to_string(),
                    context: "%L literal".to_string(),
                }
                .to_string(),
            ),
        ] {
            for (invalid, _) in positions(node) {
                let error = source
                    .rewrite_source(&DiagnosticPath::root("source"), |nodes| {
                        *nodes = invalid.nodes;
                    })
                    .unwrap_err();
                assert!(matches!(
                    error,
                    SigilStitchError::InvalidRewrittenSource { context, reason }
                        if context == "source" && reason == expected_reason
                ));
                assert_eq!(
                    source.render_standalone(&TypeScript::new(), 80).unwrap(),
                    "original"
                );
            }
        }
    }

    #[test]
    fn rewritten_type_preconditions_preserve_the_diagnostic_path() {
        let source = CodeBlock::of("original", ()).unwrap();
        for (invalid, expected_path) in positions(CodeNode::TypeRef(TypeName::primitive(" "))) {
            let error = source
                .rewrite_source(&DiagnosticPath::root("source"), |nodes| {
                    *nodes = invalid.nodes;
                })
                .unwrap_err();
            assert!(matches!(
                error,
                SigilStitchError::InvalidTypeName { context, reason }
                    if context == expected_path && reason == "primitive spelling must not be blank"
            ));
        }
    }
}
