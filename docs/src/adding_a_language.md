# Adding a Language

This guide follows the accepted 0.7 target interface. Complete type-name
lowering, fallible import resolution, and direct renderer-event methods are
documented before their implementation. The current source still exposes the
compatibility-backed methods described in the legacy appendix. Target-state
signatures in this chapter are pseudocode contracts, not an assertion that each
method is already available on `main`.

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

These methods are used by the renderer (`code_renderer.rs`) and type lowering:

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
| `indent_unit()` | Delegates to legacy `block_syntax()` | Exact indentation bytes |
| `render_statement_end()` | Delegates to legacy `block_syntax()` | Complete statement-end suffix |
| `render_block_open()` | Delegates to legacy block hooks | Complete opener suffix for one `BlockIntent` |
| `render_block_close()` | Delegates to legacy block hooks | Complete final closer |
| `render_branch_transition()` | Delegates to legacy block hooks | Outgoing closer plus connector whitespace |
| `lower_type_name()` | Frozen pre-0.6.8 compatibility lowering | Validate and lower one complete type expression |

Override `render_verbatim_string()` if your language has string interpolation (e.g., Bash `"$x"`, TypeScript `` `${x}` ``, Python `f"{x}"`).

Implement all four renderer-event methods as local target-language behavior.
For keyword-delimited languages, match on `BlockIntent`; for brace languages,
several arms may deliberately return the same bytes. The legacy
`block_syntax()`, `block_open_for()`, `block_close_for()`, and intent-aware
bridge hooks remain supported only for old nodes and external adapters. A new
adapter does not assemble current renderer behavior from that shared config.

`rewrite_nodes()` is the language-local source-tree correction seam for cases
that require a tree-level view after macro expansion or declaration lowering.
The core calls it exactly once for each source `CodeBlock`, then validates the
rewritten tree, lowers every `TypeRef`, collects imports, resolves aliases, and
renders without rewriting again. The hook sees semantic, unaliased type
references and must not depend on resolved imports or rendered type text.

Use the existing recursive walker when a rule must visit `Nested` or `Sequence`
children; the core does not call the hook separately for them. The hook does
not run for type-name-lowering results, raw content, raw import metadata, or a
public `FileSpec::validate()` call. Semantic rejection belongs in the
applicable validation or lowering hook; structural errors left by rewrite fail
during the core's post-rewrite validation.

Prefer intent-keyed structural rewrites for blocks. Declaration grammar belongs
to language-local declaration lowering, and type grammar belongs to
`lower_type_name()`. Do not add rewrite context, capability, configuration, or
per-syntax hooks. The existing declaration syntax structs are a compatibility
path, not the place to add another ordering or placement concept.

Every new adapter must override `lower_type_name()`. Its provided default
exists only so a pre-0.6.8 external adapter can continue to compile while it
uses the frozen presentation configuration. A new implementation matches the
complete `TypeName`, returns a non-empty structured `CodeBlock` for every
accepted form, and returns `SigilStitchError` for unsupported forms. See
[TypeName Validation and Lowering](type_name_lowering.md) for the output
contract.

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

### Renderer Events

The accepted renderer requests five complete language-owned results:

```text
indent_unit() -> borrowed indentation bytes
render_statement_end() -> complete statement suffix or error
render_block_open(intent, condition) -> opener suffix or error
render_block_close(intent, condition) -> final closer or error
render_branch_transition(intent, condition) -> outgoing closer and connector or error
```

These methods expose operations the renderer actually performs, not a public
grammar matrix. A language may use private local helpers, but punctuation,
keywords, and event ordering stay in its module. The provided defaults read
`BlockSyntaxConfig` only to preserve 0.6.8 external adapters. The shared config
is deprecated compatibility state and receives no new fields.

#### Standalone Override Methods

These methods don't belong to a config struct but have sensible defaults you can override:

- `escape_reserved()` -- how reserved words are escaped.
- `qualify_import_reference()` -- receives the module, original name, and
  resolved name after complete-set alias assignment. The default returns the
  resolved name; Go prefixes its package and Haskell uses a module-qualified
  original name when an alias was assigned, paired with a `qualified` import
  for that symbol. The two-argument `qualify_import_name()` is the frozen
  0.6.8 bridge.
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
use sigil_stitch::lang::{
    BlockIntent, CodeLang, RendererLang, TypeIntent, ValidatedFields,
    ValidatedType,
};
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::type_name::TypeName;

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

    fn indent_unit(&self) -> &str { "    " }

    fn render_statement_end(&self) -> Result<&str, SigilStitchError> {
        Ok(";")
    }

    fn render_block_open(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<&str, SigilStitchError> {
        Ok(" {")
    }

    fn render_block_close(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<&str, SigilStitchError> {
        Ok("}")
    }

    fn render_branch_transition(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<String, SigilStitchError> {
        Ok("} ".to_owned())
    }

    fn lower_type_name(
        &self,
        type_name: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        lower_your_lang_type_name(type_name)
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

This target-state walkthrough is intentionally ignored by rustdoc until the
staged interfaces land. The runnable `CodeLang` rustdoc example in the crate
compiles as part of `cargo test --doc`; use that current example as the
executable contract while migrating.

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

`lower_type_name()` owns generic application and every other type-expression
form. Declaration placement—where declared type parameters, bounds, bases,
constructors, and members appear—belongs in the relevant complete declaration
lowerer.

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

## Type-Name Lowering

Implement one pure, fallible `lower_type_name()` match for the complete
`TypeName`. The adapter owns representability, precedence, punctuation,
wrapping, string escaping, qualified-name spelling, and target-derived imports.
It may use private local helpers, but it must not expose a public matrix of
syntax fragments.

The returned `CodeBlock` is validation evidence. It must be non-empty and may
contain only type-expression structure. Nested semantic types must be lowered
recursively. Leave only terminal import-aware `TypeRef` values for the core to
resolve later; never leave an unresolved array, optional, union, function, or
other compound `TypeName` in the result. Statement boundaries, block-control
nodes, and declaration fragments are invalid in this block.

Use `%T` for terminal imported symbols introduced by lowering. For example,
Python's `StringLiteral` branch composes an importable `typing.Literal` leaf
with a structured string-literal node. Import collection then sees the same
symbol that final rendering uses. Do not return a parallel import list.

Return an error when the target cannot preserve a variant exactly. Identity
lowering and "closest equivalent" substitutions are valid only when they are
semantically exact for that target. In particular, a language without string
singleton types rejects `TypeName::StringLiteral` instead of widening it to a
string primitive.

After source rewrite, the core recursively invokes the hook for `TypeRef` nodes
in direct, nested, and sequenced blocks before import collection, validates each
returned block, and aborts the complete file on any failure. Direct and pretty
rendering then consume the same fully lowered tree. The complete contract and
compatibility rules are in [TypeName Validation and
Lowering](type_name_lowering.md).
