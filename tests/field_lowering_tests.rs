#![allow(deprecated)]

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, OptionalFieldStyle,
    TypeDeclSyntaxConfig,
};
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn render_field(lang: &dyn CodeLang, field: &FieldSpec) -> Result<String, SigilStitchError> {
    field
        .emit(lang, DeclarationContext::Member)?
        .render_standalone(lang, 120)
}

fn render_type(
    lang: impl CodeLang,
    filename: &str,
    ty: TypeSpec,
) -> Result<String, SigilStitchError> {
    FileSpec::builder_with(filename, lang)
        .add_type(ty)
        .build()?
        .render(120)
}

fn field(name: &str, ty: TypeName) -> FieldSpec {
    FieldSpec::of(name, ty)
}

#[test]
fn direct_and_type_owned_field_paths_agree_on_success_and_failure() {
    let ts = sigil_stitch::lang::typescript::TypeScript::new();
    let accepted = field("name", TypeName::primitive("string"));
    let direct = render_field(&ts, &accepted).unwrap();
    let nested = render_type(
        ts,
        "owned.ts",
        TypeSpec::builder("Owned", TypeKind::Class)
            .add_field(accepted)
            .build()
            .unwrap(),
    )
    .unwrap();
    assert!(nested.contains(direct.trim()), "{nested}");

    let js = sigil_stitch::lang::javascript::JavaScript::new();
    let rejected = field("typed", TypeName::primitive("string"));
    let direct = rejected.emit(&js, DeclarationContext::Member).unwrap_err();
    let nested = TypeSpec::builder("Owned", TypeKind::Class)
        .add_field(rejected)
        .build()
        .unwrap()
        .validate(&js)
        .unwrap_err();
    assert_eq!(
        std::mem::discriminant(&direct),
        std::mem::discriminant(&nested)
    );
}

#[test]
fn strict_built_ins_reject_ownerless_top_level_fields() {
    let field = field("name", TypeName::primitive("string"));
    let error = field
        .emit(
            &sigil_stitch::lang::typescript::TypeScript::new(),
            DeclarationContext::TopLevel,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFieldContext { .. }
    ));
}

#[test]
fn field_lowering_preserves_direct_and_pretty_renderer_parity() {
    let lang = sigil_stitch::lang::typescript::TypeScript::new();
    let direct = FieldSpec::builder("value", TypeName::primitive("Result"))
        .initializer(CodeBlock::of("call(alpha, beta)", ()).unwrap())
        .build()
        .unwrap();
    let pretty = FieldSpec::builder("value", TypeName::primitive("Result"))
        .initializer(CodeBlock::of("call(alpha,%Wbeta)", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render_field(&lang, &direct).unwrap(),
        render_field(&lang, &pretty).unwrap()
    );
}

#[test]
fn deserialized_field_intrinsics_fail_closed_and_aggregate() {
    let empty_block: CodeBlock =
        serde_json::from_value(serde_json::json!({ "nodes": [] })).unwrap();
    let nested_empty: CodeBlock = serde_json::from_value(serde_json::json!({
        "nodes": [{ "Nested": { "nodes": [] } }]
    }))
    .unwrap();

    let mut invalid_name =
        serde_json::to_value(field("valid", TypeName::primitive("String"))).unwrap();
    invalid_name["name"] = serde_json::json!("");
    invalid_name["modifiers"]["is_async"] = serde_json::json!(true);
    invalid_name["initializer"] = serde_json::to_value(empty_block).unwrap();
    let invalid_name: FieldSpec = serde_json::from_value(invalid_name).unwrap();

    let mut invalid_nested =
        serde_json::to_value(field("other", TypeName::primitive("String"))).unwrap();
    invalid_nested["initializer"] = serde_json::to_value(nested_empty).unwrap();
    let invalid_nested: FieldSpec = serde_json::from_value(invalid_nested).unwrap();

    let file = FileSpec::builder_with("invalid.rs", sigil_stitch::lang::rust::Rust::new())
        .add_type(
            TypeSpec::builder("Broken", TypeKind::Struct)
                .add_field(invalid_name)
                .add_field(invalid_nested)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = file.validate().unwrap_err()
    else {
        panic!("expected aggregate field validation")
    };
    assert!(error_count >= 4, "{errors:#?}");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, SigilStitchError::EmptyName { .. }))
    );
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, SigilStitchError::InvalidFieldModifiers { .. }))
    );
    assert!(
        errors
            .iter()
            .filter(|error| matches!(error, SigilStitchError::EmptyFieldOperand { .. }))
            .count()
            >= 2
    );
}

#[test]
fn escaped_and_normalized_field_name_collisions_are_rejected() {
    for (lang, first, second) in [
        (
            Box::new(sigil_stitch::lang::haskell::Haskell::new()) as Box<dyn CodeLang>,
            "type",
            "type'",
        ),
        (
            Box::new(sigil_stitch::lang::ocaml::OCaml::new()) as Box<dyn CodeLang>,
            "type",
            "type_",
        ),
        (
            Box::new(sigil_stitch::lang::rust::Rust::new()) as Box<dyn CodeLang>,
            "Å",
            "Å",
        ),
        (
            Box::new(sigil_stitch::lang::python::Python::new()) as Box<dyn CodeLang>,
            "A",
            "Ａ",
        ),
    ] {
        let ty = TypeSpec::builder("Collision", TypeKind::Struct)
            .add_field(field(first, TypeName::primitive("Value")))
            .add_field(field(second, TypeName::primitive("Value")))
            .build()
            .unwrap();
        assert!(
            matches!(
                ty.validate(lang.as_ref()),
                Err(SigilStitchError::InvalidField { .. })
            ),
            ".{}",
            lang.file_extension()
        );
    }
}

#[test]
fn python_normalizes_after_keyword_escaping() {
    let ty = TypeSpec::builder("Names", TypeKind::Class)
        .add_field(field("class", TypeName::primitive("str")))
        .add_field(field("ｃｌａｓｓ", TypeName::primitive("str")))
        .build()
        .unwrap();
    let output = render_type(sigil_stitch::lang::python::Python::new(), "names.py", ty).unwrap();
    assert!(output.contains("class_: str"), "{output}");
    assert!(output.contains("ｃｌａｓｓ: str"), "{output}");
}

#[test]
fn rust_and_python_accept_combining_mark_identifier_continuations() {
    let name = "e\u{301}";
    assert!(
        render_field(
            &sigil_stitch::lang::rust::Rust::new(),
            &field(name, TypeName::primitive("String")),
        )
        .is_ok()
    );
    assert!(
        render_field(
            &sigil_stitch::lang::python::Python::new(),
            &field(name, TypeName::primitive("str")),
        )
        .is_ok()
    );
}

#[test]
fn go_uses_its_exact_unicode_identifier_categories() {
    let go = sigil_stitch::lang::go::Go::new();
    assert!(render_field(&go, &field("Ⅰ", TypeName::primitive("int"))).is_err());
    assert!(render_field(&go, &field("名1", TypeName::primitive("int"))).is_ok());
}

#[test]
fn javascript_and_typescript_accept_ecmascript_property_names() {
    for lang in [
        Box::new(sigil_stitch::lang::javascript::JavaScript::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::typescript::TypeScript::new()) as Box<dyn CodeLang>,
    ] {
        for name in [
            "\u{037a}",
            "\u{0e33}",
            "\u{309b}",
            "\u{309c}",
            "a\u{200c}b",
            "\"foo-bar\"",
            "123",
        ] {
            let ty = if lang.file_extension() == "js" {
                TypeName::primitive("")
            } else {
                TypeName::primitive("string")
            };
            render_field(lang.as_ref(), &field(name, ty))
                .unwrap_or_else(|error| panic!(".{}, {name:?}: {error}", lang.file_extension()));
        }
    }
}

#[test]
fn javascript_class_backed_type_kinds_keep_valid_fields() {
    let lang = sigil_stitch::lang::javascript::JavaScript::new();
    for kind in [TypeKind::Interface, TypeKind::Trait, TypeKind::Enum] {
        let ty = TypeSpec::builder("Backed", kind)
            .add_field(field("value", TypeName::primitive("")))
            .build()
            .unwrap();
        assert!(ty.validate(&lang).is_ok(), "{kind:?}");
    }
}

#[test]
fn javascript_special_property_names_are_checked_canonically() {
    let js = sigil_stitch::lang::javascript::JavaScript::new();
    for name in [
        "constructor",
        "#constructor",
        "\"constructor\"",
        "\"\\x63onstructor\"",
    ] {
        assert!(
            render_field(&js, &field(name, TypeName::primitive(""))).is_err(),
            "{name}"
        );
    }
    for name in ["prototype", "\"prototype\""] {
        let field = FieldSpec::builder(name, TypeName::primitive(""))
            .is_static()
            .build()
            .unwrap();
        assert!(render_field(&js, &field).is_err(), "{name}");
    }
    let private_prototype = FieldSpec::builder("#prototype", TypeName::primitive(""))
        .is_static()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&js, &private_prototype).unwrap().trim(),
        "static #prototype;"
    );
    assert_eq!(
        render_field(&js, &field("class", TypeName::primitive("")))
            .unwrap()
            .trim(),
        "class;"
    );
}

#[test]
fn go_accepts_safe_tags_and_rejects_annotations_and_unsafe_tags() {
    let go = sigil_stitch::lang::go::Go::new();
    let plain = field("Name", TypeName::primitive("string"));
    assert_eq!(render_field(&go, &plain).unwrap().trim(), "Name string");
    let tagged = FieldSpec::builder("Name", TypeName::primitive("string"))
        .tag("json:\"name\"")
        .build()
        .unwrap();
    assert_eq!(
        render_field(&go, &tagged).unwrap().trim(),
        "Name string `json:\"name\"`"
    );
    for invalid in [
        FieldSpec::builder("Name", TypeName::primitive("string"))
            .tag("json:`name`")
            .build()
            .unwrap(),
        FieldSpec::builder("Name", TypeName::primitive("string"))
            .annotation(CodeBlock::of("raw", ()).unwrap())
            .build()
            .unwrap(),
        FieldSpec::builder("Name", TypeName::primitive("string"))
            .annotate(AnnotationSpec::new("json"))
            .build()
            .unwrap(),
    ] {
        assert!(render_field(&go, &invalid).is_err());
    }
}

#[test]
fn c_and_cpp_reject_interleaved_declarators_and_preserve_binding_constness() {
    let c = sigil_stitch::lang::c::C::new();
    let cpp = sigil_stitch::lang::cpp::Cpp::new();
    for ty in [
        TypeName::array(TypeName::primitive("int")),
        TypeName::slice(TypeName::primitive("int")),
        TypeName::function(
            vec![TypeName::primitive("int")],
            TypeName::primitive("void"),
        ),
        TypeName::pointer(TypeName::array(TypeName::primitive("int"))),
    ] {
        assert!(render_field(&c, &field("value", ty)).is_err());
    }
    let c_pointer = FieldSpec::builder("value", TypeName::pointer(TypeName::primitive("int")))
        .is_readonly()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&c, &c_pointer).unwrap().trim(),
        "int* const value;"
    );
    let cpp_pointer = FieldSpec::builder("value", TypeName::pointer(TypeName::primitive("int")))
        .is_readonly()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&cpp, &cpp_pointer).unwrap().trim(),
        "int* const value;"
    );
}

#[test]
fn c_and_cpp_preserve_valid_top_level_declarations() {
    let field = field("count", TypeName::primitive("int"));
    for lang in [
        Box::new(sigil_stitch::lang::c::C::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::cpp::Cpp::new()) as Box<dyn CodeLang>,
    ] {
        let output = field
            .emit(lang.as_ref(), DeclarationContext::TopLevel)
            .unwrap()
            .render_standalone(lang.as_ref(), 120)
            .unwrap();
        assert_eq!(output.trim(), "int count;");
    }
}

#[test]
fn c_top_level_preserves_static_initializers() {
    let field = FieldSpec::builder("count", TypeName::primitive("int"))
        .is_static()
        .initializer(CodeBlock::of("1", ()).unwrap())
        .build()
        .unwrap();
    let lang = sigil_stitch::lang::c::C::new();
    let output = field
        .emit(&lang, DeclarationContext::TopLevel)
        .unwrap()
        .render_standalone(&lang, 120)
        .unwrap();
    assert_eq!(output.trim(), "static int count = 1;");
    assert!(render_field(&lang, &field).is_err());
}

#[test]
fn cpp_preserves_pre_cpp17_static_constant_initializers() {
    let field = FieldSpec::builder("answer", TypeName::primitive("int"))
        .is_static()
        .is_readonly()
        .initializer(CodeBlock::of("42", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::cpp::Cpp::new(), &field)
            .unwrap()
            .trim(),
        "static const int answer = 42;"
    );
    let lang = sigil_stitch::lang::cpp::Cpp::new();
    let output = field
        .emit(&lang, DeclarationContext::TopLevel)
        .unwrap()
        .render_standalone(&lang, 120)
        .unwrap();
    assert_eq!(output.trim(), "static const int answer = 42;");
}

#[test]
fn cpp_rejects_static_initializers_that_need_an_unmodelled_definition() {
    let field = FieldSpec::builder("count", TypeName::primitive("int"))
        .is_static()
        .initializer(CodeBlock::of("1", ()).unwrap())
        .build()
        .unwrap();
    let error = render_field(&sigil_stitch::lang::cpp::Cpp::new(), &field).unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidField { reason, .. }
            if reason.contains("out-of-class definition")
    ));
}

#[test]
fn cpp_rejects_static_const_initializers_without_a_proven_integral_type() {
    let field_types = [
        TypeName::primitive("std::string"),
        TypeName::primitive("double"),
        TypeName::primitive("Mode"),
        TypeName::Pointer(Box::new(TypeName::primitive("int"))),
        TypeName::Reference {
            inner: Box::new(TypeName::primitive("int")),
            mutable: false,
            lifetime: None,
        },
    ];
    for field_type in field_types {
        let field = FieldSpec::builder("value", field_type)
            .is_static()
            .is_readonly()
            .initializer(CodeBlock::of("value", ()).unwrap())
            .build()
            .unwrap();
        let error = render_field(&sigil_stitch::lang::cpp::Cpp::new(), &field).unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::InvalidField { reason, .. }
                if reason.contains("proven integral type")
        ));
    }
}

#[test]
fn typescript_contract_fields_require_implicit_visibility() {
    let explicit = FieldSpec::builder("value", TypeName::primitive("string"))
        .visibility(Visibility::Public)
        .build()
        .unwrap();
    let contract = TypeSpec::builder("Contract", TypeKind::Interface)
        .add_field(explicit)
        .build()
        .unwrap();
    assert!(
        contract
            .validate(&sigil_stitch::lang::typescript::TypeScript::new())
            .is_err()
    );
}

#[test]
fn cpp_access_sections_group_and_restore_owner_defaults() {
    let ty = TypeSpec::builder("Access", TypeKind::Class)
        .add_field(field("hidden", TypeName::primitive("int")))
        .add_field(
            FieldSpec::builder("shown", TypeName::primitive("int"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = render_type(sigil_stitch::lang::cpp::Cpp::new(), "access.cpp", ty).unwrap();
    assert!(
        output.contains("int hidden;\npublic:\n    int shown;\nprivate:"),
        "{output}"
    );
}

#[test]
fn target_local_owner_and_initializer_rules_fail_closed() {
    let swift = TypeSpec::builder("ProtocolLike", TypeKind::Interface)
        .add_field(field("value", TypeName::primitive("Int")))
        .build()
        .unwrap();
    assert!(
        swift
            .validate(&sigil_stitch::lang::swift::Swift::new())
            .is_err()
    );

    let ruby = TypeSpec::builder("Record", TypeKind::Class)
        .add_field(field("value", TypeName::primitive("Object")))
        .build()
        .unwrap();
    assert!(
        ruby.validate(&sigil_stitch::lang::ruby::Ruby::new())
            .is_err()
    );

    let dart = FieldSpec::builder("value", TypeName::primitive("String"))
        .is_static()
        .is_readonly()
        .build()
        .unwrap();
    assert!(render_field(&sigil_stitch::lang::dart::Dart::new(), &dart).is_err());

    let mutable_dart = FieldSpec::builder("value", TypeName::primitive("String"))
        .is_static()
        .build()
        .unwrap();
    assert!(render_field(&sigil_stitch::lang::dart::Dart::new(), &mutable_dart).is_err());
}

#[test]
fn nullable_mutable_static_fields_keep_valid_dart_and_swift_output() {
    let dart = FieldSpec::builder("value", TypeName::optional(TypeName::primitive("String")))
        .is_static()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::dart::Dart::new(), &dart)
            .unwrap()
            .trim(),
        "static String? value;"
    );
    let untyped_dart = FieldSpec::builder("untyped", TypeName::primitive(""))
        .is_static()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::dart::Dart::new(), &untyped_dart)
            .unwrap()
            .trim(),
        "static var untyped;"
    );

    let swift = FieldSpec::builder("value", TypeName::optional(TypeName::primitive("Int")))
        .is_static()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::swift::Swift::new(), &swift)
            .unwrap()
            .trim(),
        "static var value: Int?"
    );
    let immutable_swift =
        FieldSpec::builder("value", TypeName::optional(TypeName::primitive("Int")))
            .is_static()
            .is_readonly()
            .build()
            .unwrap();
    assert!(render_field(&sigil_stitch::lang::swift::Swift::new(), &immutable_swift,).is_err());
}

#[test]
fn haskell_record_payload_has_no_trailing_comma() {
    let ty = TypeSpec::builder("Message", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Message")
                .record_payload_field(field("body", TypeName::primitive("String")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = render_type(
        sigil_stitch::lang::haskell::Haskell::new(),
        "message.hs",
        ty,
    )
    .unwrap();
    assert!(output.contains("Message { body :: String }"), "{output}");
    assert!(!output.contains("body :: String,"), "{output}");
}

#[test]
fn variant_record_payloads_use_their_own_field_profiles() {
    let variant = || {
        EnumVariantSpec::builder("Payload")
            .record_payload_field(field("value", TypeName::primitive("String")))
            .build()
            .unwrap()
    };
    let rust = TypeSpec::builder("Message", TypeKind::Enum)
        .add_variant(variant())
        .build()
        .unwrap();
    assert!(
        rust.validate(&sigil_stitch::lang::rust::Rust::new())
            .is_ok()
    );

    let ts = TypeSpec::builder("Message", TypeKind::Enum)
        .add_variant(variant())
        .build()
        .unwrap();
    assert!(matches!(
        ts.validate(&sigil_stitch::lang::typescript::TypeScript::new()),
        Err(SigilStitchError::UnsupportedVariantCapabilities { .. })
    ));
}

#[test]
fn field_type_refs_preserve_import_alias_resolution() {
    let ty = TypeSpec::builder("Users", TypeKind::Class)
        .add_field(field(
            "current",
            TypeName::importable_type("./models", "User"),
        ))
        .add_field(field(
            "legacy",
            TypeName::importable_type("./legacy", "User"),
        ))
        .build()
        .unwrap();
    let output = render_type(
        sigil_stitch::lang::typescript::TypeScript::new(),
        "users.ts",
        ty,
    )
    .unwrap();
    assert!(output.contains("User as LegacyUser"), "{output}");
    assert!(output.contains("current: User;"), "{output}");
    assert!(output.contains("legacy: LegacyUser;"), "{output}");
}

#[test]
fn c_rejects_every_nested_interleaved_declarator_shape() {
    let int = || TypeName::primitive("int");
    let array = || TypeName::array(int());
    let types = [
        TypeName::generic(array(), vec![int()]),
        TypeName::generic(int(), vec![array()]),
        TypeName::union(vec![array()]),
        TypeName::intersection(vec![array()]),
        TypeName::tuple(vec![array()]),
        TypeName::impl_trait(vec![array()]),
        TypeName::dyn_trait(vec![array()]),
        TypeName::map(array(), int()),
        TypeName::map(int(), array()),
        TypeName::associated_type(array(), None, "Item"),
        TypeName::associated_type(int(), Some(array()), "Item"),
        TypeName::wildcard_extends(array()),
        TypeName::wildcard_super(array()),
    ];
    let lang = sigil_stitch::lang::c::C::new();
    for ty in types {
        assert!(render_field(&lang, &field("value", ty)).is_err());
    }
}

#[test]
fn target_local_field_restrictions_reject_invalid_combinations() {
    let empty = || TypeName::primitive("");
    let typed = || TypeName::primitive("Value");
    let tagged = |name: &str, ty: TypeName, visibility: Visibility| {
        FieldSpec::builder(name, ty)
            .visibility(visibility)
            .tag("legacy")
            .build()
            .unwrap()
    };

    assert!(
        render_field(
            &sigil_stitch::lang::c::C::new(),
            &tagged("value", typed(), Visibility::Public),
        )
        .is_err()
    );

    let cpp = tagged("value", typed(), Visibility::PublicCrate);
    assert!(
        cpp.emit(
            &sigil_stitch::lang::cpp::Cpp::new(),
            DeclarationContext::TopLevel,
        )
        .is_err()
    );

    assert!(
        render_field(
            &sigil_stitch::lang::csharp::CSharp::new(),
            &tagged("value", typed(), Visibility::PublicSuper),
        )
        .is_err()
    );

    let dart = sigil_stitch::lang::dart::Dart::new();
    for invalid in [
        tagged("_private", typed(), Visibility::Public),
        tagged("public", typed(), Visibility::Private),
        tagged("value", typed(), Visibility::Protected),
        FieldSpec::builder("generic", TypeName::generic(typed(), vec![typed()]))
            .is_static()
            .build()
            .unwrap(),
    ] {
        assert!(render_field(&dart, &invalid).is_err());
    }

    let go = sigil_stitch::lang::go::Go::new();
    for invalid in [
        FieldSpec::builder("private", typed())
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        FieldSpec::builder("Exported", typed())
            .visibility(Visibility::Private)
            .build()
            .unwrap(),
        FieldSpec::builder("value", typed())
            .visibility(Visibility::Protected)
            .build()
            .unwrap(),
    ] {
        assert!(render_field(&go, &invalid).is_err());
    }

    for lang in [
        Box::new(sigil_stitch::lang::haskell::Haskell::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::ocaml::OCaml::new()) as Box<dyn CodeLang>,
    ] {
        assert!(
            render_field(
                lang.as_ref(),
                &FieldSpec::builder("_", typed())
                    .visibility(Visibility::Public)
                    .build()
                    .unwrap(),
            )
            .is_err()
        );
    }

    assert!(
        render_field(
            &sigil_stitch::lang::java::Java::new(),
            &tagged("_", typed(), Visibility::PublicCrate),
        )
        .is_err()
    );

    let javascript = sigil_stitch::lang::javascript::JavaScript::new();
    for invalid in [
        FieldSpec::builder("#private", empty())
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        FieldSpec::builder("public", empty())
            .visibility(Visibility::Private)
            .build()
            .unwrap(),
        tagged("value", empty(), Visibility::Protected),
    ] {
        assert!(render_field(&javascript, &invalid).is_err());
    }

    for lang in [
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::scala::Scala::new()) as Box<dyn CodeLang>,
    ] {
        let invalid = FieldSpec::builder("_", empty())
            .visibility(Visibility::PublicSuper)
            .tag("legacy")
            .build()
            .unwrap();
        assert!(render_field(lang.as_ref(), &invalid).is_err());
    }

    let php = FieldSpec::builder("value", empty())
        .visibility(Visibility::PublicCrate)
        .is_static()
        .is_readonly()
        .initializer(CodeBlock::of("seed", ()).unwrap())
        .tag("legacy")
        .build()
        .unwrap();
    assert!(render_field(&sigil_stitch::lang::php::Php::new(), &php).is_err());

    let python = sigil_stitch::lang::python::Python::new();
    for invalid in [
        FieldSpec::builder("_private", typed())
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        FieldSpec::builder("public", typed())
            .visibility(Visibility::Private)
            .build()
            .unwrap(),
        FieldSpec::builder("value", empty())
            .visibility(Visibility::PublicCrate)
            .build()
            .unwrap(),
    ] {
        assert!(render_field(&python, &invalid).is_err());
    }

    let rust = FieldSpec::builder("_", typed())
        .visibility(Visibility::Protected)
        .tag("legacy")
        .build()
        .unwrap();
    assert!(render_field(&sigil_stitch::lang::rust::Rust::new(), &rust).is_err());

    let swift = FieldSpec::builder("_", empty())
        .visibility(Visibility::PublicSuper)
        .is_static()
        .is_readonly()
        .tag("legacy")
        .build()
        .unwrap();
    assert!(render_field(&sigil_stitch::lang::swift::Swift::new(), &swift).is_err());

    let typescript = sigil_stitch::lang::typescript::TypeScript::new();
    for invalid in [
        FieldSpec::builder("#private", typed())
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        field("constructor", typed()),
        FieldSpec::builder("prototype", typed())
            .is_static()
            .build()
            .unwrap(),
        tagged("value", typed(), Visibility::Inherited),
    ] {
        assert!(render_field(&typescript, &invalid).is_err());
    }
}

#[test]
fn rich_valid_fields_cover_language_local_lowering_branches() {
    let initializer = || CodeBlock::of("seed", ()).unwrap();

    let c = FieldSpec::builder("value", TypeName::primitive("int"))
        .is_readonly()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::c::C::new(), &c)
            .unwrap()
            .trim(),
        "const int value;"
    );

    let csharp = FieldSpec::builder("fieldValue", TypeName::primitive("int"))
        .visibility(Visibility::Protected)
        .is_static()
        .is_readonly()
        .initializer(initializer())
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::csharp::CSharp::new(), &csharp)
            .unwrap()
            .trim(),
        "protected static readonly int fieldValue = seed;"
    );

    let javascript = FieldSpec::builder("value", TypeName::primitive(""))
        .initializer(initializer())
        .build()
        .unwrap();
    assert_eq!(
        render_field(
            &sigil_stitch::lang::javascript::JavaScript::new(),
            &javascript
        )
        .unwrap()
        .trim(),
        "value = seed;"
    );

    for (lang, expected) in [
        (
            Box::new(sigil_stitch::lang::kotlin::Kotlin::new()) as Box<dyn CodeLang>,
            "protected val value: Value = seed",
        ),
        (
            Box::new(sigil_stitch::lang::scala::Scala::new()) as Box<dyn CodeLang>,
            "protected val value: Value = seed",
        ),
    ] {
        let value = FieldSpec::builder("value", TypeName::primitive("Value"))
            .visibility(Visibility::Protected)
            .is_readonly()
            .initializer(initializer())
            .build()
            .unwrap();
        assert_eq!(
            render_field(lang.as_ref(), &value).unwrap().trim(),
            expected
        );
    }

    let php_static = FieldSpec::builder("value", TypeName::primitive("int"))
        .visibility(Visibility::Private)
        .is_static()
        .initializer(initializer())
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::php::Php::new(), &php_static)
            .unwrap()
            .trim(),
        "private static int $value = seed;"
    );
    let php_readonly = FieldSpec::builder("value", TypeName::primitive("int"))
        .visibility(Visibility::Protected)
        .is_readonly()
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::php::Php::new(), &php_readonly)
            .unwrap()
            .trim(),
        "protected readonly int $value;"
    );

    let swift = FieldSpec::builder("value", TypeName::primitive("Int"))
        .visibility(Visibility::PublicCrate)
        .is_static()
        .is_readonly()
        .initializer(initializer())
        .build()
        .unwrap();
    assert_eq!(
        render_field(&sigil_stitch::lang::swift::Swift::new(), &swift)
            .unwrap()
            .trim(),
        "internal static let value: Int = seed"
    );
}

#[test]
fn swift_accepts_representatives_from_every_identifier_range() {
    let lang = sigil_stitch::lang::swift::Swift::new();
    for head in [
        '\u{00a8}',
        '\u{00aa}',
        '\u{00ad}',
        '\u{00af}',
        '\u{00b2}',
        '\u{00b7}',
        '\u{00bc}',
        '\u{00c0}',
        '\u{00d8}',
        '\u{00f8}',
        '\u{0100}',
        '\u{0370}',
        '\u{1681}',
        '\u{180f}',
        '\u{1e00}',
        '\u{200b}',
        '\u{202a}',
        '\u{203f}',
        '\u{2054}',
        '\u{2060}',
        '\u{2070}',
        '\u{2460}',
        '\u{2776}',
        '\u{2c00}',
        '\u{2e80}',
        '\u{3004}',
        '\u{3021}',
        '\u{3031}',
        '\u{3040}',
        '\u{f900}',
        '\u{fd40}',
        '\u{fdf0}',
        '\u{fe30}',
        '\u{fe47}',
        '\u{10000}',
    ] {
        let name = format!("{head}field");
        assert!(
            render_field(&lang, &field(&name, TypeName::primitive("Int"))).is_ok(),
            "{head:?}"
        );
    }
    for continuation in ['\u{0300}', '\u{1dc0}', '\u{20d0}', '\u{fe20}'] {
        let name = format!("a{continuation}");
        assert!(render_field(&lang, &field(&name, TypeName::primitive("Int"))).is_ok());
    }
}

#[test]
fn record_payload_field_rules_see_docs_and_multiple_fields() {
    for lang in [
        Box::new(sigil_stitch::lang::haskell::Haskell::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::ocaml::OCaml::new()) as Box<dyn CodeLang>,
    ] {
        let documented = TypeSpec::builder("Message", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Payload")
                    .record_payload_field(
                        FieldSpec::builder("first", TypeName::primitive("String"))
                            .doc("not valid inline")
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(documented.validate(lang.as_ref()).is_err());

        let payload = TypeSpec::builder("Message", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Payload")
                    .record_payload_field(field("first", TypeName::primitive("String")))
                    .record_payload_field(field("second", TypeName::primitive("Int")))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let filename = format!("payload.{}", lang.file_extension());
        let output = FileSpec::builder(&filename)
            .add_type(payload)
            .build()
            .unwrap()
            .render(120)
            .unwrap();
        assert!(output.contains("first"), "{output}");
        assert!(output.contains("second"), "{output}");
    }
}

#[derive(Debug, Clone, Copy)]
struct LegacyFieldLang {
    style: OptionalFieldStyle,
    type_before_name: bool,
}

impl RendererLang for LegacyFieldLang {
    fn file_extension(&self) -> &str {
        "legacy-fields"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            field_terminator: ";",
            ..BlockSyntaxConfig::default()
        }
    }
}

impl CodeLang for LegacyFieldLang {
    fn render_visibility(&self, visibility: Visibility, _: DeclarationContext) -> &str {
        if visibility == Visibility::Public {
            "public "
        } else {
            ""
        }
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            static_keyword: "shared ",
            ..FunctionSyntaxConfig::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_before_name: self.type_before_name,
            ..TypeDeclSyntaxConfig::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            annotation_prefix: "@[",
            annotation_suffix: "]",
            readonly_keyword: "fixed ",
            mutable_field_keyword: "var ",
            ..EnumAndAnnotationConfig::default()
        }
    }

    fn variable_prefix(&self) -> &str {
        "$"
    }

    fn optional_field_style(&self) -> OptionalFieldStyle {
        self.style
    }

    fn doc_before_annotations(&self) -> bool {
        false
    }

    fn type_keyword(&self, _: TypeKind) -> &str {
        "record"
    }
}

fn legacy_optional_field(style: OptionalFieldStyle, type_before_name: bool) -> String {
    let lang = LegacyFieldLang {
        style,
        type_before_name,
    };
    let field = FieldSpec::builder("name", TypeName::primitive("T"))
        .is_optional()
        .build()
        .unwrap();
    render_field(&lang, &field).unwrap().trim().to_string()
}

#[test]
fn external_adapters_keep_every_frozen_optional_field_style() {
    assert_eq!(
        legacy_optional_field(OptionalFieldStyle::NameSuffix("?"), false),
        "var $name?: T;"
    );
    assert_eq!(
        legacy_optional_field(OptionalFieldStyle::TypeSuffix("?"), false),
        "var $name: T?;"
    );
    assert_eq!(
        legacy_optional_field(
            OptionalFieldStyle::TypeWrap {
                open: "Maybe<",
                close: ">"
            },
            false
        ),
        "var $name: Maybe<T>;"
    );
    assert_eq!(
        legacy_optional_field(OptionalFieldStyle::TypePrefix("*"), false),
        "var $name: *T;"
    );
    assert_eq!(
        legacy_optional_field(OptionalFieldStyle::TypePrefix("*"), true),
        "T *$name;"
    );
    assert_eq!(
        legacy_optional_field(OptionalFieldStyle::UnionWithNone(" | "), false),
        "var $name: T | None;"
    );
    assert_eq!(
        legacy_optional_field(OptionalFieldStyle::Ignored, false),
        "var $name: T;"
    );
}

#[test]
fn external_adapter_keeps_rich_direct_and_type_sequence_compatibility() {
    let lang = LegacyFieldLang {
        style: OptionalFieldStyle::NameSuffix("?"),
        type_before_name: false,
    };
    let field = FieldSpec::builder("name", TypeName::primitive("T"))
        .visibility(Visibility::Public)
        .is_static()
        .is_readonly()
        .is_optional()
        .doc("docs")
        .annotate(AnnotationSpec::new("tracked"))
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .initializer(CodeBlock::of("seed", ()).unwrap())
        .tag("json:\"name\"")
        .build()
        .unwrap();
    let expected =
        "@[tracked]\n@raw\n// docs\npublic shared fixed $name?: T = seed `json:\"name\"`;\n";
    assert_eq!(render_field(&lang, &field).unwrap(), expected);

    let output = render_type(
        lang,
        "owner.legacy-fields",
        TypeSpec::builder("Owner", TypeKind::Class)
            .add_field(field)
            .build()
            .unwrap(),
    )
    .unwrap();
    for line in expected.lines() {
        assert!(
            output.lines().any(|candidate| candidate.trim() == line),
            "missing {line:?} in:\n{output}"
        );
    }
}
