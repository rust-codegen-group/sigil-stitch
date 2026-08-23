//! Kotlin owner-wide member validation.

use std::collections::{HashMap, HashSet};

use crate::error::{SigilStitchError, TypeMemberNameOrigin};
use crate::lang::RendererLang;
use crate::lang::kotlin::Kotlin;
use crate::spec::type_members_intent::TypeMembersIntent;

pub(crate) fn validate(
    lang: &Kotlin,
    members: TypeMembersIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, members, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Kotlin,
    members: TypeMembersIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    let fields: HashMap<_, _> = members
        .fields()
        .iter()
        .map(|field| (field.name(), field.name()))
        .collect();
    let mut seen_properties = HashSet::new();
    for property in members.properties() {
        if !seen_properties.insert(property.name()) {
            continue;
        }
        let Some(field_name) = fields.get(property.name()) else {
            continue;
        };
        errors.push(SigilStitchError::TypeMemberNameCollision {
            language: lang.file_extension().to_string(),
            type_name: members.owner_name().to_string(),
            member_name: property.name().to_string(),
            first_member: Box::new(TypeMemberNameOrigin::StoredField {
                field_name: (*field_name).to_string(),
            }),
            second_member: Box::new(property_origin(property)),
        });
    }
}

fn property_origin(property: &crate::spec::property_spec::PropertySpec) -> TypeMemberNameOrigin {
    TypeMemberNameOrigin::PropertyReadAccessor {
        property_name: property.name().to_string(),
    }
}
