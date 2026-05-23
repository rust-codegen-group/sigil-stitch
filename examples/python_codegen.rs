//! Generate a Python file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: dataclass with decorator, enum, optional types (`T | None`),
//! default parameter values, static methods, standalone functions,
//! and `$T_join` for type unions.
//!
//! Run: `cargo run --example python_codegen`

use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn build_shared_types() -> (TypeSpec,) {
    let enum_import = TypeName::importable("enum", "Enum");

    // --- Enum ---
    let level_enum = TypeSpec::builder("LogLevel", TypeKind::Enum)
        .extends(TypeName::primitive("str"))
        .extends(TypeName::raw("Enum"))
        .annotation(CodeBlock::of("# %T", (enum_import,)).unwrap())
        .add_variant(
            EnumVariantSpec::builder("DEBUG")
                .value(CodeBlock::of("%S", (StringLitArg("debug".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("INFO")
                .value(CodeBlock::of("%S", (StringLitArg("info".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("ERROR")
                .value(CodeBlock::of("%S", (StringLitArg("error".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    (level_enum,)
}

fn builder_approach() -> String {
    let json_dumps = TypeName::importable("json", "dumps");
    let (level_enum,) = build_shared_types();

    let mut to_json_body = CodeBlock::builder();
    to_json_body.add_statement(
        "return %T({'host': self.host, 'port': self.port})",
        (json_dumps.clone(),),
    );

    let to_json = FunSpec::builder("to_json")
        .doc("Serialize to JSON string.")
        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
        .returns(TypeName::primitive("str"))
        .body(to_json_body.build().unwrap())
        .build()
        .unwrap();

    let mut from_env_body = CodeBlock::builder();
    from_env_body.add_statement("return cls(host=host, port=port)", ());

    let from_env = FunSpec::builder("from_defaults")
        .is_static()
        .annotate(AnnotationSpec::new("classmethod"))
        .add_param(ParameterSpec::new("cls", TypeName::primitive("")).unwrap())
        .add_param(
            ParameterSpec::builder("host", TypeName::primitive("str"))
                .default_value(CodeBlock::of("%S", (StringLitArg("localhost".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_param(
            ParameterSpec::builder("port", TypeName::primitive("int"))
                .default_value(CodeBlock::of("8080", ()).unwrap())
                .build()
                .unwrap(),
        )
        .returns(TypeName::primitive("Config"))
        .body(from_env_body.build().unwrap())
        .build()
        .unwrap();

    // Reconstruct Config with methods
    let config_with_methods = TypeSpec::builder("Config", TypeKind::Class)
        .doc("Application configuration.")
        .annotation(
            CodeBlock::of("@%T", (TypeName::importable("dataclasses", "dataclass"),)).unwrap(),
        )
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("str"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .initializer(CodeBlock::of("8080", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("debug", TypeName::primitive("bool"))
                .initializer(CodeBlock::of("False", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "log_level",
                TypeName::optional(TypeName::primitive("LogLevel")),
            )
            .initializer(CodeBlock::of("None", ()).unwrap())
            .build()
            .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "tags",
                TypeName::generic(
                    TypeName::primitive("list"),
                    vec![TypeName::primitive("str")],
                ),
            )
            .initializer(
                CodeBlock::of(
                    "%T(default_factory=list)",
                    (TypeName::importable("dataclasses", "field"),),
                )
                .unwrap(),
            )
            .build()
            .unwrap(),
        )
        .add_method(to_json)
        .add_method(from_env)
        .build()
        .unwrap();

    let mut greet_body = CodeBlock::builder();
    greet_body.add_statement("return f'Hello, {name}!'", ());

    let greet = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap())
        .returns(TypeName::primitive("str"))
        .body(greet_body.build().unwrap())
        .build()
        .unwrap();

    // --- $T_join comparison: manual type union ---
    let date = TypeName::importable("datetime", "date");
    let mut join_body = CodeBlock::builder();
    join_body.add("JsonValue = str | int | float | bool | None | ", ());
    join_body.add("%T", (date,));

    FileSpec::builder_with("config.py", Python::new())
        .add_type(level_enum)
        .add_type(config_with_methods)
        .add_function(greet)
        .add_code(join_body.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let json_dumps = TypeName::importable("json", "dumps");
    let (level_enum,) = build_shared_types();

    let to_json_body = sigil_quote!(Python {
        return $T(json_dumps)({$S("host"): self.host, $S("port"): self.port})
    })
    .unwrap();

    let to_json = FunSpec::builder("to_json")
        .doc("Serialize to JSON string.")
        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
        .returns(TypeName::primitive("str"))
        .body(to_json_body)
        .build()
        .unwrap();

    let from_env_body = sigil_quote!(Python {
        return cls(host = host, port = port)
    })
    .unwrap();

    let from_env = FunSpec::builder("from_defaults")
        .is_static()
        .annotate(AnnotationSpec::new("classmethod"))
        .add_param(ParameterSpec::new("cls", TypeName::primitive("")).unwrap())
        .add_param(
            ParameterSpec::builder("host", TypeName::primitive("str"))
                .default_value(CodeBlock::of("%S", (StringLitArg("localhost".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_param(
            ParameterSpec::builder("port", TypeName::primitive("int"))
                .default_value(CodeBlock::of("8080", ()).unwrap())
                .build()
                .unwrap(),
        )
        .returns(TypeName::primitive("Config"))
        .body(from_env_body)
        .build()
        .unwrap();

    let config_with_methods = TypeSpec::builder("Config", TypeKind::Class)
        .doc("Application configuration.")
        .annotation(
            CodeBlock::of("@%T", (TypeName::importable("dataclasses", "dataclass"),)).unwrap(),
        )
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("str"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .initializer(CodeBlock::of("8080", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("debug", TypeName::primitive("bool"))
                .initializer(CodeBlock::of("False", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "log_level",
                TypeName::optional(TypeName::primitive("LogLevel")),
            )
            .initializer(CodeBlock::of("None", ()).unwrap())
            .build()
            .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "tags",
                TypeName::generic(
                    TypeName::primitive("list"),
                    vec![TypeName::primitive("str")],
                ),
            )
            .initializer(
                CodeBlock::of(
                    "%T(default_factory=list)",
                    (TypeName::importable("dataclasses", "field"),),
                )
                .unwrap(),
            )
            .build()
            .unwrap(),
        )
        .add_method(to_json)
        .add_method(from_env)
        .build()
        .unwrap();

    let greet_body = sigil_quote!(Python {
        return $V("Hello, {name}!")
    })
    .unwrap();

    let greet = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap())
        .returns(TypeName::primitive("str"))
        .body(greet_body)
        .build()
        .unwrap();

    // --- $T_join: type union with import tracking ---
    let types = vec![
        TypeName::primitive("str"),
        TypeName::primitive("int"),
        TypeName::primitive("float"),
        TypeName::primitive("bool"),
        TypeName::primitive("None"),
        TypeName::importable("datetime", "date"),
    ];
    let join_body = sigil_quote!(Python {
        JsonValue = $T_join(" | ", &types)
    })
    .unwrap();

    FileSpec::builder_with("config.py", Python::new())
        .add_type(level_enum)
        .add_type(config_with_methods)
        .add_function(greet)
        .add_code(join_body)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
