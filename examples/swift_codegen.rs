//! Generate a Swift file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: struct, protocol with associated type, enum with associated
//! values, static methods, optional types (`T?`), array types (`[T]`),
//! async functions, and `$T_join` for protocol composition.
//!
//! Run: `cargo run --example swift_codegen`

use sigil_stitch::lang::swift::Swift;
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
    // --- Enum with associated values ---
    let result_enum = TypeSpec::builder("NetworkResult", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(
            EnumVariantSpec::builder("success")
                .associated_type(TypeName::primitive("Data"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("failure")
                .associated_type(TypeName::primitive("Error"))
                .build()
                .unwrap(),
        )
        .add_variant(EnumVariantSpec::builder("loading").build().unwrap())
        .build()
        .unwrap();

    // --- Protocol ---
    let proto = TypeSpec::builder("Repository", TypeKind::Interface)
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
            FunSpec::builder("findAll")
                .returns(TypeName::array(TypeName::primitive("T")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    (result_enum, proto)
}

fn builder_approach() -> String {
    let url = TypeName::importable("Foundation", "URL");
    let url_session = TypeName::importable("Foundation", "URLSession");
    let (result_enum, proto) = build_shared_types();

    // --- Struct ---
    let task = TypeSpec::builder("Task", TypeKind::Struct)
        .visibility(Visibility::Public)
        .doc("A task entity.")
        .add_field(
            FieldSpec::builder("id", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("tags", TypeName::array(TypeName::primitive("String")))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("[]", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "assignee",
                TypeName::optional(TypeName::primitive("String")),
            )
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        )
        .build()
        .unwrap();

    // --- Static factory method ---
    let mut factory_body = CodeBlock::builder();
    factory_body.add_statement(
        "guard let url = %T(string: urlString) else { return nil }",
        (url,),
    );
    factory_body.add_statement("return url", ());

    let make_url = FunSpec::builder("makeURL")
        .visibility(Visibility::Public)
        .is_static()
        .returns(TypeName::optional(TypeName::primitive("URL")))
        .add_param(ParameterSpec::new("urlString", TypeName::primitive("String")).unwrap())
        .body(factory_body.build().unwrap())
        .build()
        .unwrap();

    // --- Async function ---
    let mut fetch_body = CodeBlock::builder();
    fetch_body.add_statement(
        "let (data, _) = try await %T.shared.data(from: url)",
        (url_session,),
    );
    fetch_body.add_statement("return data", ());

    let fetch_fn = FunSpec::builder("fetchData")
        .visibility(Visibility::Public)
        .is_async()
        .add_param(ParameterSpec::new("url", TypeName::primitive("URL")).unwrap())
        .returns(TypeName::primitive("Data"))
        .body(fetch_body.build().unwrap())
        .build()
        .unwrap();

    // --- $T_join comparison: manual protocol composition ---
    let codable = TypeName::importable("Foundation", "Codable");
    let hashable = TypeName::importable("Foundation", "Hashable");
    let mut join_body = CodeBlock::builder();
    join_body.add("typealias Model = ", ());
    join_body.add("%T", (codable,));
    join_body.add(" & ", ());
    join_body.add("%T", (hashable,));

    FileSpec::builder_with("Task.swift", Swift::new())
        .add_type(result_enum)
        .add_type(proto)
        .add_type(task)
        .add_function(make_url)
        .add_function(fetch_fn)
        .add_code(join_body.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let url = TypeName::importable("Foundation", "URL");
    let url_session = TypeName::importable("Foundation", "URLSession");
    let (result_enum, proto) = build_shared_types();

    let task = TypeSpec::builder("Task", TypeKind::Struct)
        .visibility(Visibility::Public)
        .doc("A task entity.")
        .add_field(
            FieldSpec::builder("id", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("tags", TypeName::array(TypeName::primitive("String")))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("[]", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "assignee",
                TypeName::optional(TypeName::primitive("String")),
            )
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        )
        .build()
        .unwrap();

    let factory_body = sigil_quote!(Swift {
        guard let url = $T(url)(string: urlString) else {
            return nil;
        }
        return url;
    })
    .unwrap();

    let make_url = FunSpec::builder("makeURL")
        .visibility(Visibility::Public)
        .is_static()
        .returns(TypeName::optional(TypeName::primitive("URL")))
        .add_param(ParameterSpec::new("urlString", TypeName::primitive("String")).unwrap())
        .body(factory_body)
        .build()
        .unwrap();

    let fetch_body = sigil_quote!(Swift {
        let(data, _) = try await $T(url_session).shared.data(from: url);
        return data;
    })
    .unwrap();

    let fetch_fn = FunSpec::builder("fetchData")
        .visibility(Visibility::Public)
        .is_async()
        .add_param(ParameterSpec::new("url", TypeName::primitive("URL")).unwrap())
        .returns(TypeName::primitive("Data"))
        .body(fetch_body)
        .build()
        .unwrap();

    // --- $T_join: protocol composition with import tracking ---
    let protocols = vec![
        TypeName::importable("Foundation", "Codable"),
        TypeName::importable("Foundation", "Hashable"),
    ];
    let join_body = sigil_quote!(Swift {
        typealias Model = $T_join(" & ", &protocols)
    })
    .unwrap();

    FileSpec::builder_with("Task.swift", Swift::new())
        .add_type(result_enum)
        .add_type(proto)
        .add_type(task)
        .add_function(make_url)
        .add_function(fetch_fn)
        .add_code(join_body)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
