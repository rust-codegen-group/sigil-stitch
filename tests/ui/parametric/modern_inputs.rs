#![deny(warnings)]

use sigil_stitch::prelude::*;

fn main() {
    let binding = GenericParamSpec::single("T").unwrap();
    let value = TypeName::parameter("T");
    let application = TypeName::application(
        TypeName::primitive("Container"),
        vec![TypeArgument::Single(value.clone())],
    );
    let callable = TypeName::callable(
        vec![CallableParam::Single {
            name: Some("value".into()),
            type_name: application,
            presence: CallableParamPresence::Required,
        }],
        value,
    );
    let _ = FunSpec::builder("work")
        .add_generic_param(binding.clone())
        .returns(callable)
        .build()
        .unwrap();
    let _ = TypeSpec::builder("Owner", TypeKind::Class)
        .add_generic_param(binding)
        .build()
        .unwrap();
    let argument = TypeArgument::Single(TypeName::primitive("Int"));
    match argument {
        TypeArgument::Single(_) | TypeArgument::Expansion { .. } => {}
        _ => {}
    }
    match (GenericParamDomain::Single { kind: None }) {
        GenericParamDomain::Single { .. }
        | GenericParamDomain::Pack { .. }
        | GenericParamDomain::Lifetime => {}
        _ => {}
    }
    match KindExpr::Type {
        KindExpr::Type | KindExpr::Named(_) | KindExpr::Constructor { .. } => {}
        _ => {}
    }
    let slot = CallableParam::Single {
        name: None,
        type_name: TypeName::primitive("Int"),
        presence: CallableParamPresence::Required,
    };
    match slot {
        CallableParam::Single { .. }
        | CallableParam::Repeated { .. }
        | CallableParam::Expansion { .. } => {}
        _ => {}
    }
}
