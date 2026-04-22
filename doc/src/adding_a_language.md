# Adding a Language

sigil-stitch supports new languages by implementing the `CodeLang` trait. The trait has 45 methods: 17 required (no default implementation) and 28 with sensible defaults. You only need to override the defaults when your language diverges from the common patterns.

This guide walks through the process using a hypothetical language, with references to real implementations you can study.

## Overview

Adding a language takes four steps:

1. Create `src/lang/your_lang.rs` implementing `CodeLang`
2. Add `pub mod your_lang;` to `src/lang/mod.rs`
3. Write integration tests in `tests/`
4. Run `just bless` to generate golden files

## The CodeLang Trait

The trait methods fall into natural groups.

### Core Methods (8 required)

These are enough for CodeBlock-level code generation:

| Method | Example (TypeScript) | Purpose |
|--------|---------------------|---------|
| `file_extension()` | `"ts"` | File extension for output files |
| `reserved_words()` | `&["async", "await", ...]` | Words that need escaping |
| `render_imports()` | `import { Foo } from '...'` | Emit the import header |
| `render_string_literal()` | `'hello'` | Language-specific string quoting |
| `render_doc_comment()` | `/** ... */` | Doc comment block |
| `line_comment_prefix()` | `"//"` | Single-line comment prefix |
| `indent_unit()` | `"  "` (2 spaces) | Indentation per level |
| `uses_semicolons()` | `true` | Statement terminator behavior |

`render_imports()` is the most complex. It receives an `ImportGroup` (deduplicated, with aliases resolved) and must emit the full import header string. Study `src/lang/typescript.rs` for ES module imports or `src/lang/rust_lang.rs` for `use` paths.

### Spec Support Methods (9 required)

These enable TypeSpec, FunSpec, and FieldSpec rendering:

| Method | Example | Purpose |
|--------|---------|---------|
| `render_visibility()` | `"public "`, `"pub "` | Visibility prefix |
| `function_keyword()` | `"function"`, `"fn"` | Function declaration keyword |
| `return_type_separator()` | `": "`, `" -> "` | Between params and return type |
| `type_keyword()` | `"class"`, `"struct"` | Type declaration keyword |
| `field_terminator()` | `","`, `";"` | After each field |
| `methods_inside_type_body()` | `true` / `false` | Key structural decision (see below) |
| `generic_constraint_keyword()` | `" extends "`, `": "` | Generic bounds |
| `generic_constraint_separator()` | `" & "`, `" + "` | Between multiple bounds |
| `super_type_keyword()` | `" extends "` | Inheritance keyword |
| `implements_keyword()` | `" implements "` | Interface keyword |

#### The `methods_inside_type_body` Decision

This is the most important method for structural correctness. It determines whether TypeSpec emits one CodeBlock or two:

- **Returns `true`** (TypeScript, Java, Python, Swift, Dart, Kotlin, C++): Methods go inside the type body. TypeSpec emits a single block: `class Foo { fields; methods; }`.
- **Returns `false`** (Rust struct/enum): Methods go in a separate `impl` block. TypeSpec emits two blocks: `struct Foo { fields }` and `impl Foo { methods }`.

The method takes a `TypeKind` parameter, so you can vary by type. Rust returns `true` for `TypeKind::Trait` (trait methods go inside) but `false` for `TypeKind::Struct` and `TypeKind::Enum`.

### Default Methods (28 methods)

These have defaults that work for most C-family languages. Override when your language differs.

**Block delimiters:**
- `block_open()` -- default `" {"`. Python overrides to `":"`.
- `block_close()` -- default `"}"`. Python overrides to `""` (indent-only).
- `empty_body()` -- default `""`. Python overrides to `"..."` (Ellipsis for abstract methods).

**Type annotations:**
- `type_annotation_separator()` -- default `": "`. Most languages use this.
- `type_before_name()` -- default `false`. C/C++/Java override to `true` for `int count` instead of `count: i32`.
- `return_type_is_prefix()` -- default `false`. C/C++/Java override to `true` for `int add(...)` instead of `fn add(...) -> int`.

**Generics:**
- `generic_open()` / `generic_close()` -- default `"<"` / `">"`. Go overrides to `"["` / `"]"`.

**Imports:**
- `qualify_import_name()` -- default passthrough. Go overrides to return `"http.Server"` (package-qualified names).

**Keywords:**
- `async_keyword()` -- default `"async "`.
- `abstract_keyword()` -- default `"abstract "`. C++ overrides to `"virtual "`.
- `readonly_keyword()` -- default `"const "`. TS uses `"readonly "`, Kotlin uses `"val "`, Java uses `"final "`.
- `mutable_field_keyword()` -- default `""`. Kotlin overrides to `"var "`.

**Enum support:**
- `enum_variant_prefix()` -- default `""`. Swift overrides to `"case "`.
- `enum_variant_separator()` -- default `","`. Python/Swift override to `""`.
- `enum_variant_trailing_separator()` -- default `false`. Rust/TypeScript override to `true`.

**Annotation support:**
- `render_annotation_prefix()` -- default `("@", "")`. Rust: `("#[", "]")`. C++: `("[[", "]]")`. C: `("__attribute__((", "))")`.

**Constructor support:**
- `constructor_keyword()` -- default `""`. Python: `"def"`. Rust: `"fn"`.
- `constructor_delegation_style()` -- default `Body` (super/this call in body). Kotlin: `Signature` (call in signature: `constructor(x) : this(x, 0)`).
- `supports_primary_constructor()` -- default `false`. Kotlin: `true`.

**Property support:**
- `property_style()` -- default `Accessor` (TS/JS: `get name()`). Swift/Kotlin: `Field` (inline get/set).
- `property_getter_keyword()` -- default `"get"`. Kotlin: `"get()"`.

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
    fn uses_semicolons(&self) -> bool { true }
    fn indent_unit(&self) -> &str { "    " }
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

    // Phase 2 methods...
    fn render_visibility(&self, vis: Visibility, _ctx: DeclarationContext) -> &str {
        match vis {
            Visibility::Public => "public ",
            Visibility::Private => "private ",
            Visibility::Protected => "protected ",
            _ => "",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str { "function" }
    fn return_type_separator(&self) -> &str { ": " }
    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Interface => "interface",
            TypeKind::Enum => "enum",
            _ => "class",
        }
    }
    fn field_terminator(&self) -> &str { ";" }
    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool { true }
    fn generic_constraint_keyword(&self) -> &str { " extends " }
    fn generic_constraint_separator(&self) -> &str { " & " }
    fn super_type_keyword(&self) -> &str { " extends " }
    fn implements_keyword(&self) -> &str { " implements " }
}
```

### 2. Register the module

Add to `src/lang/mod.rs`:
```rust,ignore
/// YourLang language support.
pub mod your_lang;
```

### 3. Write tests

Create two test files following the existing pattern:

**`tests/your_lang_tests.rs`** -- basic CodeBlock rendering:
```rust,ignore
mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::your_lang::YourLang;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_basic_statement() {
    let mut cb = CodeBlock::<YourLang>::builder();
    cb.add_statement("const x = 1", ());
    let block = cb.build().unwrap();

    let lang = YourLang::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&block).unwrap();

    golden::assert_golden("your_lang", "basic_statement", "yl", &output);
}
```

**`tests/phase2_your_lang_tests.rs`** -- spec-layer rendering (TypeSpec, FunSpec, FileSpec).

### 4. Generate golden files

```bash
just bless
```

This runs all tests with `BLESS=1`, which creates `tests/golden/your_lang/*.yl` files from the actual output. Review them manually, then commit.

### 5. Override defaults

Run the full test suite and review golden file output. Override default methods where your language's syntax differs. Common overrides:

- If your language uses indentation instead of braces: override `block_open`, `block_close`, `empty_body`
- If types come before names (`int x` instead of `x: int`): override `type_before_name`, `return_type_is_prefix`
- If generics use brackets instead of angle brackets: override `generic_open`, `generic_close`

## Reference Implementations

Study these existing implementations for patterns similar to your target:

| Language | File | Notable Patterns |
|----------|------|-----------------|
| TypeScript | `src/lang/typescript.rs` | ES module imports, type-only imports, single-quoted strings |
| Rust | `src/lang/rust_lang.rs` | `use` paths, struct+impl split, `pub(crate)` visibility |
| Python | `src/lang/python.rs` | Indent-only blocks (no braces), docstrings inside body, `from x import y` |
| Go | `src/lang/go_lang.rs` | Package-qualified names (`http.Server`), bracket generics, `func` keyword |
| C | `src/lang/c_lang.rs` | Type-before-name, `#include`, `__attribute__`, struct close semicolon |
| C++ | `src/lang/cpp_lang.rs` | `virtual` instead of `abstract`, `#include` + `using`, `[[attributes]]` |
| Bash | `src/lang/bash.rs` | Keyword-based block closers (`fi`/`done`/`esac`), `source` imports, shell escaping |

## Type Presentation

When your language uses type expressions (generics, arrays, optionals, maps, etc.), you'll need to declare how each semantic type concept renders. This is done through `present_*` methods that return `TypePresentation` data — you never build `BoxDoc` directly.

### How it works

Each `TypeName` variant (Array, Optional, Map, etc.) asks your language for a `TypePresentation` — a small enum describing the syntactic pattern:

- `GenericWrap { name }` — `name<P1, P2>` using your `generic_open()`/`generic_close()`
- `Prefix { prefix }` — `prefix inner` (e.g., Go `[]T`, Rust `*const T`)
- `Postfix { suffix }` — `inner suffix` (e.g., TypeScript `T[]`, Kotlin `T?`)
- `Delimited { open, sep, close }` — `open P1 sep P2 close` (e.g., Swift `[K: V]`, Go `map[K]V`)
- `Infix { sep }` — `P1 sep P2` (e.g., TypeScript `A | B`, Rust `A + B`)

### Methods to override

All `present_*` methods have defaults matching TypeScript conventions. Override only when your language differs:

```rust,ignore
impl CodeLang for YourLang {
    // Array: default is Postfix { suffix: "[]" } (TS: T[])
    // Override for Rust-style Vec<T>:
    fn present_array(&self) -> TypePresentation<'_> {
        TypePresentation::GenericWrap { name: "Vec" }
    }

    // Optional: default is Infix { sep: " | " } with "null" literal
    // Override for Kotlin-style T?:
    fn present_optional(&self) -> TypePresentation<'_> {
        TypePresentation::Postfix { suffix: "?" }
    }

    // Map: default is GenericWrap { name: "Map" }
    // Override for Go-style map[K]V:
    fn present_map(&self) -> TypePresentation<'_> {
        TypePresentation::Delimited { open: "map[", sep: "]", close: "" }
    }

    // Function types: default is TypeScript (A, B) => R
    fn present_function(&self) -> FunctionPresentation<'_> {
        FunctionPresentation {
            keyword: "fn",
            params_open: "(",
            params_sep: ", ",
            params_close: ")",
            arrow: " -> ",
            return_first: false,
            curried: false,
            wrapper_open: "",
            wrapper_close: "",
        }
    }
}
```

See [Type Presentation](type_presentation.md) for the full enum definition, all available methods, and examples for every supported language.
