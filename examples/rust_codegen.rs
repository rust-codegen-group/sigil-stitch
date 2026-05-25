//! Generate a Rust file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: enum with tuple/struct variants, newtype, references with lifetimes,
//! `impl Trait` / `dyn Trait`, where constraints, generic functions, `pub(crate)`,
//! `Option<T>`, tuples, derive annotations, and `$T_join` for trait bounds.
//!
//! Run: `cargo run --example rust_codegen`

use sigil_stitch::lang::rust::Rust;
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
    let hashmap = TypeName::importable("std::collections", "HashMap");

    // --- Enum with tuple and struct variants ---
    let event_enum = TypeSpec::builder("Event", TypeKind::Enum)
        .visibility(Visibility::Public)
        .annotate(AnnotationSpec::new("derive").args(["Debug", "Clone"]))
        .add_variant(
            EnumVariantSpec::builder("Click")
                .doc("A mouse click at (x, y).")
                .associated_type(TypeName::primitive("i32"))
                .associated_type(TypeName::primitive("i32"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("KeyPress")
                .associated_type(TypeName::primitive("char"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Resize")
                .add_field(
                    FieldSpec::builder("width", TypeName::primitive("u32"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("height", TypeName::primitive("u32"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_variant(EnumVariantSpec::new("Quit").unwrap())
        .build()
        .unwrap();

    // --- Newtype ---
    let user_id = TypeSpec::builder("UserId", TypeKind::Newtype)
        .visibility(Visibility::Public)
        .annotate(AnnotationSpec::new("derive").args(["Debug", "Clone", "PartialEq", "Eq", "Hash"]))
        .extends(TypeName::primitive("u64"))
        .build()
        .unwrap();

    // --- Struct with rich types ---
    let config = TypeSpec::builder("Config", TypeKind::Struct)
        .visibility(Visibility::Public)
        .annotate(AnnotationSpec::new("derive").args(["Debug", "Clone"]))
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_field(
            FieldSpec::builder(
                "name",
                TypeName::reference_with_lifetime(TypeName::primitive("str"), "'a"),
            )
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "values",
                TypeName::generic(
                    hashmap,
                    vec![TypeName::primitive("String"), TypeName::primitive("i64")],
                ),
            )
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        )
        .add_field(
            FieldSpec::builder("owner", TypeName::optional(TypeName::primitive("UserId")))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "metadata",
                TypeName::tuple(vec![
                    TypeName::primitive("String"),
                    TypeName::primitive("u64"),
                ]),
            )
            .visibility(Visibility::PublicCrate)
            .build()
            .unwrap(),
        )
        .build()
        .unwrap();

    (event_enum, user_id, config)
}

fn builder_approach() -> String {
    let display = TypeName::importable("std::fmt", "Display");
    let comment_reason = "Initialize event handler";
    let comment_label = "EVENT";
    let v_interp = "events";
    let comment_note = "validate event";
    let (event_enum, user_id, config) = build_shared_types();

    // --- Generic function with where constraint ---
    let mut body = CodeBlock::builder();
    body.add_attribute("derive(Debug)");
    body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    body.add("let _ev = %V", (VerbatimStrArg(v_interp.to_string()),));
    body.add_line();
    body.add_statement(
        "println!(\"{}\", item) %R",
        (CommentArg(comment_note.to_string()),),
    );

    let print_fn = FunSpec::builder("print_item")
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![display, TypeName::primitive("Clone")],
        )
        .add_param(
            ParameterSpec::new("item", TypeName::reference(TypeName::primitive("T"))).unwrap(),
        )
        .body(body.build().unwrap())
        .build()
        .unwrap();

    // --- Function returning impl Trait ---
    let mut handler_body = CodeBlock::builder();
    handler_body.add("move |event: &Event| {", ());
    handler_body.add_line();
    handler_body.add("    println!(\"handling {:?}\", event);", ());
    handler_body.add_line();
    handler_body.add("}", ());

    let handler_fn = FunSpec::builder("make_handler")
        .visibility(Visibility::PublicCrate)
        .returns(TypeName::impl_trait(vec![
            TypeName::primitive("Fn(&Event)"),
            TypeName::primitive("Send"),
        ]))
        .body(handler_body.build().unwrap())
        .build()
        .unwrap();

    // --- Function with dyn Trait param ---
    let mut dispatch_body = CodeBlock::builder();
    dispatch_body.add_statement("handler.handle(event)", ());

    let dispatch_fn = FunSpec::builder("dispatch")
        .visibility(Visibility::Public)
        .add_param(
            ParameterSpec::new(
                "handler",
                TypeName::reference_mut(TypeName::dyn_trait(vec![
                    TypeName::primitive("EventHandler"),
                    TypeName::primitive("Send"),
                ])),
            )
            .unwrap(),
        )
        .add_param(
            ParameterSpec::new("event", TypeName::reference(TypeName::primitive("Event"))).unwrap(),
        )
        .body(dispatch_body.build().unwrap())
        .build()
        .unwrap();

    // --- $T_join comparison: manual trait join ---
    let read = TypeName::importable("std::io", "Read");
    let write = TypeName::importable("std::io", "Write");
    let mut join_body = CodeBlock::builder();
    join_body.add("fn process(stream: &mut (dyn ", ());
    join_body.add("%T", (read,));
    join_body.add(" + ", ());
    join_body.add("%T", (write,));
    join_body.add(
        " + Send)) {\n    let mut buf = [0u8; 1024];\n    stream.read(&mut buf).unwrap();\n}",
        (),
    );

    FileSpec::builder_with("events.rs", Rust::new())
        .add_type(event_enum)
        .add_type(user_id)
        .add_type(config)
        .add_function(print_fn)
        .add_function(handler_fn)
        .add_function(dispatch_fn)
        .add_code(join_body.build().unwrap())
        .build()
        .unwrap()
        .render(100)
        .unwrap()
}

fn macro_approach() -> String {
    let display = TypeName::importable("std::fmt", "Display");
    let comment_reason = "Initialize event handler";
    let comment_label = "EVENT";
    let v_interp = "events";
    let comment_note = "validate event";
    let (event_enum, user_id, config) = build_shared_types();

    let print_body = sigil_quote!(Rust {
        $attr("derive(Debug)")
        $comment("@{comment_label}: @{comment_reason}");
        let _ev = $V("@{v_interp}");
        println!("{}", item) $comment(comment_note)
    })
    .unwrap();

    let print_fn = FunSpec::builder("print_item")
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![display, TypeName::primitive("Clone")],
        )
        .add_param(
            ParameterSpec::new("item", TypeName::reference(TypeName::primitive("T"))).unwrap(),
        )
        .body(print_body)
        .build()
        .unwrap();

    let handler_body = sigil_quote!(Rust {
        move |event: &Event| {
            println!("handling {:?}", event);
        }
    })
    .unwrap();

    let handler_fn = FunSpec::builder("make_handler")
        .visibility(Visibility::PublicCrate)
        .returns(TypeName::impl_trait(vec![
            TypeName::primitive("Fn(&Event)"),
            TypeName::primitive("Send"),
        ]))
        .body(handler_body)
        .build()
        .unwrap();

    let dispatch_body = sigil_quote!(Rust {
        handler.handle(event)
    })
    .unwrap();

    let dispatch_fn = FunSpec::builder("dispatch")
        .visibility(Visibility::Public)
        .add_param(
            ParameterSpec::new(
                "handler",
                TypeName::reference_mut(TypeName::dyn_trait(vec![
                    TypeName::primitive("EventHandler"),
                    TypeName::primitive("Send"),
                ])),
            )
            .unwrap(),
        )
        .add_param(
            ParameterSpec::new("event", TypeName::reference(TypeName::primitive("Event"))).unwrap(),
        )
        .body(dispatch_body)
        .build()
        .unwrap();

    // --- $T_join: trait bounds with import tracking ---
    let read = TypeName::importable("std::io", "Read");
    let write = TypeName::importable("std::io", "Write");
    let traits = vec![read, write, TypeName::primitive("Send")];
    let join_body = sigil_quote!(Rust {
        fn process(stream: &mut (dyn $T_join(" + ", &traits))) {
            let mut buf = [0u8; 1024];
            stream.read(&mut buf).unwrap();
        }
    })
    .unwrap();

    FileSpec::builder_with("events.rs", Rust::new())
        .add_type(event_enum)
        .add_type(user_id)
        .add_type(config)
        .add_function(print_fn)
        .add_function(handler_fn)
        .add_function(dispatch_fn)
        .add_code(join_body)
        .build()
        .unwrap()
        .render(100)
        .unwrap()
}
