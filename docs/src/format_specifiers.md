# Format Specifiers

`CodeBlock` format strings use `%`-prefixed specifiers to interpolate arguments. Each specifier consumes one argument from the args list (except `%W`, `%>`, `%<`, `%[`, `%]`, and `%%`, which consume none).

## Quick Reference

| Specifier | Name | Argument | Purpose |
|-----------|------|----------|---------|
| `%T` | Type | `TypeName<L>` | Emit type reference, track import |
| `%N` | Name | `NameArg` | Emit identifier name |
| `%S` | String | `StringLitArg` | Emit escaped string literal |
| `%L` | Literal | `&str`, `String`, `CodeBlock<L>` | Emit raw value or nested block |
| `%W` | Wrap | (none) | Soft line break point |
| `%>` | Indent | (none) | Increase indent level |
| `%<` | Dedent | (none) | Decrease indent level |
| `%[` | Begin | (none) | Start of statement |
| `%]` | End | (none) | End of statement |
| `%%` | Escape | (none) | Literal `%` character |

## `%T` -- Type Reference

The most powerful specifier. Takes a `TypeName<L>` and does two things: emits the type name in the output AND registers the import so `FileSpec::render()` can collect, deduplicate, and emit import headers automatically.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::type_name::TypeName;

let user = TypeName::<TypeScript>::importable("./models", "User");
let block = CodeBlock::<TypeScript>::of("const u: %T = getUser()", (user,)).unwrap();
// Value import (not `import type`):
//   import { User } from './models';
//   const u: User = getUser();
```

For type-only imports (TypeScript's `import type`), use `importable_type`:

```rust,ignore
let user = TypeName::<TypeScript>::importable_type("./models", "User");
// import type { User } from './models';
```

Generic types track imports recursively. Every `TypeName` nested inside the generic's parameters is collected:

```rust,ignore
let promise = TypeName::<TypeScript>::generic(
    TypeName::primitive("Promise"),
    vec![TypeName::importable("./models", "User")],
);
let block = CodeBlock::<TypeScript>::of("function load(): %T", (promise,)).unwrap();
// Promise<User> -- the User import is still tracked
```

## `%N` -- Name

Emits an identifier. Bare `&str` and `String` values map to `Arg::Literal` (for `%L`) by default, so you must use the `NameArg` wrapper when your format string contains `%N`.

```rust,ignore
use sigil_stitch::code_block::{CodeBlock, NameArg};
use sigil_stitch::lang::typescript::TypeScript;

let method_name = "getData";
let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("this.%N()", (NameArg(method_name.to_string()),));
let block = cb.build().unwrap();
// Output: this.getData();
```

## `%S` -- String Literal

Emits a language-aware quoted string. The `CodeLang::render_string_literal()` method on each language controls the quoting style and escape rules. TypeScript and JavaScript default to single quotes; Rust, Java, Go, C, C++, Swift, and Kotlin use double quotes; Dart uses single quotes; Python uses single quotes.

Requires the `StringLitArg` wrapper.

```rust,ignore
use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::typescript::TypeScript;

let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("const msg = %S", (StringLitArg("hello world".to_string()),));
let block = cb.build().unwrap();
// TypeScript output: const msg = 'hello world';
// Java output:      const msg = "hello world";
```

Special characters are escaped according to each language's rules. For example, Kotlin and Dart escape `$` to prevent string interpolation.

## `%L` -- Literal

Emits a raw value with no transformation. This is the default for bare `&str` and `String` arguments, so no wrapper is needed. Also accepts `CodeBlock<L>` for embedding nested blocks.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let mut cb = CodeBlock::<TypeScript>::builder();

// Bare string -> Arg::Literal -> used by %L
cb.add_statement("const count = %L", "42");

// Nested CodeBlock -> Arg::Code -> also used by %L
let inner = CodeBlock::<TypeScript>::of("getValue()", ()).unwrap();
cb.add_statement("const x = %L", inner);

let block = cb.build().unwrap();
// const count = 42;
// const x = getValue();
```

## `%W` -- Soft Line Break

No argument consumed. Marks a point where the Wadler-Lindig pretty printer (via the `pretty` crate) MAY insert a line break if the line exceeds the target width passed to `FileSpec::render(width)`. If the line fits within the width, `%W` renders as a space.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let mut cb = CodeBlock::<TypeScript>::builder();
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
```

Without any `%W` in a CodeBlock, the renderer does direct string concatenation with indent tracking. When `%W` is present, it builds a `pretty::BoxDoc` tree for width-aware layout. `BoxDoc` (not `RcDoc`) is used so rendered documents are `Send + Sync`.

## `%>` and `%<` -- Indent / Dedent

No argument consumed. Manually increase (`%>`) or decrease (`%<`) the indent level. Rarely needed directly because `begin_control_flow()`, `next_control_flow()`, and `end_control_flow()` manage indentation automatically. Useful when building custom block structures.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let mut cb = CodeBlock::<TypeScript>::builder();
cb.add("items: [%>\n", ());
cb.add("'first',\n", ());
cb.add("'second',\n", ());
cb.add("%<]", ());
let block = cb.build().unwrap();
// items: [
//     'first',
//     'second',
// ]
```

Indent depth must balance to zero by the time `build()` is called. An unbalanced depth produces an `UnbalancedIndent` error.

## `%[` and `%]` -- Statement Boundaries

No argument consumed. `%[` marks the start of a statement. `%]` marks the end and appends the language's statement terminator -- `;` for TypeScript, Rust, Java, C, C++, Dart; nothing for Python, Go, Kotlin, Swift.

You almost never write these directly. `add_statement()` wraps your format string in `%[...%]` and appends a newline automatically:

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let mut cb = CodeBlock::<TypeScript>::builder();

// These produce the same output:
cb.add_statement("const x = 1", ());
cb.add("%[const x = 1%]\n", ());

let block = cb.build().unwrap();
// const x = 1;
// const x = 1;
```

## `%%` -- Literal Percent

Emits a literal `%` character in the output. No argument consumed.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let block = CodeBlock::<TypeScript>::of("progress: 100%%", ()).unwrap();
// progress: 100%
```

## Arguments and the `IntoArgs` Trait

Every method that accepts a format string (`add`, `add_statement`, `begin_control_flow`, `next_control_flow`, `CodeBlock::of`) takes `args: impl IntoArgs<L>`. This trait converts Rust values into `Vec<Arg<L>>` for the format engine.

The critical rule: **bare strings map to `Arg::Literal`** (consumed by `%L`), not to `Arg::Name` or `Arg::StringLit`. To target `%N` or `%S`, use the `NameArg` and `StringLitArg` wrappers from `sigil_stitch::code_block`.

### Type-to-Arg Mapping

| Rust Type | Maps To | Consumed By |
|-----------|---------|-------------|
| `()` | empty vec | (no specifiers) |
| `TypeName<L>` | `Arg::TypeName` | `%T` |
| `&str` | `Arg::Literal` | `%L` |
| `String` | `Arg::Literal` | `%L` |
| `CodeBlock<L>` | `Arg::Code` | `%L` |
| `NameArg(String)` | `Arg::Name` | `%N` |
| `StringLitArg(String)` | `Arg::StringLit` | `%S` |
| `Vec<Arg<L>>` | passthrough | any |

### Single Argument

When a format string has exactly one specifier, pass the value directly (no tuple needed):

```rust,ignore
use sigil_stitch::type_name::TypeName;
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let user = TypeName::<TypeScript>::importable("./models", "User");
let block = CodeBlock::<TypeScript>::of("let u: %T", user).unwrap();
```

### Multiple Arguments with Tuples

For two or more specifiers, use a tuple. Tuples are supported up to 8 elements. Each element must implement `Into<Arg<L>>`.

```rust,ignore
use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::type_name::TypeName;

let user_type = TypeName::<TypeScript>::importable("./models", "User");

// Two args: a TypeName and a StringLitArg
let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("const u: %T = getUser(%S)", (user_type, StringLitArg("admin".into())));
let block = cb.build().unwrap();
// const u: User = getUser('admin');
```

### No Arguments

Pass `()` when the format string has no specifiers:

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;

let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("return null", ());
let block = cb.build().unwrap();
```

### Argument Count Validation

The builder checks that the number of argument-consuming specifiers (`%T`, `%N`, `%S`, `%L`) matches the number of arguments provided. A mismatch records a `FormatArgCount` error, surfaced when `build()` is called. The error carries the expected specifier list and the actual argument kinds so you can see exactly which slot is wrong.

```rust,ignore
// This will fail: format has 2 specifiers but only 1 argument
let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("const %N: %T = null", "x");  // &str gives one Arg::Literal
let result = cb.build();
// Err(FormatArgCount {
//     format: "const %N: %T = null",
//     expected_specifiers: vec!["%N", "%T"],
//     actual_arg_kinds:   vec!["Literal"],
// })
```

An unrecognised specifier character (anything after `%` that isn't `T`, `N`, `S`, `L`, `W`, `>`, `<`, `[`, `]`, or `%`) produces `Err(SigilStitchError::InvalidFormatSpecifier { format, specifier })` instead.
