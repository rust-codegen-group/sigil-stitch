# Changelog

## 0.6.4

### Added

- `$attr("text")` directive in `sigil_quote!` — structural annotations/attributes
  that render with the target language's prefix/suffix. Rust: `#[text]`,
  Java/Python: `@text`, C++: `[[text]]`. Blank lines after `$attr(...)` are
  automatically suppressed so the annotation attaches to the following declaration.
- `$T_join(sep, iter)` interpolation — joins `TypeName` items with a separator,
  tracking imports for each item via `%T` slots. Generates a nested `CodeBlock`
  at compile time. Use for type unions (`str | int`) in Python, trait bounds
  (`Read + Write`) in Rust, intersection types (`T & U`) in Java/TypeScript,
  protocol composition (`Codable & Hashable`) in Swift, and interface embedding
  in Go.
- **Go paren-delimited blocks** — `sigil_quote!(GoLang { ... })` now recognizes
  `const ( ... )`, `var ( ... )`, `import ( ... )`, and `type ( ... )` as
  structural blocks. `$for`, `$if`, `$C_each`, etc. expand correctly inside.
  Body content is auto-indented and the closing `)` is at the outer indent level.

### Fixed

- Blank lines after `$comment(...)` in `sigil_quote!` are now suppressed,
  matching the spec-level behavior where doc comments attach to the following
  declaration without a separator.
- `$C_each` inside object literals after `$N(...) =` no longer fails — the brace
  classifier now detects sigil-stitch statement markers inside brace groups.

## 0.6.3

### Added

- `@{expr}` compile-time interpolation inside `$L` string literals. Like
  `$V`, `$L("@{expr}")` resolves Rust expressions at macro expansion time,
  but emits the result as-is with no wrapping quotes or template delimiters.
  Suitable for type expressions, switch headers, return statements, and other
  contexts where language-specific `$V` wrapping is unwanted.

## 0.6.2

### Fixed

- `$C_each` inside method shorthand bodies (`foo() { $C_each(params); }`) no
  longer fails with "$C_each() must appear at the start of a line". The brace
  classifier now detects sigil-stitch markers in brace groups to distinguish
  method bodies from object-literal arguments.

### Internal

- Extract `BraceClassifier` from `parse_one_statement` — brace-detection
  heuristics moved to a dedicated module with a single `classify()` entry point.
- Extract `where_spec` from `fun_spec` — `TypeParamSpec`, `TypeParamKind`,
  `WhereConstraint`, `WhereClauseStyle` and their rendering functions moved
  to `src/spec/where_spec.rs`.
- Delete redundant `escape_reserved` overrides in `lua.rs` and `ocaml.rs`;
  add `render_string_literal` as a trait default, eliminating 6 identical
  overrides.
- Document `RendererLang` must-implement set (5 methods).

### Tests

- Control flow + interpolation marker tests for Bash (`$V`, `$S`, `$L` in
  `if`/`while`/`for`/`case` conditions).
- Parameterize `quote_control_flow` and `quote_basic` across all 18 languages
  via a shared `LanguageTestSuite` harness. Eliminates ~600 lines of duplicated
  test code.

## 0.6.1

### Added

- `@{expr}` compile-time interpolation inside `$V` string literals — embed Rust
  expressions that resolve at macro expansion time while preserving the target
  language's runtime interpolation sigils (`$`, `{}`, `` ` ``).
  - `@@` escapes to a literal `@`; bare `@` not followed by `{` passes through.
  - Works for all languages (macro-level feature).

### Changed

- `$V` / `%V` in Bash and Zsh is now pure passthrough — no wrapping quotes, no
  escaping. Shell interpolates by default; include quotes in the `$V` content
  when quoting is desired in the output.

### Tests

- Shebang token tests for `sigil_quote!`.
- `$V` verbatim string integration tests for all 12 languages (JavaScript,
  Python, Kotlin, Swift, Scala, Dart, C#, Java, Go + existing Bash, Zsh,
  TypeScript).

## 0.6.0

### Added

- `$V` / `%V` verbatim string literal — preserves interpolation sigils (`$`, `` ` ``)
  with minimal escaping, unlike `$S` which escapes everything.
  - Per-language rendering: Bash/Zsh `"..."`, JS/TS `` `...` ``, Python `f"..."`,
    Kotlin/Swift `"..."`, Dart `'...'`, C# `$"..."`, Scala `s"..."`.
  - `VerbatimStrArg` wrapper for the builder API.
- `MacroLang`-aware tokenizer with per-language annotation rules (Go `<-`,
  Haskell `$$`, shell dash-flags and slash-paths).
- Language-aware statement rewriting for Haskell guards and OCaml blocks.
- Shell cookbook (`cookbook_shell.md`) with comprehensive Bash/Zsh recipes.

### Changed

- `CodeLang` split into `RendererLang` (renderer-only methods) + `CodeLang`
  (spec-layer). Existing `impl CodeLang` still works; only affects custom trait
  implementations.
- Macro crate internals: `format.rs` split into `annotate.rs`, `spacing.rs`,
  and `format.rs` modules.

### Fixed

- Zsh block delimiters: `begin_control_flow("if ...")` now emits `then`/`fi`
  and `do`/`done` instead of `{`/`}`.
- Shell bracket spacing: `[ $x ]` and `[[ $x ]]` render with correct inner
  spaces in `sigil_quote!`.

### Internal

- Shared `shell_syntax.rs` for Bash/Zsh control-flow logic.
- `code_renderer.rs` block/comment resolution unified.
- `fun_spec.rs` body-emission deduplication.
- Unit tests for annotation and format parsing in macro crate.

## 0.5.3

### Fixed

- `sigil_quote!` tokenizer: single-dash flags (`-q`, `-f`, `-avz`) and path
  separators (`linux/amd64`) no longer get unwanted spaces inserted around `-`
  and `/` (#93). Uses span-adjacency detection to distinguish flags/paths from
  binary operators (`a - b`, `a / b`).
- Shell test operators like `-gt` now render tight (previously `- gt`).

### Tests

- Migrated `$L` → `$N` for identifier interpolation across all test modules and
  examples (semantic correctness — `$N` escapes reserved words, `$L` does not).
- Added `$N` keyword-escape tests for all 18 languages.
- Added 4 new spacing tests for dash-flag and slash-path edge cases.

## 0.5.2

### Added

- **`ParameterSpec::is_property()` / `is_mutable_property()`** — Kotlin `val`/`var`
  on primary constructor params without embedding keywords in the name string (#86).
  Emits the language's `readonly_keyword` or `mutable_field_keyword` before the
  parameter name.
- **`WhereClauseStyle::SeparateWhere`** — C#-style per-constraint `where` clauses
  after the signature (`where T : IComparable`). Enabled for C# by default (#88).
- **`InvalidEnum` validation** — `TypeSpec::build()` rejects enums with primary
  constructor parameters when any variant lacks a value (#87).
- **`EnumVariantSpec::has_value()`** public accessor.
- **Language-aware `rewrite_nodes()` post-build pass** — 8 languages (Bash, Zsh,
  Haskell, OCaml, Lua, Python, Ruby, Elixir) transform nodes after macro expansion
  for language-specific fixups (e.g., `=` → assignment spacing in Bash).
- **`ForRange` colon context** for C++ range-for loops
  (`for (auto& x : items)`).
- **Context-aware block delimiters** for Bash (`if`/`then`/`fi`, `do`/`done`)
  and Lua (`if`/`then`/`end`) via `block_open_for()` / `block_close_for()`.
- **Haskell `block_open_for()` expanded**: `if` → `then`, `else` → `""`,
  `case` → `of`.
- **Swift `async_suffix_before_return`** for correct `func f() async -> T`
  placement.

### Fixed

- `sigil_quote!` tokenizer: generics vs operators, prefix/postfix markers,
  span-aware group spacing, method calls on declaration keywords (`.read()`).
- Statement parser: brace blocks, destructuring, method chaining.
- Rust where clause brace on new line (#61).
- Haskell optional types with space separator (#62).
- Lua double space before function name (#52).
- C typedef function pointers, Haskell newtype deriving.
- Interface visibility, Python static, Java `@Override`.
- OCaml optional, C# bracket annotations, Dart async, Java generic params.
- 12 test input semantic fixes (C, Java, Kotlin, Rust, Go, C++, Scala).

### Tests

- Comprehensive golden test suite for all 18 languages (18 new test modules,
  ~500 new tests).
- Golden tests for macro meta-features and language edge cases.

## 0.5.1

### Added

- **Embedded types** — `TypeSpec::add_embedded(TypeName)` for Go struct
  composition and interface embedding. Renders unnamed type references before
  regular fields with automatic import tracking via `%T`.
- **`AnnotationSpec::args(iter)`** — bulk argument addition for annotations.
  Enables `AnnotationSpec::new("derive").args(["Debug", "Clone", "Serialize"])`.
- **mdbook doc-test CI** — all Rust code examples in the mdbook are now compiled
  and tested via `mdbook test -L target/debug/deps`. Added `just book-test`
  recipe and CI workflow step.
- **C# and Lua cookbook pages** — mdbook and README updated with language recipes
  and cross-language comparison entries.

### Changed

- **`%N` keyword escaping** — `%N` / `$N` now calls `lang.escape_reserved()`
  at render time. Reserved words are automatically escaped (Rust: `r#type`,
  Go/Python: `type_`). Previously emitted the name verbatim.

## 0.5.0

### Added

- **Lua language support** — full `CodeLang` implementation with `end`-delimited
  blocks, 2-space indent, `---` doc comments, `require()` imports, and all 22
  reserved words. Includes builder and `sigil_quote!` integration tests.
- **C# language support** — full `CodeLang` implementation with `{}`-delimited
  blocks, `using` imports, XML doc comments (`///`), nullable `T?` optionals,
  and C#-style visibility modifiers.
- `elseif` and `elif` recognition in `sigil_quote!` parser — enables Lua
  `if/elseif/else` and Python `if/elif/else` control flow.
- `looks_like_control_flow_header()` heuristic in `sigil_quote!` parser —
  distinguishes control-flow `{` from literal `{` (table constructors, object
  literals) for end-delimited languages.
- `close_on_transition` field on `BlockSyntaxConfig` — controls whether
  `BlockCloseTransition` emits the block-close keyword before `else`/`elseif`.
  Enables correct rendering for languages that use `else` without a preceding
  `end` (Lua, Ruby, Elixir).
- `escape_field_name` method on `CodeLang` trait — allows languages to skip
  reserved-word escaping for property/field names (TypeScript overrides to
  return names unchanged since TS property names never conflict).

### Fixed

- `sigil_quote!` spacing for unary minus (`-1` no longer becomes `- 1`) and
  `$T()` followed by generic angle brackets (`List<string>` no longer becomes
  `List < string >`).
- TypeScript interface fields using reserved words (`type`, `namespace`, etc.)
  no longer get incorrectly escaped to `type_`.
- Return type separator suppressed for untyped languages (Lua, Python) —
  prevents stray `: ` or `-> ` after function signatures.

## 0.4.4

### Fixed

- Eliminate spurious spaces around punctuation in `sigil_quote!` — a pre-scan
  annotation pass now classifies tokens structurally before formatting. Fixes
  path separators (`std::fmt::Display`), macro bangs (`println!(...)`), prefix
  operators (`&self`, `*ptr`), and generic angle brackets (`Vec<T>`,
  `HashMap<K, V>`) across all supported languages.

### Added

- Cross-language spacing test suite covering generics-vs-operators disambiguation
  for C++, Java, Kotlin, TypeScript, Swift, Rust, Scala, Dart, Go, and Python.

## 0.4.3

### Added

- `$let(binding);` meta-statement in `sigil_quote!` — Rust-level `let` bindings
  inside macro bodies. Supports all `let` forms (simple, typed, destructuring,
  mutable) and fallible expressions with `?` that propagate to the enclosing
  function. Primary use case: intermediate variable computation inside `$for`
  and `$if` bodies.

## 0.4.2

### Added

- `$for(pat in expr) { body }` meta-loop in `sigil_quote!` — compile-time
  iteration with full nesting, destructuring patterns, and all interpolation
  markers.
- `Emittable` trait and `FileMember::Spec` variant for third-party spec types
  to participate in `FileSpec` rendering and import collection.
- `ProjectSpec::build()` rejects duplicate filenames (returns
  `SigilStitchError::DuplicateFileName`).

### Fixed

- `FileSpec` serde round-trip no longer panics on render (deserialized files
  now retain their language).

### Changed (internal, non-breaking)

- `CodeLang` required methods reduced from 10 to 3 (config accessors +
  format specifiers consolidated).
- TypeName rendering and import collection extracted into separate modules.
- Import collection consolidated into a single walker.
- Format specifier definitions unified into `Specifier` enum.
- Architecture deepened: import resolution deduplication, `ImportGroup.entries`
  privatized, `EnumVariantSpec` self-contained emit.

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
