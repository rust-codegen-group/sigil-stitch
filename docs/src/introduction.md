# Introduction

sigil-stitch is a Rust library for type-safe, import-aware, width-aware code generation
across multiple languages. It combines two ideas: JavaPoet's builder model for constructing
structured code, and the Wadler-Lindig algorithm for width-aware formatting. You describe
code with builders and format specifiers, and the library handles imports, name conflicts,
indentation, and line breaking.

## Where the ideas come from

**JavaPoet's builder model.** JavaPoet (by Square) introduced the idea of building code
with `CodeBlock` format strings and structural `Spec` types (TypeSpec, FunSpec, etc.).
You write a format string like `"const user: %T = getUser()"`, pass a `TypeName` for
the `%T` slot, and the library renders the type reference *and* tracks the import.
sigil-stitch adopts this model directly, extending it from Java-only to multiple languages.

**Wadler-Lindig pretty printing.** The `pretty` crate implements the Wadler-Lindig
algorithm, which decides where to break lines based on a target width. sigil-stitch
uses this via the `%W` (soft line break) specifier -- you mark where breaks *can*
happen, and the algorithm decides where they *should* happen. Without `%W`, output
is rendered with direct string concatenation (no pretty-printer overhead).

## Four key properties

**Ergonomic multi-language.** `CodeBlock`, `TypeName`, and all spec types have no
language generic parameter. The language enters when `FileSpec` materializes a
declaration or when a renderer receives `&dyn RendererLang`. Semantic
`TypeName` and spec values can be reused across targets that support their
intent. Literal text inside a `CodeBlock` is already target syntax and is only
portable where that syntax is shared.

**Import-aware.** When you use `%T` with a `TypeName::Importable`, the library records
that import. At render time, `FileSpec` collects all imports from every code block,
deduplicates them, and resolves naming conflicts automatically. If two modules export a
type named `User`, the first one encountered keeps the simple name `User` and the second
gets an aliased name (e.g., `OtherUser`). You never write import statements by hand.

**Width-aware.** Place `%W` in a format string to mark a soft line break. When the
output fits within the target width, `%W` produces a space. When it doesn't fit, `%W`
produces a newline with proper indentation. This is the Wadler-Lindig algorithm at
work, via the `pretty` crate. You pass the target width to `FileSpec::render(width)`,
and the same code blocks produce different layouts for different widths.

**Multi-language.** `RendererLang` owns final-rendering policy, while each
`CodeLang` adapter validates declaration intent and owns its target grammar.
sigil-stitch ships with adapters for TypeScript, JavaScript, Rust, Go, Python,
Java, Kotlin, Swift, Dart, Scala, Haskell, OCaml, C, C++, C#, Lua, Bash, and Zsh.
The shared container types work with every adapter; each value must still be
representable by its selected target.

## Design philosophy

**Specs lower to structured blocks.** Specs record target-independent
declaration intent. Their `.emit()` facade performs validation and delegates
target grammar to the selected language adapter, producing `CodeBlock` trees
rather than type-bearing strings. The renderer and import collector therefore
remain independent of declaration kinds while retaining structured type
references.

**Minimal dependencies.** The runtime dependencies are `pretty` (v0.12) for
Wadler-Lindig formatting, `serde` (v1, with `derive`) so every spec can round-trip
to JSON or YAML, and `snafu` for structured errors. Everything else -- parsing
format strings, collecting imports, resolving conflicts, rendering output -- is
implemented in sigil-stitch itself.

**Two builder flavours.** Spec builders (`TypeSpec`, `FunSpec`, `FieldSpec`,
`FileSpec`, `EnumVariantSpec`, `PropertySpec`, `AnnotationSpec`, `ProjectSpec`) use an
owning chain pattern -- every setter takes `mut self` and returns `Self`, so you
chain calls fluently:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
# let body = CodeBlock::of("todo!()", ()).unwrap();
let fun = FunSpec::builder("greet")
    .returns(TypeName::primitive("string"))
    .body(body)
    .build()
    .unwrap();
# }
```

`CodeBlockBuilder` is different: its methods take `&mut self` and return
`&mut Self`, so you keep the builder in a `let mut` binding and call methods
on it:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let mut cb = CodeBlock::builder();
cb.add_statement("return user", ());
let block = cb.build().unwrap();
# }
```

## Quick orientation

There are three levels of abstraction, and you can use whichever fits:

- **CodeBlock** for code fragments. Use format specifiers (`%T`, `%S`, `%L`, `%W`)
  to interpolate values. Good for function bodies, one-off statements, and anything
  that doesn't need structural metadata.
- **Specs** (`FunSpec`, `TypeSpec`, `FieldSpec`, `ParameterSpec`, etc.) for
  declaration intent. They carry semantic facts such as visibility, annotations,
  type parameters, and modifiers; the selected adapter validates and lowers
  them to target syntax.
- **FileSpec** to render a complete file. It orchestrates the three-pass pipeline:
  materialize specs into code blocks, collect and resolve imports, then render
  everything with proper formatting. Pass a target width to `file.render(80)` and
  get a `String` back.

For multi-file output, **ProjectSpec** collects multiple `FileSpec`s and can render
them all at once or write them to disk.

## What's next

Continue to [Getting Started](getting_started.md) for a hands-on walkthrough, or
jump to [Architecture](architecture.md) for the full technical picture.
