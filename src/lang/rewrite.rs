use crate::code_node::CodeNode;

/// Recursively apply a rewrite function to all node vectors in the tree.
///
/// Visits the given `nodes` first, then recurses into any `Nested(CodeBlock)`
/// or `Sequence(Vec<CodeNode>)` children.
pub fn walk_nodes_mut(nodes: &mut Vec<CodeNode>, f: &dyn Fn(&mut Vec<CodeNode>)) {
    f(nodes);
    for node in nodes.iter_mut() {
        match node {
            CodeNode::Nested(block) => walk_nodes_mut(&mut block.nodes, f),
            CodeNode::Sequence(children) => walk_nodes_mut(children, f),
            _ => {}
        }
    }
}
