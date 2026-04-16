# sigil-stitch

Type-safe, import-aware, width-aware code generation for multiple languages.

[![Crates.io](https://img.shields.io/crates/v/sigil-stitch.svg)](https://crates.io/crates/sigil-stitch)
[![docs.rs](https://docs.rs/sigil-stitch/badge.svg)](https://docs.rs/sigil-stitch)
[![CI](https://github.com/adamcavendish/sigil-stitch/actions/workflows/ci.yml/badge.svg)](https://github.com/adamcavendish/sigil-stitch/actions/workflows/ci.yml)

sigil-stitch combines [JavaPoet](https://github.com/square/javapoet)'s builder + CodeBlock
model with [Wadler-Lindig pretty printing](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)
and multi-language support. Reference types with `%T` in format strings, and the library
tracks every import for you, resolves naming conflicts, and emits width-aware formatted output.

## Quick Example

```rust
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let user_type = TypeName::<TypeScript>::importable_type("./models", "User");

let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("const user = await getUser(%S)", ("id",));
cb.add_statement("return user as %T", (user_type.clone(),));
let body = cb.build().unwrap();

let mut fb = FileSpec::<TypeScript>::builder("user.ts");
fb.add_code(body);
let file = fb.build();

let output = file.render(80).unwrap();
assert!(output.contains("import type { User } from './models'"));
```

## Supported Languages

TypeScript, JavaScript, Rust, Go, Python, Java, Kotlin, Swift, Dart, C, C++

## Format Specifiers

| Specifier | Name | Argument Type | Purpose |
|-----------|------|---------------|---------|
| `%T` | Type | `TypeName<L>` | Emit type reference, track import |
| `%N` | Name | `&str` or `Nameable` | Emit identifier name |
| `%S` | String | `&str` | Emit escaped string literal |
| `%L` | Literal | `&str`, number, `CodeBlock<L>` | Emit raw value or nested block |
| `%W` | Wrap | (none) | Soft line break point |
| `%>` | Indent | (none) | Increase indent level |
| `%<` | Dedent | (none) | Decrease indent level |
| `%[` | Statement begin | (none) | Start of statement |
| `%]` | Statement end | (none) | End of statement (appends `;` if needed) |

## Spec Layer

Build structured declarations with the spec builders:

- **FileSpec** — top-level file with automatic import resolution
- **TypeSpec** — struct, class, interface, trait, or enum
- **FunSpec** — function or method with parameters, return type, and body
- **FieldSpec** — struct field or class property
- **ParameterSpec** — function parameter
- **ProjectSpec** — multi-file project generation
- **CodeTemplate** — reusable parameterized templates with named parameters

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
