use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go::Go;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_imports() {
    let http_server = TypeName::importable("net/http", "Server");
    let json_marshal = TypeName::importable("encoding/json", "Marshal");

    let mut b = CodeBlock::builder();
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

    let file = FileSpec::builder_with("server.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/function_with_imports.go", &output);
}

#[test]
fn test_import_grouping() {
    let fmt_println = TypeName::importable("fmt", "Println");
    let http_server = TypeName::importable("net/http", "Server");
    let gin_ctx = TypeName::importable("github.com/gin-gonic/gin", "Context");

    let mut b = CodeBlock::builder();
    b.add_statement("%T(\"hello\")", (fmt_println,));
    b.add_statement("_ = %T{}", (http_server,));
    b.add_statement("_ = %T{}", (gin_ctx,));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("main.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/import_grouping.go", &output);
}

#[test]
fn test_same_package_symbols() {
    let handler = TypeName::importable("net/http", "Handler");
    let server = TypeName::importable("net/http", "Server");
    let listen = TypeName::importable("net/http", "ListenAndServe");

    let mut b = CodeBlock::builder();
    b.add_statement("var _ %T", (handler,));
    b.add_statement("var _ %T", (server,));
    b.add_statement("_ = %T", (listen,));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("http.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/same_package_symbols.go", &output);
}
