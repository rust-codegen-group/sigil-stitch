# Introduction

sigil-stitch is a Rust library for type-safe, import-aware, width-aware code generation
across 13 languages. It combines two ideas: JavaPoet's builder model for constructing
structured code, and the Wadler-Lindig algorithm for width-aware formatting. You describe
code with builders and format specifiers, and the library handles imports, name conflicts,
indentation, and line breaking.

## Where the ideas come from

**JavaPoet's builder model.** JavaPoet (by Square) introduced the idea of building code
with `CodeBlock` format strings and structural `Spec` types (TypeSpec, FunSpec, etc.).
You write a format string like `"const user: %T = getUser()"`, pass a `TypeName` for
the `%T` slot, and the library renders the type reference *and* tracks the import.
sigil-stitch adopts this model directly, extending it from Java-only to 13 languages.

**Wadler-Lindig pretty printing.** The `pretty` crate implements the Wadler-Lindig
algorithm, which decides where to break lines based on a target width. sigil-stitch
uses this via the `%W` (soft line break) specifier -- you mark where breaks *can*
happen, and the algorithm decides where they *should* happen. Without `%W`, output
is rendered with direct string concatenation (no pretty-printer overhead).

## Four key properties

**Type-safe.** Every type in sigil-stitch is parameterized by `L: CodeLang`. A
`CodeBlock<TypeScript>` cannot accept a `TypeName<Go>`. A `FunSpec<Rust>` cannot be
added to a `FileSpec<Python>`. The compiler prevents cross-language mixing entirely --
there is no runtime check, no language tag to compare. If it compiles, the language
parameter is consistent.

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

**Multi-language.** The `CodeLang` trait abstracts everything that varies between
languages: string delimiters, statement terminators, import syntax, visibility keywords,
type formatting, annotation style, and more. sigil-stitch ships with implementations
for TypeScript, JavaScript, Rust, Go, Python, Java, Kotlin, Swift, Dart, C, C++,
Bash, and Zsh.
The same `CodeBlock`, `TypeName`, and `Spec` types work across all of them -- only the
`L` parameter changes.

## Design philosophy

**Specs emit CodeBlocks, never raw strings.** A `FunSpec` produces a `CodeBlock` via
its `.emit()` method. A `TypeSpec` does the same. The renderer and import system only
ever see `CodeBlock` trees. This means you can add new spec types -- or build your
own -- without touching the renderer or import collector. The format-specifier system
and the spec system are fully decoupled.

**Single dependency.** The only runtime dependency is the `pretty` crate (v0.12) for
Wadler-Lindig formatting. Everything else -- parsing format strings, collecting
imports, resolving conflicts, rendering output -- is implemented in sigil-stitch itself.

**Builder pattern with `&mut Self` returns.** Builders use `&mut Self` for chaining
setter calls, and `self` for the final `.build()`. This means you should *not* chain
`.build()` after setters in a single expression. Instead, use a `let mut` binding:

```rust,ignore
let mut fun = FunSpec::builder("greet");
fun.returns(TypeName::primitive("string"));
fun.body(body);
let fun = fun.build().unwrap();
```

## Quick orientation

There are three levels of abstraction, and you can use whichever fits:

- **CodeBlock** for code fragments. Use format specifiers (`%T`, `%S`, `%L`, `%W`)
  to interpolate values. Good for function bodies, one-off statements, and anything
  that doesn't need structural metadata.
- **Specs** (FunSpec, TypeSpec, FieldSpec, ParameterSpec, etc.) for structured
  declarations. They produce CodeBlocks internally but carry metadata like visibility,
  annotations, type parameters, and modifiers that the language trait uses to emit
  correct syntax.
- **FileSpec** to render a complete file. It orchestrates the three-pass pipeline:
  materialize specs into code blocks, collect and resolve imports, then render
  everything with proper formatting. Pass a target width to `file.render(80)` and
  get a `String` back.

For multi-file output, **ProjectSpec** collects multiple `FileSpec`s and can render
them all at once or write them to disk.

## What's next

Continue to [Getting Started](getting_started.md) for a hands-on walkthrough, or
jump to [Architecture](architecture.md) for the full technical picture.
