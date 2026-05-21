use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_top_level_function() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("std::fmt::Display"));

    let body = CodeBlock::of("println!(\"{}\", value)", ()).unwrap();
    let fb = FunSpec::builder("print_value")
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .add_param(ParameterSpec::new("value", TypeName::primitive("&T")).unwrap())
        .body(body);

    let output = FileSpec::builder_with("utils.rs", RustLang::new())
        .add_function(fb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/top_level_function.rs", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("format!(\"Hello, {}!\", name)", ()).unwrap();
    let fb = FunSpec::builder("greet")
        .visibility(Visibility::Public)
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body);

    let output = FileSpec::builder_with("greet.rs", RustLang::new())
        .add_function(fb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/function_with_doc.rs", &output);
}
