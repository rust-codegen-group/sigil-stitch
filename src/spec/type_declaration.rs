//! Shared semantic capability derivation for concrete type declarations.

use super::where_spec::{GenericParamView, WhereConstraint};
use crate::lang::capability::TypeDeclarationCapability;

pub(crate) fn requested_capabilities(
    parameters: &[GenericParamView<'_>],
    constraints: &[WhereConstraint],
    has_annotations: bool,
) -> Vec<TypeDeclarationCapability> {
    let mut requested = Vec::new();
    if !parameters.is_empty() {
        requested.push(TypeDeclarationCapability::ParametricPolymorphism);
    }
    if !constraints.is_empty()
        || parameters.iter().any(|parameter| {
            !parameter.bounds().is_empty() || !parameter.context_bounds().is_empty()
        })
    {
        requested.push(TypeDeclarationCapability::BoundedPolymorphism);
    }
    if parameters
        .iter()
        .any(|parameter| parameter.has_constructor_kind())
    {
        requested.push(TypeDeclarationCapability::HigherKindedPolymorphism);
    }
    if has_annotations {
        requested.push(TypeDeclarationCapability::Attributes);
    }
    requested
}
