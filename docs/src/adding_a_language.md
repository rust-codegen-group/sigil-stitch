# Adding a Language

sigil-stitch supports new languages by implementing two traits: `RendererLang` (renderer-only methods) and `CodeLang` (spec-layer methods). `CodeLang` extends `RendererLang`, so implementing `CodeLang` requires both. If you only need `CodeBlock`-level rendering without specs, `RendererLang` alone is sufficient.

`RendererLang` covers rendering essentials. `CodeLang` adds declaration
validation, materialization, and file-level behavior. The current trait also
contains pre-0.6.8 syntax configuration and structured emission hooks with
compatibility defaults.

Do not treat those declaration syntax structs as an extensible universal
grammar. New syntax dimensions belong in complete language-local lowering. See
[Declaration Specs and Language Lowering](declaration_lowering.md) for the
ownership model and migration policy.

This guide walks through the process using a hypothetical language, with references to real implementations you can study.

## Overview

Adding a language takes four steps:

1. Create `src/lang/your_lang.rs` implementing `CodeLang`
2. Add `pub mod your_lang;` to `src/lang/mod.rs`
3. Write integration tests in `tests/`
4. Run `just bless` to generate golden files

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
add `VariantCapabilityProfile`s with `with_variants()` for every owning type
kind that can contain variants.
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

### Function Lowering and Compatibility Methods

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
exposes the owner, ordered variants, payloads, annotations, following-member
state, structured-constructor arity evidence, and the presence of opaque members. The
lowerer derives position and owns all target grammar. Use
`AnnotationSpec::emit_with_syntax()` when a local annotation spelling must keep
an importable annotation name as a structured `%T` reference.

The remaining interface mixes semantic validation hooks with older grammar
fragments used by compatibility lowerers. Grammar-oriented methods must be
absorbed by complete language-local lowering rather than multiplied:

| Method | Example | Purpose |
|--------|---------|---------|
| `capabilities()` | Strict type, function, and variant profiles | Declare semantic representability by context and form |
| `render_visibility()` | `"public "`, `"pub "` | Visibility prefix |
| `function_keyword()` | `"function"`, `"fn"` | Function declaration keyword |
| `abstract_modifier_capability()` | `AbstractMethod`, `VirtualMethod` | Semantic meaning of the legacy abstract modifier |
| `function_form()` | `Function`, `Constructor`, `Destructor` | Classify declaration form for capability validation |
| `constructor_name_matches()` | `constructor`, `init`, or declaring type | Recognize implicit constructor spellings with or without an owning type |
| `static_constructor_name_matches()` | `true` / `false` for name and owner | Decide whether a constructor-shaped static member is still a constructor |
| `constructor_name_with_return_type_is_function()` | `true` / `false` | Let an explicit return type disambiguate an owner-named ordinary method |
| `constructor_name_is_valid()` | `true` / `false` for name and owner | Reject explicitly marked constructors whose names violate local syntax |
| `type_member_declaration_context()` | `Member`, `InterfaceMember` | Select concrete or contract member rules for each `TypeKind` |
| `abstract_type_modifier_is_valid()` | `true` / `false` for one `TypeKind` | Restrict explicit abstract type declarations to valid kinds |
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
| `validate_variants()` | `VariantIntent -> Result<(), _>` | Add target-local checks after crate-owned sequence validation |
| `collect_variant_validation_errors()` | `VariantIntent + error sink` | Add independent target-local sibling errors during file validation |
| `lower_variants()` | `ValidatedVariants -> CodeBlock` | Own complete variant-sequence grammar; defaults to frozen compatibility lowering |
| `type_keyword()` | `"class"`, `"struct"` | Type declaration keyword |
| `methods_inside_type_body()` | `true` / `false` | Legacy structural switch used by the compatibility type emitter |

#### Legacy `methods_inside_type_body()`

The current compatibility type emitter uses this method to decide whether a
`TypeSpec` produces one `CodeBlock` or two:

- **Returns `true`** (TypeScript, Java, Python, Swift, Dart, Kotlin, C++): Methods go inside the type body. TypeSpec emits a single block: `class Foo { fields; methods; }`.
- **Returns `false`** (Rust struct/enum): Methods go in a separate `impl` block. TypeSpec emits two blocks: `struct Foo { fields }` and `impl Foo { methods }`.

The method takes a `TypeKind` parameter, so current adapters can vary by type.
Rust returns `true` for `TypeKind::Trait` but `false` for `TypeKind::Struct` and
`TypeKind::Enum`. In the target design this structure is part of the adapter's
complete type lowering; the shared spec does not interpret a nesting switch.

### Renderer Configuration and Legacy Declaration Configuration

The current interface groups related values into six config structs. They do
not all have the same architectural role:

- `block_syntax()`, `generic_syntax()`, and `type_presentation()` participate in
  lower-level rendering seams with their own invariants.
- `function_syntax()`, `type_decl_syntax()`, and
  `enum_and_annotation()` expose declaration grammar that generic specs still
  interpret for pre-0.6.8 adapter compatibility. These accessors and their
  configuration types are deprecated. New function grammar belongs in
  `lower_function()` and new variant grammar belongs in `lower_variants()`.
  Other type/member emitters may temporarily use existing fields where no
  complete lowering seam exists yet, but must not extend them.

Do not add public flags, enums, or fields to accommodate a new language. A
previously unseen declaration form is evidence that lowering must move behind
the language adapter's complete-declaration seam. The detailed tables below
document the frozen function compatibility contract and the transitional type
and enum behavior that existing specs may still require.

Each config struct uses `..Default::default()` so current adapters only specify
values where they differ.

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

#### Legacy `function_syntax()`

Returns `FunctionSyntaxConfig` controlling function declarations:

| Field | Default | Purpose |
|-------|---------|---------|
| `return_type_separator` | `": "` | Between params and return type. Rust overrides to `" -> "`. |
| `async_keyword` | `"async "` | Async function prefix. |
| `async_suffix` | `""` | Async suffix after params. Dart: `" async"`. |
| `async_suffix_before_return` | `false` | When `true`, suffix goes before return type. Swift: `func f() async -> T`. |
| `abstract_keyword` | `"abstract "` | Abstract method prefix. C++ overrides to `"virtual "`. |
| `param_list_style` | (default) | How parameter lists are formatted. |
| `function_signature_style` | (default) | Controls overall signature layout. |
| `constructor_keyword` | `""` | Constructor keyword. Python: `"def"`. Rust: `"fn"`. |
| `constructor_delegation_style` | (default `Body`) | Super/this call placement. Kotlin, Dart, and C++ use `Signature`. |
| `where_clause_style` | `Inline` | `Inline`: bounds in `<T: Bound>`. `WhereBlock`: Rust `where\n    T: Bound,`. `SeparateWhere`: C# `where T : Bound` per constraint. |
| `empty_body` | `""` | Empty method body. Python overrides to `"..."`. |
| `type_params_before_return_type` | `false` | Legacy placement switch interpreted by the default function lowerer. Java sets it; Kotlin owns its different order in local lowering. |

#### Legacy `type_decl_syntax()`

Returns `TypeDeclSyntaxConfig` controlling type declarations:

| Field | Default | Purpose |
|-------|---------|---------|
| `type_before_name` | `false` | C/C++/Java override to `true` for `int count`. |
| `return_type_is_prefix` | `false` | C/C++/Java override to `true` for `int add(...)`. |
| `type_annotation_separator` | `": "` | Between name and type annotation. |
| `super_type_keyword` | (default) | Inheritance keyword, e.g. `" extends "`. |
| `super_type_separator` | (default) | Separator between multiple super types. |
| `super_type_subsequent_separator` | (default) | Separator for subsequent super types. |
| `implements_keyword` | (default) | Interface keyword, e.g. `" implements "`. |
| `type_alias_target_first` | `false` | C overrides to `true` for `typedef target name;`. |
| `supports_primary_constructor` | `false` | Kotlin overrides to `true`. |

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

#### Legacy `enum_and_annotation()`

Returns `EnumAndAnnotationConfig` controlling enums, annotations, and field modifiers:

| Field | Default | Purpose |
|-------|---------|---------|
| `variant_prefix` | `""` | Enum variant prefix. Swift overrides to `"case "`. |
| `variant_prefix_first` | (default) | Prefix for the first variant specifically. |
| `variant_separator` | `","` | Between enum variants. Python/Swift override to `""`. |
| `variant_trailing_separator` | `false` | Rust/TypeScript override to `true`. |
| `annotation_prefix` | `"@"` | Annotation opening. Rust: `"#["`. C++: `"[["`. |
| `annotation_suffix` | `""` | Annotation closing. Rust: `"]"`. C++: `"]]"`. |
| `readonly_keyword` | `"const "` | TS: `"readonly "`. Kotlin: `"val "`. Java: `"final "`. |
| `mutable_field_keyword` | `""` | Default mutable property-promotion keyword; Kotlin uses `"var "`. |

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
- `type_kind_suffix()` -- suffix after type close for specific type kinds.
- `emit_newtype_decl()` -- emits a structured `CodeBlock` for a newtype. The
  default is the Rust tuple struct `struct Name(Inner);`.
- `fun_block_open()` -- custom block opener for functions.
- `type_header_block_open()` -- custom block opener for type headers.
- `doc_comment_inside_body()` -- whether doc comments go inside the body (Python docstrings).
- `doc_before_annotations()` -- whether doc comments appear before annotations.
- `optional_field_style()` -- how optional fields are represented.
- `property_style()` -- default `Accessor` (TS/JS: `get name()`). Swift/Kotlin: `Field` (inline get/set).
- `property_getter_keyword()` -- default `"get"`. Kotlin: `"get()"`.
- `emit_type_context()` -- optional structured context for split function
  signatures.
- `type_body_prefix()` -- content emitted before the type body.
- `type_body_suffix()` -- content emitted after the type body.
- `emit_type_close_suffix()` -- optional structured suffix after a type's close
  delimiter, such as Haskell `deriving`.
- `render_type_param_kind()` -- how type parameters are annotated with variance.
- `line_comment_suffix()` -- suffix for line comments (default `""`).

`render_imports()` receives a deduplicated, alias-resolved `ImportGroup` and
emits the file's import header. `render_doc_comment()` emits spec-level doc
comments. Study `src/lang/typescript.rs` for ES module imports or
`src/lang/rust.rs` for `use` paths.

The three `emit_*` type hooks return `Result` so construction failures reach
`FileSpec::render()`. `emit_type_context()` and
`emit_type_close_suffix()` return `Ok(None)` when the language has no fragment
to add. Use `Arg::TypeName` or `%T` for every semantic type and compose child
blocks structurally; do not render a `TypeName` to a string inside a hook.

## Step-by-Step Walkthrough

### 1. Create the language file

Create `src/lang/your_lang.rs`. Keep semantic types in `%T` slots and return
blocks without a trailing newline; the spec caller owns surrounding whitespace,
indentation, and line breaks. Hook errors should be returned unchanged.

```rust,ignore
use sigil_stitch::code_block::{Arg, CodeBlock};
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::config::{
    BlockSyntaxConfig, GenericSyntaxConfig, TypeDeclSyntaxConfig,
};
use sigil_stitch::lang::{CodeLang, RendererLang, ValidatedFunction};
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::where_spec::{TypeParamSpec, render_type_params};
use sigil_stitch::type_name::TypeName;

#[derive(Debug, Clone, Default)]
pub struct YourLang;

impl YourLang {
    pub fn new() -> Self {
        Self
    }
}

const RESERVED: &[&str] = &["if", "else", "for", "while", /* ... */];

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

    fn lower_function(
        &self,
        function: ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut block = CodeBlock::builder();
        block.add(
            "%Lfunction %L(",
            (
                self.render_visibility(
                    function.modifiers().visibility,
                    function.declaration_context(),
                ),
                function.name(),
            ),
        );
        for (index, parameter) in function.parameters().iter().enumerate() {
            if index > 0 {
                block.add(",%W", ());
            }
            block.add("%L: %T", (parameter.name(), parameter.param_type().clone()));
        }
        block.add(")", ());
        if let Some(return_type) = function.return_type() {
            block.add(": %T", return_type.clone());
        }
        if let Some(body) = function.body() {
            block.add(" {", ());
            block.add_line();
            block.add("%>", ());
            block.add_code(body.clone());
            block.add_line();
            block.add("%<}", ());
        } else {
            block.add(";", ());
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

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum => "enum",
            TypeKind::Struct => "class",
            TypeKind::TypeAlias => "type",
            TypeKind::Newtype => "class",
        }
    }
    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool { true }

    fn emit_newtype_decl(
        &self,
        visibility: &str,
        name: &str,
        type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut args = Vec::new();
        let params = render_type_params(type_params, self, &mut args);
        args.push(Arg::TypeName(inner.clone()));
        CodeBlock::of(&format!("{visibility}opaque {name}{params} = %T"), args)
    }

    // Transitional type-declaration compatibility override. Do not add fields
    // here for new grammar; move complete type lowering behind an adapter seam.
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            super_type_keyword: " extends ",
            implements_keyword: " implements ",
            ..Default::default()
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

### 4. Generate golden files

```bash
just bless
```

This runs all tests with `BLESS=1`, which creates `test-goldens/your_lang/*.yl` files from the actual output. Review them manually, then commit.

### 5. Complete transitional non-function compatibility

Run the full test suite and review golden file output. Implement function
grammar in `lower_function()`. Type and enum declaration forms that have not
yet moved behind a complete language-local seam may still require deprecated
syntax accessors. Use existing fields only where they already express the
target, and do not add a shared field or enum for an unseen grammar dimension.
Examples of remaining transitional overrides are:

- If types come before names (`int x` instead of `x: int`): override `type_decl_syntax()` to set `type_before_name`, `return_type_is_prefix`
- If generics use brackets instead of angle brackets: override `generic_syntax()` to set `open`, `close`

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
