# Test Topology

Run the full Rust checks with:

```text
just check
```

The integration suites are grouped by the contract they protect:

- `tests/<language>/` covers builder and `sigil_quote!` output for one target.
- `tests/*_capability_matrix_tests.rs` checks the declared semantic support
  matrix independently of rendering. These suites construct adapters through
  `tests/shared/languages.rs`; each file keeps only its domain-specific expected
  capability profiles.
- `tests/*_lowering_tests.rs` checks complete declaration lowering and
  fail-closed validation.
- `tests/closed_sum_spec_tests.rs` covers the dedicated closed-sum builders,
  adapter views, first-class and extension file routes, scoped record-field
  capabilities, supported and empty output, target constraints and visibility,
  annotation validation, and same-version Serde round-trips.
- `tests/closed_sum_validation_tests.rs` covers malformed deserialized payloads,
  deterministic intrinsic/duplicate/profile error ordering, diagnostic parity
  between file routes, unsupported-form field-validation boundaries, and
  exactly-once lowering with error and empty-output rejection.
- `tests/closed_sum_lowering_tests.rs` preserves language-owned output and
  rejection regressions: ordinary-enum isolation, case identifiers and field
  metadata, supported generic combinations, nested parameter occurrence,
  payload imports and alias conflicts, caller-owned wire tags, and wide/narrow
  rendering for native, nested, and sibling representations.
- `tests/declaration_generic_lowering_tests.rs` owns the canonical 20-language
  type/function declaration matrix for zero, one, many, bounded, lifetime,
  higher-kinded, context-bound, and explicit-constraint cases. It also checks
  imported bound aliases, wide/narrow rendering, and strict missing-lowerer
  failure.
- `tests/parametric_callable_tests.rs` covers the structured parametric type
  surface: ordered application arguments, expansion imports, labelled and
  optional callable slots, repeated segments, C++ independent function-template
  packs, Haskell kinds and indexed signatures, and Scala constructor kinds.
  It also checks borrowed binding views, mixed-origin duplicate names,
  same-version owner revalidation, frozen compatibility rejection, target-local
  empty-application rules, and modern Rust lifetime constraints for every owner.
  Adapter unit tests separately exercise native generic-domain validation and
  recursive kind/parameter/import walkers without widening capability profiles.
- `tests/quote_structured_types.rs` checks modern types through existing macro
  entry points and complete file rendering: nested callable/application imports,
  compound `$T_join` alias conflicts, C++ complete expansion patterns, Haskell
  operator qualification, fail-closed unsupported types, `$C`/`$L` structured
  splices, and emitted generic closed sums. Wide/narrow output is checked against
  the builder path where applicable. These are Rust integration tests, not target-compiler
  acceptance tests; no new macro syntax is introduced.
- `tests/renderer_parity_tests.rs` covers all built-in languages on the direct
  and pretty renderer paths, the exact five-operation renderer-event matrix, a
  fully migrated external adapter that does not use legacy block config,
  non-default built-in indentation, nested and sequenced event ordering,
  fail-closed external event errors, resolved-import validation, and the exact
  output or rejection for every current `TypeName` variant in every built-in
  language.
  Every cross-language parity or capability matrix consumes the canonical
  20-adapter inventory in `tests/shared/languages.rs`; the exact type-grammar
  expectations are duplicated here deliberately as integration evidence for
  the language-owned lowerers.
- `tests/file_spec_tests.rs` owns complete-file pipeline traces and fail-closed
  ordering. Its stateful external adapter records validation, custom-spec
  emission, source rewrite, per-root type lowering, and all four renderer
  events. Its pipeline matrices cross preserve, remove, replace, and introduce
  rewrite effects with root, nested, and sequence positions; cover headers,
  stored code, one-block specs, and every block from multi-block specs; and
  verify primitive, importable, compound, invalid, and target-derived raw import
  metadata without rewriting opaque bytes. The same suite covers standalone
  rendering and borrowed resolver behavior.
- `tests/project_spec_tests.rs` owns cross-file orchestration: ordered
  aggregation preserves each file's complete member diagnostics, validation
  finishes before any file emission, and validation or later render failures
  leave the filesystem untouched.
- `tests/closed_sum_file_spec_tests.rs` checks first-class same-version file
  serialization, rejection of type-erased spec serialization, and an ordered
  project trace: all models validate, the earlier file renders in memory,
  a failing declaration lowerer skips preparation and later files, and no
  destination file changes. Private file-storage unit tests check all public
  member conversions and first-class versus extension-spec routing.
- `tests/closed_sum_regression_tests.rs` covers target-local record names and
  normalized parameters, ordered declaration/type and case-form/record
  diagnostics, and case-only Dart metadata. The closed-sum lowering suite
  checks positional and record imports in all eight adapters, including
  direct and pretty rendering for the algebraic lowerers.
- `tests/closed_sum_contract_tests.rs` checks complete validated views,
  declaration/case metadata preservation, malformed same-version JSON,
  intrinsic builder failures, and target-owned generic and lifetime grammar.
- `tests/import_spec_tests.rs` exercises public target import forms. The focused
  unit matrix in `src/import.rs` owns conflict-set construction, resolver
  validation, semantic identity deduplication, and stable passthrough ordering.
- `tests/shared/golden.rs` owns checked source goldens. Missing and mismatched
  fixtures never write by default, matching fixtures succeed without mutation,
  and only explicit blessing creates or replaces files. Update fixtures only
  with `just bless`, then inspect every changed fixture, rerun the focused helper
  test without `BLESS`, and retain a semantic assertion for changes that
  blessing alone could conceal.
- `tests/compatibility_0_6_8.rs` is the external-crate compatibility fixture
  for exact 0.6.8 signatures, frozen bridges, marker recovery, legacy import
  resolution, and documented TypeName JSON values.
- `tests/compatibility_semver_script.rs` exercises the pinned semver report
  parser against zero, expected, duplicate, malformed, missing, unexpected,
  and aborted-run fixtures.
- `tests/ui/parametric/` checks downstream deprecation diagnostics for old
  variants, constructors, and both released binding builders, plus warning-free
  modern construction and non-exhaustive matching.
- `tests/generated-source/` documents the local generated-source compiler
  acceptance command, its paired positive/negative use-sites, required tools,
  and diagnostic protocol. It is not part of ordinary Rust CI.
  The Rust runner in `examples/source_acceptance.rs` has synthetic diagnostic
  protocol tests included in ordinary Rust checks; these do not invoke compilers.

Run only the compatibility gates with:

```text
cargo test --test compatibility_0_6_8
just semver-check
```

Focused cross-language and fixture-harness checks are:

```text
cargo test --test capability_matrix_tests
cargo test --test field_capability_matrix_tests
cargo test --test function_capability_matrix_tests
cargo test --test property_capability_matrix_tests
cargo test --test variant_capability_matrix_tests
cargo test --test closed_sum_spec_tests
cargo test --test closed_sum_validation_tests
cargo test --test closed_sum_lowering_tests
cargo test --test renderer_parity_tests
cargo test --test declaration_generic_lowering_tests
cargo test --test quote_structured_types
cargo test --test typescript shared::golden::tests
```

Focused pipeline and import-resolution checks are:

```text
cargo test --test file_spec_tests
cargo test --test import_spec_tests
cargo test import::tests
```

When adding a built-in language, update `tests/shared/languages.rs`; every
registry-driven parity and capability suite must then pass. When adding a
`TypeName` variant, update the exhaustive list in `renderer_parity_tests.rs` and
the owning language lowerers. When adding a `CodeNode` variant, update the
exhaustive lowered-output classifier in `src/type_name_lowering/validation.rs`.
When changing a 0.6.8 bridge or approving a semver break, update the exact
compatibility fixture, manifest or allowlist, and its README in the same change.

The non-gating TypeName materialization benchmark reports throughput for
wide and moderately nested trees at three input sizes. Matched legacy/modern
application and callable cases also measure direct rendering and complete
`FileSpec` rendering with repeated imports, mixed request evidence, and name
conflicts. Inputs are constructed outside timed iterations, and matched files
must render identical source. Benchmark smoke checks run with
`cargo bench --bench type_name_lowering -- --test`; they are not timing evidence:

```text
just bench-type-name-lowering
```

Use its Criterion output as review evidence. Timing ratios are not a CI or
merge threshold; exact rewrite and lowering counts remain functional test
assertions.

The compatibility fixtures live under `tests/compatibility/`. Their manifest
is bounded to public behavior supported from 0.6.8; they must not acquire
binary-serialization, enum-ordinal, field-order, or general cross-version
Serde promises.

Renderer-event ordering and failures are exercised through public `CodeBlock`
construction and external adapters in `renderer_parity_tests.rs`. The same
ordered statement/open/transition/close trace runs through both adapters and
proves that rendering stops at the first language error.
