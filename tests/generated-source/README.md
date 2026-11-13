# Generated Source Acceptance

This local integration command compiles complete files emitted by
`examples/generated_source_acceptance.rs` through public `FileSpec`,
declaration, `TypeName`, and `FloatValue` APIs:

```text
just generated-source-acceptance
```

The runner requires Cargo, rustc, Clang C++, Go, GHC, a JDK's `javac`, Vite+'s
`vp`, and Scala 3's `scalac` on PATH. TypeScript runs through
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
The fixtures have been checked with Apple Clang 21.0.0, GHC 9.14.1, OpenJDK
21.0.12, Go 1.27.1, TypeScript 7.0.2 through vp, and Homebrew Scala 3.10.0.
The Rust floating-value fixtures have been checked with rustc 1.99.0.

Each negative fixture shares the positive declaration and setup. A unique
`acceptance-failure` marker identifies its intended failing use-site; the
runner requires an error at that line and the matching diagnostic. A setup
failure or a note at that line does not count as success.
Go's `go build` diagnostics use `file:line:column` without an `error` keyword;
that header rule is limited to Go. Another diagnostic cannot supply the
expected message for the marked use-site.
Rust selects `--error-format=short` to keep the error location and diagnosis in
one header; it uses the existing exact-location protocol, not a new parser.

| Fixture family | Positive use-sites | Negative use-sites |
|----------------|--------------------|--------------------|
| Rust | `no_std` const assertions recover requested binary32/binary64 bits, including signed zeros, subnormal/normal boundaries, infinities, quiet/signaling NaN payloads and every individual bit; local names cannot capture primitive paths | Use a generated binary32 expression where binary64 is required |
| C++ | Independent packs of different lengths, matching simultaneous `Pair<As, Bs>...`, empty packs and `Bundle<>` | Mismatched simultaneous pack lengths |
| Haskell | `Array 2 Int -> Array 3 Int -> Array 5 Int` | Result declared as `Array 6 Int` |
| Go | Zero/single/multiple declaration and callable returns, receiver/interface conformance, imported result slots and destructuring | Assign a two-result call to one scalar |
| Java | A checked `IOException` declaration and caller that handles it | Caller omits handling; a lowerable `List<IOException>` exception shape rejected by `javac` |
| TypeScript | Supplied and omitted optional scalar slots; explicit async empty returns used as `Promise<void>`; exact integer singletons, negative-literal arrays and literal unions | Missing required slot; incompatible argument; wrong integer; bigint supplied to a number singleton |
| Scala | `List` supplied to a constructor-kind binder; signed Int/Long singleton boundaries, integer unions, containers and callable types | Scalar `Int` supplied as that constructor; wrong Int/Long singleton or integer outside a union |

The emitter also asserts full expansion patterns, requested callable labels,
indexed results/imports, and constructor kinds. Compiler success alone would
not prove those semantic facts. Caller-supplied fixture bodies are not executed.
Go fixtures are compiled with `go build`; no generated function is run.
Rust bit-preservation assertions are evaluated by rustc at compile time; no
generated executable is run.

The TypeScript integer fixtures prove singleton/domain restrictions at use-sites.
They do not prove exact preservation for arbitrary decimal input: the compiler
can accept rounded number literals. Rust tests in `integer_literal_types.rs`
separately check exact binary64 values, rounded neighbors and overflow rejection.
Scala fixtures likewise check exact Int/Long spellings and reject wrong singleton
values; Rust tests separately reject out-of-range inputs before source emission.

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
