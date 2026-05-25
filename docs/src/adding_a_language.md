# Adding a Language

sigil-stitch supports new languages by implementing two traits: `RendererLang` (renderer-only methods) and `CodeLang` (spec-layer methods). `CodeLang` extends `RendererLang`, so implementing `CodeLang` requires both. If you only need `CodeBlock`-level rendering without specs, `RendererLang` alone is sufficient.

The `RendererLang` trait has 14 methods covering rendering essentials. `CodeLang` adds the spec-layer methods: 4 required plus 6 config struct accessors and override methods — all with sensible defaults. You only need to override the defaults when your language diverges from the common patterns.

This guide walks through the process using a hypothetical language, with references to real implementations you can study.

## Overview

Adding a language takes four steps:

1. Create `src/lang/your_lang.rs` implementing `CodeLang`
2. Add `pub mod your_lang;` to `src/lang/mod.rs`
3. Write integration tests in `tests/`
4. Run `just bless` to generate golden files

If your language has tokenizer conflicts in `sigil_quote!` that the universal heuristics
can't handle (e.g., shell flags, Go channel operators), you may also need to add a
`MacroLang` variant. See [Language-Aware Tokenizer](macrolang.md) for details.

## The RendererLang Trait

These methods are used by the renderer (`code_renderer.rs`) and type rendering:

### Core Methods (6 required)

These are enough for CodeBlock-level code generation:

| Method | Example (TypeScript) | Purpose |
|--------|---------------------|---------|
| `file_extension()` | `"ts"` | File extension for output files |
| `reserved_words()` | `&["async", "await", ...]` | Words that need escaping |
| `render_imports()` | `import { Foo } from '...'` | Emit the import header |
| `render_string_literal()` | `'hello'` | Language-specific string quoting |
| `render_doc_comment()` | `/** ... */` | Doc comment block |
| `line_comment_prefix()` | `"//"` | Single-line comment prefix |

### Override Methods (with defaults)

| Method | Default | Purpose |
|--------|---------|---------|
| `render_verbatim_string()` | Delegates to `render_string_literal()` | Minimal escaping for interpolated strings |

Override `render_verbatim_string()` if your language has string interpolation (e.g., Bash `"$x"`, TypeScript `` `${x}` ``, Python `f"{x}"`).

`render_imports()` is the most complex. It receives an `ImportGroup` (deduplicated, with aliases resolved) and must emit the full import header string. Study `src/lang/typescript.rs` for ES module imports or `src/lang/rust.rs` for `use` paths.

## The CodeLang Trait

Extends `RendererLang` with the additional methods needed by the spec layer.

### Spec Support Methods (4 required)

These enable TypeSpec, FunSpec, and FieldSpec rendering:

| Method | Example | Purpose |
|--------|---------|---------|
| `render_visibility()` | `"public "`, `"pub "` | Visibility prefix |
| `function_keyword()` | `"function"`, `"fn"` | Function declaration keyword |
| `type_keyword()` | `"class"`, `"struct"` | Type declaration keyword |
| `methods_inside_type_body()` | `true` / `false` | Key structural decision (see below) |

#### The `methods_inside_type_body` Decision

This is the most important method for structural correctness. It determines whether TypeSpec emits one CodeBlock or two:

- **Returns `true`** (TypeScript, Java, Python, Swift, Dart, Kotlin, C++): Methods go inside the type body. TypeSpec emits a single block: `class Foo { fields; methods; }`.
- **Returns `false`** (Rust struct/enum): Methods go in a separate `impl` block. TypeSpec emits two blocks: `struct Foo { fields }` and `impl Foo { methods }`.

The method takes a `TypeKind` parameter, so you can vary by type. Rust returns `true` for `TypeKind::Trait` (trait methods go inside) but `false` for `TypeKind::Struct` and `TypeKind::Enum`.

### Config Struct Accessors and Default Methods

Instead of dozens of individual trait methods, the v2.0 API groups related configuration into 6 config structs returned by accessor methods. Each struct uses `..Default::default()` so you only specify fields where your language differs. The remaining standalone override methods cover cases that don't fit neatly into a struct.

#### `block_syntax()`

Returns `BlockSyntaxConfig` controlling block delimiters and formatting:

| Field | Default | Purpose |
|-------|---------|---------|
| `block_open` | `" {"` | Opening delimiter. Python overrides to `":"`. |
| `block_close` | `"}"` | Closing delimiter. Python overrides to `""` (indent-only). |
| `indent_unit` | `"  "` (2 spaces) | Indentation per level. |
| `uses_semicolons` | `true` | Statement terminator behavior. |
| `field_terminator` | `","` | After each field. Java/C++ override to `";"`. |
| `type_close_terminator` | (default) | Terminator after closing brace for types. |
| `bases_close` | (default) | Closing syntax for base-class lists. |

#### `function_syntax()`

Returns `FunctionSyntaxConfig` controlling function declarations:

| Field | Default | Purpose |
|-------|---------|---------|
| `return_type_separator` | `": "` | Between params and return type. Rust overrides to `" -> "`. |
| `async_keyword` | `"async "` | Async function prefix. |
| `async_suffix` | `""` | Async suffix after params. Dart: `" async"`. |
| `async_suffix_before_return` | `false` | When `true`, suffix goes before return type. Swift: `func f() async -> T`. |
| `abstract_keyword` | `"abstract "` | Abstract method prefix. C++ overrides to `"virtual "`. |
| `param_list_style` | (default) | How parameter lists are formatted. |
| `function_signature_style` | (default) | Controls overall signature layout. |
| `constructor_keyword` | `""` | Constructor keyword. Python: `"def"`. Rust: `"fn"`. |
| `constructor_delegation_style` | (default `Body`) | Super/this call placement. Kotlin: `Signature`. |
| `where_clause_style` | `Inline` | `Inline`: bounds in `<T: Bound>`. `WhereBlock`: Rust `where\n    T: Bound,`. `SeparateWhere`: C# `where T : Bound` per constraint. |
| `empty_body` | `""` | Empty method body. Python overrides to `"..."`. |

#### `type_decl_syntax()`

Returns `TypeDeclSyntaxConfig` controlling type declarations:

| Field | Default | Purpose |
|-------|---------|---------|
| `type_before_name` | `false` | C/C++/Java override to `true` for `int count`. |
| `return_type_is_prefix` | `false` | C/C++/Java override to `true` for `int add(...)`. |
| `type_annotation_separator` | `": "` | Between name and type annotation. |
| `super_type_keyword` | (default) | Inheritance keyword, e.g. `" extends "`. |
| `super_type_separator` | (default) | Separator between multiple super types. |
| `super_type_subsequent_separator` | (default) | Separator for subsequent super types. |
| `implements_keyword` | (default) | Interface keyword, e.g. `" implements "`. |
| `type_alias_target_first` | `false` | C overrides to `true` for `typedef target name;`. |
| `supports_primary_constructor` | `false` | Kotlin overrides to `true`. |

#### `generic_syntax()`

Returns `GenericSyntaxConfig` controlling generic/type-parameter syntax:

| Field | Default | Purpose |
|-------|---------|---------|
| `open` | `"<"` | Generic opening bracket. Go overrides to `"["`. |
| `close` | `">"` | Generic closing bracket. Go overrides to `"]"`. |
| `application_style` | (default) | How generics are applied to types. |
| `constraint_keyword` | `": "` | Generic bounds keyword. Java/TS override to `" extends "`. |
| `constraint_separator` | `" + "` | Between multiple bounds. Java/TS override to `" & "`. |
| `context_bound_keyword` | (default) | Context bound syntax (e.g. Scala's `:`). |

#### `enum_and_annotation()`

Returns `EnumAndAnnotationConfig` controlling enums, annotations, and field modifiers:

| Field | Default | Purpose |
|-------|---------|---------|
| `variant_prefix` | `""` | Enum variant prefix. Swift overrides to `"case "`. |
| `variant_prefix_first` | (default) | Prefix for the first variant specifically. |
| `variant_separator` | `","` | Between enum variants. Python/Swift override to `""`. |
| `variant_trailing_separator` | `false` | Rust/TypeScript override to `true`. |
| `annotation_prefix` | `"@"` | Annotation opening. Rust: `"#["`. C++: `"[["`. |
| `annotation_suffix` | `""` | Annotation closing. Rust: `"]"`. C++: `"]]"`. |
| `readonly_keyword` | `"const "` | TS: `"readonly "`. Kotlin: `"val "`. Java: `"final "`. |
| `mutable_field_keyword` | `""` | Kotlin overrides to `"var "`. |

#### `type_presentation()`

Returns `TypePresentationConfig` controlling how semantic types (arrays, optionals, maps, tuples, references, function types, etc.) are rendered. See the [Type Presentation](#type-presentation) section below for details.

#### Standalone Override Methods

These methods don't belong to a config struct but have sensible defaults you can override:

- `escape_reserved()` -- how reserved words are escaped.
- `qualify_import_name()` -- default passthrough. Go overrides to return `"http.Server"` (package-qualified names).
- `module_separator()` -- returns `Option<&str>`. Default `None`. Override to `Some("::")` (Rust/C++) or `Some(".")` (Go/Python/Java/etc.) to enable `TypeName::qualified()` inline rendering.
- `type_kind_suffix()` -- suffix after type close for specific type kinds.
- `render_newtype_line()` -- default emits Rust tuple struct `struct Name(Inner);`. Go: `type Name Inner`, Kotlin: `value class Name(val value: Inner)`, Python: `Name = NewType("Name", Inner)`, C: `typedef Inner Name;`.
- `fun_block_open()` -- custom block opener for functions.
- `type_header_block_open()` -- custom block opener for type headers.
- `doc_comment_inside_body()` -- whether doc comments go inside the body (Python docstrings).
- `doc_before_annotations()` -- whether doc comments appear before annotations.
- `optional_field_style()` -- how optional fields are represented.
- `property_style()` -- default `Accessor` (TS/JS: `get name()`). Swift/Kotlin: `Field` (inline get/set).
- `property_getter_keyword()` -- default `"get"`. Kotlin: `"get()"`.
- `render_type_context()` -- additional context for type rendering.
- `type_body_prefix()` -- content emitted before the type body.
- `type_body_suffix()` -- content emitted after the type body.
- `render_type_close_suffix()` -- suffix after type close brace.
- `render_type_param_kind()` -- how type parameters are annotated with variance.
- `line_comment_suffix()` -- suffix for line comments (default `""`).

## Step-by-Step Walkthrough

### 1. Create the language file

Create `src/lang/your_lang.rs`:

```rust,ignore
use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

#[derive(Debug, Clone, Default)]
pub struct YourLang;

impl YourLang {
    pub fn new() -> Self {
        Self
    }
}

const RESERVED: &[&str] = &["if", "else", "for", "while", /* ... */];

impl CodeLang for YourLang {
    fn file_extension(&self) -> &str { "yl" }
    fn reserved_words(&self) -> &[&str] { RESERVED }
    fn line_comment_prefix(&self) -> &str { "//" }

    fn render_string_literal(&self, s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let mut out = String::from("/**\n");
        for line in lines {
            out.push_str(&format!(" * {line}\n"));
        }
        out.push_str(" */\n");
        out
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        // Build your import statements from imports.by_module()
        let mut out = String::new();
        for (module, entries) in imports.by_module() {
            let names: Vec<&str> = entries.iter().map(|e| e.resolved_name.as_str()).collect();
            out.push_str(&format!("import {{ {} }} from \"{}\";\n", names.join(", "), module));
        }
        out
    }

    // Spec support methods...
    fn render_visibility(&self, vis: Visibility, _ctx: DeclarationContext) -> &str {
        match vis {
            Visibility::Public => "public ",
            Visibility::Private => "private ",
            Visibility::Protected => "protected ",
            _ => "",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str { "function" }
    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum => "enum",
            TypeKind::Struct => "class",
            TypeKind::TypeAlias => "type",
            TypeKind::Newtype => "class",
        }
    }
    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool { true }

    // Config struct overrides...
    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            uses_semicolons: true,
            indent_unit: "    ",
            field_terminator: ";",
            ..Default::default()
        }
    }
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            super_type_keyword: " extends ",
            implements_keyword: " implements ",
            ..Default::default()
        }
    }
    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            constraint_keyword: " extends ",
            constraint_separator: " & ",
            ..Default::default()
        }
    }
    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: ": ",
            ..Default::default()
        }
    }
}
```

### 2. Register the module

Add to `src/lang/mod.rs`:
```rust,ignore
/// YourLang language support.
pub mod your_lang;
```

### 3. Write tests

Create a test directory `tests/your_lang/` with a `main.rs` entry point and submodules:

**`tests/your_lang/main.rs`**:
```rust,ignore
mod golden;

mod quote_basic;
mod builder_basic;
```

**`tests/your_lang/quote_basic.rs`** -- `sigil_quote!` macro tests:
```rust,ignore
use sigil_stitch::prelude::*;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.yl")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic_statement() {
    let block = sigil_quote!(YourLang {
        const x = 1;
    });
    golden::assert_golden("your_lang/basic_statement.yl", &render(&block));
}
```

**`tests/your_lang/builder_basic.rs`** -- builder API tests (CodeBlock, TypeSpec, FunSpec, FileSpec).

### 4. Generate golden files

```bash
just bless
```

This runs all tests with `BLESS=1`, which creates `test-goldens/your_lang/*.yl` files from the actual output. Review them manually, then commit.

### 5. Override defaults

Run the full test suite and review golden file output. Override config struct accessors and default methods where your language's syntax differs. Common overrides:

- If your language uses indentation instead of braces: override `block_syntax()` to set `block_open`, `block_close`; override `function_syntax()` to set `empty_body`
- If types come before names (`int x` instead of `x: int`): override `type_decl_syntax()` to set `type_before_name`, `return_type_is_prefix`
- If generics use brackets instead of angle brackets: override `generic_syntax()` to set `open`, `close`

## Reference Implementations

Study these existing implementations for patterns similar to your target:

| Language | File | Notable Patterns |
|----------|------|-----------------|
| TypeScript | `src/lang/typescript.rs` | ES module imports, type-only imports, single-quoted strings |
| Rust | `src/lang/rust.rs` | `use` paths, struct+impl split, `pub(crate)` visibility |
| Python | `src/lang/python.rs` | Indent-only blocks (no braces), docstrings inside body, `from x import y` |
| Go | `src/lang/go.rs` | Package-qualified names (`http.Server`), bracket generics, `func` keyword |
| C | `src/lang/c.rs` | Type-before-name, `#include`, `__attribute__`, struct close semicolon |
| C++ | `src/lang/cpp.rs` | `virtual` instead of `abstract`, `#include` + `using`, `[[attributes]]` |
| Bash | `src/lang/bash.rs` | Keyword-based block closers (`fi`/`done`/`esac`), `source` imports, shell escaping |
| Scala | `src/lang/scala.rs` | `case class`, `trait`, `[T]` generics, `<:` bounds, `= {`/`}` blocks |
| Haskell | `src/lang/haskell.rs` | Split signature style, `where`/indentation blocks, postfix generics, `deriving` |
| OCaml | `src/lang/ocaml.rs` | Postfix generics, `let` keyword, `= `/indentation blocks, `open Module` imports, `module_block` helper |

## Type Presentation

When your language uses type expressions (generics, arrays, optionals, maps, etc.), you configure how each semantic type concept renders by returning a `TypePresentationConfig` from the `type_presentation()` accessor. You never build `BoxDoc` directly.

### How it works

Each `TypeName` variant (Array, Optional, Map, etc.) uses your language's `TypePresentationConfig` to determine the syntactic pattern via `TypePresentation` — a small enum:

- `GenericWrap { name }` — `name<P1, P2>` using your `generic_syntax().open`/`generic_syntax().close`
- `Prefix { prefix }` — `prefix inner` (e.g., Go `[]T`, Rust `*const T`)
- `Postfix { suffix }` — `inner suffix` (e.g., TypeScript `T[]`, Kotlin `T?`)
- `Surround { prefix, suffix }` — `prefix inner suffix` (e.g., C++ `const T&`, C `const T*`)
- `Delimited { open, sep, close }` — `open P1 sep P2 close` (e.g., Swift `[K: V]`, Go `map[K]V`)
- `Infix { sep }` — `P1 sep P2` (e.g., TypeScript `A | B`, Rust `A + B`)

### Configuring type presentation

All fields in `TypePresentationConfig` have defaults matching TypeScript conventions. Override only when your language differs:

```rust,ignore
impl CodeLang for YourLang {
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            // Array: default is Postfix { suffix: "[]" } (TS: T[])
            // Override for Rust-style Vec<T>:
            array: TypePresentation::GenericWrap { name: "Vec" },

            // Optional: default is Infix { sep: " | " } with "null" literal
            // Override for Kotlin-style T?:
            optional: TypePresentation::Postfix { suffix: "?" },

            // Map: default is GenericWrap { name: "Map" }
            // Override for Go-style map[K]V:
            map: TypePresentation::Delimited { open: "map[", sep: "]", close: "" },

            // Tuple: default is Delimited { open: "(", sep: ", ", close: ")" }
            // TS overrides to "[", "]" for [A, B] syntax. This shows Go-style (A, B):
            tuple: TypePresentation::Delimited { open: "(", sep: ", ", close: ")" },

            // Reference: default is Prefix { prefix: "" } (identity — for GC languages)
            // Override for Rust-style &T:
            reference: TypePresentation::Prefix { prefix: "&" },

            // Function types: default is TypeScript (A, B) => R
            function: FunctionPresentation {
                keyword: "fn",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: " -> ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },

            ..Default::default()
        }
    }
}
```

See [Type Presentation](type_presentation.md) for the full enum definition, all available fields, and examples for every supported language.
