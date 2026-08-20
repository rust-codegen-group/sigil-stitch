# Design

These chapters record the intended seams and ownership rules behind
sigil-stitch. They complement the [architecture overview](architecture.md),
which explains how data flows through the implementation.

- [Declaration Specs and Language Lowering](declaration_lowering.md) defines
  the distinction between declaration intent, semantic capabilities,
  target-language grammar, the rendering IR, and final rendering.
- [Type Presentation](type_presentation.md) describes the separate seam for
  lowering semantic `TypeName` values into width-aware documents.
- [Language-Aware Tokenizer](macrolang.md) describes the private typed pipeline
  used by `sigil_quote!`.

Design chapters describe the accepted direction even while a compatibility
path is still being migrated. Each such chapter calls out transitional behavior
explicitly so it is not mistaken for the desired interface.

These chapters document the selected design and its invariants, not a catalogue
of every rejected alternative. When the history of a hard-to-reverse trade-off
is important, it belongs in a focused record under `docs/adr/`.
