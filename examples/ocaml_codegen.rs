//! Generate an OCaml file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: record types, type aliases, curried functions,
//! `module` / `module type` blocks, `open` imports, and postfix generics.
//!
//! Run: `cargo run --example ocaml_codegen`

use sigil_stitch::lang::ocaml::OCaml;
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
    // --- Record type (person) ---
    let person = TypeSpec::builder("person", TypeKind::Struct)
        .doc("A person record.")
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("age", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("email", TypeName::optional(TypeName::primitive("string")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Type alias: type string_list = string list ---
    let string_list = TypeSpec::builder("string_list", TypeKind::TypeAlias)
        .extends(TypeName::application(
            TypeName::primitive("list"),
            vec![TypeArgument::Single(TypeName::primitive("string"))],
        ))
        .build()
        .unwrap();

    (person, string_list)
}

fn builder_approach() -> String {
    let (person, string_list) = build_shared_types();
    let comment_reason = "Initialize module state";
    let comment_label = "NOTE";
    let v_interp = "module";
    let comment_note = "validate expression";

    // --- Function with curried params: greet ---
    let mut greet_body = CodeBlock::builder();
    greet_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    greet_body.add("let mod_name = %V", (VerbatimStrArg(v_interp.to_string()),));
    greet_body.add_line();
    greet_body.add(
        "\"Hello, \" ^ p.name ^ \"!\" %R",
        (CommentArg(comment_note.to_string()),),
    );

    let greet_fn = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("p", TypeName::primitive("person")).unwrap())
        .returns(vec![TypeName::primitive("string")])
        .body(greet_body.build().unwrap())
        .build()
        .unwrap();

    // --- Generic function: map_pair ---
    let list_map = TypeName::importable("List", "map");

    let mut map_pair_body = CodeBlock::builder();
    map_pair_body.add("%T f [x; x]", (list_map,));

    let map_pair_fn = FunSpec::builder("map_pair")
        .add_generic_param(GenericParamSpec::single("'a").unwrap())
        .add_generic_param(GenericParamSpec::single("'b").unwrap())
        .add_param(
            ParameterSpec::new(
                "f",
                TypeName::callable(
                    vec![CallableParam::Single {
                        name: None,
                        type_name: TypeName::primitive("'a"),
                        presence: CallableParamPresence::Required,
                    }],
                    vec![TypeName::primitive("'b")],
                ),
            )
            .unwrap(),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("'a")).unwrap())
        .returns(vec![TypeName::application(
            TypeName::primitive("list"),
            vec![TypeArgument::Single(TypeName::primitive("'b"))],
        )])
        .body(map_pair_body.build().unwrap())
        .build()
        .unwrap();

    // --- Module block ---
    let mut module_body = CodeBlock::builder();
    module_body.add("let capitalize s = String.capitalize_ascii s", ());
    module_body.add_line();

    let module_code = OCaml::module_block("StringUtils", module_body.build().unwrap()).unwrap();

    // --- Module signature ---
    let mut sig_body = CodeBlock::builder();
    sig_body.add("val capitalize : string -> string", ());
    sig_body.add_line();

    let sig_code = OCaml::module_sig_block("STRING_UTILS", sig_body.build().unwrap()).unwrap();

    FileSpec::builder_with("person.ml", OCaml::new())
        .add_type(person)
        .add_type(string_list)
        .add_function(greet_fn)
        .add_function(map_pair_fn)
        .add_code(module_code)
        .add_code(sig_code)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let (person, string_list) = build_shared_types();
    let comment_reason = "Initialize module state";
    let comment_label = "NOTE";
    let v_interp = "module";
    let comment_note = "validate expression";

    // --- Function with curried params: greet (using sigil_quote!) ---
    let greet_body = sigil_quote!(OCaml {
        $comment("@{comment_label}: @{comment_reason}");
        let mod_name = $V("@{v_interp}")
        "Hello, " ^ p.name ^ "!" $comment(comment_note)
    })
    .unwrap();

    let greet_fn = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("p", TypeName::primitive("person")).unwrap())
        .returns(vec![TypeName::primitive("string")])
        .body(greet_body)
        .build()
        .unwrap();

    // --- Generic function: map_pair (using sigil_quote!) ---
    let list_map = TypeName::importable("List", "map");

    let map_pair_body = sigil_quote!(OCaml {
        $T(list_map) f [x; x]
    })
    .unwrap();

    let map_pair_fn = FunSpec::builder("map_pair")
        .add_generic_param(GenericParamSpec::single("'a").unwrap())
        .add_generic_param(GenericParamSpec::single("'b").unwrap())
        .add_param(
            ParameterSpec::new(
                "f",
                TypeName::callable(
                    vec![CallableParam::Single {
                        name: None,
                        type_name: TypeName::primitive("'a"),
                        presence: CallableParamPresence::Required,
                    }],
                    vec![TypeName::primitive("'b")],
                ),
            )
            .unwrap(),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("'a")).unwrap())
        .returns(vec![TypeName::application(
            TypeName::primitive("list"),
            vec![TypeArgument::Single(TypeName::primitive("'b"))],
        )])
        .body(map_pair_body)
        .build()
        .unwrap();

    // --- Module block (must use OCaml::module_block) ---
    let mut module_body = CodeBlock::builder();
    module_body.add("let capitalize s = String.capitalize_ascii s", ());
    module_body.add_line();

    let module_code = OCaml::module_block("StringUtils", module_body.build().unwrap()).unwrap();

    // --- Module signature (must use OCaml::module_sig_block) ---
    let mut sig_body = CodeBlock::builder();
    sig_body.add("val capitalize : string -> string", ());
    sig_body.add_line();

    let sig_code = OCaml::module_sig_block("STRING_UTILS", sig_body.build().unwrap()).unwrap();

    FileSpec::builder_with("person.ml", OCaml::new())
        .add_type(person)
        .add_type(string_list)
        .add_function(greet_fn)
        .add_function(map_pair_fn)
        .add_code(module_code)
        .add_code(sig_code)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
