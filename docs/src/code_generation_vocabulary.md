# Code Generation Vocabulary

This appendix defines the vocabulary used throughout sigil-stitch. The
[architecture overview](architecture.md) describes how these concepts flow
through the implementation, while [Declaration Specs and Language
Lowering](declaration_lowering.md) records their ownership boundaries.

## Declaration Intent

### Declaration spec

A structured, language-independent request for a declaration such as a type,
function, field, or variant. It records semantic intent, not target-language
token placement or spelling.

### Capability

A semantic feature that a target language may support, require, or reject for
a particular declaration kind and context. A capability describes what can be
represented; it is not a switch for target grammar.

### Language adapter

The target-specific boundary that decides whether declaration intent is
representable and converts accepted intent into target-language structure. It
owns detailed declaration grammar such as keyword order, punctuation, and
metadata placement.

### Function intent

A complete function declaration classified by its role and context before
target-language validation. It remains semantic and does not contain a
partially rendered signature.

### Variant sequence

The ordered variants owned by one type declaration, together with the presence
of members that follow them. A language adapter handles the sequence as a
whole; first and last positions are derived from that sequence.

## Variant Data

### Discriminant

An explicit value that identifies an enum member in a representation where
members map to values. It is distinct from an expression passed to an enum
constructor.

### Constructor arguments

Expressions passed when an enum entry constructs an instance of its declaring
enum type. They are values evaluated at the declaration site, not types carried
by a sum-type case.

### Positional payload

Types carried in order by a sum-type constructor or enum case. The payload has
positions but no field names.

### Record payload

Named, typed fields carried by a sum-type constructor or enum case. These are
case-local payload fields, not ordinary members of the enclosing type.

## Transformation Boundaries

### Lowering

The conversion of validated declaration intent into structured output that
follows one target language's grammar. Lowering decides source structure but
does not perform final layout.

### Rendering

The final interpretation of structured output into source text, including
layout, indentation, import aliases, and width-aware line breaking. Rendering
does not decide whether a declaration is representable.

### Escape hatch

An explicitly target-specific payload embedded in otherwise structured intent
when the shared declaration vocabulary cannot express a source fragment. An
escape hatch deliberately gives up portability for that fragment.
