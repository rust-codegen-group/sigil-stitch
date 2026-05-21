use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_method_with_return() {
    let body = CodeBlock::of("return a + b;", ()).unwrap();

    let ts = TypeSpec::builder("Calculator", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("Add")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("int"))
                .add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap())
                .add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Calculator.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/method_with_return.cs", &output);
}

#[test]
fn test_async_method() {
    let body = CodeBlock::of("return await repo.GetByIdAsync(id);", ()).unwrap();

    let ts = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("GetUserAsync")
                .visibility(Visibility::Public)
                .is_async()
                .returns(TypeName::primitive("Task<User>"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserService.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/async_method.cs", &output);
}

#[test]
fn test_generic_method() {
    let tp = TypeParamSpec::new("T");

    let body = CodeBlock::of("return a.CompareTo(b) > 0 ? a : b;", ()).unwrap();

    let ts = TypeSpec::builder("Utils", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("Max")
                .visibility(Visibility::Public)
                .is_static()
                .add_type_param(tp)
                .add_where_constraint(
                    TypeName::primitive("T"),
                    vec![TypeName::primitive("IComparable<T>")],
                )
                .returns(TypeName::primitive("T"))
                .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
                .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Utils.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/generic_method.cs", &output);
}

#[test]
fn test_constructor() {
    let body = CodeBlock::of("this.Name = name;\nthis.Age = age;", ()).unwrap();

    let ts = TypeSpec::builder("Person", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("Person")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .add_param(ParameterSpec::new("age", TypeName::primitive("int")).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Person.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/constructor.cs", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return $\"Hello, {name}!\";", ()).unwrap();

    let ts = TypeSpec::builder("Greeter", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("Greet")
                .visibility(Visibility::Public)
                .doc("<summary>")
                .doc("Greets a user by name.")
                .doc("</summary>")
                .doc("<param name=\"name\">The name to greet.</param>")
                .doc("<returns>A greeting string.</returns>")
                .returns(TypeName::primitive("string"))
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Greeter.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/function_with_doc.cs", &output);
}

#[test]
fn test_async_suppressed_in_interface() {
    let ts = TypeSpec::builder("IUserService", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("GetUserAsync")
                .is_async()
                .returns(TypeName::primitive("Task<User>"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("SaveUserAsync")
                .is_async()
                .returns(TypeName::primitive("Task"))
                .add_param(ParameterSpec::new("user", TypeName::primitive("User")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("IUserService.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/async_interface.cs", &output);
}
