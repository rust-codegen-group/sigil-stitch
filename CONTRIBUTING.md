# Contributing to sigil-stitch

Thank you for your interest in contributing!

## Project Overview

sigil-stitch is a structured, language-aware Rust code generation toolkit. It
keeps semantic declarations and type references structured until a selected
language adapter validates and lowers them, then prepares imports and renders
width-aware output for the target language.

**Workspace layout:**

```
sigil-stitch/
├── src/                    # Main library crate
│   ├── code_block.rs       # CodeBlock and format string parsing
│   ├── code_node.rs        # Structured source-tree nodes for CodeBlock
│   ├── code_renderer.rs    # Prepared-tree semantic renderer
│   ├── code_template.rs    # Named-parameter templates
│   ├── type_name.rs        # Semantic TypeName enum + legacy presentation data
│   ├── type_name_render.rs # TypeName renderer retiring to the 0.6.8 bridge
│   ├── import.rs           # Import types and complete-set conflict resolution
│   ├── import_collector.rs # Import extraction from CodeBlock trees
│   ├── error.rs            # Error types (snafu)
│   ├── lang/               # Language traits, adapters, and compatibility seams
│   │   ├── mod.rs          # RendererLang and CodeLang traits
│   │   ├── config.rs       # Shared grammar types retiring to compatibility
│   │   ├── rewrite.rs      # Runtime node rewrite walker
│   │   ├── typescript.rs   # One file per language: typescript, javascript,
│   │   └── ...             # rust, go, python, java, kotlin,
│   │                       # swift, dart, scala, haskell, ocaml, c,
│   │                       # cpp, bash, zsh, lua, csharp
│   └── spec/               # Structural builders (emit CodeBlocks)
│       ├── type_spec.rs    # Class, struct, interface, trait, enum, type alias, newtype
│       ├── closed_sum_spec.rs      # Complete closed-sum declarations
│       ├── closed_sum_case_spec.rs # Unit, positional, and record cases
│       ├── fun_spec.rs     # Functions and methods
│       ├── field_spec.rs   # Struct fields / class properties
│       ├── file_spec.rs    # Top-level file orchestrator
│       ├── project_spec.rs # Multi-file generation
│       └── ...             # ParameterSpec, PropertySpec, AnnotationSpec, etc.
├── macros/                 # sigil_quote! proc macro crate
│   └── src/parse/          # Tokenizer: lang.rs (MacroLang), format.rs, statements.rs
├── tests/                  # Integration tests (one directory per language)
│   ├── <lang>/             # main.rs + quote_*.rs + builder_*.rs submodules
│   └── shared/golden.rs    # Golden test assertion helper
├── test-goldens/           # Golden test output files (one directory per language)
└── docs/                   # mdbook documentation
```

**Key dependencies:** `pretty` (Wadler-Lindig formatting), `serde` + `derive` (serialization), `snafu` (errors).

For a deeper dive, see the [Architecture](docs/src/architecture.md) chapter.

## Build and Test Commands

Requires Rust edition 2024, MSRV 1.88.0. Install [just](https://github.com/casey/just) for the task runner.

```bash
# Core workflow
just test              # cargo nextest + doctests
just lint              # cargo clippy -- -D warnings
just check             # fmt + test + lint
just example-smoke     # run every ordinary example

# Golden tests
just bless             # BLESS=1 cargo test — regenerate golden files

# Coverage
just coverage          # summary table
just coverage-html     # HTML report, opens in browser
just coverage-lcov     # LCOV output for CI/editors
just coverage-nextest  # LCOV from the nextest test suite
just semver-check      # verify the active patch compatibility delta

# Documentation
just doc               # cargo doc --no-deps --open
just book              # mdbook build docs
just book-test         # compile and run mdBook examples
just book-serve        # mdbook serve with live reload
```

CI runs formatting, clippy, documentation, mdBook, semver, example-smoke,
nextest, MSRV, and coverage jobs on every PR. The local equivalents are the
recipes listed above; the test and coverage jobs use `cargo-nextest`.

## Code Style

- **Formatting:** `cargo fmt` with default rustfmt settings. CI enforces `cargo fmt --check`.
- **Linting:** `cargo clippy -- -D warnings`. All warnings are errors in CI (`RUSTFLAGS=-Dwarnings`).
- **No comments by default.** Only add a comment when the _why_ is non-obvious — a hidden constraint, a workaround, a surprising invariant. Don't explain what the code does; well-named identifiers do that.
- **No unnecessary abstractions.** Three similar lines are better than a premature helper. Don't add features, refactoring, or error handling beyond what the task requires.
- **Builder pattern:** Spec builders (`TypeSpec`, `ClosedSumSpec`, `ClosedSumCaseSpec`, `FunSpec`, `FieldSpec`, `FileSpec`, etc.) take `mut self` and return `Self` for every setter -- chain them fluently: `FunSpec::builder("f").returns(vec![t]).body(b).build()`. `CodeBlockBuilder` takes `&mut self` -- use a `let mut` binding and call methods on it.
- **Trait objects for language:** Public types no longer carry a language generic. The language enters at render time as `&dyn CodeLang`. `FileSpec` stores the language internally. `CodeBlock`, `TypeName`, and all specs have language-agnostic Rust types, but literal `CodeBlock` content is target-associated rather than automatically portable.
- **`BoxDoc` never appears in a language hook:** Current complete declaration
  lowerers and the accepted type-name seam return structured `CodeBlock`
  values. `type_name_render.rs` is being retired to the frozen 0.6.8 bridge;
  only the final renderer interprets prepared blocks into `BoxDoc` values.

## Testing

**Unit tests** live alongside source code in `#[cfg(test)]` modules.

**Integration tests** are in `tests/`, organized by language directory:
- `tests/<lang>/main.rs` — test harness entry point with submodules
- `tests/<lang>/quote_*.rs` — `sigil_quote!` macro tests
- `tests/<lang>/builder_*.rs` — builder API tests

**Golden tests** compare rendered output against files in `test-goldens/<lang>/`. The workflow:

1. Tests call `golden::assert_golden("lang/test_name.ext", &output)`
2. On normal runs, the assertion compares output against the golden file and fails on mismatch
3. `BLESS=1 cargo test` (or `just bless`) writes actual output to the golden files
4. Review the diffs in `test-goldens/` manually, then commit

When your change affects rendered output:
1. Run `just bless` to update golden files
2. `git diff test-goldens/` to review what changed
3. Commit the updated golden files together with your code change

**Adding a language:** See [Adding a Language](docs/src/adding_a_language.md) for the full walkthrough — implement `CodeLang`, add tests, run `just bless`, review golden files.

## Security Considerations

sigil-stitch generates source code. Malformed or malicious input to the code generator could produce dangerous output (injection, path traversal, etc.). Keep these principles in mind:

- **String literals are escaped.** `%S` / `StringLitArg` passes through `lang.render_string_literal()`, which escapes quotes, backslashes, and control characters per language. Never bypass this by using `%L` for user-supplied strings.
- **Identifiers from `%N` / `NameArg` are not escaped.** If an identifier comes from untrusted input, validate it before passing it to the code generator.
- **`%L` and `Raw` are escape hatches.** They emit content verbatim with no sanitization. Only use them for trusted, developer-controlled values.
- **File paths in `FileSpec` and `ProjectSpec`** are used as-is when writing to disk via `write_to()`. Validate paths before passing them in if they come from external input.
- **No network access.** The library is purely in-memory. `ProjectSpec::write_to()` writes to local disk but never makes network calls.
- **Dependencies are minimal.** `pretty`, `serde`, `snafu` — all well-audited, widely-used crates. No `unsafe` in sigil-stitch itself.

## Development Guide

The [sigil-stitch book](docs/src/SUMMARY.md) covers the internals:

- [Architecture](docs/src/architecture.md) — ownership, source preparation, import resolution, and rendering
- [TypeName Validation and Lowering](docs/src/type_name_lowering.md) — fallible language-owned type-expression lowering
- [Language-Aware Tokenizer](docs/src/macrolang.md) — `MacroLang` enum, annotation heuristics, runtime rewrite passes
- [Adding a Language](docs/src/adding_a_language.md) — implementing the CodeLang trait step by step

Build the book locally:

```bash
just book          # build the book
just book-serve    # live-reload
```

## License

Licensed under either of Apache License 2.0 or MIT License at your option. Any contribution intentionally submitted for inclusion in the work shall be dual licensed as above, without any additional terms or conditions.
