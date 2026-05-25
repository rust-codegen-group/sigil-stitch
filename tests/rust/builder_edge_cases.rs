use super::golden;

use sigil_stitch::code_block::{CodeBlock, NameArg};
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_optional_field() {
    let output = FileSpec::builder_with("config.rs", Rust::new())
        .add_type(
            TypeSpec::builder("Config", TypeKind::Struct)
                .visibility(Visibility::Public)
                .add_field(
                    FieldSpec::builder("name", TypeName::primitive("String"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("description", TypeName::primitive("String"))
                        .is_optional()
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

    golden::assert_golden("rust/optional_field.rs", &output);
}

#[test]
fn test_derive_annotation() {
    let derive = CodeBlock::of("#[derive(Debug, Clone, PartialEq)]", ()).unwrap();
    let tb = TypeSpec::builder("Point", TypeKind::Struct)
        .visibility(Visibility::Public)
        .annotation(derive)
        .add_field(
            FieldSpec::builder("x", TypeName::primitive("f64"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("y", TypeName::primitive("f64"))
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder_with("point.rs", Rust::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/derive_annotation.rs", &output);
}

// ── %N keyword escaping ─────────────────────────────────

#[test]
fn test_name_escapes_rust_keywords() {
    let keywords = [
        "type", "fn", "struct", "enum", "trait", "impl", "mod", "use", "let", "mut",
    ];
    for kw in keywords {
        let block = CodeBlock::of("let %N = value", NameArg(kw.into())).unwrap();
        let file = FileSpec::builder_with("test.rs", Rust::new())
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("r#{kw}")),
            "Expected 'r#{kw}' for reserved word '{kw}', got: {output}"
        );
    }
}

#[test]
fn test_name_no_escape_non_keywords() {
    let names = ["user_id", "count", "my_struct", "TypeName", "snake_case"];
    for name in names {
        let block = CodeBlock::of("let %N = value", NameArg(name.into())).unwrap();
        let file = FileSpec::builder_with("test.rs", Rust::new())
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("let {name} = value")),
            "Expected 'let {name} = value' in output, got: {output}"
        );
    }
}

#[test]
fn test_name_escape_in_format_string() {
    let mut cb = CodeBlock::builder();
    cb.add(
        "pub %N: %T",
        (NameArg("type".into()), TypeName::primitive("String")),
    );
    cb.add_line();
    let block = cb.build().unwrap();

    let file = FileSpec::builder_with("test.rs", Rust::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("pub r#type: String"));
}

// ── AnnotationSpec .args() ──────────────────────────────

#[test]
fn test_annotation_args_with_type_spec() {
    let output = FileSpec::builder_with("config.rs", Rust::new())
        .add_type(
            TypeSpec::builder("Config", TypeKind::Struct)
                .visibility(Visibility::Public)
                .annotate(AnnotationSpec::new("derive").args(["Debug", "Clone", "Serialize"]))
                .add_field(
                    FieldSpec::builder("name", TypeName::primitive("String"))
                        .visibility(Visibility::Public)
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

    golden::assert_golden("rust/annotation_args_derive.rs", &output);
}

#[test]
fn test_annotation_args_serde_field() {
    let output = FileSpec::builder_with("model.rs", Rust::new())
        .add_type(
            TypeSpec::builder("User", TypeKind::Struct)
                .visibility(Visibility::Public)
                .annotate(AnnotationSpec::new("derive").args(["Debug", "Serialize", "Deserialize"]))
                .annotate(AnnotationSpec::new("serde").args(["rename_all = \"camelCase\""]))
                .add_field(
                    FieldSpec::builder("user_name", TypeName::primitive("String"))
                        .visibility(Visibility::Public)
                        .annotate(AnnotationSpec::new("serde").arg("rename = \"userName\""))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("email_address", TypeName::primitive("String"))
                        .visibility(Visibility::Public)
                        .annotate(AnnotationSpec::new("serde").arg("rename = \"emailAddress\""))
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

    golden::assert_golden("rust/annotation_args_serde.rs", &output);
}
