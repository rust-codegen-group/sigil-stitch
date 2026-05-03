# Changelog

## 0.4.1

### Fixed

- `$C_each` trailing blank line when used as `FunSpec` body inside a `TypeSpec` —
  `ends_with_newline_or_block_close()` now recurses into `Nested` nodes, fixing
  the unwanted blank line before the closing `}` in generated constructors/methods.

## 0.4.0

### Added

- `$+` line continuation marker in `sigil_quote!` — suppresses automatic
  line-break splitting for multi-line expressions (Haskell, OCaml, long calls).

### Changed

- **Breaking:** `sigil_quote!` now splits statements on source line breaks.
  Each line in the macro body becomes a separate statement, matching
  Kotlin/Python semantics. Previously, tokens without `;` or `{ }` were
  collected into a single statement regardless of line breaks. Use `$+` at
  end of line to continue an expression on the next line.

### Fixed

- Kotlin `?.` safe-call spacing — `response.body?.string()` no longer gets
  an unwanted space before `?`. Uses lookahead to distinguish `?.` (Joint,
  suppress space) from ternary `?` (Alone, allow space).

## 0.3.4

### Changed

- Colon spacing in `sigil_quote!` redesigned with a `ColonContext` enum
  (`TypeAnnotation`, `MapEntry`, `Ternary`, `PathSeparator`, `WalrusAssign`)
  and `SpacingState` struct. All colon spacing decisions now go through the
  enum via exhaustive match, replacing the previous unconditional suppress.

### Fixed

- Ternary `? :` colon spacing — `x ? y : z` now renders with a space before
  `:` instead of `x ? y: z`. Context resets at statement boundaries.
- Go/walrus `:=` spacing — `x := 42` now renders with a space before `:`
  instead of `x:= 42`. Detected via one-token lookahead.
- `$C_each` trailing blank line — spliced blocks that already end with a
  newline (from `add_statement`) no longer produce an extra blank line before
  the next statement.

## 0.3.3

### Added

- `$C_each(iter)` in `sigil_quote!` — splice each `CodeBlock` from an iterable
  into the builder sequentially.
- `$if(cond) { ... } $else_if(cond) { ... } $else { ... }` in `sigil_quote!` —
  meta-conditionals that control which builder calls are emitted at runtime.
- `$join(sep, iter)` in `sigil_quote!` — inline separator-joined list rendering.
- Keyword spacing in `sigil_quote!` — control-flow keywords (`if`, `for`,
  `while`, etc.) now emit a space before `(` in the format string.

### Changed

- `macros/src/parse.rs` split into `parse/{mod,types,statements,format,util}.rs`
  for navigability.
- CI workflows now use `just` commands from the Justfile.
- Justfile recipes aligned with CI (`--workspace`, `--all-targets`).

## 0.3.2

### Added

- `VariantValueFormat` enum in `EnumAndAnnotationConfig` to control how enum
  variant values render: `Assignment` (`= val`) vs `ConstructorArg` (`(val)`).
- `variants_before_fields` flag to emit enum variants before fields in the type
  body (Java/Kotlin pattern).
- `variant_section_terminator` field to emit a separator (e.g., `;`) after the
  last enum variant when fields or methods follow.

### Fixed

- Java and Kotlin enum rendering now produces valid syntax: constructor-arg values,
  variants-first ordering, and semicolon terminator between variants and class body.

## 0.3.1

### Added

- `TypeName::qualified()` constructor for inline module-qualified type rendering
  (e.g., `serde_json::Value`, `super::Foo`) without generating import statements.
- `.qualify()` modifier to convert an existing `TypeName::importable()` into
  qualified inline rendering.
- `CodeLang::module_separator()` trait method returning `Option<&str>`. Languages
  with module-qualified paths (`::` for Rust/C++, `.` for Go/Python/Java/Kotlin/
  Scala/Swift/Dart/Haskell/OCaml) return `Some(sep)`. Languages without inline
  qualification (TypeScript, JavaScript, C, Bash, Zsh) return `None`, and
  qualified types silently fall back to unqualified rendering.

## 0.3.0

### Breaking

- **Erased `<L: CodeLang>` generic from all public types.** `CodeBlock`,
  `TypeName`, all spec types, and `FileSpec` are no longer parameterized by
  language. The language enters at render time via `&dyn CodeLang`.
- **Spec builders use owning-chain `(mut self) -> Self` pattern.** `TypeSpec`,
  `FunSpec`, `FieldSpec`, `FileSpec`, and all other spec builders now consume
  `self` and return `Self`. Chain calls fluently instead of using `&mut self`.
  `CodeBlockBuilder` is unchanged (still `&mut self`).
- **`CodeBlock` internals replaced with `Vec<CodeNode>` tree IR.** The parallel
  vectors (`formats`, `args`, etc.) are replaced by a single `Vec<CodeNode>`
  containing typed nodes. External API is unchanged but internal
  representations differ.
- **`CodeLang` trait reorganized into 6 config struct accessors.** The many
  individual trait methods are now grouped into `block_syntax()`,
  `function_syntax()`, `type_decl_syntax()`, `generic_syntax()`,
  `enum_and_annotation()`, and `type_presentation()`. Each returns a config
  struct with `..Default::default()` support.

### Added

- Data-driven `TypePresentation` layer: languages declare syntactic patterns
  (GenericWrap, Prefix, Postfix, Surround, Delimited, Infix) instead of
  building `BoxDoc` directly. A single rendering engine handles all patterns.
- `FunctionPresentation` struct for declarative function type rendering across
  all languages.
- `TypeName::Tuple` variant with cross-language rendering.
- `TypeName::Reference` variant with mutable/immutable distinction.
- `TypePresentation::Surround` for C++ `const T&` and similar patterns.
- `TypeKind::TypeAlias` and `TypeKind::Newtype` with cross-language emission.
- `GenericApplicationStyle` enum: `AngleBracket`, `SquareBracket`,
  `PostfixJuxtaposition` (OCaml), `PrefixJuxtaposition` (Haskell).
- `AssociatedTypeStyle` for TypeScript `Foo["bar"]`, Rust `Foo::Bar`, etc.
- `ImplTrait` and `DynTrait` TypeName variants.
- `Wildcard` TypeName variant with language-specific rendering.
- Where-clause support for `FunSpec` and `TypeSpec`.
- `TypeParamKind` for higher-kinded type parameters and lifetime parameters.
- Scala language support (`sigil_stitch::lang::scala::Scala`).
- Haskell language support (`sigil_stitch::lang::haskell::Haskell`), including
  split signature style, `deriving` via `.implements()`, and Haddock comments.
- OCaml language support (`sigil_stitch::lang::ocaml::OCaml`), including
  postfix generics, `module_block`/`module_sig_block` helpers, and OCamldoc.
- `sigil_quote!` improvements: `$open("text")` for custom block openers,
  `$>`/`$<` for explicit indent/dedent control.
- Per-language `sigil_quote!` golden tests for all 16 languages.

### Fixed

- String escaping for Swift (backslash and interpolation escaping) and Scala
  (triple-quote and interpolation escaping).
- Wildcard rendering across all languages.
- Where-clause indentation alignment.
- Doc-comment emission unified across all spec emitters.
- Format parser byte indexing replaced with `char_indices` for correct
  handling of multi-byte characters.
- Multi-line literal re-indentation in `CodeRenderer`.
- Golden file quality improvements for Go, Kotlin, JavaScript, C++, and Bash.

### Documentation

- Complete documentation rewrite for the 0.3.0 API across 13 mdbook chapters.
- Per-language cookbook pages expanded to 12 languages (TypeScript, Rust, Go,
  Python, Java, Kotlin, Swift, C++, C, Scala, Haskell, OCaml) with 4–5
  recipes each.
- New chapters: Type Presentation, Code Templates.
- Architecture chapter rewritten to cover CodeNode IR, config structs, and
  the three-pillar refactoring.

## 0.2.2

### Fixed

- Multi-line literal indentation in `CodeRenderer::render_direct` so doc
  comments (and any multi-line `%L`) inside indented blocks re-indent every
  line.

## 0.2.1

### Added

- `TypeName::ReadonlyArray` renders `readonly T[]` in TypeScript and falls back
  to the same shape in other languages, so interface fields can express
  read-only arrays without reaching for `Raw` or `Generic<ReadonlyArray<T>>`.
- `FieldSpec::is_optional()` marks a field whose key may be absent. Rendering
  is language-specific and delegates to the new `CodeLang::optional_field_style`
  hook:
  - TypeScript → `name?: T`
  - Rust → `Option<T>`
  - Go → `name *T`
  - Python → `T | None`
  - Java → `Optional<T>` (caller must import `java.util.Optional`)
  - C++ → `std::optional<T>` (caller must `#include <optional>`)
  - Kotlin / Swift / Dart → `T?`
  - C → `T *name`
  - JavaScript / Bash / Zsh → rendered without any optionality marker
- `OptionalFieldStyle` enum and shared `QuoteStyle` enum in a new
  `sigil_stitch::lang::config` module.
- Fluent config builders on `TypeScript`, `JavaScript`, `Python`, and `JavaLang`:
  `.with_quote_style()`, `.with_indent()`, `.with_semicolons()`,
  `.with_extension()` (available per-language as appropriate).
- `.with_indent()` and `.with_extension()` on the remaining languages
  (`RustLang`, `GoLang`, `Kotlin`, `Swift`, `DartLang`, `CLang`, `CppLang`,
  `Bash`, `Zsh`). Every language struct now has a public `extension: String`
  field; defaults are unchanged.

### Changed

- **Breaking:** `TypeScript::single_quotes: bool` replaced by
  `TypeScript::quote_style: QuoteStyle`. Migration: swap
  `TypeScript { single_quotes: true, .. }` for the default, or use
  `TypeScript::new().with_quote_style(QuoteStyle::Double)` for double quotes.
- **Breaking:** `JavaScript::single_quotes: bool` replaced by
  `JavaScript::quote_style: QuoteStyle`. Same migration as TypeScript.
- **Breaking:** `TypeScript` now exposes `uses_semicolons: bool` and
  `extension: String` fields so projects can emit Prettier-style
  semicolon-less output or `.tsx` files without a custom `CodeLang` impl.
- `Python` gains `quote_style` and `extension` fields for `.pyi` stubs and
  Black-style double quotes.
- `JavaLang` gains an `extension` field.

## 0.1.0

Initial release.

### Code generation

- Type-safe, import-aware, width-aware code generation across 13 languages:
  TypeScript, JavaScript, Rust, Go, Python, Java, Kotlin, Swift, Dart, C, C++,
  Bash, Zsh.
- `CodeBlock` with format specifiers: `%T` (type, tracks imports), `%N` (name),
  `%S` (string literal), `%L` (literal or nested block), `%W` (soft line break),
  `%>`/`%<` (indent/dedent), `%[`/`%]` (statement boundaries), `%%` (escape).
- Three-pass rendering pipeline: materialize specs, collect and resolve imports,
  render with Wadler-Lindig pretty printing via the `pretty` crate.
- Automatic import conflict resolution with first-wins simple names and
  module-derived aliases.

### Spec layer

- `TypeSpec` (struct / class / interface / trait / enum), `FunSpec`,
  `FieldSpec`, `ParameterSpec`, `PropertySpec`, `AnnotationSpec`,
  `EnumVariantSpec`, `ImportSpec`.
- `FileSpec` for per-file orchestration; `ProjectSpec` for multi-file output
  with filesystem writing.
- `CodeTemplate` for reusable named-parameter templates (`#{name:K}` syntax).
- `sigil_quote!` proc macro for inline target-language code with `$T` / `$S` /
  `$N` / `$L` / `$C` / `$W` interpolation markers.

### Type-safe language parameterization

- Every type carries an `L: CodeLang` phantom parameter. Cross-language mixing
  is rejected at compile time.
- `TypeName` variants: `Importable`, `Primitive`, `Generic`, `Array`, `Union`,
  `Optional`, `Function`, `Map`, `Pointer`, `Slice`, `Raw`. All recursively
  collect imports.

### Errors and diagnostics

- `SigilStitchError` via `snafu`, no panics from builder paths.
- `FormatArgCount` includes the format string, expected specifiers, and actual
  argument kinds so mismatches point at the exact slot.
- `InvalidFormatSpecifier` surfaces unrecognised `%` sequences at build time.
- `TypeSpec::build()` rejects duplicate field names within a single type.

### Serialization

- All spec types derive `serde::Serialize` and `serde::Deserialize`. Specs
  round-trip through JSON, YAML, or any serde format. No feature flag required.

### Threading

- Rendering uses `pretty::BoxDoc` internally, so rendered documents are
  `Send + Sync` and can cross thread boundaries.

### Documentation

- mdbook at `doc/` covering introduction, getting started, architecture, format
  specifiers, spec layer, code templates, `sigil_quote!`, and adding a language.
- Published via GitHub Pages.
