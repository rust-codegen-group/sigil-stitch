#![allow(deprecated)]

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::{
    LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
use sigil_stitch::lang::config::{EnumAndAnnotationConfig, VariantValueFormat};
use sigil_stitch::lang::{CodeLang, RendererLang, ValidatedVariants, VariantIntent};
use sigil_stitch::spec::annotation_spec::{AnnotationNameRef, AnnotationSpec};
use sigil_stitch::spec::enum_variant_spec::{EnumVariantSpec, VariantContext};
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[derive(Debug)]
struct LegacyLang;

impl RendererLang for LegacyLang {
    fn file_extension(&self) -> &str {
        "legacy-variants"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for LegacyLang {}

#[derive(Debug)]
struct LegacyVariantsFirstLang;

impl RendererLang for LegacyVariantsFirstLang {
    fn file_extension(&self) -> &str {
        "legacy-variants-first"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for LegacyVariantsFirstLang {
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            variants_before_fields: true,
            ..EnumAndAnnotationConfig::default()
        }
    }
}

#[derive(Debug)]
struct LegacyRichSyntaxLang;

impl RendererLang for LegacyRichSyntaxLang {
    fn file_extension(&self) -> &str {
        "legacy-rich-variants"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for LegacyRichSyntaxLang {
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            variant_prefix: "next ",
            variant_prefix_first: Some("first "),
            variant_separator: " |",
            variant_trailing_separator: true,
            variant_value_format: VariantValueFormat::ConstructorArg,
            variant_section_terminator: ";",
            annotation_prefix: "@[",
            annotation_suffix: "]",
            ..EnumAndAnnotationConfig::default()
        }
    }

    fn doc_before_annotations(&self) -> bool {
        false
    }
}

#[derive(Debug)]
struct SemanticViewLang;

const SEMANTIC_TYPES: &[TypeCapabilityProfile<'_>] = &[TypeCapabilityProfile::new(
    TypeKind::Enum,
    &[TypeCapability::Variants],
)];
const SEMANTIC_VARIANTS: &[VariantCapabilityProfile<'_>] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[
        VariantCapability::Discriminant,
        VariantCapability::ConstructorArguments,
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
        VariantCapability::Attributes,
    ],
)];

impl RendererLang for SemanticViewLang {
    fn file_extension(&self) -> &str {
        "semantic-variants"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for SemanticViewLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(SEMANTIC_TYPES)
            .with_variants(SEMANTIC_VARIANTS)
    }

    fn validate_variants(&self, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
        assert_eq!(variants.owner_name(), "Semantic");
        assert_eq!(variants.owner_kind(), TypeKind::Enum);
        assert!(variants.has_following_members());
        assert_eq!(variants.variants().len(), 4);

        let discriminated = &variants.variants()[0];
        assert_eq!(discriminated.name(), "Discriminated");
        assert_eq!(discriminated.doc(), &["Discriminated variant"]);
        assert!(discriminated.discriminant().is_some());
        assert_eq!(discriminated.annotations().len(), 1);
        assert!(matches!(
            discriminated.annotation_specs()[0].name(),
            AnnotationNameRef::Importable(name)
                if name == &TypeName::importable("annotations", "Tracked")
        ));

        let constructed = &variants.variants()[1];
        assert_eq!(constructed.constructor_arguments().len(), 2);

        let positional = &variants.variants()[2];
        assert_eq!(
            positional.positional_payload(),
            &[TypeName::primitive("First"), TypeName::primitive("Second")]
        );

        let record = &variants.variants()[3];
        let field = &record.record_payload()[0];
        assert_eq!(field.name(), "payload");
        assert_eq!(field.field_type(), &TypeName::primitive("Payload"));
        assert_eq!(field.modifiers().visibility, Visibility::Private);
        assert!(field.modifiers().is_readonly);
        assert_eq!(field.doc(), &["Payload field"]);
        assert!(field.initializer().is_some());
        assert_eq!(field.annotations().len(), 1);
        assert_eq!(field.annotation_specs().len(), 1);
        assert_eq!(field.tag(), Some("payload"));
        assert!(field.is_optional());
        Ok(())
    }

    fn lower_variants(
        &self,
        variants: ValidatedVariants<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        assert_eq!(variants.owner_name(), "Semantic");
        CodeBlock::of("semantic variants", ())
    }
}

#[test]
fn strict_built_ins_reject_ownerless_positional_emission() {
    let variant = EnumVariantSpec::new("Only").unwrap();
    let error = variant
        .emit(
            &sigil_stitch::lang::rust::Rust::new(),
            VariantContext {
                is_first: true,
                is_last: true,
                has_trailing_members: false,
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::VariantOwnerRequired {
            language,
            variant_name,
        } if language == "rs" && variant_name == "Only"
    ));

    let mut builder = CodeBlock::builder();
    let error = variant
        .emit_into(
            &mut builder,
            &sigil_stitch::lang::rust::Rust::new(),
            VariantContext {
                is_first: true,
                is_last: true,
                has_trailing_members: false,
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::VariantOwnerRequired { .. }
    ));
}

#[test]
fn permissive_external_adapter_keeps_direct_and_sequence_compatibility() {
    let variant = EnumVariantSpec::builder("Legacy")
        .value(CodeBlock::of("7", ()).unwrap())
        .build()
        .unwrap();
    let direct = render_block(
        &LegacyLang,
        &variant
            .emit(
                &LegacyLang,
                VariantContext {
                    is_first: true,
                    is_last: true,
                    has_trailing_members: false,
                },
            )
            .unwrap(),
    );
    assert_eq!(direct, "Legacy = 7\n");

    let output = FileSpec::builder_with("Legacy.legacy-variants", LegacyLang)
        .add_type(
            TypeSpec::builder("Legacy", TypeKind::Enum)
                .add_variant(variant)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("Legacy = 7"), "{output}");
}

#[test]
fn permissive_external_adapter_preserves_full_legacy_variant_grammar() {
    let decorated_tuple = EnumVariantSpec::builder("Tuple")
        .doc("Tuple docs")
        .annotate(AnnotationSpec::new("tracked"))
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .associated_type(TypeName::primitive("First"))
        .associated_type(TypeName::primitive("Second"))
        .build()
        .unwrap();
    let tuple = render_block(
        &LegacyRichSyntaxLang,
        &decorated_tuple
            .emit(
                &LegacyRichSyntaxLang,
                VariantContext {
                    is_first: true,
                    is_last: false,
                    has_trailing_members: false,
                },
            )
            .unwrap(),
    );
    assert!(
        tuple.contains("@[tracked]\n@raw\n// Tuple docs\nfirst Tuple(First, Second) |"),
        "{tuple}"
    );

    let record = EnumVariantSpec::builder("Record")
        .add_field(FieldSpec::of("payload", TypeName::primitive("Payload")))
        .build()
        .unwrap();
    let record = render_block(
        &LegacyRichSyntaxLang,
        &record
            .emit(
                &LegacyRichSyntaxLang,
                VariantContext {
                    is_first: false,
                    is_last: true,
                    has_trailing_members: true,
                },
            )
            .unwrap(),
    );
    assert!(record.contains("next Record {"), "{record}");
    assert!(record.contains("payload: Payload,"), "{record}");
    assert!(record.contains("};"), "{record}");

    let constructor = EnumVariantSpec::builder("Constructed")
        .constructor_argument(CodeBlock::of("1", ()).unwrap())
        .constructor_argument(CodeBlock::of("2", ()).unwrap())
        .build()
        .unwrap();
    let constructor = render_block(
        &LegacyRichSyntaxLang,
        &constructor
            .emit(
                &LegacyRichSyntaxLang,
                VariantContext {
                    is_first: false,
                    is_last: false,
                    has_trailing_members: false,
                },
            )
            .unwrap(),
    );
    assert_eq!(constructor, "next Constructed(1, 2) |\n");

    let legacy_value = EnumVariantSpec::builder("Value")
        .value(CodeBlock::of("7", ()).unwrap())
        .build()
        .unwrap();
    let value = render_block(
        &LegacyRichSyntaxLang,
        &legacy_value
            .emit(
                &LegacyRichSyntaxLang,
                VariantContext {
                    is_first: false,
                    is_last: false,
                    has_trailing_members: false,
                },
            )
            .unwrap(),
    );
    assert_eq!(value, "next Value(7) |\n");

    let documented = EnumVariantSpec::builder("Documented")
        .doc("Before annotations")
        .annotate(AnnotationSpec::new("tracked"))
        .build()
        .unwrap();
    let documented = render_block(
        &LegacyLang,
        &documented
            .emit(
                &LegacyLang,
                VariantContext {
                    is_first: true,
                    is_last: true,
                    has_trailing_members: false,
                },
            )
            .unwrap(),
    );
    assert!(
        documented.starts_with("// Before annotations\n@tracked\nDocumented"),
        "{documented}"
    );
}

#[test]
fn php_and_python_lower_documented_and_explicit_enum_members() {
    let php = TypeSpec::builder("Status", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Ready")
                .doc("Ready docs")
                .annotate(AnnotationSpec::new("Tracked"))
                .annotation(CodeBlock::of("#[Raw]", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(EnumVariantSpec::new("Pending").unwrap())
        .build()
        .unwrap();
    let php = render_type(&php, &sigil_stitch::lang::php::Php::new());
    assert!(php.contains("Ready docs"), "{php}");
    assert!(php.contains("#[Tracked]"), "{php}");
    assert!(php.contains("#[Raw]"), "{php}");
    assert!(php.contains("case Ready;"), "{php}");
    assert!(php.contains("case Pending;"), "{php}");

    let python = TypeSpec::builder("Status", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Ready")
                .doc("Ready docs")
                .doc("")
                .discriminant(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(EnumVariantSpec::new("Pending").unwrap())
        .build()
        .unwrap();
    let python = render_type(&python, &sigil_stitch::lang::python::Python::new());
    assert!(python.contains("# Ready docs\n    #"), "{python}");
    assert!(python.contains("Ready = 1"), "{python}");
    assert!(python.contains("Pending = 'Pending'"), "{python}");
}

#[test]
fn permissive_external_adapter_keeps_configured_variant_placement() {
    let make_spec = || {
        TypeSpec::builder("Container", TypeKind::Enum)
            .add_field(FieldSpec::of("field", TypeName::primitive("int")))
            .add_variant(EnumVariantSpec::new("Variant").unwrap())
            .build()
            .unwrap()
    };

    let fields_first = render_type(&make_spec(), &LegacyLang);
    assert!(
        fields_first.find("field").unwrap() < fields_first.find("Variant").unwrap(),
        "{fields_first}"
    );

    let variants_first = render_type(&make_spec(), &LegacyVariantsFirstLang);
    assert!(
        variants_first.find("Variant").unwrap() < variants_first.find("field").unwrap(),
        "{variants_first}"
    );
}

#[test]
fn permissive_direct_emission_still_rejects_incompatible_new_forms() {
    let variant = EnumVariantSpec::builder("Mixed")
        .value(CodeBlock::of("1", ()).unwrap())
        .positional_payload(TypeName::primitive("Payload"))
        .build()
        .unwrap();

    assert!(matches!(
        variant.emit(
            &LegacyLang,
            VariantContext {
                is_first: true,
                is_last: true,
                has_trailing_members: false,
            },
        ),
        Err(SigilStitchError::IncompatibleVariantCapabilities { .. })
    ));
}

#[test]
fn unsupported_and_incompatible_semantics_fail_before_lowering() {
    let unsupported = TypeSpec::builder("DartValue", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("one")
                .discriminant(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .emit(&sigil_stitch::lang::dart::Dart::new())
        .unwrap_err();
    assert!(matches!(
        unsupported,
        SigilStitchError::UnsupportedVariantCapabilities {
            capabilities,
            ..
        } if capabilities == vec![VariantCapability::Discriminant]
    ));

    let incompatible = TypeSpec::builder("Mixed", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Mixed")
                .discriminant(CodeBlock::of("1", ()).unwrap())
                .positional_payload(TypeName::primitive("Payload"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .emit(&sigil_stitch::lang::rust::Rust::new())
        .unwrap_err();
    assert!(matches!(
        incompatible,
        SigilStitchError::IncompatibleVariantCapabilities { capabilities, .. }
            if capabilities
                == vec![
                    VariantCapability::Discriminant,
                    VariantCapability::PositionalPayload,
                ]
    ));
}

#[test]
fn discriminants_and_constructor_arguments_have_distinct_lowering() {
    let typescript = render_type(
        &TypeSpec::builder("Direction", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Up")
                    .discriminant(CodeBlock::of("1", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::typescript::TypeScript::new(),
    );
    assert!(typescript.contains("Up = 1,"), "{typescript}");

    let java = render_type(
        &TypeSpec::builder("Choice", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("ONE")
                    .constructor_argument(CodeBlock::of("1", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .extra_member(CodeBlock::of("private Choice(int value) {}\n", ()).unwrap())
            .build()
            .unwrap(),
        &sigil_stitch::lang::java::Java::new(),
    );
    assert!(java.contains("ONE(1);"), "{java}");
    assert!(!java.contains("ONE = 1"), "{java}");
}

#[test]
fn ambiguous_legacy_values_are_rejected_when_no_valid_local_meaning_exists() {
    let languages: Vec<Box<dyn CodeLang>> = vec![
        Box::new(sigil_stitch::lang::dart::Dart::new()),
        Box::new(sigil_stitch::lang::haskell::Haskell::new()),
        Box::new(sigil_stitch::lang::ocaml::OCaml::new()),
        Box::new(sigil_stitch::lang::php::Php::new()),
        Box::new(sigil_stitch::lang::scala::Scala::new()),
        Box::new(sigil_stitch::lang::swift::Swift::new()),
    ];

    for lang in languages {
        let spec = TypeSpec::builder("Legacy", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Legacy")
                    .value(CodeBlock::of("1", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(matches!(
            spec.emit(lang.as_ref()),
            Err(SigilStitchError::UnsupportedLegacyVariantValue { .. })
        ));
    }
}

#[test]
fn legacy_values_keep_valid_language_local_meanings() {
    let legacy_variant = || {
        EnumVariantSpec::builder("Legacy")
            .value(CodeBlock::of("1", ()).unwrap())
            .build()
            .unwrap()
    };
    let typescript = render_type(
        &TypeSpec::builder("Legacy", TypeKind::Enum)
            .add_variant(legacy_variant())
            .build()
            .unwrap(),
        &sigil_stitch::lang::typescript::TypeScript::new(),
    );
    assert!(typescript.contains("Legacy = 1,"), "{typescript}");

    let java = render_type(
        &TypeSpec::builder("Legacy", TypeKind::Enum)
            .add_variant(legacy_variant())
            .extra_member(CodeBlock::of("private Legacy(int value) {}\n", ()).unwrap())
            .build()
            .unwrap(),
        &sigil_stitch::lang::java::Java::new(),
    );
    assert!(java.contains("Legacy(1)"), "{java}");
}

#[test]
fn adapter_validation_receives_the_complete_read_only_sequence() {
    let record_field = FieldSpec::builder("payload", TypeName::primitive("Payload"))
        .visibility(Visibility::Private)
        .is_readonly()
        .doc("Payload field")
        .initializer(CodeBlock::of("default_payload", ()).unwrap())
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .annotate(AnnotationSpec::new("structured"))
        .tag("payload")
        .is_optional()
        .build()
        .unwrap();
    let spec = TypeSpec::builder("Semantic", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Discriminated")
                .doc("Discriminated variant")
                .discriminant(CodeBlock::of("1", ()).unwrap())
                .annotation(CodeBlock::of("@raw", ()).unwrap())
                .annotate(AnnotationSpec::importable(TypeName::importable(
                    "annotations",
                    "Tracked",
                )))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Constructed")
                .constructor_argument(CodeBlock::of("first", ()).unwrap())
                .constructor_argument(CodeBlock::of("second", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Positional")
                .positional_payload(TypeName::primitive("First"))
                .positional_payload(TypeName::primitive("Second"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Record")
                .record_payload_field(record_field)
                .build()
                .unwrap(),
        )
        .extra_member(CodeBlock::of("after", ()).unwrap())
        .build()
        .unwrap();

    let blocks = spec.emit(&SemanticViewLang).unwrap();
    assert!(!blocks.is_empty());
}

#[test]
fn payload_types_and_structured_annotations_preserve_import_tracking() {
    let file = FileSpec::builder_with("Message.rs", sigil_stitch::lang::rust::Rust::new())
        .add_type(
            TypeSpec::builder("Message", TypeKind::Enum)
                .add_variant(
                    EnumVariantSpec::builder("Current")
                        .annotate(AnnotationSpec::importable(TypeName::importable(
                            "crate::annotations",
                            "Tracked",
                        )))
                        .positional_payload(TypeName::importable("crate::models", "User"))
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Legacy")
                        .record_payload_field(FieldSpec::of(
                            "user",
                            TypeName::importable("crate::legacy", "User"),
                        ))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("use crate::annotations::Tracked;"),
        "{output}"
    );
    assert!(output.contains("use crate::models::User;"), "{output}");
    assert!(
        output.contains("use crate::legacy::User as LegacyUser;"),
        "{output}"
    );
    assert!(output.contains("#[Tracked]"), "{output}");
    assert!(output.contains("Current(User)"), "{output}");
    assert!(output.contains("user: LegacyUser"), "{output}");
}

#[test]
fn nested_variant_values_preserve_direct_and_pretty_layout_paths() {
    let value = CodeBlock::of("first_operand%W+%Wsecond_operand", ()).unwrap();
    let variant = EnumVariantSpec::builder("Combined")
        .discriminant(value)
        .build()
        .unwrap();
    let make_file = || {
        FileSpec::builder_with(
            "Combined.ts",
            sigil_stitch::lang::typescript::TypeScript::new(),
        )
        .add_type(
            TypeSpec::builder("Combined", TypeKind::Enum)
                .add_variant(variant.clone())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
    };

    let direct = make_file().render(120).unwrap();
    let pretty = make_file().render(20).unwrap();
    assert!(
        direct.contains("Combined = first_operand + second_operand,"),
        "{direct}"
    );
    assert!(pretty.contains("first_operand\n"), "{pretty}");
    assert!(pretty.contains("+ second_operand,"), "{pretty}");
}

#[test]
fn file_validation_aggregates_independent_variant_failures() {
    let invalid = TypeSpec::builder("Invalid", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("One")
                .discriminant(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Two")
                .positional_payload(TypeName::primitive("Payload"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let file = FileSpec::builder_with("invalid.dart", sigil_stitch::lang::dart::Dart::new())
        .add_type(invalid)
        .build()
        .unwrap();

    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = file.validate().unwrap_err()
    else {
        panic!("expected FileSpecValidation");
    };
    assert_eq!(error_count, 2);
    assert!(errors.iter().all(|error| matches!(
        error,
        SigilStitchError::UnsupportedVariantCapabilities { .. }
    )));
}

#[test]
fn intrinsic_variant_uniqueness_errors_are_aggregated() {
    let invalid = TypeSpec::builder("Invalid", TypeKind::Enum)
        .add_variant(EnumVariantSpec::new("Duplicate").unwrap())
        .add_variant(EnumVariantSpec::new("Duplicate").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Record")
                .record_payload_field(FieldSpec::of("value", TypeName::primitive("i32")))
                .record_payload_field(FieldSpec::of("value", TypeName::primitive("String")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let file = FileSpec::builder_with("invalid.rs", sigil_stitch::lang::rust::Rust::new())
        .add_type(invalid)
        .build()
        .unwrap();

    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = file.validate().unwrap_err()
    else {
        panic!("expected FileSpecValidation");
    };
    assert_eq!(error_count, 2, "{errors:#?}");
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::DuplicateVariantName { variant_name, .. }
            if variant_name == "Duplicate"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::DuplicateVariantRecordFieldName {
            variant_name,
            field_name,
        } if variant_name == "Record" && field_name == "value"
    )));
}

#[test]
fn capability_and_adapter_local_sibling_errors_are_aggregated() {
    let invalid = TypeSpec::builder("Invalid", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Explicit")
                .discriminant(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Legacy")
                .value(CodeBlock::of("2", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("AlsoLegacy")
                .value(CodeBlock::of("3", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let file = FileSpec::builder_with("invalid.dart", sigil_stitch::lang::dart::Dart::new())
        .add_type(invalid)
        .build()
        .unwrap();

    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = file.validate().unwrap_err()
    else {
        panic!("expected FileSpecValidation");
    };
    assert_eq!(error_count, 3, "{errors:#?}");
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::UnsupportedVariantCapabilities { variant_name, .. }
            if variant_name == "Explicit"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::UnsupportedLegacyVariantValue { variant_name, .. }
            if variant_name == "Legacy"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::UnsupportedLegacyVariantValue { variant_name, .. }
            if variant_name == "AlsoLegacy"
    )));
}

#[test]
fn adapter_local_record_field_sibling_errors_are_aggregated() {
    fn assert_aggregated(filename: &str, lang: impl CodeLang) {
        let invalid_field = |variant_name: &str, field_name: &str| {
            EnumVariantSpec::builder(variant_name)
                .record_payload_field(
                    FieldSpec::builder(field_name, TypeName::primitive("i32"))
                        .tag("unsupported")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap()
        };
        let invalid = TypeSpec::builder("Invalid", TypeKind::Enum)
            .add_variant(invalid_field("First", "first"))
            .add_variant(invalid_field("Second", "second"))
            .build()
            .unwrap();
        let file = FileSpec::builder_with(filename, lang)
            .add_type(invalid)
            .build()
            .unwrap();

        let SigilStitchError::FileSpecValidation {
            error_count,
            errors,
            ..
        } = file.validate().unwrap_err()
        else {
            panic!("expected FileSpecValidation for {filename}");
        };
        assert_eq!(error_count, 2, "{filename}: {errors:#?}");
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, SigilStitchError::InvalidVariantRecordField { .. })),
            "{filename}: {errors:#?}"
        );
    }

    assert_aggregated("invalid.rs", sigil_stitch::lang::rust::Rust::new());
    assert_aggregated("invalid.hs", sigil_stitch::lang::haskell::Haskell::new());
    assert_aggregated("invalid.ml", sigil_stitch::lang::ocaml::OCaml::new());
}

#[test]
fn c_and_cpp_raw_annotations_follow_the_enumerator_name() {
    let c = render_type(
        &TypeSpec::builder("Choice", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("A")
                    .annotation(CodeBlock::of("__attribute__((deprecated))", ()).unwrap())
                    .annotate(AnnotationSpec::new("unused"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::c::C::new(),
    );
    assert!(
        c.contains("A __attribute__((deprecated)) __attribute__((unused))"),
        "{c}"
    );

    let cpp = render_type(
        &TypeSpec::builder("Choice", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("A")
                    .annotation(CodeBlock::of("[[deprecated]]", ()).unwrap())
                    .annotate(AnnotationSpec::new("maybe_unused"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::cpp::Cpp::new(),
    );
    assert!(cpp.contains("A [[deprecated]] [[maybe_unused]]"), "{cpp}");
}

#[test]
fn dart_enum_value_metadata_precedes_values_and_members_follow_a_semicolon() {
    let dart = render_type(
        &TypeSpec::builder("Status", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("current")
                    .annotate(AnnotationSpec::new("deprecated"))
                    .annotation(CodeBlock::of("@raw", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .extra_member(CodeBlock::of("final int code;\n", ()).unwrap())
            .build()
            .unwrap(),
        &sigil_stitch::lang::dart::Dart::new(),
    );
    let structured = dart.find("@deprecated").unwrap();
    let raw = dart.find("@raw").unwrap();
    let value = dart.find("current;").unwrap();
    assert!(structured < raw && raw < value, "{dart}");
    assert!(dart.contains("current;\n"), "{dart}");
    assert!(dart.contains("final int code;"), "{dart}");
}

#[test]
fn haskell_parenthesizes_compound_positional_payload_types() {
    let haskell = render_type(
        &TypeSpec::builder("Payload", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Generic")
                    .positional_payload(TypeName::generic(
                        TypeName::primitive("Maybe"),
                        vec![TypeName::primitive("Int")],
                    ))
                    .build()
                    .unwrap(),
            )
            .add_variant(
                EnumVariantSpec::builder("Callback")
                    .positional_payload(TypeName::function(
                        vec![TypeName::primitive("Int")],
                        TypeName::primitive("Bool"),
                    ))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::haskell::Haskell::new(),
    );
    assert!(haskell.contains("Generic (Maybe Int)"), "{haskell}");
    assert!(haskell.contains("Callback (Int -> Bool)"), "{haskell}");
}

#[test]
fn ocaml_parenthesizes_each_compound_positional_payload_type() {
    let ocaml = render_type(
        &TypeSpec::builder("Payload", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Tupled")
                    .positional_payload(TypeName::tuple(vec![
                        TypeName::primitive("int"),
                        TypeName::primitive("string"),
                    ]))
                    .build()
                    .unwrap(),
            )
            .add_variant(
                EnumVariantSpec::builder("Callback")
                    .positional_payload(TypeName::function(
                        vec![TypeName::primitive("int")],
                        TypeName::primitive("string"),
                    ))
                    .positional_payload(TypeName::primitive("bool"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::ocaml::OCaml::new(),
    );
    assert!(ocaml.contains("Tupled of (int * string)"), "{ocaml}");
    assert!(
        ocaml.contains("Callback of (int -> string) * bool"),
        "{ocaml}"
    );
}

#[test]
fn haskell_and_ocaml_escape_record_payload_field_names() {
    let spec = || {
        TypeSpec::builder("Payload", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Record")
                    .record_payload_field(FieldSpec::of("type", TypeName::primitive("Value")))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };

    let haskell = render_type(&spec(), &sigil_stitch::lang::haskell::Haskell::new());
    assert!(haskell.contains("type' :: Value"), "{haskell}");

    let ocaml = render_type(&spec(), &sigil_stitch::lang::ocaml::OCaml::new());
    assert!(ocaml.contains("type_ : Value"), "{ocaml}");
}

#[test]
fn haskell_and_ocaml_reject_record_field_names_that_collide_after_escaping() {
    let spec = |escaped_name: &str| {
        TypeSpec::builder("Payload", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Record")
                    .record_payload_field(FieldSpec::of("type", TypeName::primitive("Value")))
                    .record_payload_field(FieldSpec::of(escaped_name, TypeName::primitive("Value")))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };

    assert!(matches!(
        spec("type'").emit(&sigil_stitch::lang::haskell::Haskell::new()),
        Err(SigilStitchError::InvalidVariantRecordField { field_name, .. })
            if field_name == "type'"
    ));
    assert!(matches!(
        spec("type_").emit(&sigil_stitch::lang::ocaml::OCaml::new()),
        Err(SigilStitchError::InvalidVariantRecordField { field_name, .. })
            if field_name == "type_"
    ));

    let cross_constructor = TypeSpec::builder("Payload", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("First")
                .record_payload_field(FieldSpec::of("type", TypeName::primitive("Int")))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Second")
                .record_payload_field(FieldSpec::of("type'", TypeName::primitive("String")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        cross_constructor.emit(&sigil_stitch::lang::haskell::Haskell::new()),
        Err(SigilStitchError::InvalidVariantRecordField { field_name, .. })
            if field_name == "type'"
    ));
}

#[test]
fn haskell_rejects_reused_record_selectors_with_incompatible_types() {
    let spec = TypeSpec::builder("Payload", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Integer")
                .record_payload_field(FieldSpec::of("value", TypeName::primitive("Int")))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Text")
                .record_payload_field(FieldSpec::of("value", TypeName::primitive("String")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    assert!(matches!(
        spec.emit(&sigil_stitch::lang::haskell::Haskell::new()),
        Err(SigilStitchError::InvalidVariantRecordField { field_name, .. })
            if field_name == "value"
    ));
}

#[test]
fn java_and_kotlin_constructor_arguments_require_constructor_evidence() {
    let spec = || {
        TypeSpec::builder("Choice", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("ONE")
                    .constructor_argument(CodeBlock::of("1", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };

    for lang in [
        &sigil_stitch::lang::java::Java::new() as &dyn CodeLang,
        &sigil_stitch::lang::kotlin::Kotlin::new() as &dyn CodeLang,
    ] {
        assert!(matches!(
            spec().emit(lang),
            Err(SigilStitchError::MissingVariantConstructor { variant_name, .. })
                if variant_name == "ONE"
        ));
    }
}

#[test]
fn structured_and_opaque_constructor_evidence_remain_accepted() {
    let java = TypeSpec::builder("Choice", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("ONE")
                .constructor_argument(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Choice")
                .is_constructor()
                .add_param(ParameterSpec::of("value", TypeName::primitive("int")))
                .body(CodeBlock::of("this.value = value", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(java.emit(&sigil_stitch::lang::java::Java::new()).is_ok());

    let kotlin = TypeSpec::builder("Choice", TypeKind::Enum)
        .add_primary_constructor_param(ParameterSpec::of("value", TypeName::primitive("Int")))
        .add_variant(
            EnumVariantSpec::builder("ONE")
                .constructor_argument(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(
        kotlin
            .emit(&sigil_stitch::lang::kotlin::Kotlin::new())
            .is_ok()
    );

    let opaque = TypeSpec::builder("Choice", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("ONE")
                .constructor_argument(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .extra_member(CodeBlock::of("private Choice(int value) {}\n", ()).unwrap())
        .build()
        .unwrap();
    assert!(opaque.emit(&sigil_stitch::lang::java::Java::new()).is_ok());
}

#[test]
fn java_and_kotlin_require_a_compatible_constructor_arity() {
    let constructor = |name: &str| {
        FunSpec::builder(name)
            .is_constructor()
            .add_param(ParameterSpec::of("value", TypeName::primitive("int")))
            .body(CodeBlock::of("", ()).unwrap())
            .build()
            .unwrap()
    };
    let invalid = |constructor_name: &str, argument_count: usize| {
        let mut variant = EnumVariantSpec::builder("INVALID");
        for argument in 0..argument_count {
            variant =
                variant.constructor_argument(CodeBlock::of("%L", argument.to_string()).unwrap());
        }
        TypeSpec::builder("Choice", TypeKind::Enum)
            .add_variant(variant.build().unwrap())
            .add_method(constructor(constructor_name))
            .build()
            .unwrap()
    };

    for (lang, constructor_name) in [
        (
            &sigil_stitch::lang::java::Java::new() as &dyn CodeLang,
            "Choice",
        ),
        (
            &sigil_stitch::lang::kotlin::Kotlin::new() as &dyn CodeLang,
            "constructor",
        ),
    ] {
        for argument_count in [0, 2] {
            assert!(matches!(
                invalid(constructor_name, argument_count).emit(lang),
                Err(SigilStitchError::IncompatibleVariantConstructorArguments {
                    variant_name,
                    argument_count: actual,
                    ..
                }) if variant_name == "INVALID" && actual == argument_count
            ));
        }
    }
}

#[test]
fn java_constructor_overloads_accept_each_matching_variant_arity() {
    let constructor = |parameter_count: usize| {
        let mut constructor = FunSpec::builder("Choice").is_constructor();
        for index in 0..parameter_count {
            constructor = constructor.add_param(ParameterSpec::of(
                &format!("value{index}"),
                TypeName::primitive("int"),
            ));
        }
        constructor
            .body(CodeBlock::of("", ()).unwrap())
            .build()
            .unwrap()
    };
    let variant = |name: &str, argument_count: usize| {
        let mut variant = EnumVariantSpec::builder(name);
        for argument in 0..argument_count {
            variant =
                variant.constructor_argument(CodeBlock::of("%L", argument.to_string()).unwrap());
        }
        variant.build().unwrap()
    };
    let spec = TypeSpec::builder("Choice", TypeKind::Enum)
        .add_variant(variant("ONE", 1))
        .add_variant(variant("TWO", 2))
        .add_method(constructor(1))
        .add_method(constructor(2))
        .build()
        .unwrap();

    assert!(spec.emit(&sigil_stitch::lang::java::Java::new()).is_ok());
}

#[test]
fn empty_variant_operands_fail_intrinsic_validation() {
    let assert_empty_operand = |spec: TypeSpec, lang: &dyn CodeLang, expected: &str| {
        assert!(matches!(
            spec.emit(lang),
            Err(SigilStitchError::EmptyVariantOperand { operand, .. })
                if operand == expected
        ));
    };

    assert_empty_operand(
        TypeSpec::builder("Empty", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("VALUE")
                    .discriminant(CodeBlock::of("%L", "").unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::typescript::TypeScript::new(),
        "discriminant",
    );
    assert_empty_operand(
        TypeSpec::builder("Empty", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("VALUE")
                    .constructor_argument(
                        CodeBlock::of("%L", CodeBlock::of("", ()).unwrap()).unwrap(),
                    )
                    .build()
                    .unwrap(),
            )
            .extra_member(CodeBlock::of("private Empty(int value) {}", ()).unwrap())
            .build()
            .unwrap(),
        &sigil_stitch::lang::java::Java::new(),
        "constructor argument 0",
    );
    assert_empty_operand(
        TypeSpec::builder("Empty", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("VALUE")
                    .positional_payload(TypeName::raw(""))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::haskell::Haskell::new(),
        "positional payload type 0",
    );
    assert_empty_operand(
        TypeSpec::builder("Empty", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("VALUE")
                    .record_payload_field(FieldSpec::of("value", TypeName::raw("")))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::rust::Rust::new(),
        "record field \"value\" type",
    );
}

#[test]
fn ruby_rejects_structured_variant_metadata_but_preserves_opaque_blocks() {
    let structured = TypeSpec::builder("Choice", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("VALUE")
                .annotate(AnnotationSpec::new("deprecated"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        structured.emit(&sigil_stitch::lang::ruby::Ruby::new()),
        Err(SigilStitchError::InvalidVariantAnnotation { variant_name, .. })
            if variant_name == "VALUE"
    ));

    let opaque = render_type(
        &TypeSpec::builder("Choice", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("VALUE")
                    .annotation(CodeBlock::of("extend TargetSpecificMetadata", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &sigil_stitch::lang::ruby::Ruby::new(),
    );
    assert!(opaque.contains("extend TargetSpecificMetadata"), "{opaque}");
    assert!(!opaque.contains("# deprecated"), "{opaque}");
}

fn render_type(spec: &TypeSpec, lang: &dyn CodeLang) -> String {
    spec.emit(lang)
        .unwrap()
        .iter()
        .map(|block| render_block(lang, block))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_block(lang: &dyn CodeLang, block: &CodeBlock) -> String {
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, 80);
    renderer.render(block).unwrap()
}
