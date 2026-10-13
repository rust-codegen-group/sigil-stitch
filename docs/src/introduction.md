# Introduction

sigil-stitch is a structured, language-aware Rust code generation toolkit. You can build
source with composable `CodeBlock`s, declaration specs, or the `sigil_quote!` macro. Semantic
types, functions, properties, enums, closed sums, annotations, and imports stay structured
until a selected language adapter validates the intent and lowers it to target syntax. File
and project rendering then rewrites and validates the prepared source tree, resolves import
aliases and naming conflicts, and applies Wadler-Lindig layout for width-aware output across
the supported languages.

## How the pieces fit

**Structured source construction.** `CodeBlock` is the shared source container for typed
format arguments, nested blocks, annotations, and layout markers. `TypeName` values remain
semantic references, so `%T` can contribute both target-language source and import evidence.
Declaration specs record intent such as visibility, type parameters, properties, variants,
and ordered returns without choosing target-language punctuation or keyword order.

**Language-owned lowering and rendering.** The selected adapter decides whether an intent is
representable and lowers it to target-language structure. File and project rendering then
rewrites and validates the complete source tree, lowers every type reference, resolves all
imports, and chooses direct or Wadler-Lindig layout for the prepared tree. A target width
controls soft breaks such as `%W`; source that does not need a break stays compact.

## Four key properties

**Ergonomic multi-language.** `CodeBlock`, `TypeName`, and all spec types have no
language generic parameter. The language enters when `FileSpec` materializes a
declaration or when a renderer receives `&dyn RendererLang`. Semantic
`TypeName` and spec values can be reused across targets that support their
intent. Literal text inside a `CodeBlock` is already target syntax and is only
portable where that syntax is shared.

**Import-aware.** When you use `%T` with a `TypeName::Importable`, the library records
that import. `FileSpec` rewrites and validates each materialized source tree,
lowers every complete type name, then collects and resolves imports through a
fallible complete-set resolver. Its default policy uses encounter order only as
a deterministic tie-break; callers can supply a different borrowed policy for
one render. You never write ordinary import statements by hand.

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
Wadler-Lindig formatting, `serde` (v1, with `derive`) for same-version
round-trips of first-class serializable specs with a caller-provided format such
as JSON or YAML, and `snafu` for structured errors. `FileSpec::add_spec()`
extension members are not serializable; a deserialized `FileSpec`, or each file
inside a deserialized `ProjectSpec`, needs a language configured again before
rendering. These representations are not a versioned wire protocol. Everything
else -- parsing
format strings, collecting imports, resolving conflicts, rendering output -- is
implemented in sigil-stitch itself.

**Two builder flavours.** Spec builders (`TypeSpec`, `ClosedSumSpec`,
`ClosedSumCaseSpec`, `FunSpec`, `FieldSpec`,
`FileSpec`, `EnumVariantSpec`, `PropertySpec`, `AnnotationSpec`, `ProjectSpec`) use an
owning chain pattern -- every setter takes `mut self` and returns `Self`, so you
chain calls fluently:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
# let body = CodeBlock::of("todo!()", ()).unwrap();
let fun = FunSpec::builder("greet")
    .returns(vec![TypeName::primitive("string")])
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
- **FileSpec** to render a complete file. It lowers specs, rewrites and validates
  each source tree, lowers type references, resolves the imports in the prepared
  blocks, and then renders with no further rewrite or type lowering. See
  [Architecture](architecture.md) for the complete pipeline. Pass a target width
  to `file.render(80)` and get a `String` back.

For multi-file output, **ProjectSpec** collects multiple `FileSpec`s and can render
them all at once or write them to disk.

## What's next

Continue to [Getting Started](getting_started.md) for a hands-on walkthrough, or
jump to [Architecture](architecture.md) for the full technical picture.
