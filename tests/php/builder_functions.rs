use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_top_level_function() {
    let file = FileSpec::builder_with("greet.php", Php::new())
        .add_function(
            FunSpec::builder("greet")
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .returns(TypeName::primitive("string"))
                .body(CodeBlock::of("return \"Hello, \" . $name . \"!\";", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/top_level_function.php", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return $a + $b;", ()).unwrap();
    let fun = FunSpec::builder("add")
        .visibility(Visibility::Public)
        .doc("Add returns the sum of two integers.")
        .add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("add.php", Php::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("php/function_with_doc.php", &output);
}

#[test]
fn test_method() {
    let body = CodeBlock::of("return \"Name: \" . $this->name;", ()).unwrap();
    let fun = FunSpec::builder("getName")
        .visibility(Visibility::Public)
        .returns(TypeName::primitive("string"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("user.php", Php::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("php/method.php", &output);
}
