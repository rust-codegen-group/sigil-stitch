//! Generate a Go file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: struct with tags, embedded types (composition), interface,
//! `[]T` slices, `map[K]V` maps, receiver methods, Go generics,
//! `$T_join` for interface composition, and `const ( ... )` paren blocks
//! with `$for`.
//!
//! Run: `cargo run --example go_codegen`

use sigil_stitch::lang::go_lang::GoLang;
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
    // --- Interface ---
    let logger = TypeSpec::builder("Logger", TypeKind::Interface)
        .doc("Logger defines a structured logging interface.")
        .add_method(
            FunSpec::builder("Info")
                .add_param(ParameterSpec::new("msg", TypeName::primitive("string")).unwrap())
                .add_param(
                    ParameterSpec::new(
                        "fields",
                        TypeName::map(TypeName::primitive("string"), TypeName::primitive("any")),
                    )
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Error")
                .add_param(ParameterSpec::new("msg", TypeName::primitive("string")).unwrap())
                .add_param(ParameterSpec::new("err", TypeName::primitive("error")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Struct with embedded type, tags, slices, maps ---
    let server = TypeSpec::builder("Server", TypeKind::Struct)
        .doc("Server is an HTTP server with middleware support.")
        .add_embedded(TypeName::primitive("Logger"))
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("string"))
                .tag("json:\"host\" yaml:\"host\"")
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .tag("json:\"port\" yaml:\"port\"")
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "routes",
                TypeName::map(
                    TypeName::primitive("string"),
                    TypeName::primitive("http.HandlerFunc"),
                ),
            )
            .build()
            .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "middleware",
                TypeName::slice(TypeName::primitive("Middleware")),
            )
            .build()
            .unwrap(),
        )
        .build()
        .unwrap();

    (logger, server)
}

fn builder_approach() -> String {
    let http_listen = TypeName::importable("net/http", "ListenAndServe");
    let fmt_sprintf = TypeName::importable("fmt", "Sprintf");
    let sort_slice = TypeName::importable("sort", "Slice");
    let (logger, server) = build_shared_types();

    // --- Receiver method: Start ---
    let mut start_body = CodeBlock::builder();
    start_body.add_statement("addr := %T(\"%%s:%%d\", s.host, s.port)", (fmt_sprintf,));
    start_body.add_statement("return %T(addr, nil)", (http_listen,));

    let start_fn = FunSpec::builder("Start")
        .doc("Start begins listening on the configured address.")
        .receiver(
            ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap(),
        )
        .returns(TypeName::primitive("error"))
        .body(start_body.build().unwrap())
        .build()
        .unwrap();

    // --- Receiver method: AddRoute ---
    let mut add_body = CodeBlock::builder();
    add_body.add_statement("s.routes[path] = handler", ());

    let add_route = FunSpec::builder("AddRoute")
        .receiver(
            ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap(),
        )
        .add_param(ParameterSpec::new("path", TypeName::primitive("string")).unwrap())
        .add_param(ParameterSpec::new("handler", TypeName::primitive("http.HandlerFunc")).unwrap())
        .body(add_body.build().unwrap())
        .build()
        .unwrap();

    // --- Generic function ---
    let mut sort_body = CodeBlock::builder();
    sort_body.add_statement(
        "%T(items, func(i, j int) bool { return items[i] < items[j] })",
        (sort_slice,),
    );
    sort_body.add_statement("return items", ());

    let sort_fn = FunSpec::builder("SortSlice")
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("~int | ~string")))
        .add_param(ParameterSpec::new("items", TypeName::slice(TypeName::primitive("T"))).unwrap())
        .returns(TypeName::slice(TypeName::primitive("T")))
        .body(sort_body.build().unwrap())
        .build()
        .unwrap();

    // --- $T_join comparison: manual interface embedding ---
    let reader = TypeName::importable("io", "Reader");
    let writer = TypeName::importable("io", "Writer");
    let closer = TypeName::importable("io", "Closer");
    let mut join_body = CodeBlock::builder();
    join_body.add("type FileOps interface {", ());
    join_body.add_line();
    join_body.add("    %T", (reader,));
    join_body.add_line();
    join_body.add("    %T", (writer,));
    join_body.add_line();
    join_body.add("    %T", (closer,));
    join_body.add_line();
    join_body.add("}", ());

    // --- const paren block comparison: manual enum generation ---
    let mut enum_body = CodeBlock::builder();
    let variants = ["Alpha", "Beta", "Gamma"];
    enum_body.add("const (", ());
    enum_body.add_line();
    enum_body.add("%>", ());
    for v in &variants {
        enum_body.add(
            "%L %L = %S",
            (
                *v,
                format!("{v}Kind").as_str(),
                StringLitArg(v.to_lowercase()),
            ),
        );
        enum_body.add_line();
    }
    enum_body.add("%<", ());
    enum_body.add(")", ());

    FileSpec::builder_with("server.go", GoLang::new())
        .header(CodeBlock::of("package server", ()).unwrap())
        .add_type(logger)
        .add_type(server)
        .add_function(start_fn)
        .add_function(add_route)
        .add_function(sort_fn)
        .add_code(join_body.build().unwrap())
        .add_code(enum_body.build().unwrap())
        .build()
        .unwrap()
        .render(100)
        .unwrap()
}

fn macro_approach() -> String {
    let http_listen = TypeName::importable("net/http", "ListenAndServe");
    let fmt_sprintf = TypeName::importable("fmt", "Sprintf");
    let sort_slice = TypeName::importable("sort", "Slice");
    let (logger, server) = build_shared_types();

    let start_body = sigil_quote!(GoLang {
        addr := $T(fmt_sprintf)("%s:%d", s.host, s.port)
        return $T(http_listen)(addr, nil)
    })
    .unwrap();

    let start_fn = FunSpec::builder("Start")
        .doc("Start begins listening on the configured address.")
        .receiver(
            ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap(),
        )
        .returns(TypeName::primitive("error"))
        .body(start_body)
        .build()
        .unwrap();

    let add_body = sigil_quote!(GoLang {
        s.routes[path] = handler
    })
    .unwrap();

    let add_route = FunSpec::builder("AddRoute")
        .receiver(
            ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap(),
        )
        .add_param(ParameterSpec::new("path", TypeName::primitive("string")).unwrap())
        .add_param(ParameterSpec::new("handler", TypeName::primitive("http.HandlerFunc")).unwrap())
        .body(add_body)
        .build()
        .unwrap();

    let sort_body = sigil_quote!(GoLang {
        $T(sort_slice)(items, func(i, j int) bool {
            return items[i] < items[j]
        })
        return items
    })
    .unwrap();

    let sort_fn = FunSpec::builder("SortSlice")
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("~int | ~string")))
        .add_param(ParameterSpec::new("items", TypeName::slice(TypeName::primitive("T"))).unwrap())
        .returns(TypeName::slice(TypeName::primitive("T")))
        .body(sort_body)
        .build()
        .unwrap();

    // --- $T_join: interface composition with import tracking ---
    let ifaces = vec![
        TypeName::importable("io", "Reader"),
        TypeName::importable("io", "Writer"),
        TypeName::importable("io", "Closer"),
    ];
    let join_body = sigil_quote!(GoLang {
        type FileOps interface {
            $T_join("\n", &ifaces)
        }
    })
    .unwrap();

    // --- const paren block: generate enum-like constants with $for ---
    let variants = ["Alpha", "Beta", "Gamma"];
    let enum_body = sigil_quote!(GoLang {
        const (
        $for(v in &variants) {
            $L("@{v} @{v}Kind = \"@{v}\"")
        }
        )
    })
    .unwrap();

    FileSpec::builder_with("server.go", GoLang::new())
        .header(CodeBlock::of("package server", ()).unwrap())
        .add_type(logger)
        .add_type(server)
        .add_function(start_fn)
        .add_function(add_route)
        .add_function(sort_fn)
        .add_code(join_body)
        .add_code(enum_body)
        .build()
        .unwrap()
        .render(100)
        .unwrap()
}
