use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_params() {
    let body = CodeBlock::of("x + y", ()).unwrap();
    let fun = FunSpec::builder("add")
        .returns(TypeName::primitive("Int"))
        .add_param(ParameterSpec::new("x", TypeName::primitive("Int")).unwrap())
        .add_param(ParameterSpec::new("y", TypeName::primitive("Int")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Add.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_params.hs", &output);
}

#[test]
fn test_function_with_import() {
    let map_type = TypeName::importable("Data.Map", "Map");

    let body = CodeBlock::of("Data.Map.empty", ()).unwrap();
    let fun = FunSpec::builder("emptyMap")
        .returns(map_type)
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("EmptyMap.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_import.hs", &output);
}

#[test]
fn test_function_with_context() {
    let body = CodeBlock::of("show x", ()).unwrap();
    let fun = FunSpec::builder("display")
        .add_type_param(
            sigil_stitch::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(TypeName::primitive("Show")),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Display.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_context.hs", &output);
}

#[test]
fn test_function_no_body() {
    let fun = FunSpec::builder("add")
        .returns(TypeName::primitive("Int"))
        .add_param(ParameterSpec::new("x", TypeName::primitive("Int")).unwrap())
        .add_param(ParameterSpec::new("y", TypeName::primitive("Int")).unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Add.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_no_body.hs", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("putStrLn (\"Hello, \" ++ name)", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(TypeName::primitive("IO ()"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("greet.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_doc.hs", &output);
}

#[test]
fn test_multi_constraint_context() {
    let body = CodeBlock::of("show x", ()).unwrap();
    let fun = FunSpec::builder("display")
        .add_type_param(
            sigil_stitch::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(TypeName::primitive("Show"))
                .with_bound(TypeName::primitive("Eq")),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Display.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_multi_context.hs", &output);
}
