# sigil_quote! Macro

`sigil_quote!` lets you write target-language code inline and have it expand to
`CodeBlockBuilder` method calls at compile time. It's the recommended way to build
`CodeBlock`s when the structure is known ahead of time.

For background on the `%` format specifiers that `sigil_quote!` expands to, see
[Format Specifiers](format_specifiers.md). For a hands-on introduction, see
[Getting Started](getting_started.md).

## Basic Usage

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let user_type = TypeName::importable_type("./models", "User");

let block = sigil_quote!(TypeScript {
    const user: $T(user_type) = await getUser($S("id"));
    if (!user) {
        throw new Error($S("not found"));
    }
    return user;
}).unwrap();
# }
```

The macro takes a language type followed by a braced body of target-language code.
It returns `Result<CodeBlock, SigilStitchError>`.

## Interpolation Markers

| Syntax | Specifier | Argument Type | Purpose |
|--------|-----------|---------------|---------|
| `$T(expr)` | `%T` | `TypeName` | Type reference, tracks imports |
| `$N(expr)` | `%N` | `impl ToString` | Name identifier |
| `$S(expr)` | `%S` | `impl ToString` | String literal (quoted in output) |
| `$V(expr)` | `%V` | `impl ToString` | Verbatim string (interpolation preserved) |
| `$L(expr)` | `%L` | `impl Into<Arg>` | Literal value, nested code, or parsed fragment |
| `$C(expr)` | `%L` | `CodeBlock` | Nested code block |
| `$W` | `%W` | (none) | Soft line-break point |
| `$>` | `%>` | (none) | Increase indent level |
| `$<` | `%<` | (none) | Decrease indent level |
| `$$` | `$` | (none) | Literal dollar sign |
| `$C_each(expr)` | — | `impl IntoIterator<Item: Into<CodeBlock>>` | Splice each code block from iterable |
| `$attr("text")` | — | `impl ToString` | Structural annotation (language-specific prefix/suffix) |
| `$T_join(sep, iter)` | `%T` | separator + `impl IntoIterator<Item: TypeName>` | Type name join with per-item import tracking |
| `$if(cond) { ... }` | — | Rust expression | Meta-conditional (runtime codegen control) |
| `$for(pat in expr) { ... }` | — | Rust pattern + iterable | Meta-loop (emit body per iteration) |
| `$let(binding);` | — | Rust `let` binding | Rust-level variable binding inside macro body |
| `$join(sep, iter)` | `%L` | separator + `impl IntoIterator<Item: ToString>` | Separator-joined list |
| `$+` | — | (none) | Line continuation (suppress line-break split) |

### Types (`$T`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let user_type = TypeName::importable_type("./models", "User");
let block = sigil_quote!(TypeScript {
    const user: $T(user_type) = getUser();
}).unwrap();
// Expands to: __sigil_builder.add_statement("const user: %T = getUser()", (user_type,));
// The import collector picks up User and generates: import type { User } from './models'
# }
```

### Names (`$N`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let var_name = "myVariable";
let block = sigil_quote!(TypeScript {
    const $N(var_name) = 42;
}).unwrap();
// Output: const myVariable = 42;
# }
```

### String Literals (`$S`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let block = sigil_quote!(TypeScript {
    console.log($S("hello world"));
}).unwrap();
// Output: console.log('hello world');  (TypeScript uses single quotes)
# }
```

### Verbatim Strings (`$V`)

Emits a string with minimal escaping — interpolation sigils are preserved. Use this when generating code that uses the target language's string interpolation.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let block = sigil_quote!(Bash {
    echo $V("$HOME/.config")
}).unwrap();
// Output: echo "$HOME/.config"
// (Compare with $S which would produce: echo "\$HOME/.config")
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let block = sigil_quote!(TypeScript {
    const greeting = $V("Hello, ${name}!");
}).unwrap();
// Output: const greeting = `Hello, ${name}!`;
# }
```

Complex shell patterns — braced defaults, command substitution, arithmetic:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let block = sigil_quote!(Bash {
    local config_dir = $V("${XDG_CONFIG_HOME:-$HOME/.config}")
    local version = $V("$(cat ${PROJECT_ROOT}/VERSION)")
    local next_port = $V("$((BASE_PORT + ${#services[@]}))")
    echo $V("Deploying ${APP_NAME} v${version} (PID=$$)")
}).unwrap();
// Output:
//   local config_dir = "${XDG_CONFIG_HOME:-$HOME/.config}"
//   local version = "$(cat ${PROJECT_ROOT}/VERSION)"
//   local next_port = "$((BASE_PORT + ${#services[@]}))"
//   echo "Deploying ${APP_NAME} v${version} (PID=$$)"
# }
```

#### `@{expr}` interpolation

Embed Rust expressions inside `$V` or `$L` string literals with `@{expr}`. These are resolved at compile time while the rest passes through for the target language's runtime:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let registry = "ghcr.io/myorg";
let app = "api";
let block = sigil_quote!(Bash {
    docker push $V("@{registry}/@{app}:${TAG}")
}).unwrap();
// Output: docker push ghcr.io/myorg/api:${TAG}
# }
```

Use `$V` when the output should be wrapped in the target language's string delimiter; use `$L` when you need plain unwrapped text (e.g., type expressions, switch headers).

Use `@@` to emit a literal `@`. Bare `@` not followed by `{` passes through unchanged. Works with all languages.

### Literals (`$L`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let default_val = "0";
let block = sigil_quote!(TypeScript {
    const count = $L(default_val);
}).unwrap();
// Output: const count = 0;
# }
```

`$L` can also splice structured code via `CodeBlock` or `CodeFragment`. Use
`CodeFragment` when the snippet contains format markers such as `%>` / `%<` and
must carry indentation state instead of rendering those markers as text:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::python::Python;
# use sigil_stitch::spec::file_spec::FileSpec;
# fn main() {
let early_return = CodeFragment::of("if enabled:\n%>return value%<", ()).unwrap();

let block = sigil_quote!(Python {
    def choose(enabled: bool, value: str) -> str: {
        $L(early_return)
        return "fallback"
    }
}).unwrap();

let output = FileSpec::builder_with("demo.py", Python::new())
    .add_code(block)
    .build()
    .unwrap()
    .render(80)
    .unwrap();

assert!(output.contains("if enabled:\n        return value"));
# }
```

Raw strings passed through `$L` are not reparsed. A raw string containing `%>` or
`%<` fails with `UnresolvedIndentMarker`; wrap that snippet in `CodeFragment::of`
when the markers are intended to control indentation.

`CodeFragment` must have balanced indentation markers. Write `%>...%<` inside the
fragment, not `%>...` with the expectation that the caller will dedent later.

### Nested Code Blocks (`$C`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let inner = CodeBlock::of("doSomething()", ()).unwrap();
let block = sigil_quote!(TypeScript {
    $C(inner);
}).unwrap();
// Output: doSomething();
# }
```

### Dollar Escape (`$$`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let block = sigil_quote!(TypeScript {
    const price = $$100;
}).unwrap();
// Output contains: $ 100
// Note: the tokenizer inserts a space between $ and 100
# }
```

## Statement Rules

The macro classifies each line based on how it ends:

### Semicolons: `add_statement()`

Lines ending with `;` become statement calls (the renderer adds the language's
statement terminator):

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    const x = 1;        // -> add_statement("const x = 1", ())
    const y = x + 1;    // -> add_statement("const y = x + 1", ())
})?;
# Ok(())
# }
```

### Brace Groups: Control Flow

Lines ending with `{ ... }` (without a trailing `;`) become control flow:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    if (x > 0) {            // -> begin_control_flow("if(x > 0)", ())
        return true;         // -> add_statement("return true", ())
    }                        // -> end_control_flow()
})?;
# Ok(())
# }
```

### Object Literals vs Control Flow

A `{ ... }` followed by `;` is treated as part of a statement, not control flow.
This is how the macro distinguishes object literals:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    const config = { timeout: 5000 };    // statement (has trailing ;)
    if (ready) {                          // control flow (no trailing ;)
        start();
    }
})?;
# Ok(())
# }
```

### Blank Lines: `add_line()`

Blank lines in the macro body insert visual separators:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    const a = 1;

    const b = 2;    // blank line above becomes add_line()
})?;
# Ok(())
# }
```

### Comments: `$comment(expr)`

Rust's proc macro tokenizer strips `//` comments, so they're invisible to the macro.
Use `$comment()` instead. The argument can be any Rust expression that evaluates to
something displayable — a string literal, a variable, `format!(...)`, or any type
implementing `ToString`.

**Statement-level comments** appear at the start of a line:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    $comment("Initialize the connection pool");
    const pool = createPool();
})?;
// Output:
// // Initialize the connection pool
// const pool = createPool();
# Ok(())
# }
```

**Dynamic expressions** work as the argument:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let msg = "Initialize the connection pool";
sigil_quote!(TypeScript {
    $comment(msg);
    const pool = createPool();
})?;
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let name = "Foo";
sigil_quote!(TypeScript {
    $comment(format!("Class: {name}"));
    const x = 0;
})?;
# Ok(())
# }
```

**Inline comments** appear after a statement on the same line:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let msg = "cleanup";
sigil_quote!(TypeScript {
    doStuff($S("x")) $comment(msg)
})?;
// Output: doStuff('x') // cleanup
# Ok(())
# }
```

#### `@{expr}` interpolation

Embed Rust expressions inside `$comment` string literals with `@{expr}`. These are
resolved at compile time:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let name = "World";
sigil_quote!(TypeScript {
    $comment("Hello @{name}");
    const x = 0;
})?;
// Output: // Hello World
# Ok(())
# }
```

`@{...}` interpolation also works in inline comments:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let count = 42;
sigil_quote!(TypeScript {
    doStuff() $comment("processed @{count} items")
})?;
// Output: doStuff() // processed 42 items
# Ok(())
# }
```

Use `@@` to emit a literal `@`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    $comment("user@@host");
})?;
// Output: // user@host
# Ok(())
# }
```

An optional trailing `;` after `$comment(...)` is consumed silently and does not
affect the output.

### Annotations (`$attr`)

`$attr("text")` emits a structural annotation/attribute rendered with the target
language's syntax. This keeps the macro body language-agnostic — write `$attr("override")`
and it renders as `@override` in TypeScript/Java, `#[override]` in Rust, or
`[[override]]` in C++.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    $attr("injectable()");
    class MyService {}
})?;
// Output:
// @injectable()
// class MyService {}
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::rust::Rust;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Rust {
    $attr("derive(Debug, Clone, Serialize, Deserialize)");
    struct Config {}
})?;
// Output:
// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct Config {}
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::cpp::Cpp;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Cpp {
    $attr("nodiscard");
    Result compute();
})?;
// Output: [[nodiscard]] Result compute();
# Ok(())
# }
```

Each language defines its own prefix/suffix via `attribute_syntax()`. Stacking
multiple `$attr` lines is common:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    $attr("injectable()");
    $attr("singleton()");
    class AppService {}
})?;
// Output:
// @injectable()
// @singleton()
// class AppService {}
# Ok(())
# }
```

`$attr` works inside `$if` blocks for conditional annotations:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::rust::Rust;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let needs_serde = true;
sigil_quote!(Rust {
    $attr("derive(Debug, Clone)");
    $if(needs_serde) {
        $attr("serde(rename_all = \"camelCase\")");
    }
    struct Config {}
})?;
# Ok(())
# }
```

## Control Flow

### if / else / else if

The macro detects `else` and `else if` chains after closing braces:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
})?;
# Ok(())
# }
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

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    for (const item of items) {
        process(item);
    }
})?;
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    try {
        riskyOperation();
    } catch (e) {
        handleError(e);
    }
})?;
# Ok(())
# }
```

### Nested Control Flow

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    if (users.length > 0) {
        for (const user of users) {
            if (user.active) {
                process(user);
            }
        }
    }
})?;
# Ok(())
# }
```

### Interpolation in Conditions

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let error_type = TypeName::importable_type("./errors", "NotFoundError");
sigil_quote!(TypeScript {
    if (!user) {
        throw new $T(error_type)($S("not found"));
    }
})?;
# Ok(())
# }
```

## Context-Aware Block Delimiters

By default, `{ ... }` in `sigil_quote!` uses the language's `block_syntax().block_open`.
Language backends can override the opener and closer per condition via `block_open_for`
and `block_close_for`. For example, Bash maps `if` → `then`/`fi` and `for` → `do`/`done`,
while Haskell maps `class` → `where`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::haskell::Haskell;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Haskell type class — block_open_for returns " where" for "class ..."
sigil_quote!(Haskell {
    class Functor f {
        fmap :: (a -> b) -> f a -> f b;
    }
})?;
// Output: class Functor f where
//             fmap :: (a -> b) -> f a -> f b
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::ocaml::OCaml;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
// OCaml module — block_open_for returns " = struct" for "module ..."
sigil_quote!(OCaml {
    module Foo {
        let x = 42;
    }
})?;
// Output: module Foo = struct
//             let x = 42
# Ok(())
# }
```

Bash maps control-flow keywords to their shell delimiters:

| Condition | Open | Close |
|-----------|------|-------|
| `if ...` | `; then` | `fi` |
| `for ...` | `; do` | `done` |
| `while ...` | `; do` | `done` |
| `else` | `""` | `""` |
| `elif ...` | `; then` | `""` |

Lua similarly maps `if` → `then`/`end` and `for`/`while` → `do`/`end`.

## Manual Indent / Dedent (`$>` / `$<`)

Use `$>` and `$<` as standalone directives to control indent level without
control flow blocks:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    namespace Foo {
    $>
    const x = 1;
    const y = 2;
    $<
    }
})?;
// Output:
// namespace Foo {
//     const x = 1;
//     const y = 2;
// }
# Ok(())
# }
```

These map to the `%>` and `%<` format specifiers in `CodeBlockBuilder`.

## Splicing Code Block Iterables (`$C_each`)

`$C_each(expr)` iterates over a collection of `CodeBlock` values and splices each
one into the builder sequentially. It must appear at the start of a line.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
# let fields = vec!["name", "age"];
let blocks: Vec<CodeBlock> = fields
    .iter()
    .map(|f| CodeBlock::of(&format!("this.{f} = null"), ()).unwrap())
    .collect();

let _ = sigil_quote!(TypeScript {
    $C_each(blocks);
});
// Output:
// this.name = null;
// this.age = null;
# }
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

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let include_debug = true;

sigil_quote!(TypeScript {
    const x = 1;
    $if(include_debug) {
        console.log($S("debug: x ="), x);
    }
})?;
// When include_debug is true, output includes the console.log line.
// When false, it's omitted entirely.
# Ok(())
# }
```

### `$else_if` and `$else`

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mode = "production";

sigil_quote!(TypeScript {
    $if(mode == "debug") {
        console.log($S("debug mode"));
    } $else_if(mode == "test") {
        console.log($S("test mode"));
    } $else {
        console.log($S("production mode"));
    }
})?;
# Ok(())
# }
```

The conditions are arbitrary Rust expressions evaluated at runtime. The braces
delimit which `sigil_quote!` statements are conditionally included — they do **not**
produce target-language block syntax.

### Nesting with Target-Language Control Flow

Meta-conditionals can wrap target-language control flow and vice versa:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let use_guard = true;

sigil_quote!(TypeScript {
    $if(use_guard) {
        if (!user) {
            throw new Error($S("unauthorized"));
        }
    }
})?;
# Ok(())
# }
```

## Meta-Loops (`$for`)

`$for` iterates over a Rust collection at compile time, emitting the body statements
once per iteration. Like `$if`, it controls **which builder calls are made** — it
does not produce target-language loop syntax.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let fields = vec!["name", "age", "email"];

sigil_quote!(TypeScript {
    $for(f in &fields) {
        this.$N(*f) = null;
    }
})?;
// Output:
// this.name = null;
// this.age = null;
// this.email = null;
# Ok(())
# }
```

### Destructuring Patterns

Any Rust `for` pattern works:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let entries = vec![("x", "number"), ("y", "string")];

sigil_quote!(TypeScript {
    $for((name, ty) in &entries) {
        let $N(*name): $L(*ty);
    }
})?;
// Output:
// let x: number;
// let y: string;
# Ok(())
# }
```

### Nesting

`$for` can nest inside `$if` and vice versa, and can contain target-language
control flow:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let variants = vec!["A", "B", "C"];

sigil_quote!(TypeScript {
    $for(v in &variants) {
        case $S(*v):
            return $S(*v);
    }
})?;
# Ok(())
# }
```

### Combining with Interpolation Markers

All interpolation markers (`$T`, `$N`, `$S`, `$L`, `$C`, `$W`, `$join`) work
inside `$for` bodies, and the loop variable is in scope:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn get_types() -> Vec<TypeName> { vec![TypeName::primitive("User")] }
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let types: Vec<TypeName> = get_types();

sigil_quote!(TypeScript {
    $for(t in &types) {
        import type { $T(t.clone()) };
    }
})?;
# Ok(())
# }
```

### Inline Expressions

`$for` and `$if` also work **inline** — inside parenthesized groups, array
literals, object literals, and function arguments. They no longer need to be
at column 0:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let items = vec!["hostname", "platform", "arch"];

sigil_quote!(TypeScript {
    const defaultKeys = [$for(item in &items) { $S(*item), }];
})?;
// Output: const defaultKeys = ['hostname', 'platform', 'arch'];
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let is_admin = true;

sigil_quote!(TypeScript {
    setPermissions($if(is_admin) { "read-write" } $else { "read-only" });
})?;
// Output: setPermissions("read-write");
# Ok(())
# }
```

The `$if` / `$else_if` / `$else` chain also works inline:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let level: u32 = 2;

sigil_quote!(TypeScript {
    const label = $if(level == 0) { "trace" } $else_if(level == 1) { "debug" } $else { "info" };
})?;
// Output: const label = "info";
# Ok(())
# }
```

Inline meta-directives produce `ParsedSplice` output — the body is spliced
directly into place without synthetic block delimiters. This means no stray
`{}` in C-like languages and no stray `:` in Python.

## Meta-Bindings (`$let`)

`$let` introduces a Rust-level `let` binding inside the macro body. It emits a
real `let` statement in the generated Rust code, making it possible to compute
intermediate values — including fallible expressions with `?` — inside `$for` and
`$if` bodies.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let fields = vec![("name", "String"), ("age", "u32")];

sigil_quote!(TypeScript {
    $for((name, ty) in &fields) {
        $let(upper = name.to_uppercase());
        const $N(upper): $L(*ty);
    }
})?;
// Output:
// const NAME: String;
// const AGE: u32;
# Ok(())
# }
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
    let block = sigil_quote!(Rust {
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

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let items = vec!["a", "b", "c"];

sigil_quote!(TypeScript {
    const values = [$join(", ", items)];
})?;
// Output: const values = [a, b, c];
# Ok(())
# }
```

The separator is any Rust expression that evaluates to something accepted by
`Vec<String>::join()` (typically a `&str`). Each item is converted via `ToString`.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let fields = vec!["name", "age", "email"];
let assignments: Vec<String> = fields.iter().map(|f| format!("this.{f} = {f}")).collect();

sigil_quote!(TypeScript {
    $join(";\n", assignments)
})?;
// Output:
// this.name = name;
// this.age = age;
// this.email = email
# Ok(())
# }
```

### Type Join (`$T_join`)

`$T_join(sep, iter)` joins `TypeName` items with a separator, tracking imports for
each item. Unlike `$join` (which calls `.to_string()` on each element), `$T_join`
uses `%T` slots so every type in the join contributes its import to the file.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let types = vec![
    TypeName::importable_type("./models", "User"),
    TypeName::importable_type("./models", "Admin"),
    TypeName::primitive("null"),
];
sigil_quote!(TypeScript {
    export type Actor = $T_join(" | ", &types);
})?;
// Output: export type Actor = User | Admin | null;
// Imports: import type { Admin, User } from './models'
# Ok(())
# }
```

The separator can be any string — `" | "` for TypeScript unions, `" & "` for
intersections, `" + "` for Rust trait bounds, `"\n"` for Go interface embedding:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::rust::Rust;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let traits = vec![
    TypeName::importable_type("./traits", "Serializable"),
    TypeName::importable_type("./traits", "Cloneable"),
];
sigil_quote!(Rust {
    fn process(stream: &mut (dyn $T_join(" + ", &traits))) {}
})?;
// Output: fn process(stream: &mut (dyn Serializable + Cloneable)) {}
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::go::Go;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let ifaces = vec![
    TypeName::importable_type("./io", "Reader"),
    TypeName::importable_type("./io", "Writer"),
];
sigil_quote!(Go {
    type FileOps interface {
        $T_join("\n", &ifaces)
    }
})?;
// Output:
// type FileOps interface {
//     Reader
//     Writer
// }
# Ok(())
# }
```

## Line Continuation (`$+`)

`sigil_quote!` splits statements on line breaks — each source line becomes a
separate statement in the generated code. This works well for languages like
Kotlin and Python where each line is typically a statement.

For expressions that span multiple lines (common in Haskell, OCaml, or long
function calls), place `$+` at the end of a line to suppress the split and
continue the statement on the next line:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::haskell::Haskell;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Haskell {
    mapM_ $+
        putStrLn $+
        items
})?;
// Output: mapM_ putStrLn items
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::kotlin::Kotlin;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Kotlin {
    val result = someFunction( $+
        arg1, arg2);
})?;
// Output: val result = someFunction(arg1, arg2);
# Ok(())
# }
```

Without `$+`, each source line becomes its own statement. For semicolon-based
languages, `;` still takes priority as the statement terminator regardless of
line breaks.

## Multi-Language Support

The same syntax works with any language type:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::python::Python;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Python {
    if x > 0:
        return True
})?;
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::go::Go;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Go {
    x := 42;
})?;
# Ok(())
# }
```

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(Rust {
    let x: i32 = 42;
})?;
# Ok(())
# }
```

## Paren-Delimited Blocks (Go)

Go uses parenthesized blocks for multi-line declarations — `const ( ... )`,
`var ( ... )`, `import ( ... )`, and `type ( ... )`. `sigil_quote!` recognizes
these as structural blocks, so `$for`, `$if`, `$C_each`, and other directives
expand inside them:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::go::Go;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let variants = vec!["A", "B", "C"];

sigil_quote!(Go {
    const (
    $for(v in &variants) {
        $L("@{v}Kind @{v} = \"@{v}\"");
    }
    )
})?;
// Output:
// const (
//     AKind A = "A"
//     BKind B = "B"
//     CKind C = "C"
// )
# Ok(())
# }
```

The paren-block body is indented automatically (the codegen emits `%>` after the
opening header and `%<` before the closing `)`). Interpolation markers, meta-loops,
and meta-conditionals all work normally inside the block.

This detection is language-aware — only `Go` recognizes `const`, `var`,
`import`, and `type` as paren-block headers. In other languages, `const ( ... )`
is treated as a plain statement.

## Known Limitations and Quirks

### Language-Aware Tokenization

`sigil_quote!` recognizes certain language identifiers and applies language-specific spacing
rules at compile time. For example, shell languages (Bash, Zsh) get correct handling of flags
(`-q`, `--amend`), paths (`/usr/local/bin`), and standalone dots (`find .`). Go gets tight
`<-ch` channel receive, and Haskell gets correct `$ operator` spacing.

Languages without dedicated support use universal heuristics that handle most cases correctly.
See [Language-Aware Tokenizer (MacroLang)](macrolang.md) for the full design.

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

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
sigil_quote!(TypeScript {
    const x: $T(TypeName::primitive("string")) = $S("hello".to_uppercase());
})?;
# Ok(())
# }
```

### Blank Line Detection

Blank line detection uses `proc_macro2` span locations. It requires the
`span-locations` feature (enabled by the macros crate). If spans aren't available,
blank lines may not be detected.
