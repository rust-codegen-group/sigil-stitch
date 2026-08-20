# Code Generation Model

sigil-stitch describes source-level intent independently of any one target
language, then delegates representability and concrete syntax to a target
language adapter.

## Language

**Declaration spec**:
A structured request for a declaration, such as a type, function, field, or
variant. It contains semantic facts and may contain explicitly opaque target
code, but it does not describe target token placement or spelling.
_Avoid_: Syntax spec, format spec

**Capability**:
A semantic feature that a target language can support, require, or reject in a
particular declaration context.
_Avoid_: Syntax option, rendering flag

**Language adapter**:
The target-specific module that validates whether declaration intent is
representable and lowers that intent into target-language structure.
_Avoid_: Format configuration

**Function intent**:
A read-only classified declaration presented to a language adapter before
target validation. Successful validation produces a `ValidatedFunction` for
lowering; generic specs do not inspect opaque target payloads to reach it.
_Avoid_: Partially rendered signature

**Lowering**:
The conversion of validated declaration intent into structure that follows one
target language's grammar.
_Avoid_: Formatting, rendering

**Escape hatch**:
An explicitly target-specific payload embedded in otherwise structured intent,
used when the shared declaration vocabulary cannot express a source fragment.
_Avoid_: Portable declaration intent
