# Design

These chapters record the intended seams and ownership rules behind
sigil-stitch. They complement the [architecture overview](architecture.md),
which explains how data flows through the implementation.

- [Declaration Specs and Language Lowering](declaration_lowering.md) defines
  the distinction between declaration intent, semantic capabilities,
  target-language grammar, structured source blocks, and final rendering.
- [TypeName Validation and Lowering](type_name_lowering.md) defines the
  fallible language-owned seam that materializes semantic `TypeName` values
  before import collection.
- [Language-Aware Tokenizer](macrolang.md) describes the private typed pipeline
  used by `sigil_quote!`.

Design chapters describe the accepted 0.7 built-in architecture. Declaration
lowering is implemented; type-name lowering is the next accepted migration and
is documented before implementation so its public boundary is fixed first.
Frozen 0.6.8 compatibility lowerers remain for external adapters and legacy
direct facades; each chapter points to the compatibility appendix where that
boundary matters.

These chapters document the selected design and its invariants, not a catalogue
of every rejected alternative. When the history of a hard-to-reverse trade-off
is important, it belongs in a focused record under `docs/adr/`.
