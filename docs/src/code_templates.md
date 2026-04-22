# Code Templates

`CodeTemplate` provides named parameters on top of `CodeBlock`'s positional format strings. Templates are language-agnostic: you define the pattern once, then apply it with concrete arguments for any target language.

## Syntax

Templates use `#{name:K}` for named parameters, where `K` specifies the kind:

| Kind | Specifier | Argument Type |
|------|-----------|---------------|
| `T`  | `%T`      | `TypeName<L>` |
| `N`  | `%N`      | `NameArg`     |
| `S`  | `%S`      | `StringLitArg`|
| `L`  | `%L`      | `&str`, `String`, or `CodeBlock<L>` |

Use `##` to emit a literal `#` character.

Bare positional specifiers (`%T`, `%N`, etc.) are rejected in templates. You must use the named `#{...}` syntax.

## Basic Usage

```rust,ignore
use sigil_stitch::code_template::CodeTemplate;
use sigil_stitch::code_block::NameArg;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::type_name::TypeName;

let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();

let block = tmpl.apply::<TypeScript>()
    .set("var", NameArg("user".into()))
    .set("type", TypeName::<TypeScript>::primitive("string"))
    .set("init", "null")
    .build()
    .unwrap();
// Output: const user: string = null
```

The template is parsed once by `CodeTemplate::new()`. The language enters at `.apply::<TypeScript>()` time.

## Reuse Across Types

The same template works for different types and values:

```rust,ignore
let field_tmpl = CodeTemplate::new("#{name:N}: #{type:T}").unwrap();

// Apply for a string field
let string_field = field_tmpl.apply::<TypeScript>()
    .set("name", NameArg("username".into()))
    .set("type", TypeName::<TypeScript>::primitive("string"))
    .build()
    .unwrap();

// Apply for a number field
let number_field = field_tmpl.apply::<TypeScript>()
    .set("name", NameArg("age".into()))
    .set("type", TypeName::<TypeScript>::primitive("number"))
    .build()
    .unwrap();
```

## Reuse Across Languages

Since templates are language-agnostic, the same template can target different languages:

```rust,ignore
use sigil_stitch::lang::rust_lang::RustLang;

let decl = CodeTemplate::new("#{name:N}: #{type:T} = #{value:L}").unwrap();

// TypeScript
let ts_block = decl.apply::<TypeScript>()
    .set("name", NameArg("count".into()))
    .set("type", TypeName::<TypeScript>::primitive("number"))
    .set("value", "0")
    .build()
    .unwrap();

// Rust
let rs_block = decl.apply::<RustLang>()
    .set("name", NameArg("count".into()))
    .set("type", TypeName::<RustLang>::primitive("i32"))
    .set("value", "0")
    .build()
    .unwrap();
```

## Duplicate Parameters

The same parameter name can appear multiple times. The value you set is used at each occurrence:

```rust,ignore
let tmpl = CodeTemplate::new("#{type:T} -> #{type:T}").unwrap();

let block = tmpl.apply::<TypeScript>()
    .set("type", TypeName::<TypeScript>::primitive("string"))
    .build()
    .unwrap();
// Output: string -> string
```

## Import Tracking

Templates using `#{name:T}` track imports just like `%T` in CodeBlocks. When the resulting CodeBlock is rendered inside a FileSpec, all type references are collected for the import header:

```rust,ignore
let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = new #{type:T}()").unwrap();
let user = TypeName::<TypeScript>::importable_type("./models", "User");

let block = tmpl.apply::<TypeScript>()
    .set("var", NameArg("user".into()))
    .set("type", user)
    .build()
    .unwrap();
// When rendered: import type { User } from './models'
// Output:        const user: User = new User()
```

## Validation

`.build()` validates that:
- All parameters have been set (missing parameters produce an error)
- Argument kinds match the parameter kind (`#{name:T}` must receive a `TypeName`, not a string)

```rust,ignore
let tmpl = CodeTemplate::new("#{name:N}: #{type:T}").unwrap();

// Missing parameter
let result = tmpl.apply::<TypeScript>()
    .set("name", NameArg("x".into()))
    // forgot to set "type"
    .build();
assert!(result.is_err());
```

## Introspection

Use `param_names()` to inspect a template's parameters:

```rust,ignore
let tmpl = CodeTemplate::new("#{name:N}: #{type:T} = #{init:L}").unwrap();
let params = tmpl.param_names();
// [("name", ParamKind::Name), ("type", ParamKind::Type), ("init", ParamKind::Literal)]
```

## When to Use Templates vs CodeBlock

- **CodeBlock**: When you're building code imperatively and the structure varies at runtime.
- **CodeTemplate**: When you have a fixed pattern that gets reused with different values. Templates make the pattern explicit and prevent positional argument errors.
- **sigil_quote!**: When you can write the target code inline at compile time.
