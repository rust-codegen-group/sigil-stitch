//! Generate a Scala file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: case class (data class), trait, bounded type params,
//! context bounds, higher-kinded types (`F[_]`), and generic functions.
//!
//! Run: `cargo run --example scala_codegen`

use sigil_stitch::lang::scala::Scala;
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
    // --- Trait with bounded type param ---
    let repo = TypeSpec::builder("Repository", TypeKind::Interface)
        .add_generic_param(GenericParamSpec::single("T").unwrap())
        .add_method(
            FunSpec::builder("findById")
                .returns(vec![TypeName::optional(TypeName::primitive("T"))])
                .add_param(ParameterSpec::new("id", TypeName::primitive("Long")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .returns(vec![TypeName::primitive("Unit")])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Case class ---
    let user = TypeSpec::builder("User", TypeKind::Struct)
        .doc("A user entity.")
        .add_primary_constructor_param(
            ParameterSpec::new("id", TypeName::primitive("Long")).unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::new("name", TypeName::primitive("String")).unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::new("email", TypeName::primitive("String")).unwrap(),
        )
        .build()
        .unwrap();

    (repo, user)
}

fn builder_approach() -> String {
    let list_buffer = TypeName::importable("scala.collection.mutable", "ListBuffer");
    let comment_reason = "Initialize service state";
    let comment_label = "NOTE";
    let v_interp = "items";
    let comment_note = "validate input";
    let (repo, user) = build_shared_types();

    // --- Generic function with context bound ---
    let mut sort_body = CodeBlock::builder();
    sort_body.add_attribute("Override");
    sort_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    sort_body.add("val _it = %V", (VerbatimStrArg(v_interp.to_string()),));
    sort_body.add_line();
    sort_body.add_statement("items.sorted %R", (CommentArg(comment_note.to_string()),));

    let sort_fn = FunSpec::builder("sortItems")
        .add_generic_param(
            GenericParamSpec::single("T")
                .unwrap()
                .with_context_bound(TypeName::primitive("Ordering"))
                .unwrap(),
        )
        .add_param(
            ParameterSpec::new(
                "items",
                TypeName::application(
                    TypeName::primitive("List"),
                    vec![TypeArgument::Single(TypeName::primitive("T"))],
                ),
            )
            .unwrap(),
        )
        .returns(vec![TypeName::application(
            TypeName::primitive("List"),
            vec![TypeArgument::Single(TypeName::primitive("T"))],
        )])
        .body(sort_body.build().unwrap())
        .build()
        .unwrap();

    // --- Function using import ---
    let mut collect_body = CodeBlock::builder();
    collect_body.add("val buf = new %T[String]()", (list_buffer,));
    collect_body.add_line();
    collect_body.add("buf += name", ());
    collect_body.add_line();
    collect_body.add("buf.toList", ());

    let collect_fn = FunSpec::builder("collectNames")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(vec![TypeName::application(
            TypeName::primitive("List"),
            vec![TypeArgument::Single(TypeName::primitive("String"))],
        )])
        .body(collect_body.build().unwrap())
        .build()
        .unwrap();

    // --- Higher-kinded type param ---
    let functor = TypeSpec::builder("Functor", TypeKind::Interface)
        .add_generic_param(
            GenericParamSpec::new(
                "F",
                GenericParamDomain::Single {
                    kind: Some(KindExpr::Constructor {
                        parameters: vec![KindExpr::Type],
                        result: Box::new(KindExpr::Type),
                    }),
                },
            )
            .unwrap(),
        )
        .add_method(
            FunSpec::builder("map")
                .add_generic_param(GenericParamSpec::single("A").unwrap())
                .add_generic_param(GenericParamSpec::single("B").unwrap())
                .add_param(
                    ParameterSpec::new(
                        "fa",
                        TypeName::application(
                            TypeName::primitive("F"),
                            vec![TypeArgument::Single(TypeName::primitive("A"))],
                        ),
                    )
                    .unwrap(),
                )
                .add_param(
                    ParameterSpec::new(
                        "f",
                        TypeName::callable(
                            vec![CallableParam::Single {
                                name: None,
                                type_name: TypeName::primitive("A"),
                                presence: CallableParamPresence::Required,
                            }],
                            vec![TypeName::primitive("B")],
                        ),
                    )
                    .unwrap(),
                )
                .returns(vec![TypeName::application(
                    TypeName::primitive("F"),
                    vec![TypeArgument::Single(TypeName::primitive("B"))],
                )])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    FileSpec::builder_with("User.scala", Scala::new())
        .add_type(repo)
        .add_type(user)
        .add_type(functor)
        .add_function(sort_fn)
        .add_function(collect_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let list_buffer = TypeName::importable("scala.collection.mutable", "ListBuffer");
    let comment_reason = "Initialize service state";
    let comment_label = "NOTE";
    let v_interp = "items";
    let comment_note = "validate input";
    let (repo, user) = build_shared_types();

    let sort_body = sigil_quote!(Scala {
        $attr("Override")
        $comment("@{comment_label}: @{comment_reason}");
        val _it = $V("@{v_interp}")
        items.sorted $comment(comment_note)
    })
    .unwrap();

    let sort_fn = FunSpec::builder("sortItems")
        .add_generic_param(
            GenericParamSpec::single("T")
                .unwrap()
                .with_context_bound(TypeName::primitive("Ordering"))
                .unwrap(),
        )
        .add_param(
            ParameterSpec::new(
                "items",
                TypeName::application(
                    TypeName::primitive("List"),
                    vec![TypeArgument::Single(TypeName::primitive("T"))],
                ),
            )
            .unwrap(),
        )
        .returns(vec![TypeName::application(
            TypeName::primitive("List"),
            vec![TypeArgument::Single(TypeName::primitive("T"))],
        )])
        .body(sort_body)
        .build()
        .unwrap();

    let collect_body = sigil_quote!(Scala {
        val buf = new $T(list_buffer)[String]()
        buf += name
        buf.toList
    })
    .unwrap();

    let collect_fn = FunSpec::builder("collectNames")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(vec![TypeName::application(
            TypeName::primitive("List"),
            vec![TypeArgument::Single(TypeName::primitive("String"))],
        )])
        .body(collect_body)
        .build()
        .unwrap();

    let functor = TypeSpec::builder("Functor", TypeKind::Interface)
        .add_generic_param(
            GenericParamSpec::new(
                "F",
                GenericParamDomain::Single {
                    kind: Some(KindExpr::Constructor {
                        parameters: vec![KindExpr::Type],
                        result: Box::new(KindExpr::Type),
                    }),
                },
            )
            .unwrap(),
        )
        .add_method(
            FunSpec::builder("map")
                .add_generic_param(GenericParamSpec::single("A").unwrap())
                .add_generic_param(GenericParamSpec::single("B").unwrap())
                .add_param(
                    ParameterSpec::new(
                        "fa",
                        TypeName::application(
                            TypeName::primitive("F"),
                            vec![TypeArgument::Single(TypeName::primitive("A"))],
                        ),
                    )
                    .unwrap(),
                )
                .add_param(
                    ParameterSpec::new(
                        "f",
                        TypeName::callable(
                            vec![CallableParam::Single {
                                name: None,
                                type_name: TypeName::primitive("A"),
                                presence: CallableParamPresence::Required,
                            }],
                            vec![TypeName::primitive("B")],
                        ),
                    )
                    .unwrap(),
                )
                .returns(vec![TypeName::application(
                    TypeName::primitive("F"),
                    vec![TypeArgument::Single(TypeName::primitive("B"))],
                )])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    FileSpec::builder_with("User.scala", Scala::new())
        .add_type(repo)
        .add_type(user)
        .add_type(functor)
        .add_function(sort_fn)
        .add_function(collect_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
