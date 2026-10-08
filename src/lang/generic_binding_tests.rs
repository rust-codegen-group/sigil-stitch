//! Adapter-local representability checks independent of capability gating.

use crate::prelude::*;
use crate::spec::type_spec::TypeIntent;

#[test]
fn native_generic_validators_preserve_type_kinds_and_reject_named_kinds() {
    let languages: Vec<(&str, Box<dyn super::CodeLang>)> = vec![
        ("csharp", Box::new(super::csharp::CSharp::new())),
        ("dart", Box::new(super::dart::Dart::new())),
        ("go", Box::new(super::go::Go::new())),
        ("java", Box::new(super::java::Java::new())),
        ("kotlin", Box::new(super::kotlin::Kotlin::new())),
        ("ocaml", Box::new(super::ocaml::OCaml::new())),
        ("python", Box::new(super::python::Python::new())),
        ("rust", Box::new(super::rust::Rust::new())),
        ("swift", Box::new(super::swift::Swift::new())),
    ];
    for (id, lang) in languages {
        for (kind, accepted) in [
            (KindExpr::Type, true),
            (KindExpr::Named(TypeName::primitive("Nat")), false),
        ] {
            let binding = GenericParamSpec::new(
                if id == "ocaml" { "t" } else { "T" },
                GenericParamDomain::Single { kind: Some(kind) },
            )
            .unwrap();
            let kind = match id {
                "go" | "ocaml" | "rust" => TypeKind::Struct,
                _ => TypeKind::Class,
            };
            let name = if id == "ocaml" { "box" } else { "Box" };
            let ty = TypeSpec::builder(name, kind)
                .add_generic_param(binding.clone())
                .add_field(
                    FieldSpec::builder(
                        "value",
                        TypeName::parameter(if id == "ocaml" { "t" } else { "T" }),
                    )
                    .build()
                    .unwrap(),
                )
                .build()
                .unwrap();
            let result = lang.validate_type(TypeIntent::new(&ty));
            assert_eq!(result.is_ok(), accepted, "{id}: {result:?}");
            if ["ocaml", "python"].contains(&id) {
                continue;
            }
            let function = FunSpec::builder("work")
                .add_generic_param(binding)
                .build()
                .unwrap();
            let intent = function
                .intent_in_type(lang.as_ref(), DeclarationContext::Member, "Box")
                .unwrap();
            let result = lang.validate_function(intent);
            assert_eq!(result.is_ok(), accepted, "{id}: {result:?}");
        }
    }
}

#[test]
fn cpp_type_templates_require_one_final_pack() {
    let lang = crate::lang::cpp::Cpp::new();
    for (bindings, accepted) in [
        (vec![GenericParamSpec::pack("Ts").unwrap()], true),
        (
            vec![
                GenericParamSpec::pack("Ts").unwrap(),
                GenericParamSpec::single("T").unwrap(),
            ],
            false,
        ),
        (
            vec![
                GenericParamSpec::pack("Ts").unwrap(),
                GenericParamSpec::pack("Us").unwrap(),
            ],
            false,
        ),
        (
            vec![
                GenericParamSpec::new(
                    "T",
                    GenericParamDomain::Single {
                        kind: Some(KindExpr::Named(TypeName::primitive("Nat"))),
                    },
                )
                .unwrap(),
            ],
            false,
        ),
    ] {
        let mut builder = TypeSpec::builder("Bundle", TypeKind::Struct);
        for binding in bindings {
            builder = builder.add_generic_param(binding);
        }
        let ty = builder.build().unwrap();
        let result = lang.validate_type(TypeIntent::new(&ty));
        assert_eq!(result.is_ok(), accepted, "{result:?}");
    }
}

#[test]
fn cpp_function_templates_reject_bound_annotations_and_virtual_members() {
    let lang = super::cpp::Cpp::new();
    for binding in [
        GenericParamSpec::single("class").unwrap(),
        GenericParamSpec::single("T")
            .unwrap()
            .with_bound(TypeName::primitive("Clone"))
            .unwrap(),
        GenericParamSpec::single("T")
            .unwrap()
            .with_context_bound(TypeName::primitive("Show"))
            .unwrap(),
        GenericParamSpec::new(
            "T",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Named(TypeName::primitive("Nat"))),
            },
        )
        .unwrap(),
    ] {
        let function = FunSpec::builder("work")
            .add_generic_param(binding)
            .build()
            .unwrap();
        let intent = function
            .intent_in_type(&lang, DeclarationContext::Member, "Box")
            .unwrap();
        assert!(
            lang.validate_function(intent)
                .unwrap_err()
                .to_string()
                .contains("without kind or bound annotations")
        );
    }
    for builder in [
        FunSpec::builder("work").is_abstract(),
        FunSpec::builder("work").is_override(),
    ] {
        let function = builder
            .add_generic_param(GenericParamSpec::single("T").unwrap())
            .build()
            .unwrap();
        let intent = function
            .intent_in_type(&lang, DeclarationContext::Member, "Box")
            .unwrap();
        assert!(
            lang.validate_function(intent)
                .unwrap_err()
                .to_string()
                .contains("cannot be virtual or overriding")
        );
    }
    let function = FunSpec::builder("work")
        .add_generic_param(GenericParamSpec::single("T").unwrap())
        .add_where_constraint(TypeName::parameter("T"), vec![TypeName::primitive("Clone")])
        .build()
        .unwrap();
    let intent = function
        .intent_in_type(&lang, DeclarationContext::Member, "Box")
        .unwrap();
    assert!(
        lang.validate_function(intent)
            .unwrap_err()
            .to_string()
            .contains("target-specific source")
    );
}

#[test]
fn haskell_and_scala_local_validators_reject_pack_and_lifetime_domains() {
    for binding in [
        GenericParamSpec::pack("Ts").unwrap(),
        GenericParamSpec::lifetime("'a").unwrap(),
    ] {
        let function = FunSpec::builder("work")
            .add_generic_param(binding)
            .build()
            .unwrap();
        let parameter = function.generic_params().next().unwrap();
        assert!(
            super::haskell::validate_generic_domain(&parameter)
                .unwrap_err()
                .to_string()
                .contains("do not express")
        );
        assert!(
            super::scala::validate_generic_domain(&parameter)
                .unwrap_err()
                .to_string()
                .contains("do not express")
        );
    }
}
