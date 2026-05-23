# sigil-stitch

Type-safe, import-aware, width-aware code generation for multiple languages.

[![Crates.io](https://img.shields.io/crates/v/sigil-stitch.svg)](https://crates.io/crates/sigil-stitch)
[![docs.rs](https://docs.rs/sigil-stitch/badge.svg)](https://docs.rs/sigil-stitch)
[![CI](https://github.com/adamcavendish/sigil-stitch/actions/workflows/ci.yml/badge.svg)](https://github.com/adamcavendish/sigil-stitch/actions/workflows/ci.yml)

sigil-stitch combines [JavaPoet](https://github.com/square/javapoet)'s builder + CodeBlock
model with [Wadler-Lindig pretty printing](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)
and multi-language support. Reference types with `%T` in format strings, and the library
tracks every import for you, resolves naming conflicts, and emits width-aware formatted output.

## Quick Start

```bash
cargo add sigil-stitch
```

Requires Rust edition 2024, MSRV 1.88.0. Runtime dependencies: `pretty` (Wadler-Lindig formatting), `serde` with `derive` (every spec type implements `Serialize`/`Deserialize` out of the box, so you can round-trip specs as JSON or YAML), and `snafu` (structured errors).

## Builder vs Macro

sigil-stitch offers two ways to build code. Both produce the same `CodeBlock` with
the same import tracking and rendering.

**Builder API** -- programmatic, good for dynamic code generation:

```rust
use sigil_stitch::prelude::*;
use sigil_stitch::code_block::StringLitArg;

let user_type = TypeName::importable_type("./models", "User");

let mut cb = CodeBlock::builder();
cb.add_statement(
    "const user: %T = await getUser(%S)",
    (user_type.clone(), StringLitArg("id".into())),
);
cb.add_statement("return user", ());
let body = cb.build().unwrap();

let file = FileSpec::builder("user.ts")
    .add_code(body)
    .build()
    .unwrap();

let output = file.render(80).unwrap();
assert!(output.contains("import type { User } from './models'"));
assert!(output.contains("const user: User = await getUser('id');"));
```

**`sigil_quote!` macro** -- inline target-language code, less ceremony:

```rust
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let user_type = TypeName::importable_type("./models", "User");

let body = sigil_quote!(TypeScript {
    const user: $T(user_type) = await getUser($S("id"));
    if (!user) {
        throw new Error($S("not found"));
    }
    return user;
}).unwrap();
```

The macro uses `$T`/`$S`/`$N`/`$L`/`$C`/`$V`/`$W` interpolation markers that expand to
the equivalent `%T`/`%S`/`%N`/`%L`/`%V` format specifiers at compile time. It also
supports `$V("@{expr}")` for compile-time interpolation in verbatim strings,
`$attr("text")` for structural annotations with language-specific rendering,
`$T_join(sep, iter)` for type-name joins with per-item import tracking,
`$C_each` for splicing iterables of code blocks, `$if`/`$else_if`/`$else`
for meta-conditionals, `$for` for compile-time iteration, `$let` for Rust-level
variable bindings, `$join` for separator-joined lists, and `$+` for line
continuation in multi-line expressions. Go `const (`, `var (`, `import (`, and
`type (` paren-delimited blocks are recognized as structural blocks so `$for`,
`$if`, and `$C_each` expand inside them.

## Format Specifiers

| Specifier | Name | Argument Type | Purpose |
|-----------|------|---------------|---------|
| `%T` | Type | `TypeName` | Emit type reference, track import |
| `%N` | Name | `NameArg` | Emit identifier name, escape reserved words |
| `%S` | String | `StringLitArg` | Emit escaped string literal |
| `%V` | Verbatim | `VerbatimStrArg` | Emit string with interpolation preserved |
| `%L` | Literal | `&str`, number, `CodeBlock` | Emit raw value or nested block |
| `%W` | Wrap | (none) | Soft line break point |
| `%>` | Indent | (none) | Increase indent level |
| `%<` | Dedent | (none) | Decrease indent level |
| `%[` | Statement begin | (none) | Start of statement |
| `%]` | Statement end | (none) | End of statement (appends `;` if needed) |

Bare `&str` maps to `%L`. Use `NameArg` for `%N`, `StringLitArg` for `%S`, and `VerbatimStrArg` for `%V`.
See the [Format Specifiers](docs/src/format_specifiers.md) chapter for the full deep dive.

## The Spec Layer

Build structured declarations with the spec builders:

| Spec | Purpose |
|------|---------|
| **ParameterSpec** | Function parameter (name + type + default + variadic + property) |
| **FieldSpec** | Struct field / class property (visibility, static, readonly) |
| **FunSpec** | Function or method (params, return type, body, async, abstract) |
| **TypeSpec** | Class, struct, interface, trait, enum, type alias, newtype, embedded types |
| **PropertySpec** | Computed property with getter/setter |
| **AnnotationSpec** | `@Override`, `#[derive(...)]`, `[[nodiscard]]` |
| **EnumVariantSpec** | Enum variant with optional value, tuple, or struct fields |
| **ImportSpec** | Explicit imports (aliased, side-effect, wildcard) |
| **FileSpec** | Top-level file with automatic import resolution |
| **ProjectSpec** | Multi-file project generation |
| **CodeTemplate** | Reusable parameterized templates with named parameters |

All specs emit `CodeBlock`s internally, so import tracking works everywhere.
See [Building Functions & Fields](docs/src/functions_and_fields.md), [Building Types & Enums](docs/src/types_and_enums.md), and [Files & Projects](docs/src/files_and_projects.md) for examples and the full API.

## Supported Languages

| Language   | Extension | Semicolons | Import Style       |
|------------|-----------|------------|--------------------|
| TypeScript | `.ts`     | yes        | ES modules         |
| JavaScript | `.js`     | yes        | ES modules         |
| Rust       | `.rs`     | yes        | `use` paths        |
| Go         | `.go`     | no         | package imports    |
| Python     | `.py`     | no         | `import`/`from`    |
| Java       | `.java`   | yes        | package imports    |
| Kotlin     | `.kt`     | no         | package imports    |
| Swift      | `.swift`  | no         | `import` module    |
| Dart       | `.dart`   | yes        | package imports    |
| Scala      | `.scala`  | no         | `import` paths     |
| Haskell    | `.hs`     | no         | `import` module    |
| OCaml      | `.ml`     | no         | `open` module      |
| C          | `.c`      | yes        | `#include`         |
| C++        | `.cpp`    | yes        | `#include`/`using` |
| C#         | `.cs`     | yes        | `using` namespace  |
| Lua        | `.lua`    | no         | `require()`        |
| Bash       | `.bash`   | no         | `source`           |
| Zsh        | `.zsh`    | no         | `source`           |

## Documentation

The [sigil-stitch book](docs/src/SUMMARY.md) covers everything in depth:

**User Guide:**

- [Introduction](docs/src/introduction.md) -- what it is and how the pieces fit together
- [Getting Started](docs/src/getting_started.md) -- first CodeBlock, first FileSpec, first output
- [Format Specifiers](docs/src/format_specifiers.md) -- deep dive on `%T`, `%N`, `%S`, `%V`, `%L`, `%W`, and friends
- [TypeName](docs/src/type_name.md) -- type references, import tracking, cross-language rendering
- [Building Functions & Fields](docs/src/functions_and_fields.md) -- ParameterSpec, FieldSpec, FunSpec
- [Building Types & Enums](docs/src/types_and_enums.md) -- TypeSpec, PropertySpec, AnnotationSpec, EnumVariantSpec
- [Files & Projects](docs/src/files_and_projects.md) -- ImportSpec, FileSpec, ProjectSpec
- [sigil_quote! Macro](docs/src/sigil_quote.md) -- inline code with `$T`/`$S`/`$N`/`$L`/`$V`/`$C_each`/`$if`/`$for`/`$let`/`$join`/`$+`/`$attr`/`$T_join` interpolation, plus Go paren-block support
- [Code Templates](docs/src/code_templates.md) -- reusable `#{name:K}` templates
- [Language Cookbook](docs/src/language_cookbook.md) -- idiomatic recipes per language

**Development Guide:**

- [Architecture](docs/src/architecture.md) -- four layers, three-pass pipeline, import resolution
- [Type Presentation](docs/src/type_presentation.md) -- data-driven cross-language type rendering
- [Adding a Language](docs/src/adding_a_language.md) -- implementing the CodeLang trait step by step

## MSRV

The minimum supported Rust version is **1.88.0** (edition 2024, let-chains).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
