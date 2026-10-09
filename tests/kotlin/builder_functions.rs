use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_suspend_function() {
    let user = TypeName::importable("com.example.model", "User");

    let body = CodeBlock::of("return api.fetchUser(id)", ()).unwrap();
    let fun = FunSpec::builder("fetchUser")
        .is_async()
        .returns(vec![user])
        .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Api.kt", Kotlin::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/suspend_function.kt", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return \"Hello, $name!\"", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .visibility(Visibility::Public)
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(vec![TypeName::primitive("String")])
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder("greet.kt")
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/function_with_doc.kt", &output);
}
