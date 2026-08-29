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
- `tests/declaration_generic_lowering_tests.rs` owns the canonical 20-language
  type/function declaration matrix for zero, one, many, bounded, lifetime,
  higher-kinded, context-bound, and explicit-constraint cases. It also checks
  imported bound aliases, wide/narrow rendering, and strict missing-lowerer
  failure.
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
cargo test --test renderer_parity_tests
cargo test --test declaration_generic_lowering_tests
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
wide and moderately nested trees at three input sizes:

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
