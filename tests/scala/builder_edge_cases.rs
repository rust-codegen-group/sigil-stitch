use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::scala::Scala;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_string_dollar_escape() {
    let body = CodeBlock::<Scala>::of(
        "val greeting = %S\nval template = %S\nprintln(greeting)",
        (
            StringLitArg("Hello ${name}!".into()),
            StringLitArg("Price: $100".into()),
        ),
    )
    .unwrap();
    let mut fb = FunSpec::<Scala>::builder("greet");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("greet.scala", Scala::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/string_dollar_escape.scala", &output);
}
