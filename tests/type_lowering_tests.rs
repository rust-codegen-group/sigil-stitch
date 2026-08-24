use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec};
use sigil_stitch::type_name::TypeName;

fn render_type(lang: impl CodeLang, filename: &str, type_: TypeSpec) -> String {
    FileSpec::builder_with(filename, lang)
        .add_type(type_)
        .build()
        .unwrap()
        .render(100)
        .unwrap()
}

fn render_type_dyn(lang: &dyn CodeLang, type_: TypeSpec) -> Result<String, SigilStitchError> {
    let blocks = type_.emit(lang)?;
    let imports = ImportGroup::new();
    blocks
        .iter()
        .map(|block| CodeRenderer::new(lang, &imports, 100).render(block))
        .collect::<Result<Vec<_>, _>>()
        .map(|blocks| blocks.join("\n"))
}

fn minimal_type(name: &str, kind: TypeKind, extension: &str) -> TypeSpec {
    let mut builder = TypeSpec::builder(name, kind);
    if matches!(kind, TypeKind::TypeAlias | TypeKind::Newtype) {
        builder = builder.extends(TypeName::primitive("Value"));
    } else if kind == TypeKind::Enum {
        builder = builder.add_variant(EnumVariantSpec::new("Value").unwrap());
    } else if (matches!(kind, TypeKind::Class | TypeKind::Struct)
        && !matches!(extension, "js" | "rb"))
        || (extension == "c" && matches!(kind, TypeKind::Interface | TypeKind::Trait))
    {
        if extension == "kt" && kind == TypeKind::Struct {
            builder = builder.add_primary_constructor_param(
                ParameterSpec::builder("value", TypeName::primitive("Value"))
                    .is_property()
                    .build()
                    .unwrap(),
            );
        } else {
            builder = builder.add_field(
                FieldSpec::builder("field", TypeName::primitive("Value"))
                    .build()
                    .unwrap(),
            );
        }
    }
    builder.build().unwrap()
}

#[test]
fn type_validation_aggregates_intrinsic_adapter_and_child_errors() {
    let valid = TypeSpec::builder("Valid", TypeKind::Struct)
        .add_field(
            FieldSpec::builder("field", TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let mut serialized = serde_json::to_value(valid).unwrap();
    serialized["name"] = serde_json::json!("type");
    serialized["modifiers"]["is_static"] = serde_json::json!(true);
    serialized["fields"][0]["name"] = serde_json::json!("");
    let invalid: TypeSpec = serde_json::from_value(serialized).unwrap();

    let file = FileSpec::builder_with("invalid.rs", sigil_stitch::lang::rust::Rust::new())
        .add_type(invalid)
        .build()
        .unwrap();
    let SigilStitchError::FileSpecValidation { errors, .. } = file.validate().unwrap_err() else {
        panic!("expected aggregate type validation");
    };

    assert!(
        errors
            .iter()
            .any(|error| matches!(error, SigilStitchError::InvalidTypeModifiers { .. }))
    );
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::InvalidTypeDeclaration { reason, .. }
            if reason.contains("Rust reserves")
    )));
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, SigilStitchError::EmptyName { .. }))
    );
}

#[test]
fn alias_and_newtype_inputs_that_have_no_grammar_fail_instead_of_disappearing() {
    let alias = TypeSpec::builder("Alias", TypeKind::TypeAlias)
        .extends(TypeName::primitive("String"))
        .implements(TypeName::primitive("Display"))
        .extra_member(CodeBlock::of("hidden", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        alias.emit(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeAlias { .. })
    ));

    let newtype = TypeSpec::builder("Wrapper", TypeKind::Newtype)
        .extends(TypeName::primitive("String"))
        .add_primary_constructor_param(ParameterSpec::of("value", TypeName::primitive("String")))
        .build()
        .unwrap();
    assert!(matches!(
        newtype.emit(&sigil_stitch::lang::kotlin::Kotlin::new()),
        Err(SigilStitchError::InvalidTypeAlias { .. })
    ));
}

#[test]
fn target_local_type_identifiers_preserve_dollar_names() {
    let generic = || {
        TypeSpec::builder("$Widget", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("$T"))
            .build()
            .unwrap()
    };

    let java = render_type(
        sigil_stitch::lang::java::Java::new(),
        "$Widget.java",
        generic(),
    );
    assert!(java.contains("class $Widget<$T>"), "{java}");

    let typescript = render_type(
        sigil_stitch::lang::typescript::TypeScript::new(),
        "$Widget.ts",
        generic(),
    );
    assert!(typescript.contains("class $Widget<$T>"), "{typescript}");

    let dart = render_type(
        sigil_stitch::lang::dart::Dart::new(),
        "$Widget.dart",
        generic(),
    );
    assert!(dart.contains("class $Widget<$T>"), "{dart}");

    let scala = render_type(
        sigil_stitch::lang::scala::Scala::new(),
        "$Widget.scala",
        generic(),
    );
    assert!(scala.contains("class $Widget[$T]"), "{scala}");

    let javascript = render_type(
        sigil_stitch::lang::javascript::JavaScript::new(),
        "$Widget.js",
        TypeSpec::builder("$Widget", TypeKind::Class)
            .build()
            .unwrap(),
    );
    assert!(javascript.contains("class $Widget"), "{javascript}");

    assert!(matches!(
        generic().emit(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { .. })
    ));
}

#[test]
fn scala_validates_raw_higher_kinded_type_parameters() {
    for raw in [" ", "not-a-kind", "[_"] {
        let error = TypeSpec::builder("Container", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("F").with_kind(TypeParamKind::Raw(raw.to_string())))
            .build()
            .unwrap()
            .emit(&sigil_stitch::lang::scala::Scala::new())
            .unwrap_err();

        assert!(matches!(
            error,
            SigilStitchError::InvalidTypeParameter { reason, .. }
                if reason.contains("higher-kinded")
        ));
    }

    let valid = render_type(
        sigil_stitch::lang::scala::Scala::new(),
        "Container.scala",
        TypeSpec::builder("Container", TypeKind::Class)
            .add_type_param(
                TypeParamSpec::new("F").with_kind(TypeParamKind::Raw("[_[_]]".to_string())),
            )
            .build()
            .unwrap(),
    );
    assert!(valid.contains("class Container[F[_[_]]]"), "{valid}");
}

#[test]
fn c_alias_preserves_documentation() {
    let alias = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .doc("Distance in meters.")
        .extends(TypeName::primitive("double"))
        .build()
        .unwrap();
    let output = render_type(sigil_stitch::lang::c::C::new(), "meters.h", alias);
    assert_eq!(
        output.trim(),
        "/* Distance in meters. */\ntypedef double Meters;"
    );
}

#[test]
fn csharp_merges_direct_context_and_explicit_constraints() {
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("Direct"))
                .with_context_bound(TypeName::primitive("Context")),
        )
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Explicit")],
        )
        .build()
        .unwrap();
    let output = render_type(sigil_stitch::lang::csharp::CSharp::new(), "Box.cs", type_);
    assert!(
        output.contains("class Box<T>\n    where T : Direct, Context, Explicit\n{"),
        "{output}"
    );
}

#[test]
fn kotlin_emits_additional_and_explicit_bounds_on_constrained_newtypes() {
    let type_ = TypeSpec::builder("Wrapper", TypeKind::Newtype)
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("First"))
                .with_bound(TypeName::primitive("Second")),
        )
        .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Third")])
        .extends(TypeName::primitive("T"))
        .build()
        .unwrap();
    let output = render_type(
        sigil_stitch::lang::kotlin::Kotlin::new(),
        "Wrapper.kt",
        type_,
    );
    assert!(
        output.contains("value class Wrapper<T : First>(val value: T) where T : Second, T : Third")
    );
}

#[test]
fn javascript_emits_structured_type_annotations() {
    let type_ = TypeSpec::builder("Entity", TypeKind::Class)
        .annotate(AnnotationSpec::new("sealed"))
        .build()
        .unwrap();
    let output = render_type(
        sigil_stitch::lang::javascript::JavaScript::new(),
        "entity.js",
        type_,
    );
    assert!(output.starts_with("@sealed\nclass Entity {"), "{output}");
}

#[test]
fn c_and_cpp_place_structured_type_attributes_in_valid_grammar_positions() {
    let c = TypeSpec::builder("Packed", TypeKind::Struct)
        .annotate(AnnotationSpec::new("packed"))
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let c = render_type(sigil_stitch::lang::c::C::new(), "packed.h", c);
    assert!(c.contains("struct __attribute__((packed)) Packed {"), "{c}");

    let cpp = TypeSpec::builder("Task", TypeKind::Class)
        .annotate(AnnotationSpec::new("nodiscard"))
        .add_type_param(TypeParamSpec::new("T"))
        .build()
        .unwrap();
    let cpp = render_type(sigil_stitch::lang::cpp::Cpp::new(), "task.hpp", cpp);
    assert!(
        cpp.contains("template <typename T>\nclass [[nodiscard]] Task {"),
        "{cpp}"
    );
}

#[test]
fn kotlin_data_classes_require_structured_constructor_properties() {
    let empty = TypeSpec::builder("Empty", TypeKind::Struct)
        .build()
        .unwrap();
    assert!(matches!(
        empty.emit(&sigil_stitch::lang::kotlin::Kotlin::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("data classes require")
    ));

    let plain_parameter = TypeSpec::builder("User", TypeKind::Struct)
        .add_primary_constructor_param(ParameterSpec::of("name", TypeName::primitive("String")))
        .build()
        .unwrap();
    assert!(matches!(
        plain_parameter.emit(&sigil_stitch::lang::kotlin::Kotlin::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("must declare a val or var property")
    ));
}

#[test]
fn primary_constructor_validation_rejects_names_and_empty_defaults_without_target_syntax() {
    for lang in [
        &sigil_stitch::lang::kotlin::Kotlin::new() as &dyn CodeLang,
        &sigil_stitch::lang::scala::Scala::new(),
    ] {
        let duplicate = TypeSpec::builder("Pair", TypeKind::Class)
            .add_primary_constructor_param(ParameterSpec::of("value", TypeName::primitive("Int")))
            .add_primary_constructor_param(ParameterSpec::of("value", TypeName::primitive("Int")))
            .build()
            .unwrap();
        assert!(matches!(
            duplicate.emit(lang),
            Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
                if reason.contains("duplicate")
        ));

        let empty_default = TypeSpec::builder("Defaulted", TypeKind::Class)
            .add_primary_constructor_param(
                ParameterSpec::builder("value", TypeName::primitive("Int"))
                    .default_value(CodeBlock::builder().build().unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(matches!(
            empty_default.emit(lang),
            Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
                if reason.contains("empty default value")
        ));
    }
}

#[test]
fn kotlin_initializes_header_owned_superclasses_but_not_implemented_contracts() {
    let type_ = TypeSpec::builder("Child", TypeKind::Class)
        .extends(TypeName::primitive("Base"))
        .implements(TypeName::primitive("Contract"))
        .build()
        .unwrap();
    let output = render_type(sigil_stitch::lang::kotlin::Kotlin::new(), "Child.kt", type_);
    assert!(
        output.contains("class Child : Base(), Contract {"),
        "{output}"
    );
}

#[test]
fn scala_empty_case_classes_keep_the_required_parameter_list() {
    let type_ = TypeSpec::builder("UnitRecord", TypeKind::Struct)
        .build()
        .unwrap();
    let output = render_type(
        sigil_stitch::lang::scala::Scala::new(),
        "UnitRecord.scala",
        type_,
    );
    assert!(output.contains("case class UnitRecord() {"), "{output}");
}

#[test]
fn typescript_contracts_reject_decorators_that_cannot_be_emitted() {
    let type_ = TypeSpec::builder("Contract", TypeKind::Interface)
        .annotate(AnnotationSpec::new("sealed"))
        .build()
        .unwrap();
    assert!(matches!(
        type_.emit(&sigil_stitch::lang::typescript::TypeScript::new()),
        Err(SigilStitchError::UnsupportedTypeCapabilities { capabilities, .. })
            if capabilities == vec![sigil_stitch::lang::capability::TypeCapability::Attributes]
    ));
}

#[test]
fn kotlin_and_scala_aliases_preserve_docs_and_visibility() {
    let kotlin = TypeSpec::builder("Name", TypeKind::TypeAlias)
        .visibility(Visibility::Private)
        .doc("A private name.")
        .extends(TypeName::primitive("String"))
        .build()
        .unwrap();
    let kotlin = render_type(sigil_stitch::lang::kotlin::Kotlin::new(), "Name.kt", kotlin);
    assert!(kotlin.contains("A private name."), "{kotlin}");
    assert!(
        kotlin.contains("private typealias Name = String"),
        "{kotlin}"
    );

    let scala = TypeSpec::builder("Name", TypeKind::TypeAlias)
        .visibility(Visibility::Private)
        .doc("A private name.")
        .extends(TypeName::primitive("String"))
        .build()
        .unwrap();
    let scala = render_type(sigil_stitch::lang::scala::Scala::new(), "Name.scala", scala);
    assert!(scala.contains("A private name."), "{scala}");
    assert!(scala.contains("private type Name = String"), "{scala}");
}

#[test]
fn typescript_type_aliases_preserve_direct_bounds() {
    let alias = TypeSpec::builder("Box", TypeKind::TypeAlias)
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("object")))
        .extends(TypeName::primitive("T"))
        .build()
        .unwrap();
    let output = render_type(
        sigil_stitch::lang::typescript::TypeScript::new(),
        "box.ts",
        alias,
    );
    assert_eq!(output.trim(), "export type Box<T extends object> = T;");
}

#[test]
fn non_context_bound_languages_reject_context_bound_intent() {
    for lang in [
        &sigil_stitch::lang::java::Java::new() as &dyn CodeLang,
        &sigil_stitch::lang::kotlin::Kotlin::new(),
        &sigil_stitch::lang::swift::Swift::new(),
        &sigil_stitch::lang::typescript::TypeScript::new(),
    ] {
        let type_ = TypeSpec::builder("Box", TypeKind::Class)
            .add_type_param(
                TypeParamSpec::new("T").with_context_bound(TypeName::primitive("Ordering")),
            )
            .build()
            .unwrap();
        assert!(
            matches!(
                type_.emit(lang),
                Err(SigilStitchError::InvalidTypeParameter { .. })
            ),
            ".{}",
            lang.file_extension()
        );
    }
}

#[test]
fn java_and_typescript_reject_detached_where_constraints() {
    for lang in [
        &sigil_stitch::lang::java::Java::new() as &dyn CodeLang,
        &sigil_stitch::lang::typescript::TypeScript::new(),
    ] {
        let type_ = TypeSpec::builder("Box", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Bound")])
            .build()
            .unwrap();
        assert!(
            matches!(
                type_.emit(lang),
                Err(SigilStitchError::InvalidTypeDeclaration { .. })
            ),
            ".{}",
            lang.file_extension()
        );
    }
}

#[test]
fn attached_constraint_languages_reject_unknown_subjects() {
    for lang in [
        &sigil_stitch::lang::csharp::CSharp::new() as &dyn CodeLang,
        &sigil_stitch::lang::kotlin::Kotlin::new(),
        &sigil_stitch::lang::swift::Swift::new(),
    ] {
        let type_ = TypeSpec::builder("Box", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(
                TypeName::primitive("Unknown"),
                vec![TypeName::primitive("Bound")],
            )
            .build()
            .unwrap();
        assert!(
            matches!(
                type_.emit(lang),
                Err(SigilStitchError::InvalidTypeParameter { .. })
            ),
            ".{}",
            lang.file_extension()
        );
    }
}

#[test]
fn single_inheritance_adapters_reject_multiple_nominal_supertypes() {
    for lang in [
        &sigil_stitch::lang::csharp::CSharp::new() as &dyn CodeLang,
        &sigil_stitch::lang::dart::Dart::new(),
        &sigil_stitch::lang::kotlin::Kotlin::new(),
        &sigil_stitch::lang::scala::Scala::new(),
        &sigil_stitch::lang::swift::Swift::new(),
    ] {
        let type_ = TypeSpec::builder("Child", TypeKind::Class)
            .extends(TypeName::primitive("FirstBase"))
            .extends(TypeName::primitive("SecondBase"))
            .build()
            .unwrap();
        assert!(
            matches!(
                type_.emit(lang),
                Err(SigilStitchError::InvalidTypeDeclaration { .. })
            ),
            ".{}",
            lang.file_extension()
        );
    }
}

#[test]
fn swift_structs_reject_nominal_inheritance() {
    let type_ = TypeSpec::builder("Child", TypeKind::Struct)
        .extends(TypeName::primitive("Base"))
        .build()
        .unwrap();
    assert!(matches!(
        type_.emit(&sigil_stitch::lang::swift::Swift::new()),
        Err(SigilStitchError::UnsupportedTypeCapabilities { capabilities, .. })
            if capabilities == vec![sigil_stitch::lang::capability::TypeCapability::NominalSubtyping]
    ));
}

#[test]
fn empty_structured_type_annotation_names_fail_before_lowering() {
    let type_ = TypeSpec::builder("Entity", TypeKind::Class)
        .annotate(AnnotationSpec::new(""))
        .build()
        .unwrap();
    assert!(matches!(
        type_.emit(&sigil_stitch::lang::typescript::TypeScript::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("structured annotation")
    ));
}

#[test]
fn top_level_type_visibility_must_have_target_syntax() {
    for (lang, visibility) in [
        (
            &sigil_stitch::lang::csharp::CSharp::new() as &dyn CodeLang,
            Visibility::Private,
        ),
        (
            &sigil_stitch::lang::kotlin::Kotlin::new(),
            Visibility::Protected,
        ),
        (
            &sigil_stitch::lang::scala::Scala::new(),
            Visibility::Protected,
        ),
        (
            &sigil_stitch::lang::swift::Swift::new(),
            Visibility::Protected,
        ),
    ] {
        let type_ = TypeSpec::builder("Entity", TypeKind::Class)
            .visibility(visibility)
            .build()
            .unwrap();
        assert!(
            matches!(
                type_.emit(lang),
                Err(SigilStitchError::InvalidTypeDeclaration { .. })
            ),
            ".{}",
            lang.file_extension()
        );
    }
}

#[test]
fn every_supported_builtin_type_kind_reaches_its_language_owned_lowerer() {
    fn assert_kinds(lang: &dyn CodeLang, kinds: &[TypeKind]) {
        for &kind in kinds {
            let name = if lang.file_extension() == "ml" {
                format!("{kind:?}Sample").to_ascii_lowercase()
            } else {
                format!("{kind:?}Sample")
            };
            let output = render_type_dyn(lang, minimal_type(&name, kind, lang.file_extension()))
                .unwrap_or_else(|error| {
                    panic!(
                        ".{} failed to lower {kind:?}: {error}",
                        lang.file_extension()
                    )
                });
            assert!(
                output.contains(&name),
                ".{} {kind:?}: {output}",
                lang.file_extension()
            );
        }
    }

    const RECORDS_ENUM_ALIAS: &[TypeKind] = &[
        TypeKind::Class,
        TypeKind::Struct,
        TypeKind::Interface,
        TypeKind::Trait,
        TypeKind::Enum,
        TypeKind::TypeAlias,
    ];
    const NOMINAL_TYPES: &[TypeKind] = &[
        TypeKind::Class,
        TypeKind::Struct,
        TypeKind::Interface,
        TypeKind::Trait,
        TypeKind::Enum,
    ];
    const ALL_TYPES: &[TypeKind] = &[
        TypeKind::Class,
        TypeKind::Struct,
        TypeKind::Interface,
        TypeKind::Trait,
        TypeKind::Enum,
        TypeKind::TypeAlias,
        TypeKind::Newtype,
    ];

    assert_kinds(&sigil_stitch::lang::c::C::new(), RECORDS_ENUM_ALIAS);
    assert_kinds(&sigil_stitch::lang::cpp::Cpp::new(), RECORDS_ENUM_ALIAS);
    assert_kinds(&sigil_stitch::lang::csharp::CSharp::new(), NOMINAL_TYPES);
    assert_kinds(&sigil_stitch::lang::dart::Dart::new(), RECORDS_ENUM_ALIAS);
    assert_kinds(
        &sigil_stitch::lang::go::Go::new(),
        &[
            TypeKind::Class,
            TypeKind::Struct,
            TypeKind::Interface,
            TypeKind::Trait,
            TypeKind::TypeAlias,
            TypeKind::Newtype,
        ],
    );
    assert_kinds(&sigil_stitch::lang::haskell::Haskell::new(), ALL_TYPES);
    assert_kinds(&sigil_stitch::lang::java::Java::new(), NOMINAL_TYPES);
    assert_kinds(
        &sigil_stitch::lang::javascript::JavaScript::new(),
        NOMINAL_TYPES,
    );
    assert_kinds(&sigil_stitch::lang::kotlin::Kotlin::new(), ALL_TYPES);
    assert_kinds(
        &sigil_stitch::lang::ocaml::OCaml::new(),
        &[
            TypeKind::Class,
            TypeKind::Struct,
            TypeKind::Enum,
            TypeKind::TypeAlias,
        ],
    );
    assert_kinds(
        &sigil_stitch::lang::php::Php::new(),
        &[
            TypeKind::Class,
            TypeKind::Struct,
            TypeKind::Interface,
            TypeKind::Trait,
            TypeKind::Enum,
            TypeKind::Newtype,
        ],
    );
    assert_kinds(&sigil_stitch::lang::python::Python::new(), ALL_TYPES);
    assert_kinds(&sigil_stitch::lang::ruby::Ruby::new(), NOMINAL_TYPES);
    assert_kinds(&sigil_stitch::lang::rust::Rust::new(), ALL_TYPES);
    assert_kinds(&sigil_stitch::lang::scala::Scala::new(), ALL_TYPES);
    assert_kinds(&sigil_stitch::lang::swift::Swift::new(), NOMINAL_TYPES);
    assert_kinds(
        &sigil_stitch::lang::typescript::TypeScript::new(),
        RECORDS_ENUM_ALIAS,
    );
}

#[test]
fn every_builtin_type_lowerer_rejects_reserved_declaration_names() {
    let cases: &[(&dyn CodeLang, &str)] = &[
        (&sigil_stitch::lang::c::C::new(), "while"),
        (&sigil_stitch::lang::cpp::Cpp::new(), "class"),
        (&sigil_stitch::lang::csharp::CSharp::new(), "class"),
        (&sigil_stitch::lang::dart::Dart::new(), "class"),
        (&sigil_stitch::lang::go::Go::new(), "type"),
        (&sigil_stitch::lang::haskell::Haskell::new(), "data"),
        (&sigil_stitch::lang::java::Java::new(), "class"),
        (&sigil_stitch::lang::javascript::JavaScript::new(), "class"),
        (&sigil_stitch::lang::kotlin::Kotlin::new(), "class"),
        (&sigil_stitch::lang::ocaml::OCaml::new(), "type"),
        (&sigil_stitch::lang::php::Php::new(), "class"),
        (&sigil_stitch::lang::python::Python::new(), "class"),
        (&sigil_stitch::lang::ruby::Ruby::new(), "class"),
        (&sigil_stitch::lang::rust::Rust::new(), "struct"),
        (&sigil_stitch::lang::scala::Scala::new(), "class"),
        (&sigil_stitch::lang::swift::Swift::new(), "class"),
        (&sigil_stitch::lang::typescript::TypeScript::new(), "class"),
    ];

    for &(lang, name) in cases {
        let error = minimal_type(name, TypeKind::Class, lang.file_extension())
            .emit(lang)
            .unwrap_err();
        assert!(
            matches!(error, SigilStitchError::InvalidTypeDeclaration { ref reason, .. }
                if reason.contains("reserves")
                    || reason.contains("keyword")
                    || reason.contains("constant name")),
            ".{}: {error}",
            lang.file_extension()
        );
    }
}

#[test]
fn shared_type_validation_rejects_invalid_identifiers_and_parameter_subjects() {
    let invalid_name = minimal_type("not-valid", TypeKind::Class, "ts")
        .emit(&sigil_stitch::lang::typescript::TypeScript::new())
        .unwrap_err();
    assert!(matches!(
        invalid_name,
        SigilStitchError::InvalidTypeDeclaration { reason, .. }
            if reason.contains("valid declaration identifier")
    ));

    let lifetime = TypeSpec::builder("Lifetime", TypeKind::Class)
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .build()
        .unwrap()
        .emit(&sigil_stitch::lang::typescript::TypeScript::new())
        .unwrap_err();
    assert!(matches!(
        lifetime,
        SigilStitchError::InvalidTypeParameter { reason, .. }
            if reason.contains("ordinary non-keyword identifier")
    ));

    let complex_subject = TypeSpec::builder("ComplexSubject", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::generic(
                TypeName::primitive("Subject"),
                vec![TypeName::primitive("T")],
            ),
            vec![TypeName::primitive("Bound")],
        )
        .build()
        .unwrap()
        .emit(&sigil_stitch::lang::csharp::CSharp::new())
        .unwrap_err();
    assert!(matches!(
        complex_subject,
        SigilStitchError::InvalidTypeParameter { reason, .. }
            if reason.contains("must target a declared type parameter")
    ));
}

#[test]
fn target_specific_type_validation_rejects_grammar_invalid_shapes() {
    fn assert_invalid(type_: TypeSpec, lang: &dyn CodeLang, expected: &str) {
        let error = type_.emit(lang).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            ".{}: {error}",
            lang.file_extension()
        );
    }

    let field = || {
        FieldSpec::builder("field", TypeName::primitive("Value"))
            .build()
            .unwrap()
    };

    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("T"))
            .build()
            .unwrap(),
        &sigil_stitch::lang::haskell::Haskell::new(),
        "lowercase non-keyword",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("a"))
            .add_where_constraint(TypeName::primitive("b"), vec![TypeName::primitive("Bound")])
            .build()
            .unwrap(),
        &sigil_stitch::lang::haskell::Haskell::new(),
        "declared type variable",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Struct)
            .add_field(field())
            .add_variant(EnumVariantSpec::new("Value").unwrap())
            .build()
            .unwrap(),
        &sigil_stitch::lang::haskell::Haskell::new(),
        "cannot combine",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Enum).build().unwrap(),
        &sigil_stitch::lang::haskell::Haskell::new(),
        "requires at least one constructor",
    );

    assert_invalid(
        TypeSpec::builder("sample", TypeKind::Class)
            .add_field(field())
            .add_type_param(TypeParamSpec::new("T"))
            .build()
            .unwrap(),
        &sigil_stitch::lang::ocaml::OCaml::new(),
        "lowercase type-variable",
    );
    assert_invalid(
        TypeSpec::builder("record", TypeKind::Struct)
            .build()
            .unwrap(),
        &sigil_stitch::lang::ocaml::OCaml::new(),
        "empty record",
    );
    assert_invalid(
        TypeSpec::builder("variant", TypeKind::Enum)
            .build()
            .unwrap(),
        &sigil_stitch::lang::ocaml::OCaml::new(),
        "empty variant",
    );

    assert_invalid(
        TypeSpec::builder("privateName", TypeKind::Class)
            .visibility(Visibility::Public)
            .add_field(field())
            .build()
            .unwrap(),
        &sigil_stitch::lang::go::Go::new(),
        "public Go type names",
    );
    assert_invalid(
        TypeSpec::builder("Exported", TypeKind::Class)
            .visibility(Visibility::Private)
            .add_field(field())
            .build()
            .unwrap(),
        &sigil_stitch::lang::go::Go::new(),
        "private Go type names",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_field(field())
            .add_type_param(TypeParamSpec::lifetime("'a"))
            .build()
            .unwrap(),
        &sigil_stitch::lang::go::Go::new(),
        "ordinary non-keyword",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_field(field())
            .add_type_param(
                TypeParamSpec::new("T").with_context_bound(TypeName::primitive("Bound")),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::go::Go::new(),
        "do not support context bounds",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_field(field())
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Bound")])
            .build()
            .unwrap(),
        &sigil_stitch::lang::go::Go::new(),
        "attached directly",
    );

    for (parameter, expected) in [
        (TypeParamSpec::lifetime("a"), "lifetime"),
        (TypeParamSpec::lifetime("'static"), "lifetime"),
        (TypeParamSpec::new("type"), "ordinary non-keyword"),
        (
            TypeParamSpec::new("T").with_context_bound(TypeName::primitive("Bound")),
            "context bounds",
        ),
    ] {
        assert_invalid(
            TypeSpec::builder("Sample", TypeKind::Class)
                .add_field(field())
                .add_type_param(parameter)
                .build()
                .unwrap(),
            &sigil_stitch::lang::rust::Rust::new(),
            expected,
        );
    }

    for parameter in [
        ParameterSpec::builder("bad-name", TypeName::primitive("Value"))
            .build()
            .unwrap(),
        ParameterSpec::builder("values", TypeName::primitive("Value"))
            .variadic()
            .build()
            .unwrap(),
        ParameterSpec::builder("value", TypeName::primitive("Value"))
            .is_property()
            .is_mutable_property()
            .build()
            .unwrap(),
    ] {
        assert_invalid(
            TypeSpec::builder("Sample", TypeKind::Class)
                .add_primary_constructor_param(parameter)
                .build()
                .unwrap(),
            &sigil_stitch::lang::kotlin::Kotlin::new(),
            "primary-constructor parameter",
        );
    }
    assert_invalid(
        TypeSpec::builder("Child", TypeKind::Class)
            .extends(TypeName::primitive("Base"))
            .add_method(
                FunSpec::builder("constructor")
                    .is_constructor()
                    .body(CodeBlock::of("initialize()", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::kotlin::Kotlin::new(),
        "must delegate",
    );

    for parameter in [
        ParameterSpec::builder("values", TypeName::primitive("Value"))
            .variadic()
            .build()
            .unwrap(),
        ParameterSpec::builder("value", TypeName::primitive("Value"))
            .is_property()
            .is_mutable_property()
            .build()
            .unwrap(),
    ] {
        assert_invalid(
            TypeSpec::builder("Sample", TypeKind::Class)
                .add_primary_constructor_param(parameter)
                .build()
                .unwrap(),
            &sigil_stitch::lang::scala::Scala::new(),
            "primary-constructor parameter",
        );
    }
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Bound")])
            .build()
            .unwrap(),
        &sigil_stitch::lang::scala::Scala::new(),
        "attached directly",
    );

    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_type_param(
                TypeParamSpec::new("T")
                    .with_bound(TypeName::primitive("First"))
                    .with_bound(TypeName::primitive("Second")),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::dart::Dart::new(),
        "at most one direct upper bound",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Bound")])
            .build()
            .unwrap(),
        &sigil_stitch::lang::dart::Dart::new(),
        "attached directly",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Enum).build().unwrap(),
        &sigil_stitch::lang::dart::Dart::new(),
        "at least one value",
    );

    assert_invalid(
        TypeSpec::builder("Record", TypeKind::Struct)
            .build()
            .unwrap(),
        &sigil_stitch::lang::c::C::new(),
        "empty record",
    );
    assert_invalid(
        TypeSpec::builder("Empty", TypeKind::Enum).build().unwrap(),
        &sigil_stitch::lang::c::C::new(),
        "empty enum",
    );

    assert_invalid(
        TypeSpec::builder("Alias", TypeKind::TypeAlias)
            .doc("Not representable as a runtime docstring.")
            .extends(TypeName::primitive("Value"))
            .build()
            .unwrap(),
        &sigil_stitch::lang::python::Python::new(),
        "runtime docstring",
    );
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::TypeAlias)
            .add_type_param(TypeParamSpec::lifetime("'a"))
            .extends(TypeName::primitive("Value"))
            .build()
            .unwrap(),
        &sigil_stitch::lang::python::Python::new(),
        "ordinary non-keyword",
    );

    for (lang, type_) in [
        (
            &sigil_stitch::lang::php::Php::new() as &dyn CodeLang,
            TypeSpec::builder("Sample", TypeKind::Class)
                .extends(TypeName::primitive("First"))
                .extends(TypeName::primitive("Second"))
                .build()
                .unwrap(),
        ),
        (
            &sigil_stitch::lang::ruby::Ruby::new() as &dyn CodeLang,
            TypeSpec::builder("Sample", TypeKind::Class)
                .extends(TypeName::primitive("First"))
                .extends(TypeName::primitive("Second"))
                .build()
                .unwrap(),
        ),
        (
            &sigil_stitch::lang::java::Java::new() as &dyn CodeLang,
            TypeSpec::builder("Sample", TypeKind::Class)
                .extends(TypeName::primitive("First"))
                .extends(TypeName::primitive("Second"))
                .build()
                .unwrap(),
        ),
        (
            &sigil_stitch::lang::javascript::JavaScript::new() as &dyn CodeLang,
            TypeSpec::builder("Sample", TypeKind::Class)
                .extends(TypeName::primitive("First"))
                .extends(TypeName::primitive("Second"))
                .build()
                .unwrap(),
        ),
        (
            &sigil_stitch::lang::typescript::TypeScript::new() as &dyn CodeLang,
            TypeSpec::builder("Sample", TypeKind::Class)
                .extends(TypeName::primitive("First"))
                .extends(TypeName::primitive("Second"))
                .build()
                .unwrap(),
        ),
    ] {
        assert_invalid(type_, lang, "at most one");
    }
    assert_invalid(
        TypeSpec::builder("Sample", TypeKind::Class)
            .annotate(AnnotationSpec::new("unsupported"))
            .build()
            .unwrap(),
        &sigil_stitch::lang::ruby::Ruby::new(),
        "no structured declaration annotation",
    );
}
