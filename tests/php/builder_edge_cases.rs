use sigil_stitch::code_block::{CodeBlock, NameArg};
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::php::Php;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_name_escapes_php_keywords() {
    let keywords = ["class", "function", "echo", "print", "array", "list"];
    for kw in keywords {
        let block = CodeBlock::of("$obj = new %N()", NameArg(kw.into())).unwrap();
        let file = FileSpec::builder_with("test.php", Php::new())
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
    let block = CodeBlock::of("$obj = new %N()", NameArg("MyClass".into())).unwrap();
    let file = FileSpec::builder_with("test.php", Php::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("new MyClass()"));
}

#[test]
fn test_newtype_line() {
    let php = Php::new();
    let line = php.render_newtype_line("", "Name", "string");
    assert!(line.contains("class Name"));
    assert!(line.contains("__construct"));
    assert!(line.contains("private string $value"));
}

#[test]
fn test_nullable_field() {
    let file = FileSpec::builder_with("user.php", Php::new())
        .add_type(
            TypeSpec::builder("User", TypeKind::Class)
                .add_field(
                    FieldSpec::builder("name", TypeName::primitive("string"))
                        .visibility(Visibility::Public)
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("email", TypeName::optional(TypeName::primitive("string")))
                        .visibility(Visibility::Public)
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/nullable_field.php", &output);
}

#[test]
fn test_attribute_on_method() {
    let body = CodeBlock::of("return $this->name;", ()).unwrap();
    let fun = FunSpec::builder("toString")
        .visibility(Visibility::Public)
        .returns(TypeName::primitive("string"))
        .annotate(AnnotationSpec::new("Override"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("user.php", Php::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("php/attribute_on_method.php", &output);
}
