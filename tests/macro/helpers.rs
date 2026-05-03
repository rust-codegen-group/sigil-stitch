pub use sigil_stitch::code_block::{CodeBlock, NameArg, StringLitArg};
pub use sigil_stitch::import_collector;
pub use sigil_stitch::lang::haskell::Haskell;
pub use sigil_stitch::lang::java_lang::JavaLang;
pub use sigil_stitch::lang::kotlin::Kotlin;
pub use sigil_stitch::lang::ocaml::OCaml;
pub use sigil_stitch::prelude::*;
pub use sigil_stitch::spec::file_spec::FileSpec;
pub use sigil_stitch::type_name::TypeName;

pub fn render_ts(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.ts")
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
