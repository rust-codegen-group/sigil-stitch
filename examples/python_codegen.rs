//! Generate a Python file using structural specs.
//!
//! Run with: `cargo run --example python_codegen`

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::python::Python;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // Importable types.
    let json_dumps = TypeName::<Python>::importable("json", "dumps");
    let dataclass_import = TypeName::<Python>::importable("dataclasses", "dataclass");

    // Build a dataclass.
    let mut tb = TypeSpec::<Python>::builder("Config", TypeKind::Class);
    tb.doc("Application configuration.");
    tb.annotation(CodeBlock::<Python>::of("@%T", (dataclass_import,)).unwrap());

    tb.add_field(
        FieldSpec::builder("host", TypeName::primitive("str"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("port", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );

    let mut f3 = FieldSpec::builder("debug", TypeName::primitive("bool"));
    f3.initializer(CodeBlock::<Python>::of("False", ()).unwrap());
    tb.add_field(f3.build().unwrap());

    // Add a method.
    let mut to_json = FunSpec::<Python>::builder("to_json");
    to_json.doc("Serialize to JSON string.");
    to_json.add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap());
    to_json.returns(TypeName::primitive("str"));
    to_json.body(
        CodeBlock::<Python>::of(
            "return %T({'host': self.host, 'port': self.port})",
            (json_dumps,),
        )
        .unwrap(),
    );
    tb.add_method(to_json.build().unwrap());

    // Build a standalone function.
    let mut greet = FunSpec::<Python>::builder("greet");
    greet.add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap());
    greet.returns(TypeName::primitive("str"));
    greet.body(CodeBlock::<Python>::of("return f'Hello, {name}!'", ()).unwrap());

    // Assemble the file.
    let mut file = FileSpec::builder_with("config.py", Python::new());
    file.add_type(tb.build().unwrap());
    file.add_function(greet.build().unwrap());
    let spec = file.build().unwrap();

    let output = spec.render(80).unwrap();
    println!("{output}");
}
