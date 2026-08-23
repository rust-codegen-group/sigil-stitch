#![allow(deprecated)]

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::{SigilStitchError, TypeMemberNameOrigin};
use sigil_stitch::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, TypeDeclSyntaxConfig,
};
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, PropertyStyle, TypeKind, Visibility};
use sigil_stitch::spec::property_spec::PropertySpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn getter(body: &str) -> CodeBlock {
    CodeBlock::of(body, ()).unwrap()
}

fn property(name: &str, property_type: TypeName) -> PropertySpec {
    PropertySpec::builder(name, property_type)
        .getter(getter("return current"))
        .build()
        .unwrap()
}

fn render_property(
    lang: &dyn CodeLang,
    property: &PropertySpec,
    context: DeclarationContext,
    width: usize,
) -> Result<String, SigilStitchError> {
    let mut output = String::new();
    for block in property.emit(lang, context)? {
        output.push_str(&block.render_standalone(lang, width)?);
    }
    Ok(output)
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

fn assert_invalid_property(
    lang: &dyn CodeLang,
    property: &PropertySpec,
    context: DeclarationContext,
    expected_reason: &str,
) {
    let error = property.emit(lang, context).unwrap_err();
    let SigilStitchError::InvalidProperty { reason, .. } = error else {
        panic!("expected target-local property rejection, got {error:#?}")
    };
    assert!(
        reason.contains(expected_reason),
        "expected {expected_reason:?} in {reason:?}"
    );
}

#[test]
fn direct_and_type_owned_property_paths_agree_on_success_and_failure() {
    let accepted = PropertySpec::builder("name", TypeName::primitive("string"))
        .getter(getter("return this._name"))
        .setter("value", getter("this._name = value"))
        .build()
        .unwrap();
    let direct = render_property(
        &sigil_stitch::lang::typescript::TypeScript::new(),
        &accepted,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    let nested = render_type(
        sigil_stitch::lang::typescript::TypeScript::new(),
        "owned.ts",
        TypeSpec::builder("Owned", TypeKind::Class)
            .add_property(accepted)
            .build()
            .unwrap(),
    )
    .unwrap();
    for signature in ["get name(): string {", "set name(value: string) {"] {
        assert!(direct.contains(signature), "{direct}");
        assert!(nested.contains(signature), "{nested}");
    }

    let rejected = PropertySpec::builder("shared", TypeName::primitive("String"))
        .is_static()
        .getter(getter("return value"))
        .build()
        .unwrap();
    let direct = rejected
        .emit(
            &sigil_stitch::lang::kotlin::Kotlin::new(),
            DeclarationContext::Member,
        )
        .unwrap_err();
    let nested = TypeSpec::builder("Owned", TypeKind::Class)
        .add_property(rejected)
        .build()
        .unwrap()
        .validate(&sigil_stitch::lang::kotlin::Kotlin::new())
        .unwrap_err();
    assert_eq!(
        std::mem::discriminant(&direct),
        std::mem::discriminant(&nested)
    );
}

#[test]
fn strict_built_ins_reject_unsupported_property_contexts() {
    let property = property("name", TypeName::primitive("string"));
    assert!(matches!(
        property.emit(
            &sigil_stitch::lang::typescript::TypeScript::new(),
            DeclarationContext::TopLevel,
        ),
        Err(SigilStitchError::UnsupportedPropertyContext { .. })
    ));
    assert!(matches!(
        property.emit(
            &sigil_stitch::lang::rust::Rust::new(),
            DeclarationContext::Member,
        ),
        Err(SigilStitchError::UnsupportedPropertyContext { .. })
    ));

    let contract = TypeSpec::builder("Contract", TypeKind::Interface)
        .add_property(property)
        .build()
        .unwrap();
    assert!(matches!(
        contract.validate(&sigil_stitch::lang::typescript::TypeScript::new()),
        Err(SigilStitchError::UnsupportedTypeCapabilities { .. })
    ));
}

#[test]
fn property_profiles_enforce_supported_and_required_semantics() {
    let typed_javascript = property("name", TypeName::primitive("string"));
    assert!(matches!(
        typed_javascript.emit(
            &sigil_stitch::lang::javascript::JavaScript::new(),
            DeclarationContext::Member,
        ),
        Err(SigilStitchError::UnsupportedPropertyCapabilities { .. })
    ));

    let untyped_swift = property("name", TypeName::primitive(""));
    assert!(matches!(
        untyped_swift.emit(
            &sigil_stitch::lang::swift::Swift::new(),
            DeclarationContext::Member,
        ),
        Err(SigilStitchError::MissingRequiredPropertyCapabilities { .. })
    ));

    let write_only_kotlin = PropertySpec::builder("name", TypeName::primitive("String"))
        .setter("value", getter("field = value"))
        .build()
        .unwrap();
    assert!(matches!(
        write_only_kotlin.emit(
            &sigil_stitch::lang::kotlin::Kotlin::new(),
            DeclarationContext::Member,
        ),
        Err(SigilStitchError::MissingRequiredPropertyCapabilities { .. })
    ));
}

#[test]
fn deserialized_property_intrinsics_fail_closed_and_aggregate() {
    let empty: CodeBlock = serde_json::from_value(serde_json::json!({ "nodes": [] })).unwrap();

    let mut invalid_getter =
        serde_json::to_value(property("first", TypeName::primitive("String"))).unwrap();
    invalid_getter["name"] = serde_json::json!("");
    invalid_getter["getter"] = serde_json::to_value(&empty).unwrap();
    invalid_getter["modifiers"]["is_async"] = serde_json::json!(true);
    invalid_getter["modifiers"]["is_abstract"] = serde_json::json!(true);
    invalid_getter["modifiers"]["is_override"] = serde_json::json!(true);
    invalid_getter["modifiers"]["is_constructor"] = serde_json::json!(true);
    invalid_getter["modifiers"]["is_readonly"] = serde_json::json!(true);
    let invalid_getter: PropertySpec = serde_json::from_value(invalid_getter).unwrap();

    let setter = PropertySpec::builder("second", TypeName::primitive("String"))
        .setter("value", getter("stored = value"))
        .build()
        .unwrap();
    let mut invalid_setter = serde_json::to_value(setter).unwrap();
    invalid_setter["setter"]["param_name"] = serde_json::json!("");
    invalid_setter["setter"]["body"] = serde_json::to_value(&empty).unwrap();
    let invalid_setter: PropertySpec = serde_json::from_value(invalid_setter).unwrap();

    let missing_accessors = PropertySpec::builder("third", TypeName::primitive("String"))
        .build()
        .unwrap();
    let file = FileSpec::builder_with(
        "invalid.ts",
        sigil_stitch::lang::typescript::TypeScript::new(),
    )
    .add_type(
        TypeSpec::builder("Broken", TypeKind::Class)
            .add_property(invalid_getter)
            .add_property(invalid_setter)
            .add_property(missing_accessors)
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
        panic!("expected aggregate property validation")
    };
    assert!(error_count >= 6, "{errors:#?}");
    for predicate in [
        |error: &SigilStitchError| matches!(error, SigilStitchError::EmptyName { .. }),
        |error: &SigilStitchError| {
            matches!(error, SigilStitchError::InvalidPropertyModifiers { .. })
        },
        |error: &SigilStitchError| {
            matches!(error, SigilStitchError::EmptyPropertySetterParameter { .. })
        },
        |error: &SigilStitchError| {
            matches!(error, SigilStitchError::MissingPropertyAccessors { .. })
        },
    ] {
        assert!(errors.iter().any(predicate), "{errors:#?}");
    }
    assert!(
        errors
            .iter()
            .filter(|error| matches!(error, SigilStitchError::EmptyPropertyOperand { .. }))
            .count()
            >= 2,
        "{errors:#?}"
    );
}

#[test]
fn adapter_local_property_failures_are_additive() {
    let invalid = PropertySpec::builder("prototype", TypeName::primitive("Value"))
        .visibility(Visibility::PublicSuper)
        .is_static()
        .getter(getter("return stored"))
        .setter("bad-name", getter("stored = value"))
        .build()
        .unwrap();
    let file = FileSpec::builder_with(
        "invalid.ts",
        sigil_stitch::lang::typescript::TypeScript::new(),
    )
    .add_type(
        TypeSpec::builder("Broken", TypeKind::Class)
            .add_property(invalid)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
    let SigilStitchError::FileSpecValidation { errors, .. } = file.validate().unwrap_err() else {
        panic!("expected aggregate property validation")
    };
    let property_errors: Vec<_> = errors
        .iter()
        .filter_map(|error| match error {
            SigilStitchError::InvalidProperty { reason, .. } => Some(reason.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(property_errors.len(), 3, "{errors:#?}");
    for fragment in ["visibility", "prototype", "binding identifier"] {
        assert!(
            property_errors
                .iter()
                .any(|reason| reason.contains(fragment)),
            "{errors:#?}"
        );
    }
}

#[test]
fn accessor_combinations_lower_without_shared_grammar_switches() {
    let ts = sigil_stitch::lang::typescript::TypeScript::new();
    let getter_only = property("value", TypeName::primitive("string"));
    assert_eq!(
        render_property(&ts, &getter_only, DeclarationContext::Member, 120)
            .unwrap()
            .trim(),
        "get value(): string {\n  return current\n}"
    );
    let setter_only = PropertySpec::builder("value", TypeName::primitive("string"))
        .setter("next", getter("current = next"))
        .build()
        .unwrap();
    assert_eq!(
        render_property(&ts, &setter_only, DeclarationContext::Member, 120)
            .unwrap()
            .trim(),
        "set value(next: string) {\n  current = next\n}"
    );

    let kotlin = PropertySpec::builder("value", TypeName::primitive("String"))
        .getter(getter("return stored"))
        .setter("next", getter("stored = next"))
        .build()
        .unwrap();
    let output = render_property(
        &sigil_stitch::lang::kotlin::Kotlin::new(),
        &kotlin,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(output.contains("var value: String\n"), "{output}");
    assert!(output.contains("get() {"), "{output}");
    assert!(output.contains("set(next) {"), "{output}");
    assert!(!output.contains("String {"), "{output}");

    let swift = property("value", TypeName::primitive("String"));
    let output = render_property(
        &sigil_stitch::lang::swift::Swift::new(),
        &swift,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(output.starts_with("var value: String {"), "{output}");
}

#[test]
fn php_and_scala_own_their_accessor_method_grammar() {
    let property = PropertySpec::builder("name", TypeName::primitive("String"))
        .getter(getter("return stored"))
        .setter("value", getter("stored = value"))
        .build()
        .unwrap();
    let php = render_property(
        &sigil_stitch::lang::php::Php::new(),
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(php.contains("public function getName(): String {"), "{php}");
    assert!(
        php.contains("public function setName(String $value) {"),
        "{php}"
    );

    let scala = render_property(
        &sigil_stitch::lang::scala::Scala::new(),
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(scala.contains("def name: String = {"), "{scala}");
    assert!(
        scala.contains("def name_=(value: String): Unit = {"),
        "{scala}"
    );
}

#[test]
fn php_rejects_properties_with_colliding_accessor_names() {
    let ty = TypeSpec::builder("Values", TypeKind::Class)
        .add_property(property("foo", TypeName::primitive("String")))
        .add_property(property("Foo", TypeName::primitive("String")))
        .build()
        .unwrap();
    let error = render_type(sigil_stitch::lang::php::Php::new(), "values.php", ty).unwrap_err();
    let SigilStitchError::FileSpecValidation { errors, .. } = error else {
        panic!("expected aggregate validation, got {error:#?}")
    };
    assert!(matches!(
        errors.as_slice(),
        [SigilStitchError::TypeMemberNameCollision { member_name, .. }]
            if member_name == "getFoo"
    ));
}

#[test]
fn exact_duplicate_property_names_are_rejected_by_crate_validation() {
    let ty = TypeSpec::builder("Values", TypeKind::Class)
        .add_property(property("value", TypeName::primitive("String")))
        .add_property(property("value", TypeName::primitive("String")))
        .build()
        .unwrap();
    let error = ty
        .validate(&sigil_stitch::lang::typescript::TypeScript::new())
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::DuplicatePropertyName {
            type_name,
            property_name,
        } if type_name == "Values" && property_name == "value"
    ));
}

#[test]
fn php_exact_duplicate_properties_only_report_the_crate_owned_error() {
    let ty = TypeSpec::builder("Values", TypeKind::Class)
        .add_property(property("value", TypeName::primitive("String")))
        .add_property(property("value", TypeName::primitive("String")))
        .build()
        .unwrap();
    let error = render_type(sigil_stitch::lang::php::Php::new(), "values.php", ty).unwrap_err();
    let SigilStitchError::FileSpecValidation { errors, .. } = error else {
        panic!("expected aggregate validation, got {error:#?}")
    };
    assert!(matches!(
        errors.as_slice(),
        [SigilStitchError::DuplicatePropertyName {
            type_name,
            property_name,
        }] if type_name == "Values" && property_name == "value"
    ));
}

#[test]
fn stored_and_computed_members_with_one_target_name_fail_closed() {
    let cases: Vec<(Box<dyn CodeLang>, &str)> = vec![
        (
            Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
            "ts",
        ),
        (Box::new(sigil_stitch::lang::kotlin::Kotlin::new()), "kt"),
        (Box::new(sigil_stitch::lang::swift::Swift::new()), "swift"),
        (Box::new(sigil_stitch::lang::scala::Scala::new()), "scala"),
    ];

    for (lang, extension) in cases {
        let ty = TypeSpec::builder("Values", TypeKind::Class)
            .add_field(FieldSpec::of("value", TypeName::primitive("String")))
            .add_property(property("value", TypeName::primitive("String")))
            .build()
            .unwrap();
        let error = ty.validate(lang.as_ref()).unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::TypeMemberNameCollision {
                language,
                member_name,
                first_member,
                ..
            } if language == extension
                && member_name == "value"
                && first_member.as_ref() == &TypeMemberNameOrigin::StoredField {
                    field_name: "value".to_string(),
                }
        ));
    }
}

#[test]
fn target_local_member_namespaces_do_not_create_false_collisions() {
    let typescript = sigil_stitch::lang::typescript::TypeScript::new();
    TypeSpec::builder("Values", TypeKind::Class)
        .add_field(FieldSpec::of("#value", TypeName::primitive("string")))
        .add_property(property("value", TypeName::primitive("string")))
        .build()
        .unwrap()
        .validate(&typescript)
        .unwrap();
    TypeSpec::builder("Values", TypeKind::Class)
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("string"))
                .is_static()
                .build()
                .unwrap(),
        )
        .add_property(property("value", TypeName::primitive("string")))
        .build()
        .unwrap()
        .validate(&typescript)
        .unwrap();

    let swift = sigil_stitch::lang::swift::Swift::new();
    TypeSpec::builder("Values", TypeKind::Class)
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("Int"))
                .is_static()
                .initializer(getter("0"))
                .build()
                .unwrap(),
        )
        .add_property(property("value", TypeName::primitive("Int")))
        .build()
        .unwrap()
        .validate(&swift)
        .unwrap();

    for lang in [
        Box::new(typescript) as Box<dyn CodeLang>,
        Box::new(swift) as Box<dyn CodeLang>,
    ] {
        TypeSpec::builder("Values", TypeKind::Class)
            .add_property(property("value", TypeName::primitive("Int")))
            .add_method(
                FunSpec::builder("value")
                    .returns(TypeName::primitive("Void"))
                    .is_static()
                    .body(getter("return"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
            .validate(lang.as_ref())
            .unwrap();
    }
}

#[test]
fn exact_duplicate_properties_do_not_gain_target_collision_errors() {
    fn assert_only_duplicate(lang: impl CodeLang, filename: &str) {
        let ty = TypeSpec::builder("Values", TypeKind::Class)
            .add_property(property("value", TypeName::primitive("String")))
            .add_property(property("value", TypeName::primitive("String")))
            .build()
            .unwrap();
        let error = FileSpec::builder_with(filename, lang)
            .add_type(ty)
            .build()
            .unwrap()
            .render(120)
            .unwrap_err();
        let SigilStitchError::FileSpecValidation { errors, .. } = error else {
            panic!("expected aggregate validation, got {error:#?}")
        };
        assert!(matches!(
            errors.as_slice(),
            [SigilStitchError::DuplicatePropertyName {
                type_name,
                property_name,
            }] if type_name == "Values" && property_name == "value"
        ));
    }

    assert_only_duplicate(
        sigil_stitch::lang::typescript::TypeScript::new(),
        "values.ts",
    );
    assert_only_duplicate(sigil_stitch::lang::kotlin::Kotlin::new(), "values.kt");
    assert_only_duplicate(sigil_stitch::lang::swift::Swift::new(), "values.swift");
    assert_only_duplicate(sigil_stitch::lang::scala::Scala::new(), "values.scala");
}

#[test]
fn target_member_names_collide_with_explicit_methods_where_the_language_forbids_it() {
    let cases: Vec<Box<dyn CodeLang>> = vec![
        Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
        Box::new(sigil_stitch::lang::swift::Swift::new()),
        Box::new(sigil_stitch::lang::scala::Scala::new()),
    ];

    for lang in cases {
        let ty = TypeSpec::builder("Values", TypeKind::Class)
            .add_property(property("value", TypeName::primitive("String")))
            .add_method(
                FunSpec::builder("value")
                    .body(getter("return current"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let error = ty.validate(lang.as_ref()).unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::TypeMemberNameCollision {
                second_member,
                ..
            } if second_member.as_ref() == &TypeMemberNameOrigin::ExplicitMethod {
                method_name: "value".to_string(),
            }
        ));
    }
}

#[test]
fn scala_write_only_property_collides_with_stored_field_setter() {
    let ty = TypeSpec::builder("Values", TypeKind::Class)
        .add_field(FieldSpec::of("value", TypeName::primitive("String")))
        .add_property(
            PropertySpec::builder("value", TypeName::primitive("String"))
                .setter("next", getter("value = next"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let error = ty
        .validate(&sigil_stitch::lang::scala::Scala::new())
        .unwrap_err();
    assert!(
        matches!(
            &error,
            SigilStitchError::TypeMemberNameCollision {
                second_member,
                ..
            } if second_member.as_ref() == &TypeMemberNameOrigin::PropertyWriteAccessor {
                property_name: "value".to_string(),
            }
        ),
        "{error:#?}"
    );
}

#[test]
fn php_rejects_property_accessors_that_collide_with_explicit_methods() {
    let cases = [
        (
            PropertySpec::builder("foo", TypeName::primitive("String"))
                .getter(getter("return current"))
                .build()
                .unwrap(),
            "getfoo",
            "getFoo",
        ),
        (
            PropertySpec::builder("foo", TypeName::primitive("String"))
                .setter("value", getter("current = value"))
                .build()
                .unwrap(),
            "SETFOO",
            "setFoo",
        ),
    ];

    for (property, method_name, accessor_name) in cases {
        let ty = TypeSpec::builder("Values", TypeKind::Class)
            .add_property(property)
            .add_method(
                FunSpec::builder(method_name)
                    .body(getter("return current"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let error = ty
            .validate(&sigil_stitch::lang::php::Php::new())
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::TypeMemberNameCollision {
                member_name,
                second_member,
                ..
            } if member_name == accessor_name
                && second_member.as_ref() == &TypeMemberNameOrigin::ExplicitMethod {
                    method_name: method_name.to_string(),
                }
        ));
    }
}

#[test]
fn php_getter_and_setter_from_one_property_do_not_self_collide() {
    let ty = TypeSpec::builder("Values", TypeKind::Class)
        .add_property(
            PropertySpec::builder("foo", TypeName::primitive("String"))
                .getter(getter("return current"))
                .setter("value", getter("current = value"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    ty.validate(&sigil_stitch::lang::php::Php::new()).unwrap();
}

#[test]
fn each_language_lowerer_preserves_property_preambles() {
    let languages: Vec<(Box<dyn CodeLang>, TypeName, &str)> = vec![
        (
            Box::new(sigil_stitch::lang::javascript::JavaScript::new()),
            TypeName::primitive(""),
            "@tracked",
        ),
        (
            Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
            TypeName::primitive("Value"),
            "@tracked",
        ),
        (
            Box::new(sigil_stitch::lang::kotlin::Kotlin::new()),
            TypeName::primitive("Value"),
            "@tracked",
        ),
        (
            Box::new(sigil_stitch::lang::swift::Swift::new()),
            TypeName::primitive("Value"),
            "@tracked",
        ),
        (
            Box::new(sigil_stitch::lang::php::Php::new()),
            TypeName::primitive("Value"),
            "#[tracked]",
        ),
        (
            Box::new(sigil_stitch::lang::scala::Scala::new()),
            TypeName::primitive("Value"),
            "@tracked",
        ),
    ];

    for (lang, property_type, structured_annotation) in languages {
        let property = PropertySpec::builder("value", property_type)
            .doc("computed docs")
            .annotate(AnnotationSpec::new("tracked"))
            .annotation(getter("@raw"))
            .getter(getter("return stored"))
            .build()
            .unwrap();
        let output =
            render_property(lang.as_ref(), &property, DeclarationContext::Member, 120).unwrap();
        for fragment in ["computed docs", structured_annotation, "@raw"] {
            assert!(
                output.contains(fragment),
                ".{} missing {fragment:?}: {output}",
                lang.file_extension()
            );
        }
    }
}

#[test]
fn accessor_method_lowerers_attach_preambles_to_setter_only_properties() {
    let languages: Vec<(Box<dyn CodeLang>, TypeName, &str)> = vec![
        (
            Box::new(sigil_stitch::lang::javascript::JavaScript::new()),
            TypeName::primitive(""),
            "set value(next)",
        ),
        (
            Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
            TypeName::primitive("Value"),
            "set value(next: Value)",
        ),
        (
            Box::new(sigil_stitch::lang::php::Php::new()),
            TypeName::primitive("Value"),
            "function setValue(Value $next)",
        ),
        (
            Box::new(sigil_stitch::lang::scala::Scala::new()),
            TypeName::primitive("Value"),
            "def value_=(next: Value)",
        ),
    ];

    for (lang, property_type, signature) in languages {
        let property = PropertySpec::builder("value", property_type)
            .doc("write docs")
            .annotate(AnnotationSpec::new("tracked"))
            .annotation(getter("@raw"))
            .setter("next", getter("stored = next"))
            .build()
            .unwrap();
        let output =
            render_property(lang.as_ref(), &property, DeclarationContext::Member, 120).unwrap();
        for fragment in ["write docs", "tracked", "@raw", signature] {
            assert!(
                output.contains(fragment),
                ".{} missing {fragment:?}: {output}",
                lang.file_extension()
            );
        }
    }
}

#[test]
fn target_local_identifier_visibility_and_reserved_name_rules_fail_closed() {
    for lang in [
        Box::new(sigil_stitch::lang::typescript::TypeScript::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::javascript::JavaScript::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::swift::Swift::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::php::Php::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::scala::Scala::new()) as Box<dyn CodeLang>,
    ] {
        let ty = if lang.file_extension() == "js" {
            TypeName::primitive("")
        } else {
            TypeName::primitive("Value")
        };
        let invalid = property("bad-name", ty);
        assert!(
            render_property(lang.as_ref(), &invalid, DeclarationContext::Member, 120).is_err(),
            ".{}",
            lang.file_extension()
        );
    }

    let invalid_visibility = PropertySpec::builder("value", TypeName::primitive("Value"))
        .visibility(Visibility::PublicSuper)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    assert!(
        render_property(
            &sigil_stitch::lang::swift::Swift::new(),
            &invalid_visibility,
            DeclarationContext::Member,
            120,
        )
        .is_err()
    );

    for lang in [
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::swift::Swift::new()) as Box<dyn CodeLang>,
    ] {
        let reserved = property("class", TypeName::primitive("Value"));
        let output =
            render_property(lang.as_ref(), &reserved, DeclarationContext::Member, 120).unwrap();
        assert!(output.contains("`class`: Value"), "{output}");
    }
}

#[test]
fn target_local_property_policies_cover_each_rejection_boundary() {
    let javascript = sigil_stitch::lang::javascript::JavaScript::new();
    let public_private_name = PropertySpec::builder("#value", TypeName::primitive(""))
        .visibility(Visibility::Public)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    assert_invalid_property(
        &javascript,
        &public_private_name,
        DeclarationContext::Member,
        "private accessors",
    );
    let private_name = PropertySpec::builder("#value", TypeName::primitive(""))
        .visibility(Visibility::Private)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    let output =
        render_property(&javascript, &private_name, DeclarationContext::Member, 120).unwrap();
    assert!(output.contains("get #value()"), "{output}");
    let protected = PropertySpec::builder("value", TypeName::primitive(""))
        .visibility(Visibility::Protected)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    assert_invalid_property(
        &javascript,
        &protected,
        DeclarationContext::Member,
        "visibility is implicit",
    );
    for (property, reason) in [
        (
            PropertySpec::builder("constructor", TypeName::primitive(""))
                .getter(getter("return stored"))
                .build()
                .unwrap(),
            "constructor",
        ),
        (
            PropertySpec::builder("prototype", TypeName::primitive(""))
                .is_static()
                .getter(getter("return stored"))
                .build()
                .unwrap(),
            "prototype",
        ),
        (
            PropertySpec::builder("value", TypeName::primitive(""))
                .setter("bad-name", getter("stored = value"))
                .build()
                .unwrap(),
            "binding identifier",
        ),
    ] {
        assert_invalid_property(&javascript, &property, DeclarationContext::Member, reason);
    }

    let typescript = sigil_stitch::lang::typescript::TypeScript::new();
    let private_name = property("#value", TypeName::primitive("Value"));
    let output =
        render_property(&typescript, &private_name, DeclarationContext::Member, 120).unwrap();
    assert!(output.contains("get #value(): Value"), "{output}");
    assert_invalid_property(
        &typescript,
        &property("constructor", TypeName::primitive("Value")),
        DeclarationContext::Member,
        "constructor",
    );

    let kotlin = sigil_stitch::lang::kotlin::Kotlin::new();
    let invalid_setter = PropertySpec::builder("value", TypeName::primitive("Value"))
        .getter(getter("return stored"))
        .setter("bad-name", getter("stored = value"))
        .build()
        .unwrap();
    assert_invalid_property(
        &kotlin,
        &invalid_setter,
        DeclarationContext::Member,
        "setter parameter",
    );
    let invalid_visibility = PropertySpec::builder("value", TypeName::primitive("Value"))
        .visibility(Visibility::PublicSuper)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    assert_invalid_property(
        &kotlin,
        &invalid_visibility,
        DeclarationContext::Member,
        "visibility",
    );
    let contract_private = PropertySpec::builder("value", TypeName::primitive("Value"))
        .visibility(Visibility::Private)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    assert_invalid_property(
        &kotlin,
        &contract_private,
        DeclarationContext::InterfaceMember,
        "visibility",
    );
    let contract = property("value", TypeName::primitive("Value"));
    let output =
        render_property(&kotlin, &contract, DeclarationContext::InterfaceMember, 120).unwrap();
    assert!(output.starts_with("val value: Value"), "{output}");
    for (visibility, prefix) in [
        (Visibility::Private, "private val"),
        (Visibility::Protected, "protected val"),
        (Visibility::PublicCrate, "internal val"),
    ] {
        let property = PropertySpec::builder("value", TypeName::primitive("Value"))
            .visibility(visibility)
            .getter(getter("return stored"))
            .build()
            .unwrap();
        let output = render_property(&kotlin, &property, DeclarationContext::Member, 120).unwrap();
        assert!(output.starts_with(prefix), "{output}");
    }

    let php = sigil_stitch::lang::php::Php::new();
    let invalid_setter = PropertySpec::builder("value", TypeName::primitive("Value"))
        .getter(getter("return stored"))
        .setter("bad-name", getter("stored = value"))
        .build()
        .unwrap();
    assert_invalid_property(
        &php,
        &invalid_setter,
        DeclarationContext::Member,
        "setter parameter",
    );
    let invalid_visibility = PropertySpec::builder("value", TypeName::primitive("Value"))
        .visibility(Visibility::PublicCrate)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    assert_invalid_property(
        &php,
        &invalid_visibility,
        DeclarationContext::Member,
        "visibility",
    );
    for (visibility, prefix) in [
        (Visibility::Private, "private function"),
        (Visibility::Protected, "protected function"),
    ] {
        let property = PropertySpec::builder("value", TypeName::primitive("Value"))
            .visibility(visibility)
            .getter(getter("return stored"))
            .build()
            .unwrap();
        let output = render_property(&php, &property, DeclarationContext::Member, 120).unwrap();
        assert!(output.starts_with(prefix), "{output}");
    }

    let scala = sigil_stitch::lang::scala::Scala::new();
    let invalid = PropertySpec::builder("value", TypeName::primitive(""))
        .visibility(Visibility::PublicCrate)
        .setter("_", getter("stored = value"))
        .build()
        .unwrap();
    let error = invalid
        .emit(&scala, DeclarationContext::Member)
        .unwrap_err();
    assert!(
        matches!(error, SigilStitchError::InvalidProperty { .. }),
        "{error:#?}"
    );

    let swift = sigil_stitch::lang::swift::Swift::new();
    let invalid_setter = PropertySpec::builder("value", TypeName::primitive("Value"))
        .getter(getter("return stored"))
        .setter("_", getter("stored = value"))
        .build()
        .unwrap();
    assert_invalid_property(
        &swift,
        &invalid_setter,
        DeclarationContext::Member,
        "setter parameter",
    );
}

#[test]
fn static_property_support_is_explicit() {
    for lang in [
        Box::new(sigil_stitch::lang::typescript::TypeScript::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::javascript::JavaScript::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::swift::Swift::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::php::Php::new()) as Box<dyn CodeLang>,
    ] {
        let ty = if lang.file_extension() == "js" {
            TypeName::primitive("")
        } else {
            TypeName::primitive("Value")
        };
        let property = PropertySpec::builder("shared", ty)
            .is_static()
            .getter(getter("return stored"))
            .build()
            .unwrap();
        let output =
            render_property(lang.as_ref(), &property, DeclarationContext::Member, 120).unwrap();
        assert!(
            output.contains("static"),
            ".{}: {output}",
            lang.file_extension()
        );
    }

    for lang in [
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::scala::Scala::new()) as Box<dyn CodeLang>,
    ] {
        let property = PropertySpec::builder("shared", TypeName::primitive("Value"))
            .is_static()
            .getter(getter("return stored"))
            .build()
            .unwrap();
        assert!(matches!(
            property.emit(lang.as_ref(), DeclarationContext::Member),
            Err(SigilStitchError::UnsupportedPropertyCapabilities { .. })
        ));
    }
}

#[test]
fn property_type_refs_preserve_import_alias_resolution() {
    let ty = TypeSpec::builder("Users", TypeKind::Class)
        .add_property(
            PropertySpec::builder("current", TypeName::importable_type("./models", "User"))
                .annotate(AnnotationSpec::importable(TypeName::importable_type(
                    "./decorators",
                    "Tracked",
                )))
                .getter(getter("return current"))
                .build()
                .unwrap(),
        )
        .add_property(property(
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
    assert!(output.contains("from './decorators'"), "{output}");
    assert!(output.contains("@Tracked"), "{output}");
    assert!(output.contains("get current(): User {"), "{output}");
    assert!(output.contains("get legacy(): LegacyUser {"), "{output}");
}

#[test]
fn property_lowering_preserves_direct_and_pretty_renderer_parity() {
    let lang = sigil_stitch::lang::typescript::TypeScript::new();
    let direct = PropertySpec::builder("value", TypeName::primitive("Result"))
        .getter(getter("return call(alpha, beta)"))
        .build()
        .unwrap();
    let pretty = PropertySpec::builder("value", TypeName::primitive("Result"))
        .getter(CodeBlock::of("return call(alpha,%Wbeta)", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render_property(&lang, &direct, DeclarationContext::Member, 120).unwrap(),
        render_property(&lang, &pretty, DeclarationContext::Member, 120).unwrap()
    );
}

#[derive(Debug, Clone, Copy)]
struct LegacyPropertyLang {
    style: PropertyStyle,
    split_members: bool,
    docs_inside_body: bool,
}

impl RendererLang for LegacyPropertyLang {
    fn file_extension(&self) -> &str {
        "legacy-properties"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: "  ",
            ..BlockSyntaxConfig::default()
        }
    }
}

impl CodeLang for LegacyPropertyLang {
    fn render_visibility(&self, visibility: Visibility, context: DeclarationContext) -> &str {
        if visibility == Visibility::Public {
            match (self.split_members, context) {
                (true, DeclarationContext::Member) => "member ",
                (true, DeclarationContext::InterfaceMember) => "contract ",
                _ => "public ",
            }
        } else {
            ""
        }
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            static_keyword: "shared ",
            return_type_separator: " -> ",
            ..FunctionSyntaxConfig::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_annotation_separator: " :: ",
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

    fn property_style(&self) -> PropertyStyle {
        self.style
    }

    fn property_getter_keyword(&self) -> &str {
        "read"
    }

    fn doc_before_annotations(&self) -> bool {
        false
    }

    fn doc_comment_inside_body(&self) -> bool {
        self.docs_inside_body
    }

    fn type_keyword(&self, _: TypeKind) -> &str {
        "record"
    }

    fn methods_inside_type_body(&self, _: TypeKind) -> bool {
        !self.split_members
    }
}

#[test]
fn external_adapters_keep_frozen_property_compatibility() {
    let property = PropertySpec::builder("name", TypeName::primitive("T"))
        .visibility(Visibility::Public)
        .is_static()
        .doc("docs")
        .annotate(AnnotationSpec::new("tracked"))
        .annotation(getter("@raw"))
        .getter(getter("return stored"))
        .setter("value", getter("stored = value"))
        .build()
        .unwrap();

    let accessor = render_property(
        &LegacyPropertyLang {
            style: PropertyStyle::Accessor,
            split_members: false,
            docs_inside_body: false,
        },
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(
        accessor.contains("@[tracked]\n@raw\n// docs\n"),
        "{accessor}"
    );
    assert!(
        accessor.contains("public shared get name() -> T {"),
        "{accessor}"
    );
    assert!(
        accessor.contains("public shared set name($value :: T) {"),
        "{accessor}"
    );

    let field = render_property(
        &LegacyPropertyLang {
            style: PropertyStyle::Field,
            split_members: false,
            docs_inside_body: false,
        },
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(field.contains("public shared var $name :: T {"), "{field}");
    assert!(field.contains("read {"), "{field}");
    assert!(field.contains("set(value) {"), "{field}");

    let nested = render_type(
        LegacyPropertyLang {
            style: PropertyStyle::Accessor,
            split_members: false,
            docs_inside_body: false,
        },
        "legacy-properties",
        TypeSpec::builder("Owned", TypeKind::Class)
            .add_property(property)
            .build()
            .unwrap(),
    )
    .unwrap();
    assert!(nested.contains("record Owned {"), "{nested}");
    assert!(
        nested.contains("public shared get name() -> T {"),
        "{nested}"
    );
}

#[test]
fn split_external_adapters_keep_member_context_for_legacy_properties() {
    let property = PropertySpec::builder("name", TypeName::primitive("T"))
        .visibility(Visibility::Public)
        .getter(getter("return stored"))
        .build()
        .unwrap();
    let output = render_type(
        LegacyPropertyLang {
            style: PropertyStyle::Accessor,
            split_members: true,
            docs_inside_body: false,
        },
        "legacy-properties",
        TypeSpec::builder("Contract", TypeKind::Interface)
            .add_property(property)
            .build()
            .unwrap(),
    )
    .unwrap();
    assert!(output.contains("member get name() -> T {"), "{output}");
    assert!(!output.contains("contract get name()"), "{output}");
}

#[test]
fn setter_only_external_adapters_keep_the_property_preamble() {
    let property = PropertySpec::builder("name", TypeName::primitive("T"))
        .doc("docs")
        .annotate(AnnotationSpec::new("tracked"))
        .annotation(getter("@raw"))
        .setter("value", getter("stored = value"))
        .build()
        .unwrap();
    let output = render_property(
        &LegacyPropertyLang {
            style: PropertyStyle::Accessor,
            split_members: false,
            docs_inside_body: false,
        },
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(output.contains("@[tracked]\n@raw\n// docs\n"), "{output}");
    assert_eq!(output.matches("docs").count(), 1, "{output}");
}

#[test]
fn external_accessor_adapters_place_inside_body_docs_once() {
    let property = PropertySpec::builder("name", TypeName::primitive("T"))
        .doc("docs")
        .getter(getter("return stored"))
        .setter("value", getter("stored = value"))
        .build()
        .unwrap();
    let output = render_property(
        &LegacyPropertyLang {
            style: PropertyStyle::Accessor,
            split_members: false,
            docs_inside_body: true,
        },
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(
        output.contains("get name() -> T {\n  // docs\n  return stored"),
        "{output}"
    );
    assert_eq!(output.matches("docs").count(), 1, "{output}");
}

#[test]
fn external_field_adapters_place_inside_body_docs_once() {
    let property = PropertySpec::builder("name", TypeName::primitive("T"))
        .doc("docs")
        .getter(getter("return stored"))
        .build()
        .unwrap();
    let output = render_property(
        &LegacyPropertyLang {
            style: PropertyStyle::Field,
            split_members: false,
            docs_inside_body: true,
        },
        &property,
        DeclarationContext::Member,
        120,
    )
    .unwrap();
    assert!(
        output.contains("fixed $name :: T {\n  // docs\n  read {"),
        "{output}"
    );
    assert_eq!(output.matches("docs").count(), 1, "{output}");
}
