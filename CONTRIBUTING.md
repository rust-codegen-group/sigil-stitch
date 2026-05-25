# Contributing to sigil-stitch

Thank you for your interest in contributing!

## Project Overview

sigil-stitch is a Rust library for type-safe, import-aware, width-aware code generation across multiple languages. It combines JavaPoet's builder + CodeBlock model with Wadler-Lindig pretty printing.

**Workspace layout:**

```
sigil-stitch/
├── src/                    # Main library crate
│   ├── code_block.rs       # CodeBlock and format string parsing
│   ├── code_node.rs        # CodeNode IR (tree nodes for CodeBlock)
│   ├── code_renderer.rs    # Three-pass rendering pipeline
│   ├── code_template.rs    # Named-parameter templates
│   ├── type_name.rs        # TypeName enum and TypePresentation rendering engine
│   ├── import.rs           # Import types and conflict resolution
│   ├── import_collector.rs # Import extraction from CodeBlock trees
│   ├── name_allocator.rs   # Alias generation for import conflicts
│   ├── error.rs            # Error types (snafu)
│   ├── lang/               # CodeLang trait + 6 config struct accessors + language implementations
│   │   ├── mod.rs          # CodeLang trait (33 methods)
│   │   ├── config.rs       # Config structs (BlockSyntaxConfig, FunctionSyntaxConfig, etc.)
│   │   ├── rewrite.rs      # Runtime node rewrite walker
│   │   ├── typescript.rs   # One file per language: typescript, javascript,
│   │   └── ...             # rust, go, python, java, kotlin,
│   │                       # swift, dart, scala, haskell, ocaml, c,
│   │                       # cpp, bash, zsh, lua, csharp
│   └── spec/               # Structural builders (emit CodeBlocks)
│       ├── type_spec.rs    # Class, struct, interface, trait, enum, type alias, newtype
│       ├── fun_spec.rs     # Functions and methods
│       ├── field_spec.rs   # Struct fields / class properties
│       ├── file_spec.rs    # Top-level file orchestrator
│       ├── project_spec.rs # Multi-file generation
│       └── ...             # ParameterSpec, PropertySpec, AnnotationSpec, etc.
├── macros/                 # sigil_quote! proc macro crate
│   └── src/parse/          # Tokenizer: format.rs (annotations), statements.rs, types.rs (MacroLang)
├── tests/                  # Integration tests (one directory per language)
│   ├── <lang>/             # main.rs + quote_*.rs + builder_*.rs submodules
│   └── golden.rs           # Golden test assertion helper
├── test-goldens/           # Golden test output files (one directory per language)
└── docs/                   # mdbook documentation
```

**Key dependencies:** `pretty` (Wadler-Lindig formatting), `serde` + `derive` (serialization), `snafu` (errors).

For a deeper dive, see the [Architecture](docs/src/architecture.md) chapter.

## Build and Test Commands

Requires Rust edition 2024, MSRV 1.88.0. Install [just](https://github.com/casey/just) for the task runner.

```bash
# Core workflow
just test              # cargo test --workspace
just lint              # cargo clippy -- -D warnings
just check             # test + lint

# Golden tests
just bless             # BLESS=1 cargo test — regenerate golden files

# Coverage
just coverage          # summary table
just coverage-html     # HTML report, opens in browser
just coverage-lcov     # LCOV output for CI/editors

# Documentation
just doc               # cargo doc --no-deps --open
just book              # mdbook build docs
just book-serve        # mdbook serve with live reload
```

CI runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc -D warnings`, and coverage upload on every PR.

## Code Style

- **Formatting:** `cargo fmt` with default rustfmt settings. CI enforces `cargo fmt --check`.
- **Linting:** `cargo clippy -- -D warnings`. All warnings are errors in CI (`RUSTFLAGS=-Dwarnings`).
- **No comments by default.** Only add a comment when the _why_ is non-obvious — a hidden constraint, a workaround, a surprising invariant. Don't explain what the code does; well-named identifiers do that.
- **No unnecessary abstractions.** Three similar lines are better than a premature helper. Don't add features, refactoring, or error handling beyond what the task requires.
- **Builder pattern:** Spec builders (`TypeSpec`, `FunSpec`, `FieldSpec`, `FileSpec`, etc.) take `mut self` and return `Self` for every setter -- chain them fluently: `FunSpec::builder("f").returns(t).body(b).build()`. `CodeBlockBuilder` takes `&mut self` -- use a `let mut` binding and call methods on it.
- **Trait objects for language:** Public types no longer carry a language generic. The language enters at render time as `&dyn CodeLang`. `FileSpec` stores the language internally; `CodeBlock`, `TypeName`, and all specs are language-agnostic.
- **`BoxDoc` never appears in `CodeLang`:** Language implementations return pure data (`TypePresentation`, config structs). The rendering engine in `type_name.rs` and `code_renderer.rs` interprets the data into `BoxDoc`. This is a hard invariant.

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

- [Architecture](docs/src/architecture.md) — four layers, three-pass pipeline, import resolution
- [Type Presentation](docs/src/type_presentation.md) — data-driven cross-language type rendering
- [Language-Aware Tokenizer](docs/src/macrolang.md) — `MacroLang` enum, annotation heuristics, runtime rewrite passes
- [Adding a Language](docs/src/adding_a_language.md) — implementing the CodeLang trait step by step

Build the book locally:

```bash
just book          # build the book
just book-serve    # live-reload
```

## License

Licensed under either of Apache License 2.0 or MIT License at your option. Any contribution intentionally submitted for inclusion in the work shall be dual licensed as above, without any additional terms or conditions.
