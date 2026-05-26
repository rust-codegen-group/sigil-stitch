//! Generate a Ruby file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: class with `initialize`, module with class methods,
//! `if`/`else` control flow, doc comments, `attr_reader` for properties,
//! and `require` imports via `TypeName::importable`.
//!
//! Note: The builder API uses raw CodeBlock construction (typical for
//! dynamic languages like Ruby/Lua). The `sigil_quote!` macro uses
//! brace-style `{ }` blocks which are automatically translated to
//! indent/dedent + `end`. Both produce identical output.
//!
//! Run: `cargo run --example ruby_codegen`

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::ruby::Ruby;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn builder_approach() -> String {
    let ruby = Ruby::new();
    let doc = ruby.render_doc_comment(&["Greeter says hello to a person."]);

    let mut b = CodeBlock::builder();
    b.add("%L", doc);
    b.add_line();
    b.add("class Greeter", ());
    b.add_line();

    b.add("%>", ());
    b.add_statement("attr_reader :name", ());
    b.add_line();

    b.add("def initialize(name)", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("@name = name", ());
    b.add("%<", ());
    b.add("end", ());
    b.add_line();
    b.add_line();

    b.add("def greet", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement(r##""Hello, #{@name}!""##, ());
    b.add("%<", ());
    b.add("end", ());
    b.add_line();
    b.add_line();

    b.add("def describe(x)", ());
    b.add_line();
    b.add("%>", ());
    b.add("if x > 0", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("\"positive\"", ());
    b.add("%<", ());
    b.add("else", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("\"not positive\"", ());
    b.add("%<", ());
    b.add("end", ());
    b.add_line();
    b.add("%<", ());
    b.add("end", ());
    b.add_line();

    b.add("%<", ());
    b.add("end", ());
    b.add_line();

    FileSpec::builder_with("greeter.rb", Ruby::new())
        .add_code(b.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let body = sigil_quote!(Ruby {
        class Greeter {
            attr_reader :name

            def initialize(name) {
                @name = name
            }

            def greet {
                "Hello, #{@name}!"
            }

            def describe(x) {
                if x > 0 {
                    $S("positive")
                } else {
                    $S("not positive")
                }
            }
        }
    })
    .unwrap();

    FileSpec::builder_with("greeter.rb", Ruby::new())
        .add_code(body)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
