# Format Specifiers

`CodeBlock` format strings use `%`-prefixed specifiers to interpolate arguments. Each specifier consumes one argument from the args list (except `%W`, `%>`, `%<`, `%[`, `%]`, and `%%`, which consume none).

## Quick Reference

| Specifier | Name | Argument | Purpose |
|-----------|------|----------|---------|
| `%T` | Type | `TypeName` | Emit type reference, track import |
| `%N` | Name | `NameArg` | Emit identifier name |
| `%S` | String | `StringLitArg` | Emit escaped string literal |
| `%V` | Verbatim | `VerbatimStrArg` | Emit string with interpolation preserved |
| `%L` | Literal | `&str`, `String`, `CodeBlock` | Emit raw value or nested block |
| `%W` | Wrap | (none) | Soft line break point |
| `%>` | Indent | (none) | Increase indent level |
| `%<` | Dedent | (none) | Decrease indent level |
| `%[` | Begin | (none) | Start of statement |
| `%]` | End | (none) | End of statement |
| `%%` | Escape | (none) | Literal `%` character |

## `%T` -- Type Reference

The most powerful specifier. Takes a `TypeName` and does two things: emits the type name in the output AND registers the import so `FileSpec::render()` can collect, deduplicate, and emit import headers automatically.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::type_name::TypeName;
# fn main() {
let user = TypeName::importable("./models", "User");
let block = CodeBlock::of("const u: %T = getUser()", (user,)).unwrap();
// Value import (not `import type`):
//   import { User } from './models';
//   const u: User = getUser();
# }
```

For type-only imports (TypeScript's `import type`), use `importable_type`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let user = TypeName::importable_type("./models", "User");
// import type { User } from './models';
# }
```

Generic types track imports recursively. Every `TypeName` nested inside the generic's parameters is collected:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let promise = TypeName::generic(
    TypeName::primitive("Promise"),
    vec![TypeName::importable("./models", "User")],
);
let block = CodeBlock::of("function load(): %T", (promise,)).unwrap();
// Promise<User> -- the User import is still tracked
# }
```

## `%N` -- Name

Emits an identifier with automatic keyword escaping. If the name collides with a reserved word in the target language, it is escaped using the language's convention (Rust: `r#type`, Go/Python: `type_`). Bare `&str` and `String` values map to `Arg::Literal` (for `%L`) by default, so you must use the `NameArg` wrapper when your format string contains `%N`.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::{CodeBlock, NameArg};
# use sigil_stitch::prelude::*;
# fn main() {
let method_name = "getData";
let mut cb = CodeBlock::builder();
cb.add_statement("this.%N()", (NameArg(method_name.to_string()),));
let block = cb.build().unwrap();
// Output: this.getData();
# }
```

Reserved-word escaping happens at render time based on the target language:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::{CodeBlock, NameArg};
# use sigil_stitch::lang::rust_lang::RustLang;
# use sigil_stitch::spec::file_spec::FileSpec;
# use sigil_stitch::prelude::*;
# fn main() {
let field_name = "type"; // reserved in Rust
let block = CodeBlock::of("let %N = value", NameArg(field_name.into())).unwrap();
let file = FileSpec::builder_with("test.rs", RustLang::new())
    .add_code(block)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
// Output: let r#type = value
# }
```

## `%S` -- String Literal

Emits a language-aware quoted string. The `CodeLang::render_string_literal()` method on each language controls the quoting style and escape rules. TypeScript and JavaScript default to single quotes; Rust, Java, Go, C, C++, Swift, and Kotlin use double quotes; Dart uses single quotes; Python uses single quotes.

Requires the `StringLitArg` wrapper.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::{CodeBlock, StringLitArg};
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();
cb.add_statement("const msg = %S", (StringLitArg("hello world".to_string()),));
let block = cb.build().unwrap();
// TypeScript output: const msg = 'hello world';
// Java output:      const msg = "hello world";
# }
```

Special characters are escaped according to each language's rules. For example, Kotlin and Dart escape `$` to prevent string interpolation.

## `%V` -- Verbatim String Literal

Emits a string with minimal escaping — only characters that would structurally break the string delimiter are escaped, while interpolation sigils (`$`, `` ` ``, `{`, etc.) are preserved as-is. This is useful for generating code that uses the target language's string interpolation.

Requires the `VerbatimStrArg` wrapper.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::{CodeBlock, VerbatimStrArg};
# use sigil_stitch::lang::bash::Bash;
# use sigil_stitch::spec::file_spec::FileSpec;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();
cb.add("local config=%V", (VerbatimStrArg("\"${XDG_CONFIG_HOME:-$HOME/.config}\"".to_string()),));
cb.add_line();
cb.add("local version=%V", (VerbatimStrArg("\"$(git describe --tags 2>/dev/null || echo dev)\"".to_string()),));
cb.add_line();
cb.add("echo %V", (VerbatimStrArg("Deploying ${APP_NAME} v${version} (PID=$$)".to_string()),));
let block = cb.build().unwrap();
let file = FileSpec::builder_with("test.bash", Bash::new())
    .add_code(block)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
assert!(output.contains(r#""${XDG_CONFIG_HOME:-$HOME/.config}""#));
assert!(output.contains(r#""$(git describe --tags 2>/dev/null || echo dev)""#));
assert!(output.contains("Deploying ${APP_NAME} v${version} (PID=$$)"));
// Output (Bash $V is pure passthrough — users include their own quotes):
//   local config="${XDG_CONFIG_HOME:-$HOME/.config}"
//   local version="$(git describe --tags 2>/dev/null || echo dev)"
//   echo Deploying ${APP_NAME} v${version} (PID=$$)
# }
```

Per-language behavior:

| Language | `%V` output for `"$x"` | Delimiter | Escapes only |
|----------|------------------------|-----------|--------------|
| Bash/Zsh | `$x` | (passthrough) | (none) |
| JavaScript/TS | `` `$x` `` | `` `...` `` | `\` `` ` `` |
| Python | `f"$x"` | `f"..."` | `\` `"` |
| Kotlin/Swift | `"$x"` | `"..."` | `\` `"` |
| Dart | `'$x'` | `'...'` | `\` `'` |
| C# | `$"$x"` | `$"..."` | `\` `"` |
| Scala | `s"$x"` | `s"..."` | `\` `"` |
| Others | Same as `%S` | (full escaping) | All |

For Bash/Zsh, `%V` is pure passthrough — the string is emitted as-is with no wrapping quotes and no escaping. Shell interpolates by default, and users control quoting in the `%V` content itself (include `"..."` in the string when quoting is desired in the output).

For languages without string interpolation (C, C++, Go, Rust, Java, Haskell, OCaml, Lua), `%V` falls back to `%S` behavior (full escaping).

### `@{expr}` interpolation in `$V`

When using `$V` with a string literal in `sigil_quote!`, you can embed Rust expressions with `@{expr}`. These are evaluated at compile time and spliced into the verbatim output:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let registry = "ghcr.io";
let tag = "latest";
let block = sigil_quote!(Bash {
    docker push $V("@{registry}/myapp:@{tag}")
}).unwrap();
// Output: docker push ghcr.io/myapp:latest
# }
```

This is syntactic sugar — the macro transforms the string into a `format!()` call. Shell variables like `$HOME` pass through unchanged while `@{expr}` parts are resolved at Rust compile time.

Escape `@@` to emit a literal `@`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let block = sigil_quote!(Bash {
    echo $V("admin@@localhost")
}).unwrap();
// Output: echo admin@localhost
# }
```

Arbitrary Rust expressions work inside `@{...}`, including method calls:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let items = vec!["a", "b", "c"];
let block = sigil_quote!(Bash {
    echo $V("count=@{items.len()}")
}).unwrap();
// Output: echo count=3
# }
```

If `$V` receives a non-literal expression (e.g. `$V(my_var)` or `$V(format!(...))`), `@{...}` processing is skipped and the expression is used as-is.

## `%L` -- Literal

Emits a raw value with no transformation. This is the default for bare `&str` and `String` arguments, so no wrapper is needed. Also accepts `CodeBlock` for embedding nested blocks.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();

// Bare string -> Arg::Literal -> used by %L
cb.add_statement("const count = %L", "42");

// Nested CodeBlock -> Arg::Code -> also used by %L
let inner = CodeBlock::of("getValue()", ()).unwrap();
cb.add_statement("const x = %L", inner);

let block = cb.build().unwrap();
// const count = 42;
// const x = getValue();
# }
```

## `%W` -- Soft Line Break

No argument consumed. Marks a point where the Wadler-Lindig pretty printer (via the `pretty` crate) MAY insert a line break if the line exceeds the target width passed to `FileSpec::render(width)`. If the line fits within the width, `%W` renders as a space.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();
cb.add_statement("const result = someFunction(arg1,%Warg2,%Warg3,%Warg4)", ());
let block = cb.build().unwrap();

// At width 80 (fits on one line):
//   const result = someFunction(arg1, arg2, arg3, arg4);
//
// At width 40 (wraps):
//   const result = someFunction(arg1,
//       arg2,
//       arg3,
//       arg4);
# }
```

Without any `%W` in a CodeBlock, the renderer does direct string concatenation with indent tracking. When `%W` is present, it builds a `pretty::BoxDoc` tree for width-aware layout. `BoxDoc` (not `RcDoc`) is used so rendered documents are `Send + Sync`.

## `%>` and `%<` -- Indent / Dedent

No argument consumed. Manually increase (`%>`) or decrease (`%<`) the indent level. Rarely needed directly because `begin_control_flow()`, `next_control_flow()`, and `end_control_flow()` manage indentation automatically. Useful when building custom block structures.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();
cb.add("items: [%>\n", ());
cb.add("'first',\n", ());
cb.add("'second',\n", ());
cb.add("%<]", ());
let block = cb.build().unwrap();
// items: [
//     'first',
//     'second',
// ]
# }
```

Indent depth must balance to zero by the time `build()` is called. An unbalanced depth produces an `UnbalancedIndent` error.

## `%[` and `%]` -- Statement Boundaries

No argument consumed. `%[` marks the start of a statement. `%]` marks the end and appends the language's statement terminator -- `;` for TypeScript, Rust, Java, C, C++, Dart; nothing for Python, Go, Kotlin, Swift.

You almost never write these directly. `add_statement()` wraps your format string in `%[...%]` and appends a newline automatically:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();

// These produce the same output:
cb.add_statement("const x = 1", ());
cb.add("%[const x = 1%]\n", ());

let block = cb.build().unwrap();
// const x = 1;
// const x = 1;
# }
```

## `%%` -- Literal Percent

Emits a literal `%` character in the output. No argument consumed.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let block = CodeBlock::of("progress: 100%%", ()).unwrap();
// progress: 100%
# }
```

## Arguments and the `IntoArgs` Trait

Every method that accepts a format string (`add`, `add_statement`, `begin_control_flow`, `next_control_flow`, `CodeBlock::of`) takes `args: impl IntoArgs`. This trait converts Rust values into `Vec<Arg>` for the format engine.

The critical rule: **bare strings map to `Arg::Literal`** (consumed by `%L`), not to `Arg::Name` or `Arg::StringLit`. To target `%N` or `%S`, use the `NameArg` and `StringLitArg` wrappers from `sigil_stitch::code_block`.

### Type-to-Arg Mapping

| Rust Type | Maps To | Consumed By |
|-----------|---------|-------------|
| `()` | empty vec | (no specifiers) |
| `TypeName` | `Arg::TypeName` | `%T` |
| `&str` | `Arg::Literal` | `%L` |
| `String` | `Arg::Literal` | `%L` |
| `CodeBlock` | `Arg::Code` | `%L` |
| `NameArg(String)` | `Arg::Name` | `%N` |
| `StringLitArg(String)` | `Arg::StringLit` | `%S` |
| `VerbatimStrArg(String)` | `Arg::VerbatimStr` | `%V` |
| `Vec<Arg>` | passthrough | any |

### Single Argument

When a format string has exactly one specifier, pass the value directly (no tuple needed):

```rust
# extern crate sigil_stitch;
# use sigil_stitch::type_name::TypeName;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let user = TypeName::importable("./models", "User");
let block = CodeBlock::of("let u: %T", user).unwrap();
# }
```

### Multiple Arguments with Tuples

For two or more specifiers, use a tuple. Tuples are supported up to 8 elements. Each element must implement `Into<Arg>`.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::{CodeBlock, StringLitArg};
# use sigil_stitch::type_name::TypeName;
# use sigil_stitch::prelude::*;
# fn main() {
let user_type = TypeName::importable("./models", "User");

// Two args: a TypeName and a StringLitArg
let mut cb = CodeBlock::builder();
cb.add_statement("const u: %T = getUser(%S)", (user_type, StringLitArg("admin".into())));
let block = cb.build().unwrap();
// const u: User = getUser('admin');
# }
```

### No Arguments

Pass `()` when the format string has no specifiers:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();
cb.add_statement("return null", ());
let block = cb.build().unwrap();
# }
```

### Argument Count Validation

The builder checks that the number of argument-consuming specifiers (`%T`, `%N`, `%S`, `%V`, `%L`) matches the number of arguments provided. A mismatch records a `FormatArgCount` error, surfaced when `build()` is called. The error carries the expected specifier list and the actual argument kinds so you can see exactly which slot is wrong.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// This will fail: format has 2 specifiers but only 1 argument
let mut cb = CodeBlock::builder();
cb.add_statement("const %N: %T = null", "x");  // &str gives one Arg::Literal
let result = cb.build();
// Err(FormatArgCount {
//     format: "const %N: %T = null",
//     expected_specifiers: vec!["%N", "%T"],
//     actual_arg_kinds:   vec!["Literal"],
// })
# }
```

An unrecognised specifier character (anything after `%` that isn't `T`, `N`, `S`, `V`, `L`, `W`, `>`, `<`, `[`, `]`, or `%`) produces `Err(SigilStitchError::InvalidFormatSpecifier { format, specifier })` instead.
