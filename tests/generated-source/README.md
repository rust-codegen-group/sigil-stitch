# Generated Source Acceptance

This local integration command compiles complete files emitted by
`examples/generated_source_acceptance.rs` through public `FileSpec`,
declaration, and `TypeName` APIs:

```text
just generated-source-acceptance
```

The Rust runner requires Cargo, Clang C++, Go, GHC, Vite+'s `vp`, and Scala 3's
`scalac` on PATH. TypeScript runs through
`vp dlx --package typescript@7.0.2 tsc`, without a global `tsc` installation
or repository npm dependencies. The first run may download that package into
vp's package cache. The command is deliberately separate from `just check`, `just test`,
and ordinary Rust CI. Missing tools, failed generation, failed positive
compilation, and unexpected negative diagnostics all fail the command.
No global compiler installation is performed.

`manifest.json` owns exact compiler flags, positive/negative pairings, and
expected diagnostics. The runner prints compiler versions on each run and uses
a temporary directory for generated sources and compiler artifacts. These are
local prerequisites, not a compiler minimum-version support promise.
The fixtures have been checked with Apple Clang 21.0.0, GHC 9.14.1,
Go 1.27.1, TypeScript 7.0.2 through vp, and Homebrew Scala 3.9.0.

Each negative fixture shares the positive declaration and setup. A unique
`acceptance-failure` marker identifies its intended failing use-site; the
runner requires an error at that line and the matching diagnostic. A setup
failure or a note at that line does not count as success.
Go's `go build` diagnostics use `file:line:column` without an `error` keyword;
that header rule is limited to Go. Another diagnostic cannot supply the
expected message for the marked use-site.

| Fixture family | Positive use-sites | Negative use-sites |
|----------------|--------------------|--------------------|
| C++ | Independent packs of different lengths, matching simultaneous `Pair<As, Bs>...`, empty packs and `Bundle<>` | Mismatched simultaneous pack lengths |
| Haskell | `Array 2 Int -> Array 3 Int -> Array 5 Int` | Result declared as `Array 6 Int` |
| Go | Zero/single/multiple declaration and callable returns, receiver/interface conformance, imported result slots and destructuring | Assign a two-result call to one scalar |
| TypeScript | Supplied and omitted optional scalar slots; explicit async empty returns used as `Promise<void>` | Missing required slot; incompatible argument |
| Scala | `List` supplied to a constructor-kind binder | Scalar `Int` supplied as that constructor |

The emitter also asserts full expansion patterns, requested callable labels,
indexed results/imports, and constructor kinds. Compiler success alone would
not prove those semantic facts. Caller-supplied fixture bodies are not executed.
Go fixtures are compiled with `go build`; no generated function is run.

Haskell extensions are supplied explicitly by the manifest: `DataKinds`,
`KindSignatures`, `ExplicitForAll`, `TypeOperators`, and
`ExplicitNamespaces`. Sigil-stitch does not infer or collect pragmas.
Scala uses top-level declarations and therefore requires Scala 3.

To verify a selected installed tool without claiming the complete suite passed:

```text
just generated-source-acceptance --language cpp --language haskell
cargo test --example source_acceptance
```

The Rust tests exercise the diagnostic protocol with synthetic messages and run
as part of ordinary Rust checks without invoking target compilers;
they are not substitutes for target-compiler acceptance.
