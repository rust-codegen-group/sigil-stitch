use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_params() {
    let body = CodeBlock::of("List.map f xs", ()).unwrap();
    let fun = FunSpec::builder("transform")
        .returns(vec![TypeName::primitive("'b list")])
        .add_param(ParameterSpec::new("f", TypeName::primitive("'a -> 'b")).unwrap())
        .add_param(ParameterSpec::new("xs", TypeName::primitive("'a list")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("transform.ml", OCaml::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("ocaml/function_with_params.ml", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("List.map f xs", ()).unwrap();
    let fun = FunSpec::builder("transform")
        .doc("Transform a list using a mapping function.")
        .returns(vec![TypeName::primitive("'b list")])
        .add_param(ParameterSpec::new("f", TypeName::primitive("'a -> 'b")).unwrap())
        .add_param(ParameterSpec::new("xs", TypeName::primitive("'a list")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("transform.ml", OCaml::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("ocaml/function_with_doc.ml", &output);
}
