use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

/// Shorthand for a JS parameter (no type annotation).
fn param(name: &str) -> ParameterSpec {
    ParameterSpec::new(name, TypeName::primitive("")).unwrap()
}

/// Shorthand for a JS field (no type annotation).
fn field(name: &str) -> FieldSpec {
    FieldSpec::builder(name, TypeName::primitive(""))
        .build()
        .unwrap()
}

#[test]
fn test_full_module() {
    let event_emitter = TypeName::importable("events", "EventEmitter");
    let uuid = TypeName::importable("uuid", "v4");

    // EventBus class extending EventEmitter.
    let ctor_body = CodeBlock::of("super();\nthis.#handlers = new Map();", ()).unwrap();
    let pub_body = CodeBlock::of(
        "const id = %T();\nthis.emit(event, data);\nreturn id;",
        (uuid,),
    )
    .unwrap();

    let ts = TypeSpec::builder("EventBus", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("EventEmitter"))
        .doc("Application event bus.")
        .add_field(field("#handlers"))
        .add_method(
            FunSpec::builder("constructor")
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("publish")
                .add_param(param("event"))
                .add_param(param("data"))
                .body(pub_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // Trigger EventEmitter import.
    let import_trigger = CodeBlock::of("// extends %T", (event_emitter,)).unwrap();

    // Standalone exported function.
    let create_body = CodeBlock::of("return new EventBus();", ()).unwrap();
    let create = FunSpec::builder("createEventBus")
        .visibility(Visibility::Public)
        .body(create_body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("event-bus.js", JavaScript::new())
        .add_code(import_trigger)
        .add_type(ts)
        .add_function(create)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/full_module.js", &output);
}

#[test]
fn test_postfix_increment() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("for (let i = 0; i < 10; i++)", ());
    b.add_statement("count++", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("test.js", JavaScript::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/builder_postfix_increment.js", &output);
}
