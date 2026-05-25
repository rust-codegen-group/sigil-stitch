use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go::Go;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_top_level_function() {
    let fmt_sprintf = TypeName::importable("fmt", "Sprintf");

    let file = FileSpec::builder_with("greet.go", Go::new())
        .header(CodeBlock::of("package greet", ()).unwrap())
        .add_function(
            FunSpec::builder("Greet")
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .returns(TypeName::primitive("string"))
                .body(CodeBlock::of("return %T(\"Hello, %%s!\", name)", (fmt_sprintf,)).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/top_level_function.go", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return a + b", ()).unwrap();
    let fun = FunSpec::builder("Add")
        .visibility(Visibility::Public)
        .doc("Add returns the sum of two integers.")
        .add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("add.go", Go::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("go/function_with_doc.go", &output);
}
