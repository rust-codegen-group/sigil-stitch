# Adding a Language

sigil-stitch supports new languages by implementing two traits: `RendererLang` (renderer-only methods) and `CodeLang` (spec-layer methods). `CodeLang` extends `RendererLang`, so implementing `CodeLang` requires both. If you only need `CodeBlock`-level rendering without specs, `RendererLang` alone is sufficient.

`RendererLang` covers rendering essentials. `CodeLang` adds declaration
validation, materialization, and file-level behavior. The trait retains
deprecated pre-0.6.8 grammar hooks only so existing external adapters can use
the frozen compatibility lowerers.

Do not treat those declaration syntax structs as an extensible universal
grammar. New syntax dimensions belong in complete language-local lowering. See
[Declaration Specs and Language Lowering](declaration_lowering.md) for the
ownership model and [0.6.8 Legacy Compatibility and
Migration](legacy_compatibility_and_migration.md) for the deprecated surface.

This guide walks through the process using a hypothetical language, with references to real implementations you can study.

## Overview

Adding a language takes five steps:

1. Create `src/lang/your_lang.rs` with exact capabilities, validation, and complete lowerers
2. Add `pub mod your_lang;` to `src/lang/mod.rs`
3. Write integration tests in `tests/`
4. Run the full validation suite
5. Bless and inspect golden files only for intentional output changes

If your language has tokenizer conflicts in `sigil_quote!` that the universal heuristics
can't handle (e.g., shell flags, Go channel operators), you may also need to add a
`MacroLang` variant. See [Language-Aware Tokenizer](macrolang.md) for details.

## The RendererLang Trait

These methods are used by the renderer (`code_renderer.rs`) and type rendering:

### Required Methods

Only two methods have no default:

| Method | Example (TypeScript) | Purpose |
|--------|---------------------|---------|
| `file_extension()` | `"ts"` | File extension for output files |
| `line_comment_prefix()` | `"//"` | Single-line comment prefix |

### Common Overrides

| Method | Default | Purpose |
|--------|---------|---------|
| `reserved_words()` | Empty | Words that need escaping |
| `render_string_literal()` | C-style double quotes | Language-specific string quoting |
| `render_verbatim_string()` | Delegates to `render_string_literal()` | Minimal escaping for interpolated strings |
| `block_syntax()` | Brace-delimited blocks | Delimiters, indentation, and terminators |
| `block_open_for_intent()` | Delegates to legacy `block_open_for()` | Map a `BlockIntent` role to an opener |
| `block_close_for_intent()` | Delegates to legacy `block_close_for()` | Map a `BlockIntent` role to a closer |
| `type_presentation()` | TypeScript-like forms | Compound type rendering |
| `generic_syntax()` | Angle brackets | Generic application and constraints |

Override `render_verbatim_string()` if your language has string interpolation (e.g., Bash `"$x"`, TypeScript `` `${x}` ``, Python `f"{x}"`).

For keyword-delimited languages, implement `block_open_for_intent()` and
`block_close_for_intent()` as a local `match` over `BlockIntent`. The legacy
string-based `block_open_for()` / `block_close_for()` methods remain supported
only for old serialized nodes and external adapters.

`rewrite_nodes()` is available for renderer corrections that require a
tree-level view after macro expansion. Prefer intent-keyed structural rewrites
for blocks. Declaration grammar belongs to language-local lowering; the
existing declaration syntax structs are a compatibility path, not the place to
add another ordering or placement concept.

## The CodeLang Trait

Extends `RendererLang` with the additional methods needed by the spec layer.

Implement `capabilities()` for new adapters. Return a local
`LanguageCapabilities::strict()` matrix, add `TypeCapabilityProfile`s with
`with_types()`, add `FunctionCapabilityProfile`s with `with_functions()`, and
add `FieldCapabilityProfile`s with `with_fields()`,
`PropertyCapabilityProfile`s with `with_properties()`, and
`VariantCapabilityProfile`s with `with_variants()` for every supported
declaration context or owning type kind.
Function profiles are keyed by both context (`TopLevel`, `ReceiverMethod`,
`Member`, or `InterfaceMember`) and form (`Function`, `Constructor`, or
`Destructor`). Omit a profile when that combination is unsupported. Include
`ExplicitReturnType` and `TypedParameters` only where the form can represent
them. Use `with_required_capabilities()` for semantic facts that every
declaration must provide, `with_body_policy()` for required or forbidden
implementation bodies, and `with_incompatible_capabilities()` for supported
features that cannot be combined. Use `with_maximum_parameters()` for
form-specific arity limits, such as a zero-parameter destructor. Adapters
written for sigil-stitch 0.6.8 inherit
`LanguageCapabilities::permissive()` so their existing `CodeLang`
implementations remain source-compatible.

Variant profiles distinguish `Discriminant`, `ConstructorArguments`,
`PositionalPayload`, `RecordPayload`, and `Attributes`. They must not encode
keywords, delimiters, placement, or separator policy. Omit the owner profile if
the language cannot represent variants for that `TypeKind`; use an empty
capability list when simple variants are valid but no richer form is.

Field profiles are keyed by `FieldContext`: direct member emission, ordinary
members of one `TypeKind`, or record payloads of one variant owner kind. They
distinguish explicit type information, initializers, attributes, static and
readonly fields, and `OptionalPresence`. Add `ExplicitType` to the required set
only where an untyped field cannot be valid. Optional presence means the member
may be absent; value nullability is expressed separately with
`TypeName::Optional`.

Property profiles are keyed by `PropertyContext`: direct member emission or a
member of one owning `TypeKind`. They distinguish explicit type information,
read access, write access, attributes, and static behavior. Require
`ReadAccessor` where a write-only computed property is invalid and require
`ExplicitType` where inference cannot preserve the declaration. Getter/setter
spelling and whether the target uses accessor declarations, a field-style
body, or ordinary methods are lowering decisions, not capabilities.

### Declaration Lowering and Compatibility Methods

`CodeLang::validate_type()` receives one complete, read-only `TypeIntent`.
Override `collect_type_validation_errors()` when independent target-local
failures should survive file-level aggregation. `CodeLang::lower_type()` then
receives a crate-constructed `ValidatedType` whose fields, properties, methods,
and variants have already passed their own validation against the same
adapter. It returns `Vec<CodeBlock>` because a target may use one declaration
or several related blocks. The vector and every returned block must be
non-empty; sigil-stitch rejects empty output with `EmptyTypeLowering`. The type
lowerer owns preamble order, alias and newtype forms, headers, inheritance,
primary constructors, member-family order, empty bodies, closing syntax, and
output cardinality.

`CodeLang::validate_function()` receives classified, read-only `FunctionIntent`
after sigil-stitch applies its semantic capability matrix against the actual
adapter. An override returns `Result<(), SigilStitchError>` and can add
target-local checks, but cannot construct or bypass `ValidatedFunction`.
`CodeLang::lower_function()` receives the validated view and returns a
structured `CodeBlock`. New adapters implement this method as the owner of the
target's complete function grammar. Both views expose the function form and
context as well as names, types, parameters, modifiers, annotations,
constraints, delegation, suffix escape hatches, and the body.

`CodeLang::validate_variants()` and `CodeLang::lower_variants()` are the
corresponding complete-sequence seams for enum variants. Adapters that can find
multiple independent target-local errors override
`collect_variant_validation_errors()` as the additive validation entry point;
its default appends the single `validate_variants()` result. `VariantIntent`
exposes the owner, ordered variants, payloads, annotations, a
`has_non_variant_members()` fact covering fields, properties, methods,
embedded types, and opaque members, structured-constructor arity evidence, and
separate evidence that opaque members may provide constructor syntax. The
variant lowerer derives positions and owns its sequence grammar; the type
lowerer chooses the sequence's placement. Use
`AnnotationSpec::emit_with_syntax()` when a local annotation spelling must keep
an importable annotation name as a structured `%T` reference.

`CodeLang::validate_fields()` and `CodeLang::lower_fields()` form the
corresponding complete-sequence seam for fields. `FieldSequenceIntent` exposes
the semantic context, owner names when present, and the ordered read-only field
data. Override `collect_field_validation_errors()` as well when independent
sibling failures should survive file-level aggregation; its default appends
the single `validate_fields()` result. `ValidatedFields` is crate-constructed
after intrinsic, profile, and adapter-local validation. The lowerer owns all
field grammar, including documentation and annotation order, access sections,
tags, delimiters, and terminators. Keep every field type in a `%T` slot and
compose initializers or raw annotations as nested `CodeBlock`s.

`CodeLang::validate_property()` and `CodeLang::lower_property()` form the
corresponding seam for one computed property. `PropertyIntent` exposes the
semantic context, owner when present, property type, read and write bodies,
modifiers, documentation, and attributes. Override
`collect_property_validation_errors()` when multiple independent target-local
failures should survive file-level aggregation. `ValidatedProperty` is
crate-constructed after intrinsic, profile, and adapter-local validation. The
lowerer returns `Vec<CodeBlock>` because one property may become separate read
and write accessor declarations. Preserve its type in `%T` slots and compose
bodies and raw annotations structurally.

`CodeLang::validate_type_members()` is the validation-only seam for
relationships among one type's fields, computed properties, and explicit
methods after their per-family validation has run. Override
`collect_type_members_validation_errors()` when several independent owner-wide
failures should be aggregated. Use it for target-derived relationships such as
case-folded accessor/method collisions. It has no matching lowerer: properties
still lower one at a time through `lower_property()`, and the intent must not
grow placement, namespace-layout, or other grammar policy.

The remaining interface mixes semantic validation hooks with older grammar
fragments used by compatibility lowerers. Grammar-oriented methods must be
absorbed by complete language-local lowering rather than multiplied:

| Method | Example | Purpose |
|--------|---------|---------|
| `capabilities()` | Strict type, function, field, property, and variant profiles | Declare semantic representability by context and form |
| `validate_type()` | `TypeIntent -> Result<(), _>` | Add target-local checks after crate-owned complete-type validation |
| `collect_type_validation_errors()` | `TypeIntent + error sink` | Add independent target-local type failures during file validation |
| `lower_type()` | `ValidatedType -> non-empty Vec<CodeBlock>` | Own complete type-declaration grammar; permissive adapters default to frozen compatibility lowering |
| `render_visibility()` | `"public "`, `"pub "` | Visibility prefix |
| `function_keyword()` | `"function"`, `"fn"` | Function declaration keyword |
| `abstract_modifier_capability()` | `AbstractMethod`, `VirtualMethod` | Semantic meaning of the legacy abstract modifier |
| `function_form()` | `Function`, `Constructor`, `Destructor` | Classify declaration form for capability validation |
| `constructor_name_matches()` | `constructor`, `init`, or declaring type | Recognize implicit constructor spellings with or without an owning type |
| `static_constructor_name_matches()` | `true` / `false` for name and owner | Decide whether a constructor-shaped static member is still a constructor |
| `constructor_name_with_return_type_is_function()` | `true` / `false` | Let an explicit return type disambiguate an owner-named ordinary method |
| `constructor_name_is_valid()` | `true` / `false` for name and owner | Reject explicitly marked constructors whose names violate local syntax |
| `type_member_declaration_context()` | `Member`, `InterfaceMember` | Select concrete or contract member rules for each `TypeKind` |
| `function_parameters_are_typed()` | `true` / `false` for the complete list | Refine required typing for receiver spellings or shared annotations |
| `function_body_policy()` | `Required`, `Forbidden`, `Optional` | Refine profile body policy when modifiers change the rule |
| `maximum_function_parameters()` | maximum arity or `None` | Refine profile arity when modifiers change the limit |
| `function_visibility_is_valid()` | `true` / `false` | Reject form- or modifier-specific visibility before emission |
| `function_parameters_require_trailing_defaults()` | `true` / `false` | Require every defaulted parameter to follow required parameters |
| `validate_function_type_constraints()` | `Result<(), SigilStitchError>` | Validate whether the complete type-constraint set is semantically representable |
| `requires_complete_function_type_information()` | `true` / `false` | Require partial type metadata to form one complete typed declaration |
| `constructor_return_type_is_valid()` | `true` / `false` for one type | Restrict constructor return annotations after capability validation |
| `validate_function()` | `FunctionIntent -> Result<(), _>` | Add target-local checks after crate-owned semantic validation |
| `lower_function()` | `ValidatedFunction -> CodeBlock` | Own complete function grammar; defaults to the frozen compatibility lowerer |
| `validate_fields()` | `FieldSequenceIntent -> Result<(), _>` | Add target-local checks after crate-owned field validation |
| `collect_field_validation_errors()` | `FieldSequenceIntent + error sink` | Add independent target-local sibling errors during file validation |
| `lower_fields()` | `ValidatedFields -> CodeBlock` | Own complete field-sequence grammar; defaults to frozen compatibility lowering |
| `validate_property()` | `PropertyIntent -> Result<(), _>` | Add target-local checks after crate-owned property validation |
| `collect_property_validation_errors()` | `PropertyIntent + error sink` | Add independent target-local property errors during file validation |
| `lower_property()` | `ValidatedProperty -> Vec<CodeBlock>` | Own complete property grammar; defaults to frozen compatibility lowering |
| `validate_type_members()` | `TypeMembersIntent -> Result<(), _>` | Add target-local checks across semantic member families after per-family validation |
| `collect_type_members_validation_errors()` | `TypeMembersIntent + error sink` | Add independent target-derived cross-member errors during file validation |
| `validate_variants()` | `VariantIntent -> Result<(), _>` | Add target-local checks after crate-owned sequence validation |
| `collect_variant_validation_errors()` | `VariantIntent + error sink` | Add independent target-local sibling errors during file validation |
| `lower_variants()` | `ValidatedVariants -> CodeBlock` | Own complete variant-sequence grammar; defaults to frozen compatibility lowering |

Legacy type hooks such as `type_keyword()`,
`methods_inside_type_body()`, `emit_newtype_decl()`,
`abstract_type_modifier_is_valid()`, and `type_decl_syntax()` exist only for
the permissive compatibility lowerer. A new adapter does not implement them.
See the [legacy surface matrix](legacy_compatibility_and_migration.md#legacy-surface-matrix).

### Renderer Configuration

`block_syntax()`, `generic_syntax()`, and `type_presentation()` are lower-level
renderer and type-presentation seams. The deprecated declaration configuration
structs have a different role and are documented centrally in [0.6.8 Legacy
Compatibility and Migration](legacy_compatibility_and_migration.md#frozen-declaration-configuration).
Do not add public flags, enums, or fields to them for a new language.

#### `block_syntax()`

Returns `BlockSyntaxConfig` controlling block delimiters and formatting:

| Field | Default | Purpose |
|-------|---------|---------|
| `block_open` | `" {"` | Opening delimiter. Python overrides to `":"`. |
| `block_close` | `"}"` | Closing delimiter. Python overrides to `""` (indent-only). |
| `indent_unit` | `"  "` (2 spaces) | Indentation per level. |
| `uses_semicolons` | `true` | Statement terminator behavior. |
| `field_terminator` | `","` | After each field. Java/C++ override to `";"`. |
| `type_close_terminator` | (default) | Terminator after closing brace for types. |
| `bases_close` | (default) | Closing syntax for base-class lists. |

#### `generic_syntax()`

Returns `GenericSyntaxConfig` controlling generic/type-parameter syntax:

| Field | Default | Purpose |
|-------|---------|---------|
| `open` | `"<"` | Generic opening bracket. Go overrides to `"["`. |
| `close` | `">"` | Generic closing bracket. Go overrides to `"]"`. |
| `application_style` | (default) | How generics are applied to types. |
| `constraint_keyword` | `": "` | Generic bounds keyword. Java/TS override to `" extends "`. |
| `constraint_separator` | `" + "` | Between multiple bounds. Java/TS override to `" & "`. |
| `context_bound_keyword` | (default) | Context bound syntax (e.g. Scala's `:`). |

#### `type_presentation()`

Returns `TypePresentationConfig` controlling how semantic types (arrays, optionals, maps, tuples, references, function types, etc.) are rendered. See the [Type Presentation](#type-presentation) section below for details.

#### Standalone Override Methods

These methods don't belong to a config struct but have sensible defaults you can override:

- `escape_reserved()` -- how reserved words are escaped.
- `qualify_import_name()` -- receives the module, original name, and resolved
  name. The default returns the resolved name; Go prefixes its package and
  Haskell uses a module-qualified original name when an alias was assigned,
  paired with a `qualified` import for that symbol.
- `module_separator()` -- returns `Option<&str>`. Default `None`. Override to `Some("::")` (Rust/C++) or `Some(".")` (Go/Python/Java/etc.) to enable `TypeName::qualified()` inline rendering.
- `fun_block_open()` -- custom block opener for functions.
- `emit_type_context()` -- optional structured context for split function
  signatures.
- `render_type_param_kind()` -- how type parameters are annotated with variance.
- `line_comment_suffix()` -- suffix for line comments (default `""`).

Deprecated standalone declaration hooks such as type fragments, preamble
ordering, optional-field style, and property style are listed with their replacements in the
[legacy surface matrix](legacy_compatibility_and_migration.md#legacy-surface-matrix).

`render_imports()` receives a deduplicated, alias-resolved `ImportGroup` and
emits the file's import header. `render_doc_comment()` emits spec-level doc
comments. Study `src/lang/typescript.rs` for ES module imports or
`src/lang/rust.rs` for `use` paths.

Use `Arg::TypeName` or `%T` for every semantic type and compose child blocks
structurally; do not render a `TypeName` to a string inside a lowerer. A
complete sequence lowerer such as `lower_fields()` owns every line boundary
its sequence requires, including the boundary after its final declaration. The
complete type lowerer decides spacing and order among child declaration
families.

## Step-by-Step Walkthrough

### 1. Create the language file

Create `src/lang/your_lang.rs`. Keep semantic types in `%T` slots. Fragment
hooks omit surrounding whitespace, while complete lowerers own the internal
and terminating line boundaries required by their grammar. Hook errors should
be returned unchanged.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::capability::{
    FieldCapability, FieldCapabilityProfile, FieldContext, LanguageCapabilities,
    TypeCapability, TypeCapabilityProfile,
};
use sigil_stitch::lang::config::{BlockSyntaxConfig, GenericSyntaxConfig};
use sigil_stitch::lang::{
    CodeLang, RendererLang, TypeIntent, ValidatedFields, ValidatedType,
};
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

#[derive(Debug, Clone, Default)]
pub struct YourLang;

impl YourLang {
    pub fn new() -> Self {
        Self
    }
}

const RESERVED: &[&str] = &["if", "else", "for", "while", /* ... */];
const FIELD_CAPABILITIES: &[FieldCapability] = &[
    FieldCapability::ExplicitType,
    FieldCapability::Initializer,
];
const REQUIRED_FIELD_CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType];
const TYPE_PROFILES: &[TypeCapabilityProfile<'_>] = &[
    TypeCapabilityProfile::new(
        TypeKind::Class,
        &[TypeCapability::RecordFields],
    ),
];
const FIELD_PROFILES: &[FieldCapabilityProfile<'_>] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        FIELD_CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED_FIELD_CAPABILITIES),
    FieldCapabilityProfile::new(
        FieldContext::TypeMember(TypeKind::Class),
        FIELD_CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED_FIELD_CAPABILITIES),
];

impl RendererLang for YourLang {
    fn file_extension(&self) -> &str { "yl" }
    fn reserved_words(&self) -> &[&str] { RESERVED }
    fn line_comment_prefix(&self) -> &str { "//" }

    fn render_string_literal(&self, s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            uses_semicolons: true,
            indent_unit: "    ",
            field_terminator: ";",
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            constraint_keyword: " extends ",
            constraint_separator: " & ",
            ..Default::default()
        }
    }
}

impl CodeLang for YourLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        // Add the language's exact type, function, and variant profiles too.
        LanguageCapabilities::strict()
            .with_types(TYPE_PROFILES)
            .with_fields(FIELD_PROFILES)
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let mut out = String::from("/**\n");
        for line in lines {
            out.push_str(&format!(" * {line}\n"));
        }
        out.push_str(" */\n");
        out
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        let mut out = String::new();
        for entry in imports.entries() {
            out.push_str(&format!(
                "import {{ {} }} from \"{}\";\n",
                entry.resolved_name(),
                entry.module,
            ));
        }
        out
    }

    fn validate_type(&self, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
        let mut chars = type_.name().chars();
        let valid_identifier = chars
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
            && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric());
        if !valid_identifier || self.reserved_words().contains(&type_.name()) {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: "YourLang requires a non-keyword identifier".to_string(),
            });
        }
        if !matches!(
            type_.modifiers().visibility,
            Visibility::Inherited | Visibility::Public
        ) {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: "YourLang types support only inherited or public visibility".to_string(),
            });
        }
        if type_.modifiers().is_abstract || !type_.extra_members().is_empty() {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: "YourLang classes do not support abstract or opaque members".to_string(),
            });
        }
        Ok(())
    }

    fn lower_type(
        &self,
        type_: ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        let mut block = CodeBlock::builder();
        if !type_.doc().is_empty() {
            let lines: Vec<&str> = type_.doc().iter().map(String::as_str).collect();
            block.add("%L", self.render_doc_comment(&lines));
            block.add_line();
        }
        block.add(
            "%Lclass %L {",
            (
                self.render_visibility(
                    type_.modifiers().visibility,
                    DeclarationContext::TopLevel,
                ),
                type_.name(),
            ),
        );
        block.add_line();
        block.add("%>", ());
        if let Some(fields) = type_.fields() {
            block.add_code(self.lower_fields(fields.clone())?);
        }
        block.add("%<}", ());
        block.add_line();
        Ok(vec![block.build()?])
    }

    fn lower_fields(
        &self,
        fields: ValidatedFields<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut block = CodeBlock::builder();
        for field in fields.fields() {
            if !field.doc().is_empty() {
                let lines: Vec<&str> = field.doc().iter().map(String::as_str).collect();
                block.add("%L", self.render_doc_comment(&lines));
                block.add_line();
            }
            block.add(
                "%L%L: %T",
                (
                    self.render_visibility(
                        field.modifiers().visibility,
                        DeclarationContext::Member,
                    ),
                    self.escape_field_name(field.name()),
                    field.field_type().clone(),
                ),
            );
            if let Some(initializer) = field.initializer() {
                block.add(" = %L", initializer.clone());
            }
            block.add(";", ());
            block.add_line();
        }
        block.build()
    }

    // Remaining spec support methods...
    fn render_visibility(&self, vis: Visibility, _ctx: DeclarationContext) -> &str {
        match vis {
            Visibility::Public => "public ",
            Visibility::Private => "private ",
            Visibility::Protected => "protected ",
            _ => "",
        }
    }

}
```

The runnable `CodeLang` rustdoc example compiles as part of `cargo test --doc`.
Use it as the contract reference when adding or changing structured hooks.

### 2. Register the module

Add to `src/lang/mod.rs`:
```rust,ignore
/// YourLang language support.
pub mod your_lang;
```

### 3. Write tests

Create a test directory `tests/your_lang/` with a `main.rs` entry point and submodules:

**`tests/your_lang/main.rs`**:
```rust,ignore
mod golden;

mod quote_basic;
mod builder_basic;
```

**`tests/your_lang/quote_basic.rs`** -- `sigil_quote!` macro tests:
```rust,ignore
use sigil_stitch::prelude::*;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.yl")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic_statement() {
    let block = sigil_quote!(YourLang {
        const x = 1;
    });
    golden::assert_golden("your_lang/basic_statement.yl", &render(&block));
}
```

**`tests/your_lang/builder_basic.rs`** -- builder API tests (CodeBlock, TypeSpec, FunSpec, FileSpec).

### 4. Run the full validation suite

Run the repository checks after the adapter's advertised declaration families
have complete validation and lowering. A strict profile without its matching
complete lowerer is an implementation error, not a reason to bless output.

```bash
just check
```

### 5. Review intentional golden changes

```bash
just bless
```

This runs tests with `BLESS=1` and writes `test-goldens/your_lang/*.yl` from the
actual output. Use it only when the output change is intentional, then inspect
every changed fixture. A strict adapter that advertises a type profile but
omits `lower_type()` fails closed with `MissingTypeLowerer`; blessing cannot
turn an incomplete adapter into a valid one. Returning an empty vector or
empty block likewise fails with `EmptyTypeLowering`. Follow the [external-adapter
migration sequence](legacy_compatibility_and_migration.md#external-adapter-migration)
when migrating an existing adapter family by family.

`generic_syntax()` may still describe reusable type-expression presentation,
such as bracket delimiters. Declaration placement—where type parameters,
bounds, bases, constructors, and members appear—belongs in `lower_type()`.

## Reference Implementations

Study these existing implementations for patterns similar to your target:

| Language | File | Notable Patterns |
|----------|------|-----------------|
| TypeScript | `src/lang/typescript.rs` | ES module imports, type-only imports, single-quoted strings |
| Rust | `src/lang/rust.rs` | `use` paths, struct+impl split, `pub(crate)` visibility |
| Python | `src/lang/python.rs` | Indent-only blocks (no braces), docstrings inside body, `from x import y` |
| Go | `src/lang/go.rs` | Package-qualified names (`http.Server`), bracket generics, `func` keyword |
| C | `src/lang/c.rs` | Type-before-name, `#include`, `__attribute__`, struct close semicolon |
| C++ | `src/lang/cpp.rs` | `virtual` instead of `abstract`, `#include` + `using`, `[[attributes]]` |
| Bash | `src/lang/bash.rs` | Keyword-based block closers (`fi`/`done`/`esac`), `source` imports, shell escaping |
| Scala | `src/lang/scala.rs` | `case class`, `trait`, `[T]` generics, `<:` bounds, `= {`/`}` blocks |
| Haskell | `src/lang/haskell.rs` | Split signature style, `where`/indentation blocks, postfix generics, `deriving` |
| OCaml | `src/lang/ocaml.rs` | Postfix generics, `let` keyword, `= `/indentation blocks, `open Module` imports, `module_block` helper |

## Type Presentation

When your language uses type expressions (generics, arrays, optionals, maps, etc.), you configure how each semantic type concept renders by returning a `TypePresentationConfig` from the `type_presentation()` accessor. You never build `BoxDoc` directly.

### How it works

Each `TypeName` variant (Array, Optional, Map, etc.) uses your language's `TypePresentationConfig` to determine the syntactic pattern via `TypePresentation` — a small enum:

- `GenericWrap { name }` — `name<P1, P2>` using your `generic_syntax().open`/`generic_syntax().close`
- `Prefix { prefix }` — `prefix inner` (e.g., Go `[]T`, Rust `*const T`)
- `Postfix { suffix }` — `inner suffix` (e.g., TypeScript `T[]`, Kotlin `T?`)
- `Surround { prefix, suffix }` — `prefix inner suffix` (e.g., C++ `const T&`, C `const T*`)
- `Delimited { open, sep, close }` — `open P1 sep P2 close` (e.g., Swift `[K: V]`, Go `map[K]V`)
- `Infix { sep }` — `P1 sep P2` (e.g., TypeScript `A | B`, Rust `A + B`)

### Configuring type presentation

All fields in `TypePresentationConfig` have defaults matching TypeScript conventions. Override only when your language differs:

```rust,ignore
impl RendererLang for YourLang {
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            // Array: default is Postfix { suffix: "[]" } (TS: T[])
            // Override for Rust-style Vec<T>:
            array: TypePresentation::GenericWrap { name: "Vec" },

            // Optional: default is Infix { sep: " | " } with "null" literal
            // Override for Kotlin-style T?:
            optional: TypePresentation::Postfix { suffix: "?" },

            // Map: default is GenericWrap { name: "Map" }
            // Override for Go-style map[K]V:
            map: TypePresentation::Delimited { open: "map[", sep: "]", close: "" },

            // Tuple: default is Delimited { open: "(", sep: ", ", close: ")" }
            // TS overrides to "[", "]" for [A, B] syntax. This shows Go-style (A, B):
            tuple: TypePresentation::Delimited { open: "(", sep: ", ", close: ")" },

            // Reference: default is Prefix { prefix: "" } (identity — for GC languages)
            // Override for Rust-style &T:
            reference: TypePresentation::Prefix { prefix: "&" },

            // Function types: default is TypeScript (A, B) => R
            function: FunctionPresentation {
                keyword: "fn",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: " -> ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },

            ..Default::default()
        }
    }
}
```

See [Type Presentation](type_presentation.md) for the full enum definition, all available fields, and examples for every supported language.
