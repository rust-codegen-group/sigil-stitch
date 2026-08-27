# TypeName Validation and Lowering

Status: accepted 0.7 design; implementation pending.

`TypeName` records semantic type-reference structure. It does not describe a
shared target grammar. One selected language adapter must either lower the
complete value into structured output or reject it before any source text is
rendered.

This chapter defines the 0.7 type-name seam. The frozen pre-0.6.8 presentation
configuration is documented only in [0.6.8 Legacy Compatibility and
Migration](legacy_compatibility_and_migration.md).

## Ownership

The core owns:

- the language-independent `TypeName` variants and their intrinsic coherence;
- recursive discovery of every `CodeNode::TypeRef`;
- the order of source-tree rewrite, type-name lowering, import collection,
  alias resolution, and final rendering;
- validation of every adapter-produced type block; and
- all-or-error behavior when any type name is invalid or unsupported.

The selected language adapter owns:

- whether the complete type name is representable;
- target precedence, punctuation, delimiters, and wrapping;
- the spelling and placement of every accepted type construct;
- decoded string-literal quoting and escaping; and
- target-derived type references such as Python's `typing.Literal`.

No `TypeExpressionCapability`, presentation matrix, or universal syntax
configuration sits between those responsibilities. Detailed type grammar
varies together and remains local to one adapter.

## One Fallible Interface

`RendererLang` exposes one complete type-name lowering method:

```text
fn lower_type_name(
    &self,
    type_name: &TypeName,
) -> Result<CodeBlock, SigilStitchError>;
```

There is no public validated wrapper. `TypeName` is immutable, lowering is a
pure operation, and the successful non-empty `CodeBlock` is the proof that the
selected adapter accepted the value. Crate-owned callers perform intrinsic
validation before the hook and validate its output afterward.

Validation paths may invoke this pure lowering operation for every type root
and discard the successful blocks while retaining all independent failures.
Render preparation retains successful blocks for import collection and
rendering. This avoids separate validation and lowering implementations that
can disagree.

The method belongs to `RendererLang`, not only `CodeLang`, because a direct
`CodeBlock` may contain `%T` references without using declaration specs.

## Preparation Pipeline

Declaration lowering first produces source `CodeBlock`s. The selected adapter
then rewrites each source block exactly once before type names are lowered and
imports are collected:

```text
declaration lowerers and caller CodeBlocks
    -> CodeBlock tree containing TypeRef(TypeName)
    -> RendererLang::rewrite_nodes exactly once
    -> validate the rewritten source tree
    -> recursively find every TypeRef
    -> intrinsic TypeName validation
    -> RendererLang::lower_type_name
    -> validate every lowered type block
    -> collect imports from the lowered CodeBlock tree
    -> resolve aliases
    -> render through one layout adapter with no further rewrite or lowering
```

No source text is emitted until every type name in every prepared file
member has succeeded. A failure in a direct, nested, or sequenced block aborts
the complete file just like a declaration-lowering failure.

Declaration lowerers continue to place semantic `TypeName` values in `%T`
slots. They do not call `lower_type_name`, render a type early, or duplicate
type grammar.

`rewrite_nodes()` is a source-tree correction seam, not a type-lowering hook.
It runs for headers, direct caller blocks, and declaration-lowered blocks. It
does not run for blocks returned by `lower_type_name()`, opaque raw content, or
the type metadata attached to `RawContentWithImports`. Listed raw-import types
are lowered and validated only so their derived imports can be collected.

## Lowered Block Contract

A successful adapter result must:

- be non-empty;
- contain only structure meaningful inside one type expression;
- balance every `Indent` and `Dedent` marker;
- preserve soft layout choices as `SoftBreak` and nested groups;
- retain import-bearing leaves as terminal `TypeRef` nodes; and
- contain no unresolved compound `TypeName`.

The terminal type-reference leaves are target-aware `Primitive` and `Raw`
values plus unqualified `Importable` references whose aliases are resolved
later. A qualified importable reference and every compound variant must be
fully lowered by the adapter.

The crate rejects adapter output that contains statement or control-flow
nodes, is empty, leaves a compound type reference unresolved, or otherwise
cannot be interpreted as one type expression. This catches incomplete
external adapters instead of allowing recursive or silently empty output.

`CodeBlock` remains the only shared structured source container. It carries
target-associated type-expression structure rather than defining a portable
cross-language program. There is no separate type document tree and no public
`BoxDoc`-producing language hook.

## Imports Stay Structural

An adapter expresses a target-derived import by retaining an importable
`TypeName` leaf in its lowered block. It does not return a parallel import
list.

For example, Python lowers a string literal type to the structural equivalent
of:

```text
%T[%S]
```

where `%T` contains `TypeName::importable("typing", "Literal")` and `%S`
contains the decoded string value. Import collection therefore discovers
`typing.Literal` from the same structure that renders `Literal["value"]`.
Alias resolution cannot drift from the generated type syntax.

Direct `CodeRenderer` use retains its existing contract: the caller supplies
the resolved `ImportGroup`. `FileSpec` owns complete source preparation,
derived import collection, and import-header emission.

## String Literal Types

The focused 0.7 extension is:

```text
TypeName::StringLiteral(String)
```

The string is the decoded semantic value. It contains neither target quotes
nor target escape sequences. The adapter uses its language-local string
literal rules when that value is valid in a type position.

- TypeScript lowers one value to a string literal type such as `'active'`.
- Python lowers one value through `typing.Literal["active"]`. A non-empty
  direct `Union` containing only `StringLiteral` members becomes one
  `typing.Literal[...]`, preserving member order and duplicates. Mixed unions
  and nested unions lower recursively through ordinary Python union grammar;
  this direct-union rule never flattens them.
- A built-in adapter without an exact string singleton type rejects the
  variant instead of widening it to `String`, `str`, or another primitive.

Several values compose through ordinary union structure:

```text
TypeName::Union([
    TypeName::StringLiteral("active"),
    TypeName::StringLiteral("inactive"),
])
```

The core does not add `LiteralValue`, `StringEnum`, `LiteralSet`, or numeric
literal variants. A future proven type-expression semantic receives its own
explicit variant; it does not reinterpret the string payload as source code.

## Compatibility

Adding `StringLiteral` makes the pre-0.6.8 public `TypeName` enum source
incompatible for downstream exhaustive matches. The 0.7 change therefore also
marks `TypeName` as `#[non_exhaustive]` and documents the required wildcard
match. The compatibility bridge preserves supported 0.6.8 Rust constructors
and captures the specific `TypeName` JSON values documented before 0.7 as
checked fixtures. This is not a promise that every Serde representation remains
stable: sigil-stitch defines no binary serialization protocol, enum-ordinal
contract, struct-field order, or serializer-specific byte format. No
forward-compatible interpretation of unknown serialized variants is added;
deserialization returns an error instead of changing generated code silently.

`RendererLang::lower_type_name` has a provided compatibility implementation.
It reproduces pre-0.6.8 behavior for old variants through the frozen
`TypePresentationConfig`, `TypePresentation`, `GenericSyntaxConfig`, and
qualified-name accessors. It rejects `StringLiteral` and every later semantic
variant that did not exist in 0.6.8.

Every built-in adapter overrides the complete method and does not consult the
compatibility configuration. The legacy accessors and data types are
deprecated, receive no new fields or variants, and remain referenced only by
the compatibility implementation and its tests.

The pre-0.6.8 `TypeName::to_doc_with_lang()` convenience remains as a
deprecated terminal compatibility facade. The current file and standalone
rendering pipelines do not call it, and no replacement BoxDoc-producing
language hook is introduced.

## Verification Contract

The implementation must prove:

- every language in `tests/renderer_parity_tests.rs` handles every old
  `TypeName` variant through its local lowerer or rejects it explicitly;
- direct and pretty paths agree at wide widths and preserve intentional soft
  breaks at narrow widths;
- unsupported compound forms fail in direct, nested, and `Sequence` blocks;
- lowered imports survive nesting and alias collisions;
- TypeScript and Python correctly handle empty strings, both quote characters,
  backslashes, newlines, NUL, and Unicode;
- Python union lowering retains the canonical `Literal` import;
- a compatibility adapter that implements only pre-0.6.8 methods preserves
  its old output and rejects `StringLiteral`;
- empty, statement-bearing, or unresolved adapter output fails closed; and
- old serialized `TypeName` fixtures retain their exact shapes.
