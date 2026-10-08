//! Generate a C# file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: class with properties, interface, enum, generic method,
//! async/await, annotations (`[Serializable]`), override, static methods,
//! protected visibility, and nullable types (`T?`).
//!
//! Run: `cargo run --example csharp_codegen`

use sigil_stitch::lang::csharp::CSharp;
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
    // --- Enum ---
    let status = TypeSpec::builder("Status", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(EnumVariantSpec::new("Pending").unwrap())
        .add_variant(EnumVariantSpec::new("Active").unwrap())
        .add_variant(EnumVariantSpec::new("Archived").unwrap())
        .build()
        .unwrap();

    // --- Interface ---
    let iface = TypeSpec::builder("IRepository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_generic_param(GenericParamSpec::single("T").unwrap())
        .add_method(
            FunSpec::builder("FindByIdAsync")
                .is_async()
                .returns(TypeName::application(
                    TypeName::primitive("Task"),
                    vec![TypeArgument::Single(TypeName::optional(
                        TypeName::primitive("T"),
                    ))],
                ))
                .add_param(ParameterSpec::new("id", TypeName::primitive("int")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("SaveAsync")
                .is_async()
                .returns(TypeName::primitive("Task"))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    (status, iface)
}

fn builder_approach() -> String {
    let console = TypeName::importable("System", "Console");
    let (status, iface) = build_shared_types();
    let comment_label = "TODO";
    let comment_reason = "Validate entity state";
    let comment_note = "log output";
    let v_interp = "ToString";

    // --- Abstract base class ---
    let base = TypeSpec::builder("Entity", TypeKind::Class)
        .visibility(Visibility::Public)
        .is_abstract()
        .annotate(AnnotationSpec::new("Serializable"))
        .add_field(
            FieldSpec::builder("Name", TypeName::primitive("string"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("Id", TypeName::primitive("int"))
                .visibility(Visibility::Protected)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Validate")
                .visibility(Visibility::Public)
                .is_abstract()
                .returns(TypeName::primitive("bool"))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("ToString")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("string"))
                .body({
                    let mut b = CodeBlock::builder();
                    b.add_attribute("Obsolete(\"Use newMethod instead\")");
                    b.add_comment(&format!("{}: {}", comment_label, comment_reason));
                    b.add_statement(
                        "return Name; // %V %R",
                        (
                            VerbatimStrArg(v_interp.to_string()),
                            CommentArg(comment_note.to_string()),
                        ),
                    );
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Concrete class ---
    let mut validate_body = CodeBlock::builder();
    validate_body.add_statement("return !string.IsNullOrEmpty(Name)", ());

    let mut log_body = CodeBlock::builder();
    log_body.add_statement("%T.WriteLine(message)", (console,));

    let user = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Entity"))
        .add_field(
            FieldSpec::builder("Email", TypeName::primitive("string"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("Status", TypeName::primitive("Status"))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("Status.Pending", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Validate")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("bool"))
                .body(validate_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Log")
                .visibility(Visibility::Public)
                .is_static()
                .add_param(ParameterSpec::new("message", TypeName::primitive("string")).unwrap())
                .body(log_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    FileSpec::builder_with("User.cs", CSharp::new())
        .add_type(status)
        .add_type(iface)
        .add_type(base)
        .add_type(user)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let console = TypeName::importable("System", "Console");
    let (status, iface) = build_shared_types();
    let comment_label = "TODO";
    let comment_reason = "Validate entity state";
    let comment_note = "log output";
    let v_interp = "ToString";

    let base = TypeSpec::builder("Entity", TypeKind::Class)
        .visibility(Visibility::Public)
        .is_abstract()
        .annotate(AnnotationSpec::new("Serializable"))
        .add_field(
            FieldSpec::builder("Name", TypeName::primitive("string"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("Id", TypeName::primitive("int"))
                .visibility(Visibility::Protected)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Validate")
                .visibility(Visibility::Public)
                .is_abstract()
                .returns(TypeName::primitive("bool"))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("ToString")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("string"))
                .body(
                    sigil_quote!(CSharp {
                        $attr("Obsolete(\"Use newMethod instead\")");
                        $comment("@{comment_label}: @{comment_reason}");
                        $V("// @{v_interp} override");
                        return Name; $comment(comment_note)
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let user = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Entity"))
        .add_field(
            FieldSpec::builder("Email", TypeName::primitive("string"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("Status", TypeName::primitive("Status"))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("Status.Pending", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Validate")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("bool"))
                .body(
                    sigil_quote!(CSharp {
                        return !string.IsNullOrEmpty(Name);
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Log")
                .visibility(Visibility::Public)
                .is_static()
                .add_param(ParameterSpec::new("message", TypeName::primitive("string")).unwrap())
                .body(
                    sigil_quote!(CSharp {
                        $T(console).WriteLine(message);
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    FileSpec::builder_with("User.cs", CSharp::new())
        .add_type(status)
        .add_type(iface)
        .add_type(base)
        .add_type(user)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
