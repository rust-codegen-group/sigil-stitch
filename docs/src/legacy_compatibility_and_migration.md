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

Types, functions, field sequences, computed properties, and enum-variant
sequences use complete language-owned lowering for every built-in adapter.
`TypeSpec` validates one complete declaration, constructs `ValidatedType` with
validated children, and delegates once to `CodeLang::lower_type()`.

The compatibility bridge restores the exact 0.6.8 source signatures touched by
this migration and marks the shared grammar surface deprecated. A checked
external-adapter fixture overrides the complete old trait surface, the finite
documented `TypeName` JSON set is checked as `serde_json::Value`, and
`cargo-semver-checks 0.49.0` currently reports no unapproved break from tag
`0.6.8`. The compatibility manifest and fixtures live under
`tests/compatibility/`.

Type expressions are the next accepted 0.7 migration. Its target state gives
every built-in adapter complete fallible `RendererLang::lower_type_name()`
ownership before import collection. The provided default will remain only as
the frozen type-presentation bridge for external adapters written against
0.6.8.

The accepted target also uses complete-set fallible import resolution, moves
declaration-generic grammar out of `GenericSyntaxConfig`, moves direct renderer
events out of `BlockSyntaxConfig`, and makes quote handling language-local.
These are behavior-specific contracts rather than an execution sequence. The
source-read inventory below names each temporary reader and the behavior that
retires its current-path use.

The provided external-adapter lowerers remain private implementation details.
They freeze 0.6.8 behavior; they are not examples for new adapters.

## Current Configuration-Read Inventory

This inventory distinguishes current source reads from the accepted target
state. A built-in method that returns a legacy config is a provider, not by
itself evidence that the current built-in path consumes that config. Update the
inventory whenever a reader moves behind a frozen compatibility boundary.

| Shared surface | Current production readers | Classification | Retirement owner and retained boundary |
|----------------|----------------------------|----------------|----------------------------------------|
| `TypePresentationConfig`, `TypePresentation`, `FunctionPresentation`, `AssociatedTypeStyle`, `BoundsPresentation`, `WildcardPresentation`; `RendererLang::type_presentation()` | `src/type_name_render.rs` reads the complete matrix for every compound type | Language-owned type grammar currently interpreted by a shared engine | Type-name lowering moves all built-ins to complete local `lower_type_name()` implementations; only the frozen 0.6.8 default and direct compatibility facade retain the matrix |
| `RendererLang::module_separator()` | `src/type_name_render.rs` reads it for qualified type names | Language-owned type grammar | Type-name lowering owns qualified-name spelling; the old accessor remains only in the frozen type bridge |
| `GenericSyntaxConfig`; `RendererLang::generic_syntax()` in type rendering | `src/type_name_render.rs` reads generic-application delimiters and placement | Language-owned type grammar | Type-name lowering removes this read from the current `TypeName` path |
| `GenericSyntaxConfig`; `RendererLang::generic_syntax()` in declarations | `src/spec/where_spec.rs` reads declaration parameter and bound grammar; `src/lang/type_lowering/compatibility.rs` reads it for legacy types | Language-owned declaration grammar in `where_spec`; compatibility-only grammar in the legacy lowerer | Generic declaration lowering moves built-in declaration grammar into complete local lowerers; the compatibility module retains the frozen read |
| `BlockSyntaxConfig::indent_unit` | `src/code_renderer.rs`, `src/spec/where_spec.rs`, and `src/lang/csharp_function_lowering.rs` | Renderer mechanics when indenting; language-owned declaration layout where a lowerer emits literal indentation | Renderer events route renderer mechanics through `indent_unit()`; complete declaration lowerers remove built-in grammar reads, while compatibility lowerers retain their bridge reads |
| `BlockSyntaxConfig::{uses_semicolons, block_open, block_close, close_on_transition}` | `src/code_renderer.rs` and `src/lang/typescript_function_lowering.rs`; the field, function, property, and type compatibility modules also read them | Language-owned renderer-event or declaration grammar in current built-in paths; compatibility-only grammar in compatibility modules | Renderer events replace current renderer reads; complete declaration lowerers own built-in grammar, while frozen compatibility modules continue to interpret old adapters |
| `BlockSyntaxConfig::{field_terminator, type_close_terminator, bases_close}` | Only `src/lang/field_lowering/compatibility.rs` and `src/lang/type_lowering/compatibility.rs` consume these fields in production | Compatibility-only declaration grammar | No current replacement config; complete declaration lowerers own these bytes locally and the old fields remain frozen |
| `FunctionSyntaxConfig`, `OptionalFieldStyle`, `PropertyStyle`, and `property_getter_keyword()` | The function, field, property, and type compatibility modules consume the applicable surfaces | Compatibility-only declaration grammar | Already outside built-in complete lowerers; retain only for the deprecated 0.6.8 bridge |
| `TypeDeclSyntaxConfig` | The function, field, property, and type compatibility modules read it; deprecated `ParameterSpec::emit_into()` also reads it for the direct 0.6.8 parameter facade | Compatibility-only declaration grammar | Complete built-in lowerers already own these bytes; retain the reads only in frozen compatibility modules and the deprecated direct facade |
| `EnumAndAnnotationConfig` and `VariantValueFormat` | The function, field, property, type, and variant compatibility modules read them; `AnnotationSpec::emit_with()` and deprecated `ParameterSpec::emit_into()` retain direct 0.6.8 facade behavior; permissive variant dispatch reads `variants_before_fields` through the variant compatibility module | Compatibility-only annotation, parameter, and variant grammar | Complete built-in lowerers use `emit_with_syntax()` and target-local variant grammar; retain shared reads only at the named compatibility boundaries |
| Shared `QuoteStyle`, the three public `quote_style` fields, and `with_quote_style()` | One narrow helper in each of TypeScript, JavaScript, and Python normalizes the preserved field to a target-local quote character; downstream string and import rendering no longer read the shared enum | Compatibility-held user preference whose concrete grammar belongs to each language | Language-local quote handling owns escaping and conveniences; the old enum, field, and setter remain deprecated shims |

Built-in unit tests that directly inspect config-return values are temporary
migration expectations, not additional production readers.
`tests/renderer_parity_tests.rs` also protects legacy indentation compatibility;
the field/property custom-adapter tests exercise compatibility defaults; and
`tests/assert_quote_tests.rs` plus the three language unit suites protect the
quote shim. Definitions and overrides under `src/lang/*.rs` remain until the
corresponding compatibility surface can be removed in a future major version.

## Legacy Surface Matrix

| Family | Legacy surface | Compatibility behavior | Current replacement |
|--------|----------------|------------------------|---------------------|
| Capabilities | No `capabilities()` override | External adapters receive `LanguageCapabilities::permissive()` | Return a strict matrix with exact family profiles |
| Type expressions | `type_presentation()`, `TypePresentationConfig`, `TypePresentation`, `FunctionPresentation`, `generic_syntax()`, `GenericSyntaxConfig`, qualified-name presentation accessors, and `TypeName::to_doc_with_lang()` | The provided `lower_type_name()` reproduces 0.6.8 output for old `TypeName` variants and rejects `StringLiteral` or any later variant; the direct document method remains only as a deprecated terminal facade | Implement complete fallible `RendererLang::lower_type_name()` and keep imports in the returned `CodeBlock` |
| `TypeName` matching and documented JSON values | Exhaustive matches over the pre-0.6.8 variants; concrete `TypeName` JSON values documented before 0.7 | Supported Rust constructors remain; checked fixtures preserve the documented JSON values. Generic Serde support does not promise compatibility for other representations, binary encodings, enum ordinals, field order, or serializer bytes | Add a wildcard arm to downstream matches; do not reinterpret unknown data or rely on an undocumented wire format |
| Functions | `function_keyword()`, `fun_block_open()`, `function_syntax()`, `FunctionSyntaxConfig`, `ParamListStyle`, `FunctionSignatureStyle`, `ConstructorDelegationStyle`, and `WhereClauseStyle` | The provided `lower_function()` interprets them for external adapters | `validate_function()` and complete `lower_function()` |
| Types | `type_keyword()`, `methods_inside_type_body()`, `type_kind_suffix()`, `emit_newtype_decl()`, `type_header_block_open()`, `type_body_prefix()` / `type_body_suffix()`, `emit_type_close_suffix()`, `abstract_type_modifier_is_valid()`, `type_decl_syntax()`, and type-emitter reads of `function_syntax()` / `enum_and_annotation()` | The provided `lower_type()` interprets them only for permissive external adapters | `validate_type()` and complete `lower_type()` |
| Type parameters | `render_type_param_kind()` and `ParameterSpec::emit_into()` | The transitional shared generic renderer and frozen direct facade interpret them | Complete language-owned generic declaration lowering |
| Variable spelling | `variable_prefix()` | Frozen function, field, property, and type compatibility lowerers interpret the adapter's prefix | Complete language-owned declaration lowering |
| Preambles | `doc_before_annotations()`, `doc_comment_inside_body()` | Frozen compatibility lowerers may read them | Emit documentation and attributes in each complete lowerer |
| Fields | `optional_field_style()`, `OptionalFieldStyle` | The provided `lower_fields()` freezes the old field emitter | `FieldCapability`, `FieldContext`, `TypeName::Optional`, and complete `lower_fields()` |
| Properties | `property_style()`, `property_getter_keyword()`, `PropertyStyle` | The provided `lower_property()` freezes the old property emitter | `PropertyContext`, property capabilities, and complete `lower_property()` |
| Variants | `VariantContext`, `.value()`, `VariantValueFormat`, `variants_before_fields` | Only permissive external adapters retain ownerless positional lowering; strict built-ins require an owner and complete sequence | Add variants to `TypeSpec`; use `.discriminant()` or `.constructor_argument()` |
| Variant payload builders | `.associated_type()`, `.add_field()` | Deprecated aliases remain available | `.positional_payload()`, `.record_payload_field()` |
| Renderer events and block nodes | `block_syntax()`, `BlockSyntaxConfig`, `block_open_for()`, `block_close_for()`, intent-aware bridge hooks, and legacy serialized block nodes | Provided event defaults interpret old config and hooks; old nodes and external adapters remain renderable | `BlockIntent`, `indent_unit()`, `render_statement_end()`, `render_block_open()`, `render_block_close()`, and `render_branch_transition()` |

Direct `FieldSpec::emit()` and `PropertySpec::emit()` remain public facades. Their
`DeclarationContext` input is retained only as a compatibility payload. Prefer
adding members to `TypeSpec` whenever the owning `TypeKind` or other members can
affect validity.

## Frozen Grammar Configuration

The legacy structs mix renderer mechanics with type-expression and declaration
grammar. The current source still reads `block_syntax()`, `generic_syntax()`,
and `type_presentation()` on the paths listed above. The accepted target
deprecates all three shared configs: complete language-local lowerers own type
and declaration grammar, while direct renderer-event methods plus
`indent_unit()` replace final-renderer reads. Frozen compatibility lowerers may
continue interpreting the old values; none of these structs receives new
fields or variants.

### `TypePresentationConfig` and `GenericSyntaxConfig`

These values describe the pre-0.6.8 shared type-expression grammar: generic
delimiters, prefix and postfix wrappers, delimiters, infix separators,
qualified-name separators, and function-type placement. The provided
`RendererLang::lower_type_name()` default continues to interpret them for all
old `TypeName` variants so an existing external adapter remains source
compatible.

This bridge is intentionally closed. It rejects `TypeName::StringLiteral` and
every later semantic variant, even if one of the old presentation patterns
could produce plausible text. New and built-in adapters implement complete
fallible type-name lowering instead of extending the configuration.

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

| Field | Frozen compatibility meaning |
|-------|----------------------|
| `type_before_name`, `return_type_is_prefix`, `type_annotation_separator` | Type/name ordering used by compatibility lowerers |
| `super_type_keyword`, `super_type_separator`, `super_type_subsequent_separator` | Base-type grammar |
| `implements_keyword` | Implemented-interface grammar |
| `type_alias_target_first` | Alias target/name ordering |
| `supports_primary_constructor` | Legacy primary-constructor switch |

These fields may be read only by frozen compatibility lowerers. New adapters
implement complete declaration lowering instead.

### `EnumAndAnnotationConfig`

| Field | Transitional or compatibility meaning |
|-------|----------------------------------------|
| `variant_prefix`, `variant_prefix_first`, `variant_separator`, `variant_trailing_separator`, `variants_before_fields`, `variant_value_format` | Frozen external-adapter variant grammar |
| `annotation_prefix`, `annotation_suffix` | Legacy annotation spelling; complete lowerers use local structured emission |
| `readonly_keyword`, `mutable_field_keyword` | Frozen parameter/property-promotion fragments |

### Quote-style compatibility

`QuoteStyle`, the public `quote_style` fields, and
`with_quote_style(QuoteStyle)` predate 0.6.8 and remain source-compatible. They
are deprecated shims rather than a general quote configuration shared by new
languages. TypeScript, JavaScript, and Python each own quote normalization,
escaping, and output locally. Target-local single-quote and double-quote
conveniences update the preserved field so there is one stored choice and no
precedence rule.

### Import resolver compatibility

`ImportGroup::resolve()` and `resolve_with_explicit()` remain the exact
deprecated, infallible 0.6.8 algorithms. They preserve first-encountered and
explicit-entry precedence, including their established duplicate-binding
aliases. Separately named fallible complete-set entry points replace them for
new callers; the old methods are not implemented by unwrapping the new
resolver.

## Builder Migration Recipes

### Type names and exhaustive matches

`TypeName` gains `StringLiteral(String)` in 0.7 and is marked
`#[non_exhaustive]`. Downstream code that previously matched every variant must
add a wildcard arm and decide whether an unknown type should be rejected or
passed back to sigil-stitch for language-owned lowering. Do not widen an
unknown variant to `Primitive` or `Raw`.

The string-literal payload is the decoded string value. Several values compose
as `TypeName::Union`; do not preserve hand-written quotes in `Raw` when the
semantic singleton form is available. Exact fixtures cover the `TypeName` JSON
values documented before 0.7. No other Serde representation or binary format
receives a cross-version guarantee. Deserializing an unknown variant remains
an error; it is never reinterpreted as another type.

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

### Primary constructor parameters

Pass only an identifier to `ParameterSpec`. For Kotlin and Scala, use
`.is_property()` for an immutable promoted property and
`.is_mutable_property()` for a mutable one. Do not encode `val` or `var` in the
parameter name. Complete language lowerers own that spelling; unsupported
languages reject primary-constructor intent instead of ignoring it.

Haskell and OCaml constructor data is not a primary constructor. Model it with
enum-variant positional or record payloads so the algebraic-data adapter sees
the payload semantics directly.

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

1. Implement complete `lower_type_name()` handling for every accepted old
   variant and explicit errors for unsupported forms.
2. Add a strict profile for every supported semantic context or owner kind.
3. Add adapter-local validation for identifier rules, modifier combinations,
   and relationships the profile cannot express.
4. Implement the complete `lower_*` seam and preserve every accepted
   `TypeName` as a `%T` reference and every nested block as structured `%L`.
5. Cover direct and owner-aware success and failure paths, import aliases, and
   both direct and pretty renderer paths where soft breaks are reachable.
6. Remove migrated-family reads of deprecated grammar from the adapter.

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

Run the focused compatibility gates with:

```text
cargo test --test compatibility_0_6_8
just semver-check
```

The first command compiles the old adapter as an external crate, checks the
restored signatures and structural marker bridges, and compares the bounded
JSON fixtures. The second command tests the report parser and then compares the
complete `cargo-semver-checks 0.49.0` record set with the checked allowlist.
Missing, duplicate, malformed, and unexpected approved records fail closed.

For a migrated family, keep tests for:

- an adapter implementing only the 0.6.8 trait surface;
- valid legacy output preserved by the provided lowerer;
- `StringLiteral` rejected by an adapter that implements only the 0.6.8 type
  presentation surface;
- invalid or ownerless built-in input rejected before materialization;
- direct and `FileSpec` paths selecting the actual adapter;
- semantic replacements for every deprecated builder alias; and
- serialized legacy nodes or fields that remain part of the public contract.

## What Does Not Belong Here

This appendix is not a release history, exhaustive API reference, or rejected-
design catalogue. Release-by-release changes belong in `CHANGELOG.md`, exact
signatures and deprecation attributes belong in rustdoc, and durable design
rationale belongs in focused records under `docs/adr/`.
