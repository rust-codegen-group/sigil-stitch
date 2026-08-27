# Test Topology

Run the full Rust checks with:

```text
just check
```

The integration suites are grouped by the contract they protect:

- `tests/<language>/` covers builder and `sigil_quote!` output for one target.
- `tests/*_capability_matrix_tests.rs` checks the declared semantic support
  matrix independently of rendering.
- `tests/*_lowering_tests.rs` checks complete declaration lowering and
  fail-closed validation.
- `tests/renderer_parity_tests.rs` covers all built-in languages on the direct
  and pretty renderer paths.
- `tests/golden.rs` owns checked source goldens. Update them only with
  `just bless`, then inspect every changed fixture.
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

The compatibility fixtures live under `tests/compatibility/`. Their manifest
is bounded to public behavior supported from 0.6.8; they must not acquire
binary-serialization, enum-ordinal, field-order, or general cross-version
Serde promises.
