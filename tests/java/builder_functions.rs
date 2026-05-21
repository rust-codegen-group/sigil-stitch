use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return \"Hello, \" + name;", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .visibility(Visibility::Public)
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Greet.java", JavaLang::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/function_with_doc.java", &output);
}

#[test]
fn test_generic_type_params_before_return_type() {
    use sigil_stitch::spec::where_spec::TypeParamSpec;

    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("Comparable"));
    let body = CodeBlock::of("Collections.sort(list);\nreturn list;", ()).unwrap();
    let fun = FunSpec::builder("sortList")
        .visibility(Visibility::Public)
        .is_static()
        .add_type_param(tp)
        .add_param(ParameterSpec::new("list", TypeName::primitive("List<T>")).unwrap())
        .returns(TypeName::primitive("List<T>"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Sort.java", JavaLang::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(
        output.contains("<T extends Comparable> List<T> sortList"),
        "Java type params should appear before return type, got:\n{output}"
    );
    assert!(
        !output.contains("sortList<T"),
        "should NOT put type params after function name, got:\n{output}"
    );
}

#[test]
fn test_override_annotation() {
    let body = CodeBlock::of("return \"Woof!\";", ()).unwrap();

    let ts = TypeSpec::builder("Dog", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Animal"))
        .add_method(
            FunSpec::builder("speak")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("String"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Dog.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/override_method.java", &output);
}

#[test]
fn test_generic_params_before_return_golden() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("Comparable"));
    let body = CodeBlock::of("Collections.sort(list);\nreturn list;", ()).unwrap();

    let ts = TypeSpec::builder("Utils", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("sortList")
                .visibility(Visibility::Public)
                .is_static()
                .add_type_param(tp)
                .add_param(ParameterSpec::new("list", TypeName::primitive("List<T>")).unwrap())
                .returns(TypeName::primitive("List<T>"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Utils.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/generic_params_before_return.java", &output);
}
