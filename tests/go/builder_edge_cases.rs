use sigil_stitch::code_block::{CodeBlock, NameArg};
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::go::Go;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_enum() {
    // Go has no native enum syntax. The idiomatic pattern is:
    //   type Direction int
    //   const ( North Direction = iota; East; ... )
    //
    // This doesn't fit TypeSpec, so we build it as a raw CodeBlock.
    let go = Go::new();

    let mut cb = CodeBlock::builder();
    let doc = go.render_doc_comment(&["Direction represents a cardinal direction."]);
    cb.add("%L", doc);
    cb.add_line();
    cb.add("type Direction int", ());
    cb.add_line();
    cb.add_line();
    cb.add("const (", ());
    cb.add_line();
    cb.add("%>", ());
    cb.add("North Direction = iota", ());
    cb.add_line();
    cb.add("East", ());
    cb.add_line();
    cb.add("South", ());
    cb.add_line();
    cb.add("West", ());
    cb.add_line();
    cb.add("%<", ());
    cb.add(")", ());
    cb.add_line();
    let block = cb.build().unwrap();

    let file = FileSpec::builder_with("direction.go", Go::new())
        .header(CodeBlock::of("package direction", ()).unwrap())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/enum.go", &output);
}

// ── %N keyword escaping ─────────────────────────────────

#[test]
fn test_name_escapes_go_keywords() {
    let keywords = ["func", "type", "var", "map", "range", "select", "chan"];
    for kw in keywords {
        let block = CodeBlock::of("var %N int", NameArg(kw.into())).unwrap();
        let file = FileSpec::builder_with("test.go", Go::new())
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("{kw}_")),
            "Expected '{kw}_' in output for reserved word '{kw}', got: {output}"
        );
    }
    // Non-reserved word should not be escaped.
    let block = CodeBlock::of("var %N int", NameArg("myVar".into())).unwrap();
    let file = FileSpec::builder_with("test.go", Go::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("var myVar int"));
}

#[test]
fn test_name_escape_in_struct_context() {
    let mut cb = CodeBlock::builder();
    cb.add("type Params struct {", ());
    cb.add_line();
    cb.add("%>", ());
    cb.add("%N string", NameArg("type".into()));
    cb.add_line();
    cb.add("%N int", NameArg("count".into()));
    cb.add_line();
    cb.add("%<", ());
    cb.add("}", ());
    cb.add_line();
    let block = cb.build().unwrap();

    let file = FileSpec::builder_with("test.go", Go::new())
        .header(CodeBlock::of("package models", ()).unwrap())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("type_ string"));
    assert!(output.contains("count int"));
}

// ── Embedded fields ─────────────────────────────────────

#[test]
fn test_embedded_struct_basic() {
    let file = FileSpec::builder_with("models.go", Go::new())
        .header(CodeBlock::of("package models", ()).unwrap())
        .add_type(
            TypeSpec::builder("ReadWriter", TypeKind::Struct)
                .add_embedded(TypeName::primitive("Reader"))
                .add_embedded(TypeName::primitive("Writer"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/embedded_basic.go", &output);
}

#[test]
fn test_embedded_with_methods() {
    let body = CodeBlock::of("return fmt.Sprintf(\"%%s/%%s\", a.Host, a.Path)", ()).unwrap();
    let file = FileSpec::builder_with("models.go", Go::new())
        .header(CodeBlock::of("package models", ()).unwrap())
        .add_type(
            TypeSpec::builder("Endpoint", TypeKind::Struct)
                .add_embedded(TypeName::primitive("BaseConfig"))
                .add_field(
                    FieldSpec::builder("Path", TypeName::primitive("string"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_function(
            FunSpec::builder("URL")
                .add_param(
                    ParameterSpec::new("a", TypeName::pointer(TypeName::primitive("Endpoint")))
                        .unwrap(),
                )
                .returns(TypeName::primitive("string"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/embedded_with_method.go", &output);
}

#[test]
fn test_embedded_only_no_fields() {
    let file = FileSpec::builder_with("compose.go", Go::new())
        .header(CodeBlock::of("package compose", ()).unwrap())
        .add_type(
            TypeSpec::builder("Combined", TypeKind::Struct)
                .add_embedded(TypeName::primitive("Alpha"))
                .add_embedded(TypeName::primitive("Beta"))
                .add_embedded(TypeName::primitive("Gamma"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("type Combined struct {"));
    assert!(output.contains("\tAlpha\n"));
    assert!(output.contains("\tBeta\n"));
    assert!(output.contains("\tGamma\n"));
    assert!(output.contains("}"));
}

#[test]
fn test_embedded_with_importable_type() {
    let io_reader = TypeName::importable("io", "Reader");
    let io_writer = TypeName::importable("io", "Writer");

    let file = FileSpec::builder_with("rw.go", Go::new())
        .header(CodeBlock::of("package rw", ()).unwrap())
        .add_type(
            TypeSpec::builder("ReadWriter", TypeKind::Interface)
                .add_embedded(io_reader)
                .add_embedded(io_writer)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/embedded_importable.go", &output);
}
