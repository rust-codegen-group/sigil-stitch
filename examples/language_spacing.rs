//! Demonstrate language-aware spacing — the same `sigil_quote!` template
//! renders with different spacing rules depending on the target language.
//!
//! The macro tokenizer knows that:
//! - C/C++/C# use `Config*` (postfix pointer) — no space before `*`
//! - C++ uses `auto&` (postfix reference) — no space before `&`
//! - C#/TS/Swift/Kotlin/Dart use `int?` (postfix nullable) — no space before `?`
//! - TS uses `name?: string` (optional property) — `?:` stays tight
//! - Ruby uses `attr_reader :name` (symbol colon) — space before, none after `:`
//! - Ruby uses `class Dog < Animal` — space before `<` (inheritance, not generics)
//! - Bash/Zsh use `NAME=val` — no space around `=` in assignments
//! - Bash/Zsh use `--flag` — tight compound flags
//! - C doesn't treat `<` as generic open (no angle generics)
//!
//! Run: `cargo run --example language_spacing`

use sigil_stitch::lang::bash::Bash;
use sigil_stitch::lang::c::C;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::lang::ruby::Ruby;
use sigil_stitch::lang::zsh::Zsh;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn main() {
    postfix_star_pointers();
    postfix_ampersand_references();
    postfix_question_nullable();
    ts_optional_property();
    shell_assign_adjacent();
    shell_double_dash_flags();
    ruby_symbol_colon();
    ruby_inheritance_angle();
    c_no_angle_generics();
}

/// C/C++/C#: `Config*` keeps `*` tight (postfix pointer).
/// Ruby: `*` is binary multiplication — space before `*` is normal.
fn postfix_star_pointers() {
    println!("=== PostfixStar: pointer types ===\n");
    println!("  Template: `Config* p` (no space before *)");
    println!();

    let block = sigil_quote!(C { Config* p; }).unwrap();
    println!("  C:   {}", render_with(&block, &C::new()));

    let block = sigil_quote!(Cpp { Config* p; }).unwrap();
    println!("  C++: {}", render_with(&block, &Cpp::new()));

    let block = sigil_quote!(CSharp { Config* p; }).unwrap();
    println!("  C#:  {}", render_cs(&block));

    // Ruby: * is binary multiplication, not a postfix pointer
    let block = sigil_quote!(Ruby { Config* x }).unwrap();
    println!(
        "  Ruby: {}  (space before *, not a postfix pointer)",
        render_rb(&block)
    );
    println!();
}

/// C++: `auto&` keeps `&` tight (postfix reference type).
/// C: `&` is address-of — space before `&` is normal.
fn postfix_ampersand_references() {
    println!("=== PostfixAmpersand: C++ references ===\n");
    println!("  Template: `auto& x` (no space before &)");
    println!();

    let block = sigil_quote!(Cpp { auto& x = value; }).unwrap();
    println!("  C++: {}", render_with(&block, &Cpp::new()));

    let block = sigil_quote!(C { auto& x; }).unwrap();
    println!(
        "  C:   {}  (space before &, no postfix refs in C)",
        render_with(&block, &C::new())
    );
    println!();
}

/// C#/TS/Swift/Kotlin/Dart: `int?` keeps `?` tight (postfix nullable).
/// Ruby/Go/PHP: `?` is NOT a nullable type suffix.
fn postfix_question_nullable() {
    println!("=== PostfixQuestion: nullable types ===\n");
    println!("  Template: `int? count` (no space before ?)");
    println!();

    let block = sigil_quote!(CSharp { int? count; }).unwrap();
    println!("  C#:     {}", render_cs(&block));

    let block = sigil_quote!(Swift { var count: Int? = nil }).unwrap();
    println!("  Swift:  {}", render_swift(&block));

    let block = sigil_quote!(Kotlin { var count: Int? = null }).unwrap();
    println!("  Kotlin: {}", render_kotlin(&block));

    let block = sigil_quote!(Dart { int? count; }).unwrap();
    println!("  Dart:   {}", render_dart(&block));

    // Ruby: ? is method suffix/ternary, not nullable
    println!("  Ruby/PHP/Go: ? is NOT a nullable type suffix — space is normal");
    println!();
}

/// TypeScript: `name?: string` keeps `?:` tight (optional property).
/// My fix for compact ternaries correctly excludes `?:`.
fn ts_optional_property() {
    println!("=== TS optional property (`?:`) ===\n");
    println!("  Template: `name?: string`");
    println!();

    let block = sigil_quote!(TypeScript {
        interface Config {
            name?: string;
            debug?: boolean;
        }
    })
    .unwrap();
    println!("  TS: {}", render_ts(&block));
    println!();
}

/// Bash/Zsh: `NAME=val` keeps `=` tight (shell assignment).
fn shell_assign_adjacent() {
    println!("=== AssignAdjacent: shell assignments ===\n");
    println!("  Template: `NAME=value` (no space around =)");
    println!();

    let block = sigil_quote!(Bash { export NAME=value }).unwrap();
    println!("  Bash: {}", render_bash(&block));

    let block = sigil_quote!(Zsh { export NAME=value }).unwrap();
    println!("  Zsh:  {}", render_zsh(&block));
    println!("       (NAME=value tight — shell style)\n");
}

/// Bash/Zsh: `--flag` keeps dashes tight (compound flag).
fn shell_double_dash_flags() {
    println!("=== Double-dash flags: shell ===\n");
    println!("  Template: `git commit --amend --no-edit`");
    println!();

    let block = sigil_quote!(Bash { git commit --amend --no-edit }).unwrap();
    println!("  Bash: {}", render_bash(&block));

    let block = sigil_quote!(Zsh { git commit --amend --no-edit }).unwrap();
    println!("  Zsh:  {}", render_zsh(&block));
    println!("       (--amend --no-edit stay tight)\n");
}

/// Ruby: `attr_reader :name` has space before `:`, none after.
fn ruby_symbol_colon() {
    println!("=== Ruby symbol colon ===\n");
    println!("  Template: `attr_reader :name, :age`");
    println!();

    let block = sigil_quote!(Ruby { attr_reader :name, :age }).unwrap();
    println!("  Ruby: {}", render_rb(&block));
    println!("       (space before :, none after — :name :age)\n");
}

/// Ruby: `class Dog < Animal` has space before `<` (inheritance, not generics).
fn ruby_inheritance_angle() {
    println!("=== Ruby inheritance angle ===\n");

    let block = sigil_quote!(Ruby {
        class Dog < Animal {
        }
    })
    .unwrap();
    println!("  Ruby: {}", render_rb(&block));
    println!("       (class Dog < Animal — space before < preserved)\n");
}

/// C: `<` is a comparison operator, never a generic open.
fn c_no_angle_generics() {
    println!("=== C: no angle generics ===\n");

    let block = sigil_quote!(C {
        if (size < MAX_SIZE) {
            printf("ok");
        }
    })
    .unwrap();
    println!("  C: {}", render_with(&block, &C::new()));
    println!("      (< is always less-than in C, never generic)\n");
}

// ── helpers ──

fn render_with<L: sigil_stitch::lang::CodeLang + Clone>(block: &CodeBlock, lang: &L) -> String {
    FileSpec::builder_with("demo", lang.clone())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_ts(block: &CodeBlock) -> String {
    FileSpec::builder("demo.ts")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_cs(block: &CodeBlock) -> String {
    FileSpec::builder_with("demo.cs", CSharp::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_rb(block: &CodeBlock) -> String {
    FileSpec::builder_with("demo.rb", Ruby::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_bash(block: &CodeBlock) -> String {
    FileSpec::builder_with("demo.sh", Bash::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_zsh(block: &CodeBlock) -> String {
    FileSpec::builder_with("demo.zsh", Zsh::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_swift(block: &CodeBlock) -> String {
    FileSpec::builder("demo.swift")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_kotlin(block: &CodeBlock) -> String {
    FileSpec::builder("demo.kt")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}

fn render_dart(block: &CodeBlock) -> String {
    FileSpec::builder("demo.dart")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(70)
        .unwrap()
}
