# Getting Started

## Installation

Add sigil-stitch to your project:

```bash
cargo add sigil-stitch
```

Or add it directly to your `Cargo.toml`:

```toml
[dependencies]
sigil-stitch = "0.6"
```

sigil-stitch requires Rust edition 2024 and MSRV 1.88.0. Runtime dependencies (`pretty`, `serde` with `derive`, and `snafu`) are pulled in automatically. No feature flags are needed -- all spec types implement `serde::Serialize` and `serde::Deserialize` out of the box.

## Your First CodeBlock

A `CodeBlock` is a composable code fragment built from format strings and typed arguments. Here's a complete example that generates a TypeScript file with an automatic import:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::code_block::StringLitArg;
# fn main() {
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
println!("{output}");
# }
```

This produces:

```typescript
import type { User } from './models'

const user: User = await getUser('id');
return user;
```

Two things happened automatically:

- `%T` with `user_type` rendered as `User` in the code *and* added `import type { User } from './models'` at the top of the file.
- `%S` with `StringLitArg` rendered the string `"id"` as a single-quoted TypeScript string literal `'id'`.

The `()` in `cb.add_statement("return user", ())` means "no arguments" -- the format string has no specifiers, so none are needed.

## The Macro Alternative

The `sigil_quote!` macro lets you write target-language code inline, with less ceremony than the builder API. Here's the same example:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let user_type = TypeName::importable_type("./models", "User");

let body = sigil_quote!(TypeScript {
    const user: $T(user_type) = await getUser($S("id"));
    return user;
}).unwrap();
# }
```

This produces the same `CodeBlock` as the builder version above. The macro uses `$T` instead of `%T` and `$S` instead of `%S`, but the result is identical -- same import tracking, same rendering, same output when passed to `FileSpec`.

The macro is a good fit when you're writing a block of target-language code with a few interpolations. The builder is better when you're constructing code programmatically (loops, conditionals on what to emit).

## Building Structured Declarations

For functions, types, and other declarations, use the Spec layer. Specs carry structural metadata (name, return type, visibility, modifiers) and emit `CodeBlock`s internally.

Here's a function declaration:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let user_type = TypeName::importable_type("./models", "User");

let fun = FunSpec::builder("getActiveUsers")
    .returns(TypeName::array(user_type.clone()))
    .is_async()
    .body(sigil_quote!(TypeScript {
        const users = await fetchAll();
        return users.filter(u => u.active);
    }).unwrap())
    .build()
    .unwrap();

let file = FileSpec::builder("users.ts")
    .add_function(fun)
    .build()
    .unwrap();

let output = file.render(80).unwrap();
println!("{output}");
# }
```

This produces a complete TypeScript file with the function declaration, including the `async` keyword, the `User[]` return type annotation, and the import for `User`.

Notice the builder pattern: spec builders like `FunSpec::builder()` and `FileSpec::builder()` use an owning chain pattern -- setter methods like `.returns()`, `.is_async()`, and `.body()` take `mut self` and return `Self`, so you chain them fluently. The `.build()` call at the end consumes the builder and returns `Result<FunSpec>`. (`CodeBlockBuilder` is different: it uses `&mut self`, so you keep it in a `let mut` binding.)

## Specs Emit CodeBlocks

Every spec type follows the same pattern: you configure it with a builder, call `.build()`, and eventually `FileSpec` calls `.emit()` on it to get a `CodeBlock`. This means:

- You never write raw import statements. `%T` handles it.
- You never manually format function signatures. `FunSpec` handles it.
- You can mix specs and raw CodeBlocks freely in a `FileSpec`.

The renderer and import collector only see `CodeBlock` trees. They don't know or care whether a block came from a `FunSpec`, a `TypeSpec`, or a hand-written `CodeBlock::builder()` call.

## Configuring a Language

Each language type (`TypeScript`, `JavaScript`, `Python`, `Java`, and so on)
is a struct with public fields. The ones you usually want to tweak are exposed
as fluent `with_*` builders:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::lang::config::QuoteStyle;
# use sigil_stitch::prelude::*;
# fn main() {
// Prettier-style: double quotes, no semicolons, .tsx extension.
let ts = TypeScript::new()
    .with_quote_style(QuoteStyle::Double)
    .with_semicolons(false)
    .with_extension("tsx")
    .with_indent("    ");
# }
```

| Language     | `with_quote_style` | `with_indent` | `with_semicolons` | `with_extension` |
| ------------ | :---: | :---: | :---: | :---: |
| `TypeScript` | yes | yes | yes | yes |
| `JavaScript` | yes | yes | yes | yes |
| `Python`     | yes | yes | n/a | yes (e.g. `pyi`) |
| `Java`   | n/a | yes | n/a | yes |
| `Rust`   | n/a | yes | n/a | yes |
| `Go`     | n/a | yes | n/a | yes |
| `Kotlin`     | n/a | yes | n/a | yes (e.g. `kts`) |
| `Swift`      | n/a | yes | n/a | yes |
| `Dart`   | n/a | yes | n/a | yes |
| `CSharp`     | n/a | yes | n/a | yes |
| `Lua`        | n/a | yes | n/a | yes |
| `C`      | n/a | yes | n/a | yes (e.g. `h`) |
| `Cpp`    | n/a | yes | n/a | yes (e.g. `hpp`, `cxx`) |
| `Bash`       | n/a | yes | n/a | yes (e.g. `sh`) |
| `Zsh`        | n/a | yes | n/a | yes |

Language configuration is per-instance, not global: pass the configured language
into the `FileSpec` / `ProjectSpec` you want rendered with those settings.

## What's Next

Now that you've seen the basics:

- [Format Specifiers](format_specifiers.md) explains every `%` specifier in depth.
- [TypeName](type_name.md) covers type references, import tracking, and cross-language rendering.
- [Building Functions & Fields](functions_and_fields.md) covers ParameterSpec, FieldSpec, and FunSpec.
- [Building Types & Enums](types_and_enums.md) covers TypeSpec, PropertySpec, AnnotationSpec, and EnumVariantSpec.
- [Files & Projects](files_and_projects.md) covers ImportSpec, FileSpec, and ProjectSpec.
- [sigil_quote! Macro](sigil_quote.md) has the full guide for the macro syntax.
- [Code Templates](code_templates.md) covers reusable named-parameter templates.
- [Language Cookbook](language_cookbook.md) has idiomatic recipes for each supported language.
