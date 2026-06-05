//! Demonstrate inline `$for` and `$if` inside expressions across multiple
//! languages — array/dict literals, function arguments, object literals.
//!
//! Inline meta-directives work anywhere in a template, not just at column 0.
//! `$for`/`$if` are meta-selection: true/false branches and loop bodies are
//! evaluated at Rust compile time by the macro, then spliced into place.
//! Each language gets correct delimiters: TypeScript keeps `{ }` for objects,
//! Python keeps `{ }` for dicts (no stray `:`).
//!
//! Run: `cargo run --example inline_control_flow`

use sigil_stitch::lang::bash::Bash;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::lang::ruby::Ruby;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn main() {
    typescript_inline_for_array();
    typescript_inline_if_function_arg();
    typescript_inline_if_else_chain();
    typescript_inline_if_in_object_literal();
    python_inline_for_list();
    python_inline_if_dict();
    cpp_inline_for_vector();
    ruby_inline_if_expression();
    bash_inline_for_join();
    inline_for_with_type_imports();
    empty_for_clean_output();
}

// ── TypeScript ──

/// `$for` inside a TypeScript array literal — items spliced inline.
fn typescript_inline_for_array() {
    println!("--- TS: Inline $for in array ---\n");
    let items = vec!["hostname", "platform", "arch"];

    let block = sigil_quote!(TypeScript {
        const defaultKeys = [$for(item in &items) { $S(*item), }];
    })
    .unwrap();
    println!("{}\n", render_ts(&block));
}

/// `$if`/`$else` inline inside a function call argument.
fn typescript_inline_if_function_arg() {
    println!("--- TS: Inline $if/$else in function argument ---\n");
    let is_admin = true;

    let block = sigil_quote!(TypeScript {
        setPermissions($if(is_admin) { "read-write" } $else { "read-only" });
    })
    .unwrap();
    println!("{}\n", render_ts(&block));
}

/// `$if` / `$else_if` / `$else` chain — all inline in one expression.
fn typescript_inline_if_else_chain() {
    println!("--- TS: Inline $if/$else_if/$else chain ---\n");
    let level: u32 = 2;

    let block = sigil_quote!(TypeScript {
        const label = $if(level == 0) { "trace" } $else_if(level == 1) { "debug" } $else { "info" };
    })
    .unwrap();
    println!("{}\n", render_ts(&block));
}

/// `$if`/`$else` inside an object literal value position — braces stay literal.
fn typescript_inline_if_in_object_literal() {
    println!("--- TS: Inline $if/$else in object literal ---\n");
    let production = true;

    let block = sigil_quote!(TypeScript {
        const config = {
            mode: $if(production) { "production" } $else { "development" },
            debug: $if(production) { false } $else { true },
        };
    })
    .unwrap();
    println!("{}\n", render_ts(&block));
}

// ── Python ──

/// `$for` inside a Python list — no stray `:` from block delimiters.
fn python_inline_for_list() {
    println!("--- Python: Inline $for in list ---\n");
    let fields = vec!["name", "age", "email"];

    let block = sigil_quote!(Python {
        required_fields = [$for(f in &fields) { $S(*f), }]
    })
    .unwrap();
    println!("{}\n", render_py(&block));
}

/// `$if`/`$else` inside a Python dict literal — braces stay literal, no stray `:`.
fn python_inline_if_dict() {
    println!("--- Python: Inline $if/$else in dict literal ---\n");
    let is_prod = true;

    let block = sigil_quote!(Python {
        config = {
            "mode": $if(is_prod) { $S("production") } $else { $S("development") },
            "debug": $if(is_prod) { False } $else { True },
        }
    })
    .unwrap();
    println!("{}\n", render_py(&block));
}

// ── C++ ──

/// `$for` inside a C++ initializer list — `{}` stays literal.
fn cpp_inline_for_vector() {
    println!("--- C++: Inline $for in initializer list ---\n");
    let values = vec!["1", "2", "3"];

    let block = sigil_quote!(Cpp {
        std::vector<int> vec = { $for(v in &values) { $L(*v), } };
    })
    .unwrap();
    println!("{}\n", render_with(&block, &Cpp::new()));
}

// ── Ruby ──

/// `$if`/`$else` as a value expression in Ruby — selects the branch output.
fn ruby_inline_if_expression() {
    println!("--- Ruby: Inline $if/$else meta-selection ---\n");
    let flag = true;

    let block = sigil_quote!(Ruby {
        result = $if(flag) { "yes" } $else { "no" }
    })
    .unwrap();
    println!("{}\n", render_rb(&block));
}

// ── Bash ──

/// `$join` in a Bash command — builds a single space-separated argument string.
fn bash_inline_for_join() {
    println!("--- Bash: $for with $join for shell args ---\n");
    let flags = vec!["--verbose", "--no-cache", "--force"];

    // $join produces a single line; $for would produce multiline output.
    let block = sigil_quote!(Bash {
        mycommand $join(" ", &flags) target
    })
    .unwrap();
    println!("{}\n", render_bash(&block));
}

// ── Import tracking inside inline $for ──

/// `$T` inside inline `$for` still tracks and propagates imports.
fn inline_for_with_type_imports() {
    println!("--- Inline $for with $T import tracking ---\n");
    let types = vec![
        TypeName::importable_type("./models", "User"),
        TypeName::importable_type("./models", "Admin"),
    ];

    let block = sigil_quote!(TypeScript {
        const types = [$for(ty in &types) { $T(ty.clone()), }];
    })
    .unwrap();
    println!("{}\n", render_ts(&block));
}

// ── Empty / edge cases ──

/// Empty `$for` produces no output — array literal renders as `[]`.
fn empty_for_clean_output() {
    println!("--- Empty $for produces clean output ---\n");
    let items: Vec<&str> = vec![];

    let block = sigil_quote!(TypeScript {
        const keys: string[] = [$for(item in &items) { $S(*item), }];
    })
    .unwrap();
    println!("  TS:  {}", render_ts(&block));

    let block = sigil_quote!(Python {
        keys = [$for(item in &items) { $S(*item), }]
    })
    .unwrap();
    println!("  Py:  {}", render_py(&block));
    println!();
}

// ── helpers ──

fn render_ts(block: &CodeBlock) -> String {
    FileSpec::builder("demo.ts")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(60)
        .unwrap()
}

fn render_py(block: &CodeBlock) -> String {
    FileSpec::builder("demo.py")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(60)
        .unwrap()
}

fn render_rb(block: &CodeBlock) -> String {
    FileSpec::builder_with("demo.rb", Ruby::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(60)
        .unwrap()
}

fn render_bash(block: &CodeBlock) -> String {
    FileSpec::builder_with("demo.sh", Bash::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(60)
        .unwrap()
}

fn render_with<L: sigil_stitch::lang::CodeLang + Clone>(block: &CodeBlock, lang: &L) -> String {
    FileSpec::builder_with("demo", lang.clone())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(60)
        .unwrap()
}
