//! Generate a Kotlin file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: data class with primary constructor params, enum with values,
//! override methods, nullable types (`T?`), variance (`out T` / `in T`),
//! and suspend functions.
//!
//! Run: `cargo run --example kotlin_codegen`

use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn build_shared_types() -> (TypeSpec, TypeSpec) {
    let uuid = TypeName::importable("java.util", "UUID");

    // --- Enum with values ---
    let status = TypeSpec::builder("Status", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("PENDING")
                .constructor_argument(
                    CodeBlock::of("%S", (StringLitArg("pending".into()),)).unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("ACTIVE")
                .constructor_argument(
                    CodeBlock::of("%S", (StringLitArg("active".into()),)).unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("ARCHIVED")
                .constructor_argument(
                    CodeBlock::of("%S", (StringLitArg("archived".into()),)).unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::builder("label", TypeName::primitive("String"))
                .is_property()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Data class with primary constructor ---
    let task = TypeSpec::builder("Task", TypeKind::Struct)
        .doc("A task entity.")
        .add_primary_constructor_param(
            ParameterSpec::new("id", TypeName::primitive("String")).unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::new("name", TypeName::primitive("String")).unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::new("status", TypeName::primitive("Status")).unwrap(),
        )
        .add_field(
            FieldSpec::builder("done", TypeName::primitive("Boolean"))
                .initializer(CodeBlock::of("false", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "assignee",
                TypeName::optional(TypeName::primitive("String")),
            )
            .initializer(CodeBlock::of("null", ()).unwrap())
            .build()
            .unwrap(),
        )
        .build()
        .unwrap();

    // Also create a trigger for UUID import
    let _uuid_trigger = CodeBlock::of("// %T", (uuid,)).unwrap();

    (status, task)
}

fn builder_approach() -> String {
    let uuid = TypeName::importable("java.util", "UUID");
    let comment_reason = "Create a new task entity";
    let comment_label = "NOTE";
    let v_interp = "task";
    let comment_note = "ID is auto-generated via UUID";
    let (status, task) = build_shared_types();

    // --- Interface with variance ---
    let repo = TypeSpec::builder("Repository", TypeKind::Interface)
        .add_type_param(TypeParamSpec::new("T"))
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::optional(TypeName::primitive("T")))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("addAll")
                .add_param(
                    ParameterSpec::new(
                        "items",
                        TypeName::generic(
                            TypeName::primitive("List"),
                            vec![TypeName::wildcard_extends(TypeName::primitive("T"))],
                        ),
                    )
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Create function ---
    let mut create_body = CodeBlock::builder();
    create_body.add_attribute("Override");
    create_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    create_body.add(
        "val entity = %V",
        (VerbatimStrArg(format!("new {}", v_interp)),),
    );
    create_body.add_line();
    create_body.add_statement(
        "return Task(\n    id = %T.randomUUID().toString(),\n    name = name,\n    status = Status.PENDING\n) %R",
        (uuid, CommentArg(comment_note.to_string())),
    );

    let create_fn = FunSpec::builder("createTask")
        .returns(TypeName::primitive("Task"))
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .body(create_body.build().unwrap())
        .build()
        .unwrap();

    // --- Suspend function ---
    let mut fetch_body = CodeBlock::builder();
    fetch_body.add(
        "val tasks = listOf(createTask(\"alpha\"), createTask(\"beta\"))",
        (),
    );
    fetch_body.add_line();
    fetch_body.add("return tasks", ());

    let fetch_fn = FunSpec::builder("fetchTasks")
        .is_async()
        .returns(TypeName::generic(
            TypeName::primitive("List"),
            vec![TypeName::primitive("Task")],
        ))
        .body(fetch_body.build().unwrap())
        .build()
        .unwrap();

    FileSpec::builder_with("Tasks.kt", Kotlin::new())
        .add_type(status)
        .add_type(task)
        .add_type(repo)
        .add_function(create_fn)
        .add_function(fetch_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let uuid = TypeName::importable("java.util", "UUID");
    let comment_reason = "Create a new task entity";
    let comment_label = "NOTE";
    let v_interp = "task";
    let comment_note = "ID is auto-generated via UUID";
    let (status, task) = build_shared_types();

    let repo = TypeSpec::builder("Repository", TypeKind::Interface)
        .add_type_param(TypeParamSpec::new("T"))
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::optional(TypeName::primitive("T")))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("addAll")
                .add_param(
                    ParameterSpec::new(
                        "items",
                        TypeName::generic(
                            TypeName::primitive("List"),
                            vec![TypeName::wildcard_extends(TypeName::primitive("T"))],
                        ),
                    )
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let create_body = sigil_quote!(Kotlin {
        $attr("Override")
        $comment("@{comment_label}: @{comment_reason}");
        val entity = $V("new @{v_interp}")
        return Task(
            id = $T(uuid).randomUUID().toString(),
            name = name,
            status = Status.PENDING $comment(comment_note)
        )
    })
    .unwrap();

    let create_fn = FunSpec::builder("createTask")
        .returns(TypeName::primitive("Task"))
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .body(create_body)
        .build()
        .unwrap();

    let fetch_body = sigil_quote!(Kotlin {
        val tasks = listOf(createTask($S("alpha")), createTask($S("beta")))
        return tasks
    })
    .unwrap();

    let fetch_fn = FunSpec::builder("fetchTasks")
        .is_async()
        .returns(TypeName::generic(
            TypeName::primitive("List"),
            vec![TypeName::primitive("Task")],
        ))
        .body(fetch_body)
        .build()
        .unwrap();

    FileSpec::builder_with("Tasks.kt", Kotlin::new())
        .add_type(status)
        .add_type(task)
        .add_type(repo)
        .add_function(create_fn)
        .add_function(fetch_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
