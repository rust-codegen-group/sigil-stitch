use crate::code_block::{
    CodeBlock, validate_balanced_indent_markers, validate_no_unresolved_indent_markers,
};
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::import::validate_module_path;
use crate::type_name::TypeName;
use crate::type_name_lowering::DiagnosticPath;

pub(crate) fn validate_type_name(
    type_name: &TypeName,
    path: &DiagnosticPath,
) -> Result<(), SigilStitchError> {
    fn invalid(path: &DiagnosticPath, reason: impl Into<String>) -> SigilStitchError {
        SigilStitchError::InvalidTypeName {
            context: path.to_string(),
            reason: reason.into(),
        }
    }

    fn non_blank(
        value: &str,
        path: &DiagnosticPath,
        subject: &str,
    ) -> Result<(), SigilStitchError> {
        if value.trim().is_empty() {
            return Err(invalid(path, format!("{subject} must not be blank")));
        }
        Ok(())
    }

    fn no_controls(
        value: &str,
        path: &DiagnosticPath,
        subject: &str,
    ) -> Result<(), SigilStitchError> {
        if value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
        {
            return Err(invalid(
                path,
                format!("{subject} must not contain control characters"),
            ));
        }
        Ok(())
    }

    match type_name {
        TypeName::Importable {
            module,
            name,
            alias,
            ..
        } => {
            validate_module_path(module).map_err(|error| invalid(path, error.to_string()))?;
            non_blank(name, path, "imported name")?;
            no_controls(name, path, "imported name")?;
            if let Some(alias) = alias {
                non_blank(alias, path, "preferred alias")?;
                no_controls(alias, path, "preferred alias")?;
            }
        }
        TypeName::Primitive(name) => {
            non_blank(name, path, "primitive spelling")?;
            if name.contains(['\r', '\n', '\0', '\u{2028}', '\u{2029}'])
                || name
                    .chars()
                    .any(|character| character.is_control() && !character.is_whitespace())
            {
                return Err(invalid(
                    path,
                    "primitive spelling contains an invalid control character or line break",
                ));
            }
        }
        TypeName::Raw(raw) => {
            non_blank(raw, path, "raw spelling")?;
            if raw.contains('\0')
                || raw
                    .chars()
                    .any(|character| character.is_control() && !character.is_whitespace())
            {
                return Err(invalid(
                    path,
                    "raw spelling contains an invalid control character",
                ));
            }
        }
        TypeName::StringLiteral(_) => {}
        TypeName::Array(inner) => validate_type_name(inner, &path.child("array.inner"))?,
        TypeName::ReadonlyArray(inner) => {
            validate_type_name(inner, &path.child("readonly_array.inner"))?
        }
        TypeName::Pointer(inner) => validate_type_name(inner, &path.child("pointer.inner"))?,
        TypeName::Slice(inner) => validate_type_name(inner, &path.child("slice.inner"))?,
        TypeName::Optional(inner) => validate_type_name(inner, &path.child("optional.inner"))?,
        TypeName::Reference {
            inner, lifetime, ..
        } => {
            validate_type_name(inner, &path.child("reference.inner"))?;
            if let Some(lifetime) = lifetime {
                non_blank(lifetime, path, "lifetime")?;
                no_controls(lifetime, path, "lifetime")?;
            }
        }
        TypeName::Generic { base, params } => {
            if params.is_empty() {
                return Err(invalid(path, "generic parameters must not be empty"));
            }
            validate_type_name(base, &path.child("generic.base"))?;
            for (index, parameter) in params.iter().enumerate() {
                validate_type_name(parameter, &path.indexed("generic.params", index))?;
            }
        }
        TypeName::Union(members) => {
            validate_non_empty_types(members, path, "union", "union")?;
        }
        TypeName::Intersection(members) => {
            validate_non_empty_types(members, path, "intersection", "intersection")?;
        }
        TypeName::Tuple(elements) => {
            for (index, element) in elements.iter().enumerate() {
                validate_type_name(element, &path.indexed("tuple", index))?;
            }
        }
        TypeName::Map { key, value } => {
            validate_type_name(key, &path.child("map.key"))?;
            validate_type_name(value, &path.child("map.value"))?;
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            for (index, parameter) in params.iter().enumerate() {
                validate_type_name(parameter, &path.indexed("function.params", index))?;
            }
            validate_type_name(return_type, &path.child("function.return"))?;
        }
        TypeName::AssociatedType {
            base,
            qualifier,
            member,
        } => {
            validate_type_name(base, &path.child("associated.base"))?;
            if let Some(qualifier) = qualifier {
                validate_type_name(qualifier, &path.child("associated.qualifier"))?;
            }
            non_blank(member, path, "associated member")?;
            no_controls(member, path, "associated member")?;
        }
        TypeName::ImplTrait { bounds } => {
            validate_non_empty_types(bounds, path, "impl_trait", "bounds")?;
        }
        TypeName::DynTrait { bounds } => {
            validate_non_empty_types(bounds, path, "dyn_trait", "bounds")?;
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            if upper_bound.is_some() && lower_bound.is_some() {
                return Err(invalid(
                    path,
                    "wildcard cannot have both upper and lower bounds",
                ));
            }
            if let Some(bound) = upper_bound {
                validate_type_name(bound, &path.child("wildcard.upper"))?;
            }
            if let Some(bound) = lower_bound {
                validate_type_name(bound, &path.child("wildcard.lower"))?;
            }
        }
    }

    Ok(())
}

fn validate_non_empty_types(
    values: &[TypeName],
    path: &DiagnosticPath,
    edge: &str,
    subject: &str,
) -> Result<(), SigilStitchError> {
    if values.is_empty() {
        return Err(SigilStitchError::InvalidTypeName {
            context: path.to_string(),
            reason: format!("{subject} must not be empty"),
        });
    }
    for (index, value) in values.iter().enumerate() {
        validate_type_name(value, &path.indexed(edge, index))?;
    }
    Ok(())
}

pub(crate) fn validate_rewritten_block(
    block: &CodeBlock,
    path: &DiagnosticPath,
) -> Result<(), SigilStitchError> {
    validate_balanced_indent_markers(&block.nodes).map_err(|error| {
        SigilStitchError::InvalidRewrittenSource {
            context: path.to_string(),
            reason: error.to_string(),
        }
    })?;
    validate_no_unresolved_indent_markers(&block.nodes).map_err(|error| {
        SigilStitchError::InvalidRewrittenSource {
            context: path.to_string(),
            reason: error.to_string(),
        }
    })?;
    validate_source_nodes(&block.nodes, path)
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

#[expect(
    deprecated,
    reason = "validator must classify frozen node variants exhaustively"
)]
pub(crate) fn validate_lowered_block(
    block: &CodeBlock,
    language: &str,
    path: &DiagnosticPath,
) -> Result<(), SigilStitchError> {
    fn invalid(
        language: &str,
        path: &DiagnosticPath,
        reason: impl Into<String>,
    ) -> SigilStitchError {
        SigilStitchError::InvalidTypeNameLowering {
            language: language.to_string(),
            context: path.to_string(),
            reason: reason.into(),
        }
    }

    if block.is_empty() {
        return Err(invalid(language, path, "output must not be empty"));
    }
    validate_balanced_indent_markers(&block.nodes)
        .map_err(|error| invalid(language, path, error.to_string()))?;
    validate_no_unresolved_indent_markers(&block.nodes)
        .map_err(|error| invalid(language, path, error.to_string()))?;

    fn walk(
        nodes: &[CodeNode],
        language: &str,
        path: &DiagnosticPath,
    ) -> Result<(), SigilStitchError> {
        for (index, node) in nodes.iter().enumerate() {
            let node_path = path.node(index);
            match node {
                CodeNode::Literal(text) | CodeNode::InlineLiteral(text) => {
                    if text.contains(['\r', '\n', '\0'])
                        || text
                            .chars()
                            .any(|character| character.is_control() && !character.is_whitespace())
                    {
                        return Err(invalid(
                            language,
                            &node_path,
                            "text contains an invalid control character",
                        ));
                    }
                }
                CodeNode::NameRef(name) => {
                    if name.trim().is_empty() || name.chars().any(char::is_control) {
                        return Err(invalid(
                            language,
                            &node_path,
                            "semantic name must be non-blank and control-free",
                        ));
                    }
                }
                CodeNode::StringLit(_)
                | CodeNode::SoftBreak
                | CodeNode::Indent
                | CodeNode::Dedent => {}
                CodeNode::TypeRef(type_name) => {
                    validate_type_name(type_name, &node_path)?;
                    match type_name {
                        TypeName::Primitive(_) | TypeName::Raw(_) => {}
                        TypeName::Importable {
                            qualified: false, ..
                        } => {}
                        _ => {
                            return Err(invalid(
                                language,
                                &node_path,
                                "only primitive, raw, and unqualified importable terminal references may remain",
                            ));
                        }
                    }
                }
                CodeNode::Nested(block) => {
                    if block.is_empty() {
                        return Err(invalid(
                            language,
                            &node_path,
                            "nested block must not be empty",
                        ));
                    }
                    walk(&block.nodes, language, &path.nested(index))?;
                }
                CodeNode::Sequence(children) => {
                    if children.is_empty() {
                        return Err(invalid(language, &node_path, "sequence must not be empty"));
                    }
                    walk(children, language, &path.sequence(index))?;
                }
                CodeNode::VerbatimStr(_)
                | CodeNode::Comment(_)
                | CodeNode::Attribute(_)
                | CodeNode::StatementBegin
                | CodeNode::StatementEnd
                | CodeNode::Newline
                | CodeNode::BlockOpen(_)
                | CodeNode::BlockClose(_)
                | CodeNode::BranchClose(_)
                | CodeNode::BlockOpenIntent { .. }
                | CodeNode::BlockCloseIntent { .. }
                | CodeNode::BranchCloseIntent { .. } => {
                    return Err(invalid(
                        language,
                        &node_path,
                        "node is not valid inside one type expression",
                    ));
                }
            }
        }
        Ok(())
    }

    walk(&block.nodes, language, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_node::BlockIntent;

    fn block(nodes: Vec<CodeNode>) -> CodeBlock {
        CodeBlock { nodes }
    }

    fn validate(block: &CodeBlock) -> Result<(), SigilStitchError> {
        validate_lowered_block(block, "test", &DiagnosticPath::root("root"))
    }

    #[expect(
        deprecated,
        reason = "validator contract includes frozen node variants"
    )]
    fn representative_nodes() -> Vec<CodeNode> {
        vec![
            CodeNode::Literal("literal".to_string()),
            CodeNode::TypeRef(TypeName::primitive("u32")),
            CodeNode::NameRef("Name".to_string()),
            CodeNode::StringLit("value".to_string()),
            CodeNode::VerbatimStr("value".to_string()),
            CodeNode::InlineLiteral("inline".to_string()),
            CodeNode::Nested(block(vec![CodeNode::Literal("nested".to_string())])),
            CodeNode::Comment("comment".to_string()),
            CodeNode::Attribute("attribute".to_string()),
            CodeNode::SoftBreak,
            CodeNode::Indent,
            CodeNode::Dedent,
            CodeNode::StatementBegin,
            CodeNode::StatementEnd,
            CodeNode::Newline,
            CodeNode::BlockOpen("if value".to_string()),
            CodeNode::BlockClose("if value".to_string()),
            CodeNode::BranchClose("else".to_string()),
            CodeNode::BlockOpenIntent {
                condition: "if value".to_string(),
                intent: BlockIntent::If,
            },
            CodeNode::BlockCloseIntent {
                condition: "if value".to_string(),
                intent: BlockIntent::If,
            },
            CodeNode::BranchCloseIntent {
                condition: "else".to_string(),
                intent: BlockIntent::Else,
            },
            CodeNode::Sequence(vec![CodeNode::Literal("sequence".to_string())]),
        ]
    }

    #[expect(
        deprecated,
        reason = "validator contract classifies frozen node variants"
    )]
    fn allowed_inside_type_expression(node: &CodeNode) -> bool {
        match node {
            CodeNode::Literal(_)
            | CodeNode::TypeRef(_)
            | CodeNode::NameRef(_)
            | CodeNode::StringLit(_)
            | CodeNode::InlineLiteral(_)
            | CodeNode::Nested(_)
            | CodeNode::SoftBreak
            | CodeNode::Indent
            | CodeNode::Dedent
            | CodeNode::Sequence(_) => true,
            CodeNode::VerbatimStr(_)
            | CodeNode::Comment(_)
            | CodeNode::Attribute(_)
            | CodeNode::StatementBegin
            | CodeNode::StatementEnd
            | CodeNode::Newline
            | CodeNode::BlockOpen(_)
            | CodeNode::BlockClose(_)
            | CodeNode::BranchClose(_)
            | CodeNode::BlockOpenIntent { .. }
            | CodeNode::BlockCloseIntent { .. }
            | CodeNode::BranchCloseIntent { .. } => false,
        }
    }

    #[test]
    fn every_code_node_variant_has_an_explicit_lowered_output_classification() {
        for node in representative_nodes() {
            if matches!(node, CodeNode::Indent | CodeNode::Dedent) {
                continue;
            }
            let allowed = allowed_inside_type_expression(&node);
            let result = validate(&block(vec![node]));
            assert_eq!(
                result.is_ok(),
                allowed,
                "node classification disagrees with lowered-output validation: {result:?}"
            );
        }

        assert!(validate(&block(vec![CodeNode::Indent, CodeNode::Dedent])).is_ok());
    }

    #[test]
    fn lowered_output_rejects_empty_unbalanced_and_empty_sequence_blocks() {
        for invalid in [
            block(vec![]),
            block(vec![CodeNode::Indent]),
            block(vec![CodeNode::Sequence(vec![])]),
            block(vec![
                CodeNode::Literal("value".to_string()),
                CodeNode::Nested(block(vec![])),
            ]),
        ] {
            assert!(matches!(
                validate(&invalid),
                Err(SigilStitchError::InvalidTypeNameLowering { .. })
            ));
        }
    }

    #[test]
    fn lowered_output_accepts_only_terminal_type_references() {
        let terminals = [
            TypeName::primitive("u32"),
            TypeName::raw("Target.Type"),
            TypeName::importable("module", "Imported"),
        ];
        for terminal in terminals {
            assert!(validate(&block(vec![CodeNode::TypeRef(terminal)])).is_ok());
        }

        let non_terminals = [
            TypeName::array(TypeName::primitive("u32")),
            TypeName::qualified("module", "Qualified"),
        ];
        for non_terminal in non_terminals {
            assert!(matches!(
                validate(&block(vec![CodeNode::TypeRef(non_terminal)])),
                Err(SigilStitchError::InvalidTypeNameLowering { reason, .. })
                    if reason.contains("terminal references")
            ));
        }
    }

    #[test]
    fn lowered_output_rejects_invalid_text_and_names_but_accepts_string_data() {
        for invalid in [
            CodeNode::Literal("line\nbreak".to_string()),
            CodeNode::InlineLiteral("nul\0byte".to_string()),
            CodeNode::NameRef(" \t".to_string()),
        ] {
            assert!(matches!(
                validate(&block(vec![invalid])),
                Err(SigilStitchError::InvalidTypeNameLowering { .. })
            ));
        }

        assert!(
            validate(&block(vec![CodeNode::StringLit(
                "line\n\0value".to_string()
            )]))
            .is_ok()
        );
    }
}
