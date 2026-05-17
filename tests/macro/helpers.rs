pub use sigil_stitch::code_block::{CodeBlock, NameArg, StringLitArg};
pub use sigil_stitch::import_collector;
pub use sigil_stitch::lang::c_lang::CLang;
pub use sigil_stitch::lang::cpp_lang::CppLang;
pub use sigil_stitch::lang::csharp::CSharp;
pub use sigil_stitch::lang::dart::DartLang;
pub use sigil_stitch::lang::haskell::Haskell;
pub use sigil_stitch::lang::java_lang::JavaLang;
pub use sigil_stitch::lang::kotlin::Kotlin;
pub use sigil_stitch::lang::lua::Lua;
pub use sigil_stitch::lang::ocaml::OCaml;
pub use sigil_stitch::lang::scala::Scala;
pub use sigil_stitch::lang::swift::Swift;
pub use sigil_stitch::prelude::*;
pub use sigil_stitch::spec::file_spec::FileSpec;
pub use sigil_stitch::type_name::TypeName;

pub fn render_js(block: &CodeBlock) -> String {
    use sigil_stitch::lang::javascript::JavaScript;
    let file = FileSpec::builder_with("test.js", JavaScript::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_ts(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.ts")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_c(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.c", CLang::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_rs(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.rs")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_py(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.py")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_go(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.go")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_hs(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.hs", Haskell::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_ml(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.ml", OCaml::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_java(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("Test.java", JavaLang::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_kt(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.kt", Kotlin::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_cpp(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.cpp", CppLang::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_cs(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("Test.cs", CSharp::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_lua(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.lua", Lua::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_swift(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.swift", Swift::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_dart(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.dart", DartLang::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

pub fn render_scala(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.scala", Scala::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}
