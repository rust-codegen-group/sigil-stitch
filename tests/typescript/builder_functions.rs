use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_top_level_function() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("Serializable"));

    let body = CodeBlock::of("return JSON.stringify(value)", ()).unwrap();
    let fb = FunSpec::builder("serialize")
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("string"))
        .body(body);

    let output = FileSpec::builder("serialize.ts")
        .add_function(fb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/top_level_function.ts", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return `Hello, ${name}!`", ()).unwrap();
    let fb = FunSpec::builder("greet")
        .visibility(Visibility::Public)
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
        .returns(TypeName::primitive("string"))
        .body(body);

    let output = FileSpec::builder("greet.ts")
        .add_function(fb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/function_with_doc.ts", &output);
}
