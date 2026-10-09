use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_if_else() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if (x < 0)", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let body = b.build().unwrap();

    let ts = TypeSpec::builder("Logic", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("GetSign")
                .visibility(Visibility::Public)
                .returns(vec![TypeName::primitive("int")])
                .add_param(ParameterSpec::new("x", TypeName::primitive("int")).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Logic.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/control_flow.cs", &output);
}

#[test]
fn test_for_loop() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("for (var i = 0; i < items.Length; i++)", ());
    b.add_statement("Console.WriteLine(items[i])", ());
    b.end_control_flow();
    let body = b.build().unwrap();

    let file = FileSpec::builder_with("Loop.cs", CSharp::new())
        .add_code(body)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/for_loop.cs", &output);
}

#[test]
fn test_try_catch() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("try", ());
    b.add_statement("DoRiskyOperation()", ());
    b.next_control_flow("catch (Exception ex)", ());
    b.add_statement("Logger.Error(ex.Message)", ());
    b.next_control_flow("finally", ());
    b.add_statement("Cleanup()", ());
    b.end_control_flow();
    let body = b.build().unwrap();

    let file = FileSpec::builder_with("ErrorHandling.cs", CSharp::new())
        .add_code(body)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/try_catch.cs", &output);
}
