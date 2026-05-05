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
| `$C_each(expr)` | — | `impl IntoIterator<Item: Into<CodeBlock>>` | Splice each code block from iterable |
| `$if(cond) { ... }` | — | Rust expression | Meta-conditional (runtime codegen control) |
| `$for(pat in expr) { ... }` | — | Rust pattern + iterable | Meta-loop (emit body per iteration) |
| `$let(binding);` | — | Rust `let` binding | Rust-level variable binding inside macro body |
| `$join(sep, iter)` | `%L` | separator + `impl IntoIterator<Item: ToString>` | Separator-joined list |
| `$+` | — | (none) | Line continuation (suppress line-break split) |

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

## Splicing Code Block Iterables (`$C_each`)

`$C_each(expr)` iterates over a collection of `CodeBlock` values and splices each
one into the builder sequentially. It must appear at the start of a line.

```rust,ignore
let blocks: Vec<CodeBlock> = fields
    .iter()
    .map(|f| CodeBlock::of(&format!("this.{f} = null"), ()).unwrap())
    .collect();

sigil_quote!(TypeScript {
    $C_each(blocks);
})
// Output:
// this.name = null;
// this.age = null;
```

Each item in the iterable is converted via `Into<CodeBlock>`, so you can pass any
type that implements the conversion. An optional trailing `;` after `$C_each(expr)`
is consumed silently.

`$C_each` is newline-aware: blocks that already end with a newline (e.g., from
`add_statement`) are spliced as-is, while blocks that don't (e.g., from
`CodeBlock::of`) get an automatic line break appended. This prevents double blank
lines when splicing statement-built blocks.

## Meta-Conditionals (`$if` / `$else_if` / `$else`)

Meta-conditionals control which builder calls are emitted **at Rust runtime**, as
opposed to target-language `if`/`else` which emits control flow in the generated
code. Use them when the structure of the output depends on a Rust-side condition.

```rust,ignore
let include_debug = true;

sigil_quote!(TypeScript {
    const x = 1;
    $if(include_debug) {
        console.log($S("debug: x ="), x);
    }
})
// When include_debug is true, output includes the console.log line.
// When false, it's omitted entirely.
```

### `$else_if` and `$else`

```rust,ignore
let mode = "production";

sigil_quote!(TypeScript {
    $if(mode == "debug") {
        console.log($S("debug mode"));
    } $else_if(mode == "test") {
        console.log($S("test mode"));
    } $else {
        console.log($S("production mode"));
    }
})
```

The conditions are arbitrary Rust expressions evaluated at runtime. The braces
delimit which `sigil_quote!` statements are conditionally included — they do **not**
produce target-language block syntax.

### Nesting with Target-Language Control Flow

Meta-conditionals can wrap target-language control flow and vice versa:

```rust,ignore
let use_guard = true;

sigil_quote!(TypeScript {
    $if(use_guard) {
        if (!user) {
            throw new Error($S("unauthorized"));
        }
    }
})
```

## Meta-Loops (`$for`)

`$for` iterates over a Rust collection at compile time, emitting the body statements
once per iteration. Like `$if`, it controls **which builder calls are made** — it
does not produce target-language loop syntax.

```rust,ignore
let fields = vec!["name", "age", "email"];

sigil_quote!(TypeScript {
    $for(f in &fields) {
        this.$N(*f) = null;
    }
})
// Output:
// this.name = null;
// this.age = null;
// this.email = null;
```

### Destructuring Patterns

Any Rust `for` pattern works:

```rust,ignore
let entries = vec![("x", "number"), ("y", "string")];

sigil_quote!(TypeScript {
    $for((name, ty) in &entries) {
        let $N(*name): $L(*ty);
    }
})
// Output:
// let x: number;
// let y: string;
```

### Nesting

`$for` can nest inside `$if` and vice versa, and can contain target-language
control flow:

```rust,ignore
let variants = vec!["A", "B", "C"];

sigil_quote!(TypeScript {
    $for(v in &variants) {
        case $S(*v):
            return $S(*v);
    }
})
```

### Combining with Interpolation Markers

All interpolation markers (`$T`, `$N`, `$S`, `$L`, `$C`, `$W`, `$join`) work
inside `$for` bodies, and the loop variable is in scope:

```rust,ignore
let types: Vec<TypeName> = get_types();

sigil_quote!(TypeScript {
    $for(t in &types) {
        import type { $T(t.clone()) };
    }
})
```

## Meta-Bindings (`$let`)

`$let` introduces a Rust-level `let` binding inside the macro body. It emits a
real `let` statement in the generated Rust code, making it possible to compute
intermediate values — including fallible expressions with `?` — inside `$for` and
`$if` bodies.

```rust,ignore
let fields = vec![("name", "String"), ("age", "u32")];

sigil_quote!(TypeScript {
    $for((name, ty) in &fields) {
        $let(upper = name.to_uppercase());
        const $N(upper): $L(*ty);
    }
})
// Output:
// const NAME: String;
// const AGE: u32;
```

### Syntax

The content between the parentheses is emitted verbatim as `let <content>;`.
All Rust `let` forms work:

```rust,ignore
$let(x = expr);                // simple binding
$let(x: Type = expr);          // with type annotation
$let((a, b) = pair);           // destructuring
$let(mut x = 0);               // mutable binding
```

### Fallible Expressions (`?`)

The primary motivation for `$let` is supporting the `?` operator inside `$for`
bodies. Since `sigil_quote!` expands to a plain block (not a closure), `?`
propagates to the enclosing function:

```rust,ignore
fn emit_enum(en: &Enum) -> Option<FileSpec> {
    let block = sigil_quote!(RustLang {
        $for(v in &en.values) {
            $let(s = v.value.as_str()?);
            $let(variant = s.to_pascal_case());
            $if(&variant != s) {
                #[serde(rename = $S(s))]
            }
            $L(format!("{variant},"))
        }
    }).ok()?;
    // ...
}
```

Note that `?` also works directly inside interpolation expressions without
`$let` — use `$let` only when you need to bind the result for reuse:

```rust,ignore
// Simple case: ? inside $L() works without $let
$for(v in &values) {
    $L(format!("{},", v.as_str()?.to_pascal_case()))
}
```

## Separator-Joined Lists (`$join`)

`$join(sep, iter)` joins the string representations of an iterable's items with
a separator. It expands to a `%L` specifier internally.

```rust,ignore
let items = vec!["a", "b", "c"];

sigil_quote!(TypeScript {
    const values = [$join(", ", items)];
})
// Output: const values = [a, b, c];
```

The separator is any Rust expression that evaluates to something accepted by
`Vec<String>::join()` (typically a `&str`). Each item is converted via `ToString`.

```rust,ignore
let fields = vec!["name", "age", "email"];
let assignments: Vec<String> = fields.iter().map(|f| format!("this.{f} = {f}")).collect();

sigil_quote!(TypeScript {
    $join(";\n", assignments)
})
// Output:
// this.name = name;
// this.age = age;
// this.email = email
```

## Line Continuation (`$+`)

`sigil_quote!` splits statements on line breaks — each source line becomes a
separate statement in the generated code. This works well for languages like
Kotlin and Python where each line is typically a statement.

For expressions that span multiple lines (common in Haskell, OCaml, or long
function calls), place `$+` at the end of a line to suppress the split and
continue the statement on the next line:

```rust,ignore
use sigil_stitch::lang::haskell::Haskell;

sigil_quote!(Haskell {
    mapM_ $+
        putStrLn $+
        items
})
// Output: mapM_ putStrLn items
```

```rust,ignore
use sigil_stitch::lang::kotlin::Kotlin;

sigil_quote!(Kotlin {
    val result = someFunction( $+
        arg1, arg2);
})
// Output: val result = someFunction(arg1, arg2);
```

Without `$+`, each source line becomes its own statement. For semicolon-based
languages, `;` still takes priority as the statement terminator regardless of
line breaks.

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

2. **Colon spacing is context-aware.** The macro tracks a `ColonContext` to decide
   whether `:` gets a space before it:

   | Context | Example | Space before `:` |
   |---------|---------|-------------------|
   | Type annotation | `name: string` | no |
   | Map entry | `{ key: value }` | no |
   | Path separator | `std::mem` | no |
   | Ternary | `x ? y : z` | yes |
   | Walrus assign | `x := 42` | yes |

   The context is set automatically: `?` (standalone) enters ternary mode,
   `:` and `;` reset to type-annotation mode, `{` enters map-entry mode,
   and `:=` / `::` are detected via one-token lookahead. Path separators
   (`std::mem::size_of`) render tightly with no extra spaces.

3. **Other multi-character operators.** Operators like `===`, `!==`, `->`
   are tokenized as separate punctuation characters. The macro reconstructs
   them via proc_macro2's `Spacing::Joint` flag. A pre-scan annotation pass
   classifies generic angle brackets (`Vec<T>`, `HashMap<K, V>`), path
   separators (`std::mem`), macro bangs (`println!(...)`), and prefix
   operators (`&self`, `*ptr`) — these render tightly without extra spaces.
   The generic `<`/`>` heuristic relies on the preceding identifier starting
   with uppercase, so `fn foo<T>` may keep a space before `<` (use `FunSpec`
   for generic function declarations).

4. **Keyword spacing before `(`.** Control-flow keywords (`if`, `for`, `while`,
   `else`, `match`, `return`, `try`, `catch`, etc.) automatically get a space
   before `(`. Regular identifiers do not, so `myFunc(x)` stays tight while
   `if (x)` gets the expected space. This covers the common case but isn't
   configurable per-language.

5. **Negative number literals.** `-1` tokenizes as `-` then `1`, so it renders as
   `- 1` with a space. Functionally identical in all target languages.

6. **Template literals.** Backtick strings (`` `${expr}` ``) aren't representable.
   Use `$L(expr)` for dynamic content.

7. **Percent signs.** Literal `%` in your code is auto-escaped to `%%` in the
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
