use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::dart::Dart;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_async_function() {
    let user = TypeName::importable("package:myapp/models/user.dart", "User");

    let body = CodeBlock::of("return await api.fetchUser(id);", ()).unwrap();
    let fun = FunSpec::builder("fetchUser")
        .returns(TypeName::primitive("Future<User>"))
        .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
        .is_async()
        .body(body)
        .build()
        .unwrap();

    // Trigger User import.
    let trigger = CodeBlock::of("// %T", (user,)).unwrap();

    let file = FileSpec::builder_with("api.dart", Dart::new())
        .add_code(trigger)
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/async_function.dart", &output);
}

#[test]
fn test_annotated_method() {
    let body = CodeBlock::of(
        "return %S;",
        (sigil_stitch::code_block::StringLitArg("Woof!".to_string()),),
    )
    .unwrap();
    let ts = TypeSpec::builder("Dog", TypeKind::Class)
        .extends(TypeName::primitive("Animal"))
        .add_method(
            FunSpec::builder("speak")
                .returns(TypeName::primitive("String"))
                .annotation(CodeBlock::of("@override", ()).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("dog.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/annotated_method.dart", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return 'Hello, $name!';", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder("greet.dart")
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/function_with_doc.dart", &output);
}
