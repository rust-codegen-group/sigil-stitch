//! Generate a Rust file using structural specs.
//!
//! Run with: `cargo run --example rust_codegen`

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // Define types from different crate groups.
    let hashmap = TypeName::<RustLang>::importable("std::collections", "HashMap");
    let serialize = TypeName::<RustLang>::importable("serde", "Serialize");
    let deserialize = TypeName::<RustLang>::importable("serde", "Deserialize");

    // Build a struct using TypeSpec.
    let mut tb = TypeSpec::<RustLang>::builder("Config", TypeKind::Struct);
    tb.visibility(Visibility::Public);

    // Derive annotation.
    let derive = CodeBlock::<RustLang>::of("#[derive(%T, %T)]", (serialize, deserialize)).unwrap();
    tb.annotation(derive);

    // Fields.
    let mut fb1 = FieldSpec::builder("name", TypeName::primitive("String"));
    fb1.visibility(Visibility::Public);
    tb.add_field(fb1.build().unwrap());

    let mut fb2 = FieldSpec::builder(
        "values",
        TypeName::generic(
            hashmap,
            vec![TypeName::primitive("String"), TypeName::primitive("i64")],
        ),
    );
    fb2.visibility(Visibility::Public);
    tb.add_field(fb2.build().unwrap());

    // Constructor method.
    let body = CodeBlock::<RustLang>::of(
        "Self { name: name.to_string(), values: HashMap::new() }",
        (),
    )
    .unwrap();
    let mut mfb = FunSpec::<RustLang>::builder("new");
    mfb.visibility(Visibility::Public);
    mfb.add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap());
    mfb.returns(TypeName::primitive("Self"));
    mfb.body(body);
    tb.add_method(mfb.build().unwrap());

    // Build and render.
    let mut file = FileSpec::builder_with("config.rs", RustLang::new());
    file.add_type(tb.build().unwrap());
    let spec = file.build().unwrap();

    let output = spec.render(100).unwrap();
    println!("{output}");
}
