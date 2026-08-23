//! Scala owner-wide member validation.

use std::collections::{HashMap, HashSet};

use crate::error::{SigilStitchError, TypeMemberNameOrigin};
use crate::lang::RendererLang;
use crate::lang::scala::Scala;
use crate::spec::type_members_intent::TypeMembersIntent;

#[derive(Debug, Clone)]
struct SeenMember {
    emitted_name: String,
    origin: TypeMemberNameOrigin,
}

pub(crate) fn validate(
    lang: &Scala,
    members: TypeMembersIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, members, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Scala,
    members: TypeMembersIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    let mut seen = HashMap::new();
    for field in members.fields() {
        insert_first(
            &mut seen,
            field.name(),
            TypeMemberNameOrigin::StoredField {
                field_name: field.name().to_string(),
            },
        );
        if !field.modifiers().is_readonly {
            insert_first(
                &mut seen,
                &format!("{}_=", field.name()),
                TypeMemberNameOrigin::StoredField {
                    field_name: field.name().to_string(),
                },
            );
        }
    }

    let mut seen_properties = HashSet::new();
    for property in members.properties() {
        if property.getter().is_some() && seen_properties.insert((property.name(), false)) {
            insert_or_report(
                lang,
                members,
                &mut seen,
                property.name(),
                TypeMemberNameOrigin::PropertyReadAccessor {
                    property_name: property.name().to_string(),
                },
                errors,
            );
        }
        if property.setter().is_some() && seen_properties.insert((property.name(), true)) {
            let setter_name = format!("{}_=", property.name());
            insert_or_report(
                lang,
                members,
                &mut seen,
                &setter_name,
                TypeMemberNameOrigin::PropertyWriteAccessor {
                    property_name: property.name().to_string(),
                },
                errors,
            );
        }
    }

    for method in members.methods() {
        if let Some(first) = seen.get(method.name()) {
            report(
                lang,
                members,
                first,
                TypeMemberNameOrigin::ExplicitMethod {
                    method_name: method.name().to_string(),
                },
                errors,
            );
        }
    }
}

fn insert_first(seen: &mut HashMap<String, SeenMember>, name: &str, origin: TypeMemberNameOrigin) {
    seen.entry(name.to_string()).or_insert_with(|| SeenMember {
        emitted_name: name.to_string(),
        origin,
    });
}

fn insert_or_report(
    lang: &Scala,
    members: TypeMembersIntent<'_>,
    seen: &mut HashMap<String, SeenMember>,
    name: &str,
    origin: TypeMemberNameOrigin,
    errors: &mut Vec<SigilStitchError>,
) {
    if let Some(first) = seen.get(name) {
        report(lang, members, first, origin, errors);
    } else {
        insert_first(seen, name, origin);
    }
}

fn report(
    lang: &Scala,
    members: TypeMembersIntent<'_>,
    first: &SeenMember,
    second: TypeMemberNameOrigin,
    errors: &mut Vec<SigilStitchError>,
) {
    errors.push(SigilStitchError::TypeMemberNameCollision {
        language: lang.file_extension().to_string(),
        type_name: members.owner_name().to_string(),
        member_name: first.emitted_name.clone(),
        first_member: Box::new(first.origin.clone()),
        second_member: Box::new(second),
    });
}
