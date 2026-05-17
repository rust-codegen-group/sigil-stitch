use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::python::Python;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_top_level_function() {
    let json_dumps = TypeName::importable("json", "dumps");

    let file = FileSpec::builder_with("utils.py", Python::new())
        .add_function(
            FunSpec::builder("serialize")
                .doc("Serialize an object to JSON.")
                .add_param(ParameterSpec::new("value", TypeName::primitive("object")).unwrap())
                .returns(TypeName::primitive("str"))
                .body(CodeBlock::of("return %T(value)", (json_dumps,)).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/top_level_function.py", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return f\"Hello, {name}!\"", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap())
        .returns(TypeName::primitive("str"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder("greet.py")
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("python/function_with_doc.py", &output);
}

#[test]
fn test_static_method_no_keyword() {
    let body = CodeBlock::of("return cls(name, age)", ()).unwrap();

    let ts = TypeSpec::builder("User", TypeKind::Class)
        .add_method(
            FunSpec::builder("from_dict")
                .is_static()
                .annotation(CodeBlock::of("@classmethod", ()).unwrap())
                .add_param(ParameterSpec::new("cls", TypeName::primitive("")).unwrap())
                .add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap())
                .add_param(ParameterSpec::new("age", TypeName::primitive("int")).unwrap())
                .returns(TypeName::primitive("User"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("user.py", Python::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("python/static_method.py", &output);
}
