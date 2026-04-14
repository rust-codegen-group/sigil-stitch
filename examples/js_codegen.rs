//! Example: Generate a JavaScript module with sigil-stitch.
//!
//! Demonstrates:
//! - `import { X } from 'module'` (ESM imports, no `import type`)
//! - Class with `#private` fields and constructor
//! - Methods without type annotations
//! - `export class` and `export function`
//! - Class inheritance with `extends`
//! - JSDoc comments
//!
//! Run: `cargo run --example js_codegen`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

/// Shorthand for a JS parameter (no type annotation).
fn param(name: &str) -> ParameterSpec<JavaScript> {
    ParameterSpec::new(name, TypeName::primitive(""))
}

/// Shorthand for a JS field (no type annotation).
fn field(name: &str) -> FieldSpec<JavaScript> {
    FieldSpec::builder(name, TypeName::primitive("")).build()
}

fn main() {
    // --- Imports (triggered by usage in code) ---
    let event_emitter = TypeName::<JavaScript>::importable("events", "EventEmitter");
    let uuid = TypeName::<JavaScript>::importable("uuid", "v4");
    let format = TypeName::<JavaScript>::importable("./utils", "formatMessage");

    // --- Base class: Logger ---
    let mut logger_tb = TypeSpec::<JavaScript>::builder("Logger", TypeKind::Class);
    logger_tb.visibility(Visibility::Public);
    logger_tb.doc("Base logger class.");
    logger_tb.doc("");
    logger_tb.doc("@abstract");

    logger_tb.add_field(field("#name"));
    logger_tb.add_field(field("#level"));

    // Constructor
    let ctor_body =
        CodeBlock::<JavaScript>::of("this.#name = name;\nthis.#level = level || 'info';", ())
            .unwrap();
    let mut ctor = FunSpec::<JavaScript>::builder("constructor");
    ctor.add_param(param("name"));
    ctor.add_param(param("level"));
    ctor.body(ctor_body);
    logger_tb.add_method(ctor.build());

    // getName method
    let get_name_body = CodeBlock::<JavaScript>::of("return this.#name;", ()).unwrap();
    let mut get_name = FunSpec::<JavaScript>::builder("getName");
    get_name.body(get_name_body);
    logger_tb.add_method(get_name.build());

    let logger = logger_tb.build();

    // --- Derived class: ConsoleLogger ---
    let mut console_tb = TypeSpec::<JavaScript>::builder("ConsoleLogger", TypeKind::Class);
    console_tb.visibility(Visibility::Public);
    console_tb.extends(TypeName::primitive("Logger"));
    console_tb.doc("Logger that writes formatted messages to the console.");

    // Constructor
    let ctor_body2 = CodeBlock::<JavaScript>::of("super(name, 'info');", ()).unwrap();
    let mut ctor2 = FunSpec::<JavaScript>::builder("constructor");
    ctor2.add_param(param("name"));
    ctor2.body(ctor_body2);
    console_tb.add_method(ctor2.build());

    // log method — uses imports
    let log_body = CodeBlock::<JavaScript>::of(
        "const msg = %T(this.getName(), message);\nconsole.log(msg);",
        (format,),
    )
    .unwrap();
    let mut log_fn = FunSpec::<JavaScript>::builder("log");
    log_fn.add_param(param("message"));
    log_fn.body(log_body);
    console_tb.add_method(log_fn.build());

    let console_logger = console_tb.build();

    // --- EventBus class ---
    let mut bus_tb = TypeSpec::<JavaScript>::builder("EventBus", TypeKind::Class);
    bus_tb.visibility(Visibility::Public);
    bus_tb.extends(TypeName::primitive("EventEmitter"));
    bus_tb.doc("Publish-subscribe event bus.");

    bus_tb.add_field(field("#subscribers"));

    let bus_ctor_body =
        CodeBlock::<JavaScript>::of("super();\nthis.#subscribers = new Map();", ()).unwrap();
    let mut bus_ctor = FunSpec::<JavaScript>::builder("constructor");
    bus_ctor.body(bus_ctor_body);
    bus_tb.add_method(bus_ctor.build());

    let emit_body = CodeBlock::<JavaScript>::of(
        "const id = %T();\nthis.emit(event, { id, ...data });\nreturn id;",
        (uuid,),
    )
    .unwrap();
    let mut emit_fn = FunSpec::<JavaScript>::builder("publish");
    emit_fn.add_param(param("event"));
    emit_fn.add_param(param("data"));
    emit_fn.body(emit_body);
    bus_tb.add_method(emit_fn.build());

    let event_bus = bus_tb.build();

    // --- Standalone exported functions ---
    let create_logger_body =
        CodeBlock::<JavaScript>::of("return new ConsoleLogger(name);", ()).unwrap();
    let mut create_logger = FunSpec::<JavaScript>::builder("createLogger");
    create_logger.visibility(Visibility::Public);
    create_logger.add_param(param("name"));
    create_logger.body(create_logger_body);
    let create_logger_fn = create_logger.build();

    let create_bus_body = CodeBlock::<JavaScript>::of("return new EventBus();", ()).unwrap();
    let mut create_bus = FunSpec::<JavaScript>::builder("createEventBus");
    create_bus.visibility(Visibility::Public);
    create_bus.body(create_bus_body);
    let create_bus_fn = create_bus.build();

    // Async function
    let fetch_body = CodeBlock::<JavaScript>::of(
        "const response = await fetch(url);\nif (!response.ok) {\n  throw new Error(%S + response.status);\n}\nreturn response.json();",
        (StringLitArg("HTTP error: ".to_string()),),
    )
    .unwrap();
    let mut fetch_fn = FunSpec::<JavaScript>::builder("fetchJSON");
    fetch_fn.visibility(Visibility::Public);
    fetch_fn.is_async();
    fetch_fn.add_param(param("url"));
    fetch_fn.body(fetch_body);
    let fetch_json = fetch_fn.build();

    // --- Trigger imports for extends base types ---
    let import_trigger =
        CodeBlock::<JavaScript>::of("// Base classes: %T", (event_emitter,)).unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("app.js", JavaScript::new());
    fb.add_code(import_trigger);
    fb.add_type(logger);
    fb.add_type(console_logger);
    fb.add_type(event_bus);
    fb.add_function(create_logger_fn);
    fb.add_function(create_bus_fn);
    fb.add_function(fetch_json);

    let file = fb.build();
    let output = file.render(80).unwrap();
    print!("{output}");
}
