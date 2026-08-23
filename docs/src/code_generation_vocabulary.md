# Code Generation Vocabulary

This appendix defines the vocabulary used throughout sigil-stitch. The
[architecture overview](architecture.md) describes how these concepts flow
through the implementation, while [Declaration Specs and Language
Lowering](declaration_lowering.md) records their ownership boundaries.

## Declaration Intent

### Declaration spec

A structured, language-independent request for a declaration such as a type,
function, field, property, or variant. It records semantic intent, not
target-language token placement or spelling.

### Capability

A semantic feature that a target language may support, require, or reject for
a particular declaration kind and context. A capability describes what can be
represented; it is not a switch for target grammar.

### Language adapter

The target-specific boundary that decides whether declaration intent is
representable and converts accepted intent into target-language structure. It
owns detailed declaration grammar such as keyword order, punctuation, and
metadata placement.

### Target grammar

The language-specific keyword order, punctuation, precedence, escaping, and
metadata placement used to express accepted declaration intent. Target grammar
belongs to the language adapter; it is not a general format abstraction or a
capability.

### Function intent

A complete function declaration classified by its role and context before
target-language validation. It remains semantic and does not contain a
partially rendered signature.

### Field sequence

The ordered fields owned by one type declaration or one record payload,
considered together in their semantic context. A language adapter handles the
complete sequence so it can validate collisions and own sequence-level grammar.

### Field context

The semantic role in which a field sequence appears: direct emission, ordinary
type members, or a variant record payload. It identifies representability; it
does not prescribe placement, punctuation, or separators. The
`Direct(DeclarationContext)` payload retains only the pre-0.6.8 direct-emission
placement input. It is a narrow compatibility exception, not a reusable
placement or target-grammar model.

### Property intent

One computed property with a value type, read and/or write behavior, semantic
modifiers, documentation, and attributes before target-language validation.
It does not choose accessor syntax or a field-style representation.

### Property context

The semantic role in which one computed property appears: direct emission or a
member of an owning `TypeKind`. The `Direct(DeclarationContext)` payload exists
only to retain the pre-0.6.8 public emission facade; it is not a general
accessor-placement model.

### Type members intent

A validation-only view of one owning type's semantic fields, computed
properties, and explicit methods. `TypeMembersIntent` exists for relationships
that cannot be checked within one member family, such as target-derived name
collisions. It contains no target grammar, has no validated wrapper, and does
not participate in lowering. It is not a sequence-level replacement for
`PropertyIntent`.

### Read accessor and write accessor

Semantic read and write behavior supplied by a property's getter and setter
bodies. A language adapter may express that behavior as accessor declarations,
a computed-property body, or target-local methods. The capability names do not
prescribe getter keywords, setter placement, or surrounding grammar.

### Optional presence

A field semantic in which the containing value may omit the field entirely.
`FieldSpec::is_optional()` requests this meaning. It is distinct from an
optional value.

### Optional value

A value semantic in which a present field can carry the target language's
absence or null representation. `TypeName::Optional` expresses this meaning;
it does not make the field itself omissible.

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
