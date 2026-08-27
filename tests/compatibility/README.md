# 0.6.8 Compatibility Manifest

`public-api-0.6.8.txt` is the bounded inventory of public surfaces touched by
the language-owned-lowering migration. It was audited against the `0.6.8` Git
tag, not inferred from the current implementation.

Each record is `kind|public surface|retirement owner`. `compat` means the
deprecated surface remains a frozen compatibility boundary through 0.7;
the other owners name the behavior that removes current-path reads while
leaving the 0.6.8 facade available: `type-name-lowering`,
`generic-declaration-lowering`, `renderer-events`, or `quote-handling`.

The inventory is checked in three ways:

- `compatibility_0_6_8.rs` compiles exact restored signatures through the
  external crate boundary and exercises the compatibility bridges.
- `cargo-semver-checks 0.49.0` checks the entire public Rust surface against
  tag `0.6.8`; the allowlist is empty.
- The manifest test rejects malformed or duplicate inventory records and
  requires the restored signatures, JSON fixture, complete legacy grammar-hook
  inventory, quote shims, and both infallible import resolvers to be named.

`type-name-0.6.8.json` contains only the documented JSON values covered by the
0.7 migration contract. It does not define a general Serde protocol, binary
format, enum ordinal, struct field order, or serializer-byte guarantee.
