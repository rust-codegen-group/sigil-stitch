# sigil_quote! Macro

`sigil_quote!` lets you write target-language code inline and have it expand to
`CodeBlockBuilder` method calls at compile time. It's the recommended way to build
`CodeBlock`s when the structure is known ahead of time.

For background on the `%` format specifiers that `sigil_quote!` expands to, see
[Format Specifiers](format_specifiers.md). For a hands-on introduction, see
[Getting Started](getting_started.md).

## Basic Usage

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let user_type = TypeName::importable_type("./models", "User");

let block = sigil_quote!(TypeScript {
    const user: $T(user_type) = await getUser($S("id"));
    if (!user) {
        throw new Error($S("not found"));
    }
    return user;
}).unwrap();
```

The macro takes a language type followed by a braced body of target-language code.
It returns `Result<CodeBlock, SigilStitchError>`.

## Interpolation Markers

| Syntax | Specifier | Argument Type | Purpose |
|--------|-----------|---------------|---------|
| `$T(expr)` | `%T` | `TypeName` | Type reference, tracks imports |
| `$N(expr)` | `%N` | `impl ToString` | Name identifier |
| `$S(expr)` | `%S` | `impl ToString` | String literal (quoted in output) |
| `$L(expr)` | `%L` | `impl Into<Arg>` | Literal value or nested code |
| `$C(expr)` | `%L` | `CodeBlock` | Nested code block |
| `$W` | `%W` | (none) | Soft line-break point |
| `$open("text")` | — | (none) | Custom block opener override |
| `$>` | `%>` | (none) | Increase indent level |
| `$<` | `%<` | (none) | Decrease indent level |
| `$$` | `$` | (none) | Literal dollar sign |

### Types (`$T`)

```rust,ignore
let user_type = TypeName::importable_type("./models", "User");
let block = sigil_quote!(TypeScript {
    const user: $T(user_type) = getUser();
}).unwrap();
// Expands to: __sigil_builder.add_statement("const user: %T = getUser()", (user_type,));
// The import collector picks up User and generates: import type { User } from './models'
```

### Names (`$N`)

```rust,ignore
let var_name = "myVariable";
let block = sigil_quote!(TypeScript {
    const $N(var_name) = 42;
}).unwrap();
// Output: const myVariable = 42;
```

### String Literals (`$S`)

```rust,ignore
let block = sigil_quote!(TypeScript {
    console.log($S("hello world"));
}).unwrap();
// Output: console.log('hello world');  (TypeScript uses single quotes)
```

### Literals (`$L`)

```rust,ignore
let default_val = "0";
let block = sigil_quote!(TypeScript {
    const count = $L(default_val);
}).unwrap();
// Output: const count = 0;
```

### Nested Code Blocks (`$C`)

```rust,ignore
let inner = CodeBlock::of("doSomething()", ()).unwrap();
let block = sigil_quote!(TypeScript {
    $C(inner);
}).unwrap();
// Output: doSomething();
```

### Dollar Escape (`$$`)

```rust,ignore
let block = sigil_quote!(TypeScript {
    const price = $$100;
}).unwrap();
// Output contains: $ 100
// Note: the tokenizer inserts a space between $ and 100
```

## Statement Rules

The macro classifies each line based on how it ends:

### Semicolons: `add_statement()`

Lines ending with `;` become statement calls (the renderer adds the language's
statement terminator):

```rust,ignore
sigil_quote!(TypeScript {
    const x = 1;        // -> add_statement("const x = 1", ())
    const y = x + 1;    // -> add_statement("const y = x + 1", ())
})
```

### Brace Groups: Control Flow

Lines ending with `{ ... }` (without a trailing `;`) become control flow:

```rust,ignore
sigil_quote!(TypeScript {
    if (x > 0) {            // -> begin_control_flow("if(x > 0)", ())
        return true;         // -> add_statement("return true", ())
    }                        // -> end_control_flow()
})
```

### Object Literals vs Control Flow

A `{ ... }` followed by `;` is treated as part of a statement, not control flow.
This is how the macro distinguishes object literals:

```rust,ignore
sigil_quote!(TypeScript {
    const config = { timeout: 5000 };    // statement (has trailing ;)
    if (ready) {                          // control flow (no trailing ;)
        start();
    }
})
```

### Blank Lines: `add_line()`

Blank lines in the macro body insert visual separators:

```rust,ignore
sigil_quote!(TypeScript {
    const a = 1;

    const b = 2;    // blank line above becomes add_line()
})
```

### Comments: `$comment("text")`

Rust's proc macro tokenizer strips `//` comments, so they're invisible to the macro.
Use `$comment()` instead:

```rust,ignore
sigil_quote!(TypeScript {
    $comment("Initialize the connection pool");
    const pool = createPool();
})
// Output:
// // Initialize the connection pool
// const pool = createPool();
```

## Control Flow

### if / else / else if

The macro detects `else` and `else if` chains after closing braces:

```rust,ignore
sigil_quote!(TypeScript {
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
})
```

This expands to:
```rust,ignore
__sigil_builder.begin_control_flow("if(x > 0)", ());
__sigil_builder.add_statement("return 1", ());
__sigil_builder.next_control_flow("else if(x < 0)", ());
__sigil_builder.add_statement("return - 1", ());
__sigil_builder.next_control_flow("else", ());
__sigil_builder.add_statement("return 0", ());
__sigil_builder.end_control_flow();
```

### for / while / try-catch

Any tokens followed by `{ ... }` are treated as control flow:

```rust,ignore
sigil_quote!(TypeScript {
    for (const item of items) {
        process(item);
    }
})
```

```rust,ignore
sigil_quote!(TypeScript {
    try {
        riskyOperation();
    } catch (e) {
        handleError(e);
    }
})
```

### Nested Control Flow

```rust,ignore
sigil_quote!(TypeScript {
    if (users.length > 0) {
        for (const user of users) {
            if (user.active) {
                process(user);
            }
        }
    }
})
```

### Interpolation in Conditions

```rust,ignore
let error_type = TypeName::importable_type("./errors", "NotFoundError");
sigil_quote!(TypeScript {
    if (!user) {
        throw new $T(error_type)($S("not found"));
    }
})
```

## Custom Block Openers (`$open`)

By default, `{ ... }` in `sigil_quote!` uses the language's `block_syntax().block_open`:
- Brace languages (TypeScript, Go, etc.): `" {"`
- Python: `":"`
- Haskell: `" ="`

Use `$open("text")` immediately before `{` to override the opener for that block:

```rust,ignore
use sigil_stitch::lang::haskell::Haskell;

// Haskell type class needs " where" instead of the default " ="
sigil_quote!(Haskell {
    class Functor f $open(" where") {
        fmap :: (a -> b) -> f a -> f b;
    }
})
// Output: class Functor f where
//             fmap :: (a -> b) -> f a -> f b
```

```rust,ignore
use sigil_stitch::lang::ocaml::OCaml;

// OCaml module block needs " = struct" opener
sigil_quote!(OCaml {
    module Foo $open(" = struct") {
        let x = 42;
    }
})
// Output: module Foo = struct
//             let x = 42
```

Pass `$open("")` to suppress the block opener entirely.

## Manual Indent / Dedent (`$>` / `$<`)

Use `$>` and `$<` as standalone directives to control indent level without
control flow blocks:

```rust,ignore
use sigil_stitch::lang::typescript::TypeScript;

sigil_quote!(TypeScript {
    namespace Foo {
    $>
    const x = 1;
    const y = 2;
    $<
    }
})
// Output:
// namespace Foo {
//     const x = 1;
//     const y = 2;
// }
```

These map to the `%>` and `%<` format specifiers in `CodeBlockBuilder`.

## Multi-Language Support

The same syntax works with any language type:

```rust,ignore
use sigil_stitch::lang::python::Python;

sigil_quote!(Python {
    if x > 0:
        return True
})
```

```rust,ignore
use sigil_stitch::lang::go_lang::GoLang;

sigil_quote!(GoLang {
    x := 42;
})
```

```rust,ignore
use sigil_stitch::lang::rust_lang::RustLang;

sigil_quote!(RustLang {
    let x: i32 = 42;
})
```

## Known Limitations and Quirks

### Tokenization

`sigil_quote!` uses Rust's proc_macro2 tokenizer, which means the input is tokenized
as Rust tokens, not as the target language's tokens. This creates some edge cases:

1. **Single-quoted strings don't work.** `'hello'` is tokenized as a Rust lifetime.
   Use `$S("hello")` instead.

2. **Spacing around operators.** Multi-character operators like `:=`, `::`, `===`
   are tokenized as separate punctuation characters. The macro reconstructs them
   but spacing may differ slightly:
   - `x := 42` may render as `x:= 42` (`:` suppresses leading space)
   - `std::mem::size_of` works but `<u32>` may get extra spaces around `<` and `>`

3. **No space before `(` after identifiers.** The macro can't distinguish keywords
   from function calls, so `if(x)` and `fn(x)` are treated the same. Both are valid
   in all supported languages.

4. **Negative number literals.** `-1` tokenizes as `-` then `1`, so it renders as
   `- 1` with a space. Functionally identical in all target languages.

5. **Template literals.** Backtick strings (`` `${expr}` ``) aren't representable.
   Use `$L(expr)` for dynamic content.

6. **Percent signs.** Literal `%` in your code is auto-escaped to `%%` in the
   format string, so it renders correctly.

### Comments

`//` comments are stripped by the Rust tokenizer before the proc macro sees them.
Use `$comment("text")` for comments in generated code.

### Expressions in Interpolation

The expression inside `$T(...)`, `$S(...)`, etc. is passed through as an opaque
token stream. Any valid Rust expression works:

```rust,ignore
sigil_quote!(TypeScript {
    const x: $T(TypeName::primitive("string")) = $S("hello".to_uppercase());
})
```

### Blank Line Detection

Blank line detection uses `proc_macro2` span locations. It requires the
`span-locations` feature (enabled by the macros crate). If spans aren't available,
blank lines may not be detected.
