//! Phase 2 integration tests: Rust structural specs.

mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_rust_struct_with_impl() {
    // Struct.
    let mut tb = TypeSpec::<RustLang>::builder("Config", TypeKind::Struct);
    tb.visibility(Visibility::Public);
    tb.doc("Application configuration.");

    // Annotation (derive).
    let derive = CodeBlock::<RustLang>::of(
        "#[derive(%T, %T)]",
        (
            TypeName::importable("serde", "Serialize"),
            TypeName::importable("serde", "Deserialize"),
        ),
    )
    .unwrap();
    tb.annotation(derive);

    let mut fb1 = FieldSpec::builder("name", TypeName::primitive("String"));
    fb1.visibility(Visibility::Public);
    tb.add_field(fb1.build());

    let mut fb2 = FieldSpec::builder(
        "values",
        TypeName::generic(
            TypeName::importable("std::collections", "HashMap"),
            vec![TypeName::primitive("String"), TypeName::primitive("i64")],
        ),
    );
    fb2.visibility(Visibility::Public);
    tb.add_field(fb2.build());

    // Method.
    let body = CodeBlock::<RustLang>::of(
        "Self { name: name.to_string(), values: HashMap::new() }",
        (),
    )
    .unwrap();
    let mut mfb = FunSpec::<RustLang>::builder("new");
    mfb.visibility(Visibility::Public);
    mfb.add_param(ParameterSpec::new("name", TypeName::primitive("&str")));
    mfb.returns(TypeName::primitive("Self"));
    mfb.body(body);
    tb.add_method(mfb.build());

    let mut file = FileSpec::builder_with("config.rs", RustLang::new());
    file.add_type(tb.build());
    let output = file.build().render(100).unwrap();

    golden::assert_golden("rust/struct_with_impl.rs", &output);
}

#[test]
fn test_rust_generic_struct() {
    let tp = TypeParamSpec::<RustLang>::new("T")
        .with_bound(TypeName::primitive("Clone"))
        .with_bound(TypeName::primitive("Send"));

    let mut tb = TypeSpec::<RustLang>::builder("Container", TypeKind::Struct);
    tb.visibility(Visibility::Public);
    tb.add_type_param(tp);

    let mut fb = FieldSpec::builder(
        "items",
        TypeName::generic(TypeName::primitive("Vec"), vec![TypeName::primitive("T")]),
    );
    fb.visibility(Visibility::Public);
    tb.add_field(fb.build());

    let body = CodeBlock::<RustLang>::of("self.items.len()", ()).unwrap();
    let mut mfb = FunSpec::<RustLang>::builder("len");
    mfb.visibility(Visibility::Public);
    mfb.add_param(ParameterSpec::new("&self", TypeName::primitive("")));
    mfb.returns(TypeName::primitive("usize"));
    mfb.body(body);
    tb.add_method(mfb.build());

    let mut file = FileSpec::builder_with("container.rs", RustLang::new());
    file.add_type(tb.build());
    let output = file.build().render(80).unwrap();

    golden::assert_golden("rust/generic_struct.rs", &output);
}

#[test]
fn test_rust_enum() {
    let mut tb = TypeSpec::<RustLang>::builder("Color", TypeKind::Enum);
    tb.visibility(Visibility::Public);

    let derive = CodeBlock::<RustLang>::of("#[derive(Debug, Clone, Copy)]", ()).unwrap();
    tb.annotation(derive);

    tb.add_variant(EnumVariantSpec::new("Red"));
    tb.add_variant(EnumVariantSpec::new("Green"));
    tb.add_variant(EnumVariantSpec::new("Blue"));

    let mut file = FileSpec::builder_with("color.rs", RustLang::new());
    file.add_type(tb.build());
    let output = file.build().render(80).unwrap();

    golden::assert_golden("rust/enum.rs", &output);
}

#[test]
fn test_rust_top_level_function() {
    let tp =
        TypeParamSpec::<RustLang>::new("T").with_bound(TypeName::primitive("std::fmt::Display"));

    let mut fb = FunSpec::<RustLang>::builder("print_value");
    fb.visibility(Visibility::Public);
    fb.add_type_param(tp);
    fb.add_param(ParameterSpec::new("value", TypeName::primitive("&T")));
    let body = CodeBlock::<RustLang>::of("println!(\"{}\", value)", ()).unwrap();
    fb.body(body);

    let mut file = FileSpec::builder_with("utils.rs", RustLang::new());
    file.add_function(fb.build());
    let output = file.build().render(80).unwrap();

    golden::assert_golden("rust/top_level_function.rs", &output);
}

#[test]
fn test_rust_enum_tuple_variants() {
    let mut tb = TypeSpec::<RustLang>::builder("Expr", TypeKind::Enum);
    tb.visibility(Visibility::Public);

    let derive = CodeBlock::<RustLang>::of("#[derive(Debug, Clone)]", ()).unwrap();
    tb.annotation(derive);

    // Simple variant.
    tb.add_variant(EnumVariantSpec::new("Unit"));

    // Single-element tuple variant.
    let mut v_lit = EnumVariantSpec::<RustLang>::builder("Literal");
    v_lit.associated_type(TypeName::primitive("i64"));
    tb.add_variant(v_lit.build());

    // Multi-element tuple variant.
    let mut v_add = EnumVariantSpec::<RustLang>::builder("Add");
    v_add.associated_type(TypeName::generic(
        TypeName::primitive("Box"),
        vec![TypeName::primitive("Expr")],
    ));
    v_add.associated_type(TypeName::generic(
        TypeName::primitive("Box"),
        vec![TypeName::primitive("Expr")],
    ));
    tb.add_variant(v_add.build());

    let mut file = FileSpec::builder_with("expr.rs", RustLang::new());
    file.add_type(tb.build());
    let output = file.build().render(80).unwrap();

    golden::assert_golden("rust/enum_tuple.rs", &output);
}

#[test]
fn test_rust_enum_struct_variants() {
    let mut tb = TypeSpec::<RustLang>::builder("Message", TypeKind::Enum);
    tb.visibility(Visibility::Public);

    let derive = CodeBlock::<RustLang>::of("#[derive(Debug)]", ()).unwrap();
    tb.annotation(derive);

    // Simple variant.
    tb.add_variant(EnumVariantSpec::new("Quit"));

    // Struct variant.
    let mut v_move = EnumVariantSpec::<RustLang>::builder("Move");
    v_move.add_field(FieldSpec::builder("x", TypeName::primitive("i32")).build());
    v_move.add_field(FieldSpec::builder("y", TypeName::primitive("i32")).build());
    tb.add_variant(v_move.build());

    // Tuple variant.
    let mut v_write = EnumVariantSpec::<RustLang>::builder("Write");
    v_write.associated_type(TypeName::primitive("String"));
    tb.add_variant(v_write.build());

    // Another struct variant (last position).
    let mut v_color = EnumVariantSpec::<RustLang>::builder("ChangeColor");
    v_color.add_field(FieldSpec::builder("r", TypeName::primitive("u8")).build());
    v_color.add_field(FieldSpec::builder("g", TypeName::primitive("u8")).build());
    v_color.add_field(FieldSpec::builder("b", TypeName::primitive("u8")).build());
    tb.add_variant(v_color.build());

    let mut file = FileSpec::builder_with("message.rs", RustLang::new());
    file.add_type(tb.build());
    let output = file.build().render(80).unwrap();

    golden::assert_golden("rust/enum_struct.rs", &output);
}
