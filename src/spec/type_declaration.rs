//! Shared semantic capability derivation for concrete type declarations.

use super::where_spec::{TypeParamSpec, WhereConstraint};
use crate::lang::capability::TypeDeclarationCapability;

pub(crate) fn requested_capabilities(
    parameters: &[TypeParamSpec],
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
        .any(|parameter| parameter.kind().is_some())
    {
        requested.push(TypeDeclarationCapability::HigherKindedPolymorphism);
    }
    if has_annotations {
        requested.push(TypeDeclarationCapability::Attributes);
    }
    requested
}
