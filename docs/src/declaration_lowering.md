# Declaration Specs and Language Lowering

This chapter defines the ownership model for structured declarations. It is the
accepted design direction for `spec/*` and `lang/*`; pre-0.6.8 compatibility
paths that do not yet follow it are described under
[Migration](#compatibility-and-migration).

## Decision

Declaration specs describe **what to declare**. A language adapter decides
whether that intent is representable and owns **how to spell it**. Specs do not
assemble a target-language grammar by interpreting a shared collection of
keywords, separators, placement enums, or ordering flags.

The complete pipeline is:

```text
builder
  |
  v
declaration spec                 target-independent intent
  |
  +-- intrinsic validation      invariants of the intent itself
  |
  +-- capability validation     target-specific representability
  |
  v
language-local lowering         exact grammar, spelling, and token order
  |
  v
CodeBlock / CodeNode::TypeRef    structured target-language rendering IR
  |
  +-- import and alias resolution
  +-- layout and indentation
  |
  v
source text
```

This is a compiler pipeline, not a general declaration-formatting engine.

## Ownership

| Concern | Owner | Examples |
|---------|-------|----------|
| Declaration intent | `spec/*` | Name, parameters, result type, type parameters, bounds, members, visibility intent, modifiers, body |
| Intrinsic coherence | `spec/*` | Non-empty names, internally consistent parameter lists, valid builder state |
| Target representability | language capabilities and validation | Whether a context supports type parameters, requires typed parameters, permits a body, or accepts a constructor |
| Target grammar | language adapter | Keywords, ordering, placement, punctuation, modifier spelling, constructor syntax |
| Structured output | `CodeBlock` | Target literals plus semantic `TypeRef`, nesting, statement, and layout nodes |
| Final text mechanics | renderer | Imports, aliases, indentation, width decisions, and string emission |

The ownership test is deliberately simple:

- A fact about the requested declaration belongs to the spec.
- A statement that the target supports, requires, or forbids a semantic fact
  belongs to capability validation.
- A decision about which token appears, where it appears, or in what order it
  appears belongs to language-local lowering.
- A decision about import names, indentation, width, or document layout belongs
  to the renderer.

## Declaration Specs

A declaration spec is a target-independent declaration model, not the syntax
tree of a hypothetical universal language. It may be richer than any one
target. A target adapter either lowers the requested semantics or returns a
validation error; it must not silently discard unsupported intent.

For example, one function declaration may contain:

```text
name: id
type parameters: T
parameters: x of type T
result: T
body: ...
```

That intent can become:

```text
Kotlin: fun <T> id(x: T): T
Rust:   fn id<T>(x: T) -> T
Java:   <T> T id(T x)
```

There is no semantic `type parameter placement` property in the declaration.
Placement exists only after selecting a target grammar.

Specs can contain target-specific `CodeBlock` payloads for bodies, raw
annotations, suffixes, or other escape hatches. These payloads are explicitly
opaque to generic specs and shared lowerers: their presence does not make the
declaration shell or its grammar a shared syntax model. Lowering composes them
structurally and preserves their `TypeRef` nodes, but does not reinterpret their
literal syntax. A private Python validator recognizes the documented 0.6.8
`is_static` plus decorator pattern as a frozen adapter-local compatibility
exception; new behavior must use semantic intent instead of extending it.

## Capabilities Are Semantic

The shared capability vocabulary describes representability. For example,
`ParametricPolymorphism` says that a declaration context can express type
parameters; `TypedParameters` says that parameter types are supported or
required; and `FunctionBodyPolicy` says whether a body is legal. None of these
concepts describes the position or spelling of a token.

Capabilities may be contextual. A target can support a bodyful top-level
function while forbidding a body on an interface member, or support ordinary
methods while rejecting constructors. Such differences remain semantic
validation rules even though the rules are language-specific.

If a proposed capability cannot be defined without mentioning a keyword,
delimiter, token order, or formatting example, it is probably target grammar
rather than a semantic capability.

## Language-Local Lowering

The external declaration seams first validate classified intent and then lower
a complete validated declaration into a structured block:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::error::SigilStitchError;
# use sigil_stitch::lang::{FunctionIntent, ValidatedFunction};
# trait Example {
fn validate_function(
    &self,
    function: FunctionIntent<'_>,
) -> Result<(), SigilStitchError>;
fn lower_function(
    &self,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError>;
# }
```

Enum variants use the same shape at sequence granularity:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::error::SigilStitchError;
# use sigil_stitch::lang::{ValidatedVariants, VariantIntent};
# trait Example {
fn validate_variants(&self, variants: VariantIntent<'_>)
    -> Result<(), SigilStitchError>;
fn collect_variant_validation_errors(
    &self,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
);
fn lower_variants(&self, variants: ValidatedVariants<'_>)
    -> Result<CodeBlock, SigilStitchError>;
# }
```

Fields also cross the adapter boundary as one complete sequence:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::code_block::CodeBlock;
# use sigil_stitch::error::SigilStitchError;
# use sigil_stitch::lang::{FieldSequenceIntent, ValidatedFields};
# trait Example {
fn validate_fields(&self, fields: FieldSequenceIntent<'_>)
    -> Result<(), SigilStitchError>;
fn collect_field_validation_errors(
    &self,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
);
fn lower_fields(&self, fields: ValidatedFields<'_>)
    -> Result<CodeBlock, SigilStitchError>;
# }
```

`FunctionIntent` provides read-only access after context and form
classification and crate-owned semantic validation against the selected
adapter. `ValidatedFunction` can only be constructed by the crate after the
adapter's additional validation succeeds. `FunSpec::emit()` remains the
convenience facade: it delegates validation and lowering without interpreting
target grammar switches itself.

`VariantIntent` provides the owner name and kind, every variant in declaration
order, whether ordinary members follow, structured-constructor arity evidence,
and whether opaque members may provide target-specific constructor syntax. The
adapter derives first/last position and owns preambles, payload grammar,
separators, and section termination for the complete sequence. Variant
capabilities name semantic forms—discriminant, constructor arguments,
positional payload, record payload, and attributes—not their spelling. The
additive collector reports independent sibling failures; `ValidatedVariants`
is constructed only after intrinsic, profile, and every adapter-local
validation phase succeeds.

`FieldSequenceIntent` provides every field in declaration order and a semantic
`FieldContext`: direct emission, ordinary type members, or a variant record
payload. Owner and variant names are included when they exist. Field profiles
declare supported and required semantic capabilities for each context, while
the adapter-local validator handles identifier rules, escaped-name collisions,
modifier combinations, and target-specific restrictions. The additive
collector retains independent sibling failures during `FileSpec` validation;
`ValidatedFields` is created only after intrinsic, profile, and adapter-local
validation all succeed. `lower_fields()` owns the sequence's complete grammar,
including documentation, annotations, access sections, tags, separators, and
declarator restrictions.

Optional presence and optional values are separate semantics. A field marked
with `FieldSpec::is_optional()` may be absent from its containing value and
requests `FieldCapability::OptionalPresence`. A `TypeName::Optional(T)` field
is still present but may hold the target language's option or null
representation. An adapter must not substitute one meaning for the other.

Each adapter owns the complete ordering and spelling of a declaration. Private
leaf helpers may render structured fragments such as a parameter list or body,
but do not choose their relative order. Related adapters may additionally share
a genuinely family-specific lowering helper. An adapter can bypass either
without adding a new variant to a shared grammar interface.

A useful locality test is to add a language with a previously unseen syntax.
The change should be confined to that adapter, its private helpers, and its
tests. If the change requires a new shared placement enum and new branches in a
generic spec emitter, target grammar has crossed the seam.

## Compatibility and Migration

Public syntax configuration that was already part of the 0.6.8 adapter
interface cannot disappear without a deliberate compatibility decision.
During migration it may remain behind a default compatibility lowerer for
external adapters. The `function_syntax()`, `type_decl_syntax()`, and
`enum_and_annotation()` accessors and their configuration types are deprecated
to make this boundary visible to adapter authors at compile time. The preamble
ordering hooks `doc_before_annotations()` and `doc_comment_inside_body()` are
deprecated for the same reason, although remaining type and property
compatibility paths still consult them during migration.

Field lowering follows the same compatibility boundary. Strict built-in
adapters declare field profiles and implement the complete sequence seam.
Permissive external adapters retain the frozen pre-0.6.8 field emitter as the
default `lower_fields()` implementation. `optional_field_style()` and
`OptionalFieldStyle` remain available only for that deprecated compatibility
path; their historical type-prefix, type-suffix, and wrapper branches conflate
absence with nullable values and are not a semantic model for new code.

Compatibility is not permission to extend that design:

- Do not add new shared declaration-placement enums, flags, or keyword fields.
- Do not add new branches in specs to interpret target grammar.
- New built-in behavior should enter through a complete language-owned lowering
  seam.
- Concepts introduced after 0.6.8 may be changed or removed instead of being
  preserved as another compatibility layer.
- Existing built-in adapters should migrate incrementally, with rendered-output
  tests at the adapter seam and parity coverage across direct and pretty paths.

Compatibility is bounded by validity. The deprecated `VariantContext` and
ownerless `EnumVariantSpec::emit()` remain usable only with permissive external
adapters. Strict built-ins require `TypeSpec` so they can validate and lower the
complete sequence. The deprecated `.value()` field is interpreted locally only
where its old meaning is unambiguous and valid; new code uses
`.discriminant(...)` or `.constructor_argument(...)`. The private compatibility
path also preserves the old `variants_before_fields` placement for permissive
adapters; built-ins do not consult that shared grammar flag.

This policy lets external adapters keep working while the built-in
implementation moves toward the intended ownership model.

## Scope of This Decision

This decision governs declaration grammar interpreted by `spec/*`. It does not
prohibit shared semantic data, structured rendering nodes, or private reusable
helpers. It also does not by itself redesign lower-level seams such as
`TypeName` presentation or block layout; those mechanisms have their own design
documents and must be evaluated against their own callers and invariants.
