//! Owner-wide validation across semantic member families.

pub(crate) mod kotlin;
pub(crate) mod php;
pub(crate) mod scala;
pub(crate) mod swift;
pub(crate) mod typescript;

use crate::error::SigilStitchError;

pub(crate) fn validation_result(
    collect: impl FnOnce(&mut Vec<SigilStitchError>),
) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect(&mut errors);
    match errors.into_iter().next() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::code_block::CodeBlock;
    use crate::error::SigilStitchError;
    use crate::lang::{CodeLang, TypeMembersIntent};
    use crate::spec::field_spec::FieldSpec;
    use crate::spec::modifiers::TypeKind;
    use crate::spec::property_spec::PropertySpec;
    use crate::type_name::TypeName;

    #[test]
    fn direct_builtin_hooks_report_owner_wide_collisions() {
        let fields = [FieldSpec::of("value", TypeName::primitive("Value"))];
        let properties = [PropertySpec::builder("value", TypeName::primitive("Value"))
            .getter(CodeBlock::of("return value", ()).unwrap())
            .build()
            .unwrap()];
        let methods = [];
        let members =
            TypeMembersIntent::new("Values", TypeKind::Class, &fields, &properties, &methods);

        for lang in [
            Box::new(crate::lang::kotlin::Kotlin::new()) as Box<dyn CodeLang>,
            Box::new(crate::lang::scala::Scala::new()),
            Box::new(crate::lang::swift::Swift::new()),
            Box::new(crate::lang::typescript::TypeScript::new()),
        ] {
            assert!(matches!(
                lang.validate_type_members(members),
                Err(SigilStitchError::TypeMemberNameCollision {
                    member_name,
                    ..
                }) if member_name == "value"
            ));
        }
    }
}
