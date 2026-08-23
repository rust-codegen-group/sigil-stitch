# 0.6.8 Legacy Compatibility and Migration

This appendix is the reference for public behavior inherited from sigil-stitch
0.6.8. It explains what remains available, where compatibility is intentionally
restricted, and how callers and external language adapters move to the current
declaration model.

The [declaration-lowering design](declaration_lowering.md) defines the current
ownership model. This appendix documents the compatibility bridge; it does not
extend that bridge or define a second architecture.

## Compatibility Boundary

In this guide, **legacy** means a public declaration API, serialized contract,
or adapter hook that was available in 0.6.8. It does not include capability,
intent, or validated-view concepts introduced after 0.6.8.

The compatibility contract is:

- Public 0.6.8 declaration surfaces remain available during 0.7 unless an
  explicit compatibility decision says otherwise.
- Legacy grammar-oriented APIs are deprecated so new use is visible at compile
  time. They may still be read by a frozen compatibility lowerer.
- Existing external `CodeLang` implementations inherit permissive capability
  profiles and provided compatibility lowerers.
- Compatibility preserves valid old behavior when the semantic input can prove
  it. It does not require a built-in adapter to keep generating malformed or
  unverifiable target code.
- Concepts introduced after 0.6.8 may change without another compatibility
  layer.
- Requiring strict profiles or removing provided compatibility lowerers is a
  separate 0.8 decision, not an automatic consequence of deprecation.

Deprecated does not mean that the shared grammar model is still extensible. Do
not add a field, flag, or enum variant to a legacy configuration type for new
syntax.

## Which Path Applies?

| Reader | Current path | Compatibility responsibility |
|--------|--------------|------------------------------|
| Ordinary builder user | Use semantic builders and owner-aware `TypeSpec` composition | Replace deprecated aliases and direct facades when the owner affects validity |
| Existing 0.6.8 external adapter | Provided permissive profiles and frozen lowerers keep the adapter source-compatible | Migrate one declaration family at a time and retain output-parity tests |
| New external adapter | Declare strict capabilities and implement complete `validate_*` / `lower_*` seams | Do not model new grammar through deprecated configuration |
| Built-in adapter | Exact strict profiles and language-local lowering | Never consult migrated-family legacy grammar outside compatibility code |

## Current Migration State

Functions, field sequences, computed properties, and enum-variant sequences use
complete language-owned lowering for every built-in adapter. `TypeSpec` is the
remaining transitional family: its generic emitter still reads selected legacy
type-declaration configuration while complete type lowering is designed.

The provided external-adapter lowerers remain private implementation details.
They freeze 0.6.8 behavior; they are not examples for new adapters.

## Legacy Surface Matrix

| Family | Legacy surface | Compatibility behavior | Current replacement |
|--------|----------------|------------------------|---------------------|
| Capabilities | No `capabilities()` override | External adapters receive `LanguageCapabilities::permissive()` | Return a strict matrix with exact family profiles |
| Functions | `function_syntax()`, `FunctionSyntaxConfig`, `ParamListStyle`, `FunctionSignatureStyle`, `ConstructorDelegationStyle`, `WhereClauseStyle` | The provided `lower_function()` interprets them for external adapters | `validate_function()` and complete `lower_function()` |
| Types | `type_decl_syntax()`, selected `enum_and_annotation()` fields, `methods_inside_type_body()` | The transitional generic `TypeSpec` emitter still reads them | Keep existing overrides only until complete type lowering exists; do not add new fields |
| Preambles | `doc_before_annotations()`, `doc_comment_inside_body()` | Frozen lowerers and the transitional type emitter may read them | Emit documentation and attributes in each complete lowerer |
| Fields | `optional_field_style()`, `OptionalFieldStyle` | The provided `lower_fields()` freezes the old field emitter | `FieldCapability`, `FieldContext`, `TypeName::Optional`, and complete `lower_fields()` |
| Properties | `property_style()`, `property_getter_keyword()`, `PropertyStyle` | The provided `lower_property()` freezes the old property emitter | `PropertyContext`, property capabilities, and complete `lower_property()` |
| Variants | `VariantContext`, `.value()`, `VariantValueFormat`, `variants_before_fields` | Only permissive external adapters retain ownerless positional lowering; strict built-ins require an owner and complete sequence | Add variants to `TypeSpec`; use `.discriminant()` or `.constructor_argument()` |
| Variant payload builders | `.associated_type()`, `.add_field()` | Deprecated aliases remain available | `.positional_payload()`, `.record_payload_field()` |
| Block nodes | `block_open_for()`, `block_close_for()`, legacy serialized block nodes | Old nodes and external adapters remain renderable | `BlockIntent`, `block_open_for_intent()`, `block_close_for_intent()` |

Direct `FieldSpec::emit()` and `PropertySpec::emit()` remain public facades. Their
`DeclarationContext` input is retained only as a compatibility payload. Prefer
adding members to `TypeSpec` whenever the owning `TypeKind` or other members can
affect validity.

## Frozen Declaration Configuration

The legacy structs mix renderer policy with declaration grammar. Only the
declaration-grammar portion is deprecated. `block_syntax()`, `generic_syntax()`,
and `type_presentation()` remain lower-level rendering seams with separate
invariants.

### `FunctionSyntaxConfig`

| Field | 0.6.8 meaning |
|-------|---------------|
| `return_type_separator` | Text between a parameter list and suffix return type |
| `async_keyword`, `async_suffix`, `async_suffix_before_return` | Async spelling and placement |
| `abstract_keyword` | Abstract/virtual spelling |
| `param_list_style` | Tupled or curried parameter layout |
| `function_signature_style` | Merged or split declaration layout |
| `constructor_keyword`, `constructor_delegation_style` | Constructor spelling and delegation placement |
| `where_clause_style` | Inline, block, or repeated where-clause placement |
| `empty_body` | Legacy body placeholder |
| `type_params_before_return_type` | Legacy type-parameter placement switch |

Complete function lowerers own all of these choices locally. An adapter may
share private policy-free helpers, but new syntax must not add another field to
this table.

### `TypeDeclSyntaxConfig`

| Field | Transitional meaning |
|-------|----------------------|
| `type_before_name`, `return_type_is_prefix`, `type_annotation_separator` | Type/name ordering still used by the generic type emitter and some nested compatibility fragments |
| `super_type_keyword`, `super_type_separator`, `super_type_subsequent_separator` | Base-type grammar |
| `implements_keyword` | Implemented-interface grammar |
| `type_alias_target_first` | Alias target/name ordering |
| `supports_primary_constructor` | Transitional primary-constructor switch |

These fields may be used only where `TypeSpec` has not yet moved behind a
complete lowering seam.

### `EnumAndAnnotationConfig`

| Field | Transitional or compatibility meaning |
|-------|----------------------------------------|
| `variant_prefix`, `variant_prefix_first`, `variant_separator`, `variant_trailing_separator`, `variants_before_fields`, `variant_value_format` | Frozen external-adapter variant grammar |
| `annotation_prefix`, `annotation_suffix` | Legacy annotation spelling; complete lowerers use local structured emission |
| `readonly_keyword`, `mutable_field_keyword` | Frozen parameter/property-promotion fragments and transitional type behavior |

## Builder Migration Recipes

### Enum variants

- Replace direct `EnumVariantSpec::emit(..., VariantContext)` with
  `TypeSpec::add_variant()` so the adapter receives the owner and full sequence.
- Replace `.value(x)` with `.discriminant(x)` when the value identifies the
  member, or `.constructor_argument(x)` when the enum entry invokes a
  constructor.
- Replace `.associated_type(t)` with `.positional_payload(t)` and
  `.add_field(f)` with `.record_payload_field(f)`.

Strict built-ins reject an ownerless variant when first/last flags cannot prove
valid separators, payload grammar, or section termination.

### Optional fields

`FieldSpec::is_optional()` means that the containing value may omit the field.
`TypeName::Optional(T)` means that a present field can carry an absent or null
value. Replace `OptionalFieldStyle` with the semantic form actually intended;
do not infer one meaning from the other.

### Computed properties

Add a `PropertySpec` to `TypeSpec` instead of relying on direct placement when
the target's owning type or member namespaces affect validity. New adapters
lower read and write behavior from `PropertyIntent`; they do not select an
accessor model through `PropertyStyle`.

### C++ static member initializers

For class and struct members, the C++ adapter preserves the pre-C++17
`static const` spelling only when `FieldSpec` proves an integral primitive
type. It rejects:

- initialized mutable static members, which require either a C++17 `inline`
  declaration or a separate out-of-class definition; and
- initialized read-only static members whose type is not provably integral.

`TypeName` does not currently distinguish an enum type from another named
type, so the adapter does not guess from capitalization or a raw type name. Use
`TypeSpecBuilder::extra_member(CodeBlock)` for an enum-typed class constant, or
materialize the declaration and out-of-class definition as target-specific
blocks. This restriction prevents a compatibility path from silently emitting
invalid C++.

## External Adapter Migration

Migrate one declaration family at a time:

1. Add a strict profile for every supported semantic context or owner kind.
2. Add adapter-local validation for identifier rules, modifier combinations,
   and relationships the profile cannot express.
3. Implement the complete `lower_*` seam and preserve every accepted
   `TypeName` as a `%T` reference and every nested block as structured `%L`.
4. Cover direct and owner-aware success and failure paths, import aliases, and
   both direct and pretty renderer paths where soft breaks are reachable.
5. Remove migrated-family reads of deprecated grammar from the adapter. Leave
   only the temporary type-declaration overrides still required by `TypeSpec`.

Keep rendered-output fixtures while migrating. A provided default is a
compatibility bridge, not evidence that an adapter has completed the new seam.

## Python Static-Decorator Compatibility

Python retains one adapter-local compatibility recognizer for the 0.6.8 pattern
that combines `FunSpec::is_static()` with a `staticmethod` or `classmethod`
decorator. It applies only to non-constructor member and interface-member
functions.

The recognizer accepts:

- `AnnotationSpec::new("staticmethod")` or
  `AnnotationSpec::new("classmethod")`, including an importable annotation with
  that simple name; or
- an opaque annotation block made only of literal/nested-literal nodes whose
  trimmed text is exactly `@staticmethod` or `@classmethod` (and the equivalent
  attribute node).

Other spellings do not acquire static-method semantics. New code should prefer
the structured `AnnotationSpec` form. This exception is Python-local and must
not become a shared decorator parser or syntax hook.

## Compatibility Testing

For a migrated family, keep tests for:

- an adapter implementing only the 0.6.8 trait surface;
- valid legacy output preserved by the provided lowerer;
- invalid or ownerless built-in input rejected before materialization;
- direct and `FileSpec` paths selecting the actual adapter;
- semantic replacements for every deprecated builder alias; and
- serialized legacy nodes or fields that remain part of the public contract.

## What Does Not Belong Here

This appendix is not a release history, exhaustive API reference, or rejected-
design catalogue. Release-by-release changes belong in `CHANGELOG.md`, exact
signatures and deprecation attributes belong in rustdoc, and durable design
rationale belongs in focused records under `docs/adr/`.
