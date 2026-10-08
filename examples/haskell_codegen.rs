//! Generate a Haskell file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: data type with record syntax and deriving, newtype wrapper,
//! type class with `where` block, function with split type signature, and
//! type class constraints.
//!
//! Run: `cargo run --example haskell_codegen`

use sigil_stitch::lang::haskell::Haskell;
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
    // --- Data type with record syntax and deriving ---
    let person = TypeSpec::builder("Person", TypeKind::Struct)
        .doc("A person record.")
        .add_field(
            FieldSpec::builder("personName", TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("personAge", TypeName::primitive("Int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "personEmail",
                TypeName::optional(TypeName::primitive("String")),
            )
            .build()
            .unwrap(),
        )
        .implements(TypeName::primitive("Show"))
        .implements(TypeName::primitive("Eq"))
        .build()
        .unwrap();

    // --- Newtype ---
    let user_id = TypeSpec::builder("UserId", TypeKind::Newtype)
        .extends(TypeName::primitive("Int"))
        .implements(TypeName::primitive("Show"))
        .implements(TypeName::primitive("Eq"))
        .implements(TypeName::primitive("Ord"))
        .build()
        .unwrap();

    (person, user_id)
}

fn builder_approach() -> String {
    let map = TypeName::importable("Data.Map", "Map");
    let (person, user_id) = build_shared_types();
    let comment_label = "NOTE";
    let comment_reason = "Generate greeting output";
    let comment_note = "format greeting";
    let v_interp = "World";

    // --- Type alias ---
    let user_map = TypeSpec::builder("UserMap", TypeKind::TypeAlias)
        .extends(TypeName::application(
            map,
            vec![
                TypeArgument::Single(TypeName::primitive("UserId")),
                TypeArgument::Single(TypeName::primitive("Person")),
            ],
        ))
        .build()
        .unwrap();

    // --- Function with type annotation (split signature) ---
    let mut greet_body = CodeBlock::builder();
    greet_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    greet_body.add(
        "\"Hello, \" ++ personName p ++ \" from %V!\" %R",
        (
            VerbatimStrArg(v_interp.to_string()),
            CommentArg(comment_note.to_string()),
        ),
    );

    let greet_fn = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("p", TypeName::primitive("Person")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(greet_body.build().unwrap())
        .build()
        .unwrap();

    // --- Function with type class constraint ---
    let mut show_body = CodeBlock::builder();
    show_body.add("putStrLn (show x)", ());

    let print_fn = FunSpec::builder("printItem")
        .add_generic_param(
            GenericParamSpec::single("a")
                .unwrap()
                .with_bound(TypeName::primitive("Show"))
                .unwrap(),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("IO ()"))
        .body(show_body.build().unwrap())
        .build()
        .unwrap();

    FileSpec::builder_with("Person.hs", Haskell::new())
        .add_type(person)
        .add_type(user_id)
        .add_type(user_map)
        .add_function(greet_fn)
        .add_function(print_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let map = TypeName::importable("Data.Map", "Map");
    let (person, user_id) = build_shared_types();
    let comment_label = "NOTE";
    let comment_reason = "Generate greeting output";
    let comment_note = "format greeting";
    let v_interp = "World";

    let user_map = TypeSpec::builder("UserMap", TypeKind::TypeAlias)
        .extends(TypeName::application(
            map,
            vec![
                TypeArgument::Single(TypeName::primitive("UserId")),
                TypeArgument::Single(TypeName::primitive("Person")),
            ],
        ))
        .build()
        .unwrap();

    let greet_body = sigil_quote!(Haskell {
        $comment("@{comment_label}: @{comment_reason}");
        $V("\"Hello, \" ++ personName p ++ \" from @{v_interp}!\"") $comment(comment_note)
    })
    .unwrap();

    let greet_fn = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("p", TypeName::primitive("Person")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(greet_body)
        .build()
        .unwrap();

    let print_body = sigil_quote!(Haskell {
        putStrLn(show x)
    })
    .unwrap();

    let print_fn = FunSpec::builder("printItem")
        .add_generic_param(
            GenericParamSpec::single("a")
                .unwrap()
                .with_bound(TypeName::primitive("Show"))
                .unwrap(),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("IO ()"))
        .body(print_body)
        .build()
        .unwrap();

    FileSpec::builder_with("Person.hs", Haskell::new())
        .add_type(person)
        .add_type(user_id)
        .add_type(user_map)
        .add_function(greet_fn)
        .add_function(print_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
