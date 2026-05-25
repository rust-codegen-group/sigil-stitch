//! Generate a C header file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: struct, enum with valued variants, typedef, create/destroy
//! functions, printf with format-string escaping.
//!
//! Run: `cargo run --example c_codegen`

use sigil_stitch::lang::c::C;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn build_shared_types() -> (TypeSpec, TypeSpec, TypeSpec) {
    // --- Enum with valued variants ---
    let log_level = TypeSpec::builder("LogLevel", TypeKind::Enum)
        .doc("Logging severity levels.")
        .add_variant(
            EnumVariantSpec::builder("LOG_DEBUG")
                .value(CodeBlock::of("0", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("LOG_INFO")
                .value(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("LOG_WARN")
                .value(CodeBlock::of("2", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("LOG_ERROR")
                .value(CodeBlock::of("3", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Typedef ---
    let callback = TypeSpec::builder("Callback", TypeKind::TypeAlias)
        .doc("Function pointer type for event callbacks.")
        .extends(TypeName::primitive("void (*)(int, const char*)"))
        .build()
        .unwrap();

    // --- Struct ---
    let config = TypeSpec::builder("Config", TypeKind::Struct)
        .doc("Application configuration.")
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("const char*"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("level", TypeName::primitive("enum LogLevel"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("on_event", TypeName::primitive("Callback"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    (log_level, callback, config)
}

fn builder_approach() -> String {
    let malloc = TypeName::importable("stdlib.h", "malloc");
    let free = TypeName::importable("stdlib.h", "free");
    let printf = TypeName::importable("stdio.h", "printf");
    let (log_level, callback, config) = build_shared_types();
    let comment_label = "NOTE";
    let comment_reason = "Initialize variables";
    let comment_note = "validate input";
    let v_interp = "Config";

    let mut create_body = CodeBlock::builder();
    create_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    create_body.add(
        "struct Config* cfg = (struct Config*)%T(sizeof(%V));",
        (malloc, VerbatimStrArg(v_interp.to_string())),
    );
    create_body.add_line();
    create_body.add(
        "cfg->host = host; %R",
        (CommentArg(comment_note.to_string()),),
    );
    create_body.add_line();
    create_body.add("cfg->port = port;", ());
    create_body.add_line();
    create_body.add("cfg->level = LOG_INFO;", ());
    create_body.add_line();
    create_body.add("cfg->on_event = NULL;", ());
    create_body.add_line();
    create_body.add("return cfg;", ());

    let config_create = FunSpec::builder("config_create")
        .add_param(ParameterSpec::new("host", TypeName::primitive("const char*")).unwrap())
        .add_param(ParameterSpec::new("port", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("struct Config*"))
        .body(create_body.build().unwrap())
        .build()
        .unwrap();

    let destroy_body = CodeBlock::of("%T(cfg);", (free,)).unwrap();
    let config_destroy = FunSpec::builder("config_destroy")
        .add_param(ParameterSpec::new("cfg", TypeName::primitive("struct Config*")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(destroy_body)
        .build()
        .unwrap();

    let print_body = CodeBlock::of(
        "%T(%S, cfg->host, cfg->port, cfg->level);",
        (
            printf,
            StringLitArg("Config { host=%%s, port=%%d, level=%%d }\\n".to_string()),
        ),
    )
    .unwrap();
    let config_print = FunSpec::builder("config_print")
        .add_param(ParameterSpec::new("cfg", TypeName::primitive("const struct Config*")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(print_body)
        .build()
        .unwrap();

    FileSpec::builder_with("config.h", C::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(log_level)
        .add_type(callback)
        .add_type(config)
        .add_function(config_create)
        .add_function(config_destroy)
        .add_function(config_print)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let malloc = TypeName::importable("stdlib.h", "malloc");
    let free = TypeName::importable("stdlib.h", "free");
    let printf = TypeName::importable("stdio.h", "printf");
    let (log_level, callback, config) = build_shared_types();
    let comment_label = "NOTE";
    let comment_reason = "Initialize variables";
    let comment_note = "validate input";
    let v_interp = "Config";

    let create_body = sigil_quote!(C {
        $comment("@{comment_label}: @{comment_reason}");
        struct Config* cfg = (struct Config*)$T(malloc)(sizeof($V("@{v_interp}")));
        cfg->host = host; $comment(comment_note)
        cfg->port = port;
        cfg->level = LOG_INFO;
        cfg->on_event = NULL;
        return cfg;
    })
    .unwrap();

    let config_create = FunSpec::builder("config_create")
        .add_param(ParameterSpec::new("host", TypeName::primitive("const char*")).unwrap())
        .add_param(ParameterSpec::new("port", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("struct Config*"))
        .body(create_body)
        .build()
        .unwrap();

    let destroy_body = sigil_quote!(C { $T(free)(cfg); }).unwrap();
    let config_destroy = FunSpec::builder("config_destroy")
        .add_param(ParameterSpec::new("cfg", TypeName::primitive("struct Config*")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(destroy_body)
        .build()
        .unwrap();

    let print_body = sigil_quote!(C {
        $T(printf)($S("Config { host=%%s, port=%%d, level=%%d }\\n"), cfg->host, cfg->port, cfg->level);
    })
    .unwrap();
    let config_print = FunSpec::builder("config_print")
        .add_param(ParameterSpec::new("cfg", TypeName::primitive("const struct Config*")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(print_body)
        .build()
        .unwrap();

    FileSpec::builder_with("config.h", C::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(log_level)
        .add_type(callback)
        .add_type(config)
        .add_function(config_create)
        .add_function(config_destroy)
        .add_function(config_print)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
