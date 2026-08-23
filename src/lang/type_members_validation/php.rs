//! PHP owner-wide member validation.

use std::collections::{HashMap, HashSet};

use crate::error::{SigilStitchError, TypeMemberNameOrigin};
use crate::lang::RendererLang;
use crate::lang::php::Php;
use crate::lang::property_lowering::php::accessor_name;
use crate::spec::type_members_intent::TypeMembersIntent;

#[derive(Debug)]
struct SeenMember {
    emitted_name: String,
    origin: TypeMemberNameOrigin,
}

#[derive(Debug, Default)]
struct SeenMembers {
    by_emitted_name: HashMap<String, SeenMember>,
    property_accessors: HashSet<(String, String)>,
}

pub(crate) fn validate(lang: &Php, members: TypeMembersIntent<'_>) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, members, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Php,
    members: TypeMembersIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    let mut seen = SeenMembers::default();

    for property in members.properties() {
        if property.getter().is_some() {
            collect_accessor(
                lang,
                members,
                &mut seen,
                &accessor_name("get", property.name()),
                property.name(),
                TypeMemberNameOrigin::PropertyReadAccessor {
                    property_name: property.name().to_string(),
                },
                errors,
            );
        }
        if property.setter().is_some() {
            collect_accessor(
                lang,
                members,
                &mut seen,
                &accessor_name("set", property.name()),
                property.name(),
                TypeMemberNameOrigin::PropertyWriteAccessor {
                    property_name: property.name().to_string(),
                },
                errors,
            );
        }
    }

    for method in members.methods() {
        let normalized = normalize_method_name(method.name());
        if let Some(first) = seen.by_emitted_name.get(&normalized) {
            errors.push(SigilStitchError::TypeMemberNameCollision {
                language: lang.file_extension().to_string(),
                type_name: members.owner_name().to_string(),
                member_name: first.emitted_name.clone(),
                first_member: Box::new(first.origin.clone()),
                second_member: Box::new(TypeMemberNameOrigin::ExplicitMethod {
                    method_name: method.name().to_string(),
                }),
            });
        }
    }
}

fn collect_accessor(
    lang: &Php,
    members: TypeMembersIntent<'_>,
    seen: &mut SeenMembers,
    emitted_name: &str,
    property_name: &str,
    origin: TypeMemberNameOrigin,
    errors: &mut Vec<SigilStitchError>,
) {
    let normalized = normalize_method_name(emitted_name);
    if !seen
        .property_accessors
        .insert((normalized.clone(), property_name.to_string()))
    {
        return;
    }
    if let Some(first) = seen.by_emitted_name.get(&normalized) {
        errors.push(SigilStitchError::TypeMemberNameCollision {
            language: lang.file_extension().to_string(),
            type_name: members.owner_name().to_string(),
            member_name: first.emitted_name.clone(),
            first_member: Box::new(first.origin.clone()),
            second_member: Box::new(origin),
        });
    } else {
        seen.by_emitted_name.insert(
            normalized,
            SeenMember {
                emitted_name: emitted_name.to_string(),
                origin,
            },
        );
    }
}

fn normalize_method_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::spec::modifiers::TypeKind;
    use crate::spec::property_spec::PropertySpec;
    use crate::type_name::TypeName;

    fn property(name: &str) -> PropertySpec {
        PropertySpec::builder(name, TypeName::primitive("String"))
            .getter(CodeBlock::of("return current", ()).unwrap())
            .build()
            .unwrap()
    }

    #[test]
    fn single_result_validation_reports_collisions_and_accepts_unique_names() {
        let colliding = vec![property("foo"), property("Foo")];
        let intent = TypeMembersIntent::new("Values", TypeKind::Class, &[], &colliding, &[]);
        assert!(matches!(
            validate(&Php::new(), intent),
            Err(SigilStitchError::TypeMemberNameCollision { member_name, .. })
                if member_name == "getFoo"
        ));

        let unique = vec![property("foo")];
        let intent = TypeMembersIntent::new("Values", TypeKind::Class, &[], &unique, &[]);
        validate(&Php::new(), intent).unwrap();
    }
}
