use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::php::Php;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_class_with_fields() {
    let file = FileSpec::builder_with("config.php", Php::new())
        .add_type(
            TypeSpec::builder("Config", TypeKind::Class)
                .doc("Config holds application configuration.")
                .add_field(
                    FieldSpec::builder("name", TypeName::primitive("string"))
                        .visibility(Visibility::Public)
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("port", TypeName::primitive("int"))
                        .visibility(Visibility::Public)
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("debug", TypeName::primitive("bool"))
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
    golden::assert_golden("php/class_with_fields.php", &output);
}

#[test]
fn test_class_with_methods() {
    let file = FileSpec::builder_with("server.php", Php::new())
        .add_type(
            TypeSpec::builder("Server", TypeKind::Class)
                .doc("Server is an HTTP server.")
                .add_field(
                    FieldSpec::builder("host", TypeName::primitive("string"))
                        .visibility(Visibility::Private)
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("port", TypeName::primitive("int"))
                        .visibility(Visibility::Private)
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("getHost")
                        .visibility(Visibility::Public)
                        .returns(TypeName::primitive("string"))
                        .body(CodeBlock::of("return $this->host;", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("getPort")
                        .visibility(Visibility::Public)
                        .returns(TypeName::primitive("int"))
                        .body(CodeBlock::of("return $this->port;", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/class_with_methods.php", &output);
}

#[test]
fn test_interface() {
    let file = FileSpec::builder_with("repo.php", Php::new())
        .add_type(
            TypeSpec::builder("Repository", TypeKind::Interface)
                .doc("Repository defines data access methods.")
                .add_method(
                    FunSpec::builder("findById")
                        .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                        .returns(TypeName::optional(TypeName::primitive("User")))
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("save")
                        .add_param(
                            ParameterSpec::new("entity", TypeName::primitive("User")).unwrap(),
                        )
                        .returns(TypeName::primitive("void"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/interface.php", &output);
}

#[test]
fn test_enum() {
    let php = Php::new();

    let mut cb = CodeBlock::builder();
    let doc = php.render_doc_comment(&["Status represents possible states."]);
    cb.add("%L", doc);
    cb.add_line();
    cb.add("enum Status: string {", ());
    cb.add_line();
    cb.add("%>", ());
    cb.add("case Draft = \"draft\";", ());
    cb.add_line();
    cb.add("case Published = \"published\";", ());
    cb.add_line();
    cb.add("%<", ());
    cb.add("}", ());
    cb.add_line();
    let block = cb.build().unwrap();

    let file = FileSpec::builder_with("status.php", Php::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/enum.php", &output);
}
