use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Simple function with imports ===

#[test]
fn test_go_function_with_imports() {
    let http_server = TypeName::<GoLang>::importable("net/http", "Server");
    let json_marshal = TypeName::<GoLang>::importable("encoding/json", "Marshal");

    let mut b = CodeBlock::<GoLang>::builder();
    b.add("func startServer() {", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("srv := &%T{}", (http_server,));
    b.add_statement("data, _ := %T(srv)", (json_marshal,));
    b.add_statement("_ = data", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("server.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/function_with_imports.go", &output);
}

// === Import grouping: stdlib vs external ===

#[test]
fn test_go_import_grouping() {
    let fmt_println = TypeName::<GoLang>::importable("fmt", "Println");
    let http_server = TypeName::<GoLang>::importable("net/http", "Server");
    let gin_ctx = TypeName::<GoLang>::importable("github.com/gin-gonic/gin", "Context");

    let mut b = CodeBlock::<GoLang>::builder();
    b.add_statement("%T(\"hello\")", (fmt_println,));
    b.add_statement("_ = %T{}", (http_server,));
    b.add_statement("_ = %T{}", (gin_ctx,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("main.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/import_grouping.go", &output);
}

// === Control flow (no semicolons) ===

#[test]
fn test_go_control_flow() {
    let mut b = CodeBlock::<GoLang>::builder();
    b.add("func classify(x int) string {", ());
    b.add_line();
    b.add("%>", ());
    b.begin_control_flow("if x > 0", ());
    b.add_statement("return \"positive\"", ());
    b.end_control_flow();
    b.begin_control_flow("if x < 0", ());
    b.add_statement("return \"negative\"", ());
    b.end_control_flow();
    b.add_statement("return \"zero\"", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("classify.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/control_flow.go", &output);
}

// === Multiple symbols from same package ===

#[test]
fn test_go_same_package_symbols() {
    let handler = TypeName::<GoLang>::importable("net/http", "Handler");
    let server = TypeName::<GoLang>::importable("net/http", "Server");
    let listen = TypeName::<GoLang>::importable("net/http", "ListenAndServe");

    let mut b = CodeBlock::<GoLang>::builder();
    b.add_statement("var _ %T", (handler,));
    b.add_statement("var _ %T", (server,));
    b.add_statement("_ = %T", (listen,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("http.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/same_package_symbols.go", &output);
}
