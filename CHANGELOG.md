# Changelog

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
