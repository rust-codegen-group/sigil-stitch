//! Generate a Go file using structural specs.
//!
//! Run with: `cargo run --example go_codegen`

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // Importable types.
    let json_marshal = TypeName::<GoLang>::importable("encoding/json", "Marshal");
    let http_server = TypeName::<GoLang>::importable("net/http", "Server");

    // Build a struct with struct tags.
    let mut tb = TypeSpec::<GoLang>::builder("Config", TypeKind::Struct);
    tb.doc("Config holds application configuration.");

    let mut f1 = FieldSpec::builder("Host", TypeName::primitive("string"));
    f1.tag("json:\"host\"");
    tb.add_field(f1.build().unwrap());

    let mut f2 = FieldSpec::builder("Port", TypeName::primitive("int"));
    f2.tag("json:\"port\"");
    tb.add_field(f2.build().unwrap());

    // Build receiver methods as separate top-level functions.
    let mut start_fn = FunSpec::<GoLang>::builder("Start");
    start_fn.doc("Start begins listening on the configured address.");
    start_fn.receiver(
        ParameterSpec::new("c", TypeName::pointer(TypeName::primitive("Config"))).unwrap(),
    );
    start_fn.returns(TypeName::primitive("error"));
    start_fn.body(CodeBlock::<GoLang>::of("return %T(c.Host, nil)", (http_server,)).unwrap());

    let mut to_json_fn = FunSpec::<GoLang>::builder("ToJSON");
    to_json_fn.receiver(
        ParameterSpec::new("c", TypeName::pointer(TypeName::primitive("Config"))).unwrap(),
    );
    to_json_fn.returns(TypeName::raw("([]byte, error)"));
    to_json_fn.body(CodeBlock::<GoLang>::of("return %T(c)", (json_marshal,)).unwrap());

    // Assemble the file.
    let mut file = FileSpec::builder_with("config.go", GoLang::new());
    file.header(CodeBlock::<GoLang>::of("package config", ()).unwrap());
    file.add_type(tb.build().unwrap());
    file.add_function(start_fn.build().unwrap());
    file.add_function(to_json_fn.build().unwrap());
    let spec = file.build().unwrap();

    let output = spec.render(100).unwrap();
    println!("{output}");
}
