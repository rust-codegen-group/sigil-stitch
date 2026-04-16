//! Example: Generate a C header file with sigil-stitch.
//!
//! Demonstrates:
//! - `#pragma once` file header
//! - `#include` directives (system and local)
//! - Struct definition with typed fields
//! - Enum via extra_member
//! - Function declarations (no body) and function definitions (with body)
//! - Type-before-name and return-type-as-prefix emission
//!
//! Run: `cargo run --example c_codegen`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // --- Types that trigger #include ---
    let printf = TypeName::<CLang>::importable("stdio.h", "printf");
    let malloc = TypeName::<CLang>::importable("stdlib.h", "malloc");
    let free = TypeName::<CLang>::importable("stdlib.h", "free");

    // --- Enum: LogLevel ---
    let mut enum_b = TypeSpec::<CLang>::builder("LogLevel", TypeKind::Enum);
    enum_b.doc("Severity levels for the logging system.");
    let mut members = CodeBlock::<CLang>::builder();
    members.add("LOG_DEBUG,", ());
    members.add_line();
    members.add("LOG_INFO,", ());
    members.add_line();
    members.add("LOG_WARN,", ());
    members.add_line();
    members.add("LOG_ERROR", ());
    members.add_line();
    enum_b.extra_member(members.build().unwrap());
    let log_level = enum_b.build().unwrap();

    // --- Struct: Config ---
    let mut struct_b = TypeSpec::<CLang>::builder("Config", TypeKind::Struct);
    struct_b.doc("Application configuration.");
    struct_b.add_field(
        FieldSpec::builder("host", TypeName::primitive("const char*"))
            .build()
            .unwrap(),
    );
    struct_b.add_field(
        FieldSpec::builder("port", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    struct_b.add_field(
        FieldSpec::builder("log_level", TypeName::primitive("enum LogLevel"))
            .build()
            .unwrap(),
    );
    struct_b.add_field(
        FieldSpec::builder("max_connections", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    let config = struct_b.build().unwrap();

    // --- Function: config_create ---
    let mut create_body_b = CodeBlock::<CLang>::builder();
    create_body_b.add(
        "struct Config* cfg = (struct Config*)%T(sizeof(struct Config));",
        (malloc,),
    );
    create_body_b.add_line();
    create_body_b.add("cfg->host = host;", ());
    create_body_b.add_line();
    create_body_b.add("cfg->port = port;", ());
    create_body_b.add_line();
    create_body_b.add("cfg->log_level = LOG_INFO;", ());
    create_body_b.add_line();
    create_body_b.add("cfg->max_connections = 100;", ());
    create_body_b.add_line();
    create_body_b.add("return cfg;", ());
    let create_body = create_body_b.build().unwrap();

    let mut create_fn = FunSpec::<CLang>::builder("config_create");
    create_fn.add_param(ParameterSpec::new("host", TypeName::primitive("const char*")).unwrap());
    create_fn.add_param(ParameterSpec::new("port", TypeName::primitive("int")).unwrap());
    create_fn.returns(TypeName::primitive("struct Config*"));
    create_fn.body(create_body);
    let config_create = create_fn.build().unwrap();

    // --- Function: config_destroy ---
    let destroy_body = CodeBlock::<CLang>::of("%T(cfg);", (free,)).unwrap();
    let mut destroy_fn = FunSpec::<CLang>::builder("config_destroy");
    destroy_fn.add_param(ParameterSpec::new("cfg", TypeName::primitive("struct Config*")).unwrap());
    destroy_fn.returns(TypeName::primitive("void"));
    destroy_fn.body(destroy_body);
    let config_destroy = destroy_fn.build().unwrap();

    // --- Function: config_print ---
    let print_body = CodeBlock::<CLang>::of(
        "%T(%S, cfg->host, cfg->port, cfg->log_level);",
        (
            printf,
            StringLitArg("Config { host=%s, port=%d, level=%d }\\n".to_string()),
        ),
    )
    .unwrap();
    let mut print_fn = FunSpec::<CLang>::builder("config_print");
    print_fn
        .add_param(ParameterSpec::new("cfg", TypeName::primitive("const struct Config*")).unwrap());
    print_fn.returns(TypeName::primitive("void"));
    print_fn.body(print_body);
    let config_print = print_fn.build().unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("config.h", CLang::header());
    fb.header(CodeBlock::<CLang>::of("#pragma once", ()).unwrap());
    fb.add_type(log_level);
    fb.add_type(config);
    fb.add_function(config_create);
    fb.add_function(config_destroy);
    fb.add_function(config_print);

    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();
    print!("{output}");
}
