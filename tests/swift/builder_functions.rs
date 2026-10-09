use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_async_function() {
    let user = TypeName::importable("MyModule", "User");

    let body = CodeBlock::of("return try await api.fetchUser(id: id)", ()).unwrap();
    let fun = FunSpec::builder("fetchUser")
        .is_async()
        .returns(vec![user])
        .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Api.swift", Swift::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/async_function.swift", &output);
}

#[test]
fn test_override_method() {
    let body = CodeBlock::of(
        "return %S",
        (sigil_stitch::code_block::StringLitArg("Woof!".to_string()),),
    )
    .unwrap();

    let ts = TypeSpec::builder("Dog", TypeKind::Class)
        .extends(TypeName::primitive("Animal"))
        .add_method(
            FunSpec::builder("speak")
                .returns(vec![TypeName::primitive("String")])
                .is_override()
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Dog.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/override_method.swift", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return \"Hello, \\(name)!\"", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .visibility(Visibility::Public)
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(vec![TypeName::primitive("String")])
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder("greet.swift")
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/function_with_doc.swift", &output);
}
