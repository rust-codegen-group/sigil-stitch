use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_node::CodeNode;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeArgument;
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn test_class_with_fields_and_methods() {
    let body = CodeBlock::of("return this.userRepo.findById(id)", ()).unwrap();
    let tb = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("userRepo", TypeName::primitive("UserRepository"))
                .visibility(Visibility::Private)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("logger", TypeName::primitive("Logger"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getUser")
                .is_async()
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .returns(vec![TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::importable_type("./models", "User")],
                )])
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("UserService.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/class_with_methods.ts", &output);
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn test_interface_generic() {
    let tp = TypeParamSpec::new("T");
    let tb = TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .add_method(
            FunSpec::builder("findById")
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .returns(vec![TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("T")],
                )])
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .returns(vec![TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("void")],
                )])
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("Repository.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/interface_generic.ts", &output);
}

#[test]
fn test_abstract_class() {
    let body = CodeBlock::of("console.log('handled')", ()).unwrap();
    let tb = TypeSpec::builder("BaseController", TypeKind::Class)
        .visibility(Visibility::Public)
        .is_abstract()
        .add_method(
            FunSpec::builder("handleRequest")
                .is_abstract()
                .add_param(ParameterSpec::new("req", TypeName::primitive("Request")).unwrap())
                .returns(vec![TypeName::primitive("Response")])
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("log")
                .visibility(Visibility::Protected)
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("BaseController.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/abstract_class.ts", &output);
}

#[test]
fn test_class_extends_implements() {
    let body = CodeBlock::of("return true", ()).unwrap();
    let tb = TypeSpec::builder("AdminService", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::importable("./base", "BaseService"))
        .implements(TypeName::importable("./auth", "Authenticatable"))
        .implements(TypeName::importable_type("./serial", "Serializable"))
        .add_method(
            FunSpec::builder("isAdmin")
                .returns(vec![TypeName::primitive("boolean")])
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("AdminService.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/class_extends_implements.ts", &output);
}

#[test]
fn test_enum() {
    let output = FileSpec::builder("Direction.ts")
        .add_type(
            TypeSpec::builder("Direction", TypeKind::Enum)
                .visibility(Visibility::Public)
                .add_variant(
                    EnumVariantSpec::builder("Up")
                        .discriminant(CodeBlock::of("'UP'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Down")
                        .discriminant(CodeBlock::of("'DOWN'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Left")
                        .discriminant(CodeBlock::of("'LEFT'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Right")
                        .discriminant(CodeBlock::of("'RIGHT'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/enum.ts", &output);
}

#[test]
fn test_readonly_array_field() {
    let tb = TypeSpec::builder("Pet", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "tags",
                TypeName::readonly_array(TypeName::primitive("string")),
            )
            .is_readonly()
            .build()
            .unwrap(),
        );

    let output = FileSpec::builder("Pet.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(
        output.contains("readonly tags: readonly string[];"),
        "expected readonly array field; got:\n{output}"
    );
}

#[test]
fn qualified_record_alias_uses_global_utility_without_imports() {
    let target = TypeName::application(
        TypeName::qualified("globalThis", "Record"),
        vec![
            TypeArgument::Single(TypeName::string_literal("eventType")),
            TypeArgument::Single(TypeName::string_literal("data")),
        ],
    );
    let output = FileSpec::builder("record.ts")
        .add_type(
            TypeSpec::builder("Record", TypeKind::TypeAlias)
                .visibility(Visibility::Public)
                .extends(target)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(100)
        .unwrap();

    assert_eq!(
        output,
        "export type Record = globalThis.Record<'eventType', 'data'>;\n"
    );
}

#[test]
fn qualified_types_reject_module_paths_and_non_name_syntax() {
    for (module, name) in [
        ("./models", "Item"),
        ("@scope/models", "Item"),
        ("path/models", "Item"),
        ("package-name", "Item"),
        (".Domain", "Item"),
        ("Domain.", "Item"),
        ("Domain..Nested", "Item"),
        ("Domain space", "Item"),
        ("1Domain", "Item"),
        ("Domain<T>", "Item"),
        ("Domain", "Item.Member"),
        ("Domain", "1Item"),
        ("Domain", "Item[]"),
        ("Domain", "Item<T>"),
        ("Domain", "Item | number"),
        ("Domain", "Item; type Other = number"),
    ] {
        for ty in [
            TypeName::qualified(module, name),
            TypeName::importable_type(module, name).qualify(),
        ] {
            let block = CodeBlock::of("%T", (ty,)).unwrap();
            let error = block
                .render_standalone(&TypeScript::new(), 100)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    SigilStitchError::UnsupportedTypeName { language, context, reason }
                        if language == "ts"
                            && context == "standalone.node[0]"
                            && reason.contains("qualified")
                ),
                "expected qualified-type rejection for {module:?}.{name:?}"
            );
        }
    }
}

#[test]
fn qualified_types_reject_roots_that_start_other_type_forms() {
    for root in [
        "this", "typeof", "import", "keyof", "unique", "readonly", "infer", "void", "null", "true",
        "false", "new",
    ] {
        for module in [root.to_string(), format!("{root}.Nested")] {
            let block = CodeBlock::of("%T", (TypeName::qualified(&module, "Item"),)).unwrap();
            let error = block
                .render_standalone(&TypeScript::new(), 100)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    SigilStitchError::UnsupportedTypeName { language, context, reason }
                        if language == "ts"
                            && context == "standalone.node[0]"
                            && reason.contains("root")
                ),
                "expected type-reference root rejection for {module:?}"
            );
        }
    }
}

#[test]
fn qualified_type_names_preserve_namespace_spelling_on_both_renderers() {
    for (module, name, expected) in [
        ("globalThis", "Date", "globalThis.Date"),
        ("Domain.Nested", "Item", "Domain.Nested.Item"),
        ("类型", "项目", "类型.项目"),
        ("$Outer", "_Item", "$Outer._Item"),
        ("_Outer", "$Item", "_Outer.$Item"),
        ("type", "Value", "type.Value"),
        ("Models", "default", "Models.default"),
        ("Domain.readonly", "infer", "Domain.readonly.infer"),
        (
            "Node\u{200c}",
            "Value\u{200d}",
            "Node\u{200c}.Value\u{200d}",
        ),
    ] {
        for ty in [
            TypeName::qualified(module, name),
            TypeName::importable(module, name).qualify(),
            TypeName::importable_type(module, name)
                .with_alias("UnusedImportAlias")
                .qualify(),
        ] {
            let direct = CodeBlock::of("%T", (ty.clone(),)).unwrap();
            let pretty = CodeBlock::of("ref:%W%T", (ty,)).unwrap();
            assert_eq!(
                direct.render_standalone(&TypeScript::new(), 100).unwrap(),
                expected
            );
            assert_eq!(
                pretty.render_standalone(&TypeScript::new(), 100).unwrap(),
                format!("ref: {expected}")
            );
            assert_eq!(
                pretty.render_standalone(&TypeScript::new(), 8).unwrap(),
                format!("ref:\n{expected}")
            );
        }
    }
}

#[test]
#[allow(
    deprecated,
    reason = "qualified bases share lowering for released and modern applications"
)]
fn qualified_applications_preserve_argument_imports_aliases_and_wrapping() {
    let base = TypeName::qualified("globalThis", "Record").with_alias("UnusedUtilityAlias");
    let key = TypeName::string_literal("payload");
    let payload = TypeName::tuple(vec![
        TypeName::importable_type("./primary", "Payload"),
        TypeName::importable_type("./secondary", "Payload"),
        TypeName::importable_type("./models", "Detail").with_alias("ModelDetail"),
        TypeName::qualified("globalThis", "Date"),
    ]);
    for target in [
        TypeName::application(
            base.clone(),
            vec![
                TypeArgument::Single(key.clone()),
                TypeArgument::Single(payload.clone()),
            ],
        ),
        TypeName::generic(base, vec![key, payload]),
    ] {
        let file = FileSpec::builder("envelope.ts")
            .add_type(
                TypeSpec::builder("Envelope", TypeKind::TypeAlias)
                    .visibility(Visibility::Public)
                    .extends(target)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let wide = file.render(200).unwrap();
        let narrow = file.render(24).unwrap();
        for output in [&wide, &narrow] {
            assert!(output.contains("import type { Payload } from './primary'"));
            assert!(
                output.contains("import type { Payload as SecondaryPayload } from './secondary'")
            );
            assert!(output.contains("import type { Detail as ModelDetail } from './models'"));
            assert_eq!(
                output
                    .lines()
                    .filter(|line| line.starts_with("import "))
                    .count(),
                3
            );
            assert!(!output.contains("UnusedUtilityAlias"));
        }
        assert!(wide.ends_with(
            "export type Envelope = globalThis.Record<'payload', [Payload, SecondaryPayload, ModelDetail, globalThis.Date]>;\n"
        ));
        assert!(
            narrow.contains("globalThis.Record<'payload',\n"),
            "{narrow}"
        );
        assert!(narrow.contains("[Payload,\n"), "{narrow}");
        assert_eq!(
            narrow.split_whitespace().collect::<Vec<_>>(),
            wide.split_whitespace().collect::<Vec<_>>()
        );
    }
}

#[test]
fn qualified_type_errors_abort_nested_and_sequenced_files() {
    let target = TypeName::application(
        TypeName::qualified("globalThis", "Record"),
        vec![
            TypeArgument::Single(TypeName::string_literal("payload")),
            TypeArgument::Single(TypeName::qualified("./models", "Item")),
        ],
    );
    let direct = CodeBlock::of("%T", (target.clone(),)).unwrap();
    let nested = CodeBlock::of("%L", (direct.clone(),)).unwrap();
    let mut sequence = CodeBlock::builder().build().unwrap();
    sequence
        .nodes_mut()
        .push(CodeNode::Sequence(vec![CodeNode::TypeRef(target)]));

    for (block, path) in [
        (direct, "node[0]"),
        (nested, "nested[0].node[0]"),
        (sequence, "sequence[0].node[0]"),
    ] {
        let error = block
            .render_standalone(&TypeScript::new(), 100)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedTypeName { language, context, reason }
                if language == "ts"
                    && context == format!("standalone.{path}")
                    && reason.contains("qualified")
        ));
        let file = FileSpec::builder("invalid.ts")
            .add_code(CodeBlock::of("type Before = string;\n", ()).unwrap())
            .add_code(block)
            .build()
            .unwrap();
        assert!(matches!(
            file.render(100).unwrap_err(),
            SigilStitchError::UnsupportedTypeName { language, context, reason }
                if language == "ts"
                    && context == format!("member[1].block[0].{path}")
                    && reason.contains("qualified")
        ));
    }
}

#[test]
fn qualified_types_keep_intrinsic_input_errors() {
    for ty in [
        TypeName::qualified("", "Item"),
        TypeName::qualified("Domain", ""),
        TypeName::qualified("Domain\nOther", "Item"),
        TypeName::qualified("Domain", "Item\nOther"),
    ] {
        let block = CodeBlock::of("%T", (ty,)).unwrap();
        assert!(matches!(
            block.render_standalone(&TypeScript::new(), 100).unwrap_err(),
            SigilStitchError::InvalidTypeName { context, .. }
                if context == "standalone.node[0]"
        ));
    }
}
