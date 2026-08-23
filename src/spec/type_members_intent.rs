//! Owner-level semantic evidence for cross-member validation.

use crate::error::SigilStitchError;
use crate::spec::field_spec::FieldSpec;
use crate::spec::fun_spec::FunSpec;
use crate::spec::modifiers::TypeKind;
use crate::spec::property_spec::PropertySpec;

/// Read-only semantic members owned by one complete type declaration.
///
/// `TypeMembersIntent` exists only for relationships that cannot be validated
/// from one member family in isolation, such as target-derived name
/// collisions. It carries no target grammar and does not participate in
/// lowering.
#[derive(Debug, Clone, Copy)]
pub struct TypeMembersIntent<'a> {
    owner_name: &'a str,
    owner_kind: TypeKind,
    fields: &'a [FieldSpec],
    properties: &'a [PropertySpec],
    methods: &'a [FunSpec],
}

impl<'a> TypeMembersIntent<'a> {
    pub(crate) fn new(
        owner_name: &'a str,
        owner_kind: TypeKind,
        fields: &'a [FieldSpec],
        properties: &'a [PropertySpec],
        methods: &'a [FunSpec],
    ) -> Self {
        Self {
            owner_name,
            owner_kind,
            fields,
            properties,
            methods,
        }
    }

    /// Name of the declaration that owns these members.
    pub fn owner_name(self) -> &'a str {
        self.owner_name
    }

    /// Kind of declaration that owns these members.
    pub fn owner_kind(self) -> TypeKind {
        self.owner_kind
    }

    /// Semantic fields in declaration order.
    pub fn fields(self) -> &'a [FieldSpec] {
        self.fields
    }

    /// Computed properties in declaration order.
    pub fn properties(self) -> &'a [PropertySpec] {
        self.properties
    }

    /// Explicit methods in declaration order.
    pub fn methods(self) -> &'a [FunSpec] {
        self.methods
    }

    pub(crate) fn collect_intrinsic_validation_errors(self, errors: &mut Vec<SigilStitchError>) {
        let mut seen_names = std::collections::HashSet::new();
        let mut reported_names = std::collections::HashSet::new();
        for property in self.properties {
            if !seen_names.insert(property.name()) && reported_names.insert(property.name()) {
                errors.push(SigilStitchError::DuplicatePropertyName {
                    type_name: self.owner_name.to_string(),
                    property_name: property.name().to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::type_name::TypeName;

    #[test]
    fn exposes_complete_semantic_member_evidence() {
        let fields = vec![FieldSpec::of("stored", TypeName::primitive("String"))];
        let properties = vec![
            PropertySpec::builder("value", TypeName::primitive("String"))
                .getter(CodeBlock::of("return stored", ()).unwrap())
                .build()
                .unwrap(),
        ];
        let methods = vec![
            FunSpec::builder("reset")
                .body(CodeBlock::of("stored = value", ()).unwrap())
                .build()
                .unwrap(),
        ];

        let intent =
            TypeMembersIntent::new("Values", TypeKind::Class, &fields, &properties, &methods);

        assert_eq!(intent.owner_name(), "Values");
        assert_eq!(intent.owner_kind(), TypeKind::Class);
        assert_eq!(intent.fields()[0].name(), "stored");
        assert_eq!(intent.properties()[0].name(), "value");
        assert_eq!(intent.methods()[0].name(), "reset");
    }
}
