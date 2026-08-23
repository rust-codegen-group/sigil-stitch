//! TypeScript owner-wide member validation.

use std::collections::{HashMap, HashSet};

use crate::error::{SigilStitchError, TypeMemberNameOrigin};
use crate::lang::RendererLang;
use crate::lang::field_lowering::typescript::property_key;
use crate::lang::typescript::TypeScript;
use crate::spec::type_members_intent::TypeMembersIntent;

#[derive(Debug, Clone)]
struct SeenMember {
    emitted_name: String,
    origin: TypeMemberNameOrigin,
}

type MemberKey = (bool, bool, String);

pub(crate) fn validate(
    lang: &TypeScript,
    members: TypeMembersIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, members, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &TypeScript,
    members: TypeMembersIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    let mut seen = HashMap::new();
    for field in members.fields() {
        let Some(key) = member_key(field.name(), field.modifiers().is_static) else {
            continue;
        };
        seen.entry(key).or_insert_with(|| SeenMember {
            emitted_name: field.name().to_string(),
            origin: TypeMemberNameOrigin::StoredField {
                field_name: field.name().to_string(),
            },
        });
    }

    let mut seen_properties = HashSet::new();
    for property in members.properties() {
        let Some(key) = member_key(property.name(), property.modifiers().is_static) else {
            continue;
        };
        if !seen_properties.insert((key.clone(), property.name())) {
            continue;
        }
        insert_or_report(
            lang,
            members,
            &mut seen,
            key,
            property.name(),
            property_origin(property),
            errors,
        );
    }

    for method in members.methods() {
        let Some(key) = member_key(method.name(), method.modifiers.is_static) else {
            continue;
        };
        if let Some(first) = seen.get(&key) {
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

fn insert_or_report(
    lang: &TypeScript,
    members: TypeMembersIntent<'_>,
    seen: &mut HashMap<MemberKey, SeenMember>,
    key: MemberKey,
    emitted_name: &str,
    origin: TypeMemberNameOrigin,
    errors: &mut Vec<SigilStitchError>,
) {
    if let Some(first) = seen.get(&key) {
        report(lang, members, first, origin, errors);
    } else {
        seen.insert(
            key,
            SeenMember {
                emitted_name: emitted_name.to_string(),
                origin,
            },
        );
    }
}

fn report(
    lang: &TypeScript,
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

fn member_key(name: &str, is_static: bool) -> Option<MemberKey> {
    property_key(name).map(|key| (is_static, name.starts_with('#'), key))
}

fn property_origin(property: &crate::spec::property_spec::PropertySpec) -> TypeMemberNameOrigin {
    if property.getter().is_some() {
        TypeMemberNameOrigin::PropertyReadAccessor {
            property_name: property.name().to_string(),
        }
    } else {
        TypeMemberNameOrigin::PropertyWriteAccessor {
            property_name: property.name().to_string(),
        }
    }
}
