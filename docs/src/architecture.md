# Architecture

This chapter describes how sigil-stitch carries declaration intent to source
text. It covers ownership, the materialization and rendering pipeline, and
import resolution.

The function, field, property, and enum-variant declaration-lowering seams
described here are implemented for every built-in language. Type declarations
still have a pre-0.6.8 compatibility path in which the generic spec emitter
interprets shared syntax configuration. That path is transitional and must not
be expanded. See [Declaration Specs and Language
Lowering](declaration_lowering.md) for the ownership decision and [0.6.8 Legacy
Compatibility and Migration](legacy_compatibility_and_migration.md) for the
versioned compatibility contract.

## Pipeline and Ownership

```text
Declaration specs + TypeName + opaque CodeBlock payloads
                         |
                         +-- intrinsic validation
                         +-- target capability validation
                         |
                         v
              target-language adapter
              owns complete declaration lowering
                         |
                         v
                   CodeBlock tree
                         |
                         +-- collect and resolve imports
                         +-- rewrite and validate nodes
                         +-- select one layout adapter
                         |
                         v
                     source text
```

The important seam is between declaration intent and target grammar. Specs own
the former; a language adapter owns the latter. `CodeBlock` is the structured
rendering IR passed to import resolution and final rendering.

## Language Interfaces

`src/lang/mod.rs` defines two traits:

- **`RendererLang`** is the renderer-only interface used by
  `code_renderer.rs` and `TypeName::to_doc_with_lang`. It covers file
  extensions, string literals, block rendering, type presentation, and other
  final-rendering policy. Implementing it is sufficient for direct
  `CodeBlock` rendering.
- **`CodeLang: RendererLang`** adds declaration representability, lowering,
  imports, and spec-level documentation. After crate-owned validation against
  the selected adapter, `validate_function()` may add target-local checks to a
  classified `FunctionIntent`. sigil-stitch then constructs a
  `ValidatedFunction`; `lower_function()` accepts that validated read-only view
  and returns a structured `CodeBlock`. Fields follow the same pattern at
  sequence granularity: `validate_fields()` receives `FieldSequenceIntent`,
  `collect_field_validation_errors()` preserves independent sibling failures,
  and `lower_fields()` receives `ValidatedFields`. Properties use
  `PropertyIntent` with a direct-or-owner-aware `PropertyContext`;
  `collect_property_validation_errors()` preserves independent failures and
  `lower_property()` receives a crate-constructed `ValidatedProperty`. The
  adapter decides whether one property becomes separate accessor declarations,
  a computed-property body, or target-local methods. After every member family
  has been checked, one validation-only `TypeMembersIntent` containing the
  owner's semantic fields, properties, and explicit methods passes through
  `validate_type_members()` and its additive collector. This seam handles
  cross-family relationships and has no lowering counterpart. Enum variants
  likewise use a complete sequence:
  `validate_variants()` sees the owning declaration and complete ordered
  `VariantIntent`; adapters with independent per-variant checks implement the
  additive `collect_variant_validation_errors()` seam. `lower_variants()`
  receives `ValidatedVariants` and owns preambles, payload spelling,
  separators, and section termination. Callers do not assemble target
  declaration grammar from fragments, and adapters cannot construct or bypass
  the validated wrappers.

Each supported language implements both traits in its own module
(`src/lang/typescript.rs`, etc.). Control-flow nodes carry a language-neutral
`BlockIntent`; each adapter maps that intent locally through
`block_open_for_intent()` / `block_close_for_intent()`. Languages can implement
`rewrite_nodes()` for structural or literal fixups such as Go IIFE `}()` fusion
or C++ lambda `};` semicolons.

Deprecated declaration-grammar accessors remain only at compatibility
boundaries for external adapters and the transitional `TypeSpec` emitter. New
adapters and new syntax dimensions use language-owned lowering. The complete
inventory and migration replacements are in [0.6.8 Legacy Compatibility and
Migration](legacy_compatibility_and_migration.md). Stable renderer policy and
the separately documented `TypeName` presentation seam are lower-level
concerns, not permission for specs to interpret target grammar.

At the macro level, the `MacroLang` enum (`macros/src/parse/lang.rs`) provides compile-time language-aware tokenizer annotations. Languages like Bash, Zsh, Go, and Haskell get specialized spacing rules in `sigil_quote!` without runtime overhead. See [Language-Aware Tokenizer](macrolang.md).

Public container types have no language generic parameter. The language enters
as `&dyn RendererLang` for direct rendering or `&dyn CodeLang` for declaration
materialization. `FileSpec` stores a `Box<dyn CodeLang>` internally. A
`CodeBlock` can nevertheless contain target-specific literal text; language
independence of its Rust type is not a promise that every block is portable.

## Macro Front End

`sigil_quote!` has a private typed pipeline before the public `CodeBlock` layer:

```text
macro tokens
    -> parse::parse_input
    -> FormattedCode / QuoteArg / Statement IR
    -> infallible codegen
    -> caller-scope CodeBlockBuilder calls
```

Rust-bearing values cross the parser boundary as `syn::Expr`, `syn::Pat`, or
`syn::Local`; codegen quotes those nodes directly and never reparses token
strings. A `FormattedCode` privately couples each target format string to its
typed arguments, deriving the format specifier from the argument variant so
the two cannot drift apart.

Parsing returns `syn::Error`. Independent failures are combined while recovery
can advance to a reliable sibling statement, interpolation group, or loop
option boundary. No partial IR reaches codegen. Direct ordinary and raw string
literals use `syn::LitStr` decoding. A single-pass lexical boundary scan skips
Rust strings, characters, nested comments, and nested braces before each
`@{...}` body is parsed once as a Rust expression; dynamic string expressions
are not scanned.

Generated parsed blocks and splices use nested builders. Their runtime failures
flow into a local first-error slot rather than `unwrap`. Flat guarded lowering
skips later work after a helper failure, introducing a scoped continuation only
when a subsequent `$let` must remain visible to later statements. Caller `?`,
`return`, `break`, and `continue` targets remain unchanged. A validation
pass limits these guarded `$let` continuations to 128 levels so pathological
input fails with a macro diagnostic instead of exhausting rustc while parsing
the generated nesting. The public `CodeBlock`, error, and rendering contracts
are unaffected.

## Semantic Type References: TypeName

`src/type_name.rs` defines type references. Key variants:

| Variant | Example | Import Tracked? |
|---------|---------|-----------------|
| `Primitive` | `string`, `i32` | No |
| `Importable` | `User` from `./models` | Yes |
| `Generic` | `Promise<User>` | Recursively |
| `Array` | `User[]`, `Vec<User>` | Inner type tracked |
| `ReadonlyArray` | `readonly User[]` | Inner type tracked |
| `Optional` | `User?`, `Option<User>` | Inner type tracked |
| `Union` | `string \| number` | All members tracked |
| `Intersection` | `A & B`, `A + B` | All members tracked |
| `Tuple` | `[A, B]`, `(A, B)` | All members tracked |
| `Reference` | `&T`, `const T&` | Inner type tracked |
| `Function` | `(x: string) => void` | Params + return tracked |
| `Map` | `Map<string, User>` | Key + value tracked |
| `Pointer` / `Slice` | `*const T`, `&[T]` | Inner type tracked |
| `Raw` | any string | No |

Every variant that contains other types recursively collects imports via `collect_imports()`. This means `Generic(Promise, [Importable(User)])` tracks the `User` import even though `Promise` is a primitive.

TypeName also renders to `pretty::BoxDoc` for width-aware output of complex type signatures. `BoxDoc` is used (rather than `RcDoc`) so rendered documents are `Send + Sync` and can cross thread boundaries.

#### Type Presentation Layer

`TypeName` variants are *semantic* — `Array(T)` means "array of T" regardless of language. Cross-language rendering is handled by a **data-driven presentation layer**:

1. Each `TypeName` variant asks the language for a `TypePresentation` — a data enum describing the syntactic pattern (e.g., `GenericWrap`, `Prefix`, `Postfix`, `Surround`, `Delimited`, `Infix`).
2. A single rendering engine in `type_name_render.rs` interprets the pattern into `BoxDoc` output.

`BoxDoc` never appears in the `RendererLang` interface. Languages return pure
data; the engine does all rendering. See [Type
Presentation](type_presentation.md) for the full design.

## Rendering IR: CodeBlock

A `CodeBlock` stores `nodes: Vec<CodeNode>` — a tree of self-contained nodes (`Literal`, `TypeRef`, `NameRef`, `StringLit`, `Comment`, `Nested`, etc.). Format strings are parsed at build time and immediately converted to `CodeNode` nodes. Each node is self-contained: `TypeRef(TypeName)` carries its type reference directly, and control-flow nodes carry a language-neutral `BlockIntent` (`BlockOpenIntent`, `BlockCloseIntent`, `BranchCloseIntent`) with no per-language rendering policy.

CodeBlocks are immutable after construction. The builder (`CodeBlockBuilder`) validates argument counts and indent balance before producing a block.

## Declaration Specs

`src/spec/` contains builders for target-independent declaration intent.
`TypeSpec`, `FunSpec`, `FieldSpec`, and related types record what the caller
wants to declare. They are a semantic superset: target capability validation
may reject intent that one language cannot represent.

Specs enforce intrinsic coherence, select declaration context, and delegate
target representability and lowering. They do not own keyword spelling, token
order, separators, type-parameter placement, or other target grammar. The
language adapter returns `CodeBlock`, never a type-bearing raw string, so
semantic `TypeName` references survive import collection and alias resolution.

An enum is lowered as one owner-aware variant sequence. `VariantIntent`
contains the owner name and kind, all variants in declaration order, whether
ordinary members follow, the accepted arity ranges of structured constructors,
and whether opaque members may provide target-specific constructor syntax. A
language profile distinguishes discriminants, enum-entry constructor arguments,
positional payloads, record payloads, and attributes. `VariantContext` is only
the deprecated positional input to the permissive external-adapter
compatibility path; strict built-ins reject ownerless direct emission because
caller-supplied first/last flags cannot prove valid separators or section
termination.

Fields are lowered as one `FieldSequenceIntent`. Its `FieldContext`
distinguishes direct emission, ordinary type members, and variant record
payloads without carrying punctuation or a new placement policy. The
`Direct(DeclarationContext)` payload preserves only the pre-0.6.8 direct-field
placement input as a narrow compatibility exception; it is not a reusable
target-grammar abstraction. Field capability profiles declare which semantic
facts each context supports or requires.
Intrinsic checks run even when the owning type or payload form is unsupported,
so malformed serialized fields still participate in aggregate validation.
Adapter-local collection then validates identifiers, emitted-name collisions,
modifier combinations, annotations, tags, and other target rules. Only the
crate can construct `ValidatedFields`, and only after the complete sequence has
passed every phase.

`FieldCapability::OptionalPresence` means that the containing value may omit a
field. `TypeName::Optional(T)` means that a present field can carry an option or
null value. Keeping those semantics separate prevents an adapter from silently
turning absence into nullability. Built-in adapters accept optional presence
only where the target representation preserves it.

A computed property is lowered as one `PropertyIntent`. Its `PropertyContext`
distinguishes the pre-0.6.8 direct facade from a member owned by a complete type
declaration. Property profiles declare support and requirements for explicit
types, read access, write access, attributes, and static behavior. Intrinsic
validation requires at least one accessor and rejects empty bodies, empty
setter names, and unrelated deserialized modifiers. Adapter-local validation
owns identifier, visibility, accessor-combination, and other target rules.
Only the crate can construct `ValidatedProperty`, and only after every phase
succeeds.

Owner-wide validation is a separate concern from property lowering.
`TypeMembersIntent` exposes one type's name and kind plus its semantic fields,
properties, and explicit methods after the per-family checks have run. The
crate rejects exact duplicate property names; an adapter uses
`collect_type_members_validation_errors()` for relationships created by its
own lowering. PHP checks the case-insensitive method namespace that contains
derived property accessors and explicit methods. TypeScript, Kotlin, Swift,
and Scala reject field/property names that their lowering maps into the same
target-local namespace; TypeScript private names and the TypeScript and Swift
static namespaces remain distinct. TypeScript, Swift, and Scala also reject
corresponding explicit-method collisions within the same namespace. These
rules remain language-local because the namespaces and derived names differ.
This intent contains no placement or syntax data, has no validated wrapper,
and never enters the materialization pipeline.

The intended declaration path is:

```text
TypeSpec / FunSpec
        |
        +-- intrinsic validation
        +-- language capability validation
        |
        v
CodeLang complete declaration lowering
        |
        v
CodeBlock with TypeRef nodes
        |
        v
collect imports -> resolve aliases -> CodeRenderer -> source text
```

Raw bodies, annotations, suffixes, and file fragments are explicit escape
hatches. They may contain target-specific syntax, but remain opaque to generic
specs and shared lowerers; their existence does not move ownership of the
surrounding declaration grammar into the spec. A private Python validator
recognizes the documented 0.6.8 `is_static` plus decorator pattern solely as a
frozen adapter-local compatibility exception. New semantics must not extend
that recognizer or add a shared syntax hook.

The current type compatibility emitter still reads pre-0.6.8 syntax
configuration inside `TypeSpec`. Complete type-declaration migration will move
that grammar behind the language adapter while preserving a frozen default for
existing external adapters.

## Three-Pass Rendering Pipeline

`FileSpec::render(width)` drives everything. It runs three passes over the file's members.

Before materialization, `FileSpec::validate()` checks every `TypeSpec` against
the type, function, field, property, and enum-variant profiles returned by
`CodeLang::capabilities()`.
After those per-family checks, one owner-wide type-members pass rejects
intrinsic duplicate property names and lets the adapter report target-derived
cross-member collisions.
Function validation distinguishes free functions, receiver methods, concrete
members, and interface members, then selects an ordinary-function, constructor,
or destructor profile within that context. Profiles declare supported and
required semantic capabilities, body policy, and forbidden capability pairs.
This rejects missing return or parameter types, unsupported annotations,
invalid body placement, malformed rest-parameter lists, and incompatible
modifiers before plausible wrong code can render. Adapters written for
sigil-stitch 0.6.8 inherit the permissive compatibility profile.

When a strict member profile requires a return type but its constructor
profile does not, direct `FunSpec` emission preserves the legacy ambiguous
constructor-shaped member convention because it has no declaring-type owner.
`TypeSpec` has the owner context and validates constructor identities exactly:
fixed names such as `constructor` and `init`, owner-derived Java/C#/C++ names,
and Dart named constructors are classified before capability validation. New
direct-emission code should use `is_constructor()` explicitly when the name
does not identify the form on its own.

Constructor classification remains language-specific after modifiers and
return types are known. A static owner-named member may be an ordinary method
in one language and a static constructor in another; Java also permits a
same-named ordinary method when an explicit return type disambiguates it.
Modifier-aware hooks refine the selected profile's body policy, parameter
limit, visibility, default-parameter ordering, and type-constraint
representability without weakening the declared capability matrix.
Constraint validation is syntax-independent by default. Adapters whose local
lowering attaches constraint subjects to declared type parameters opt into the
shared structural check explicitly; Rust retains its broader where-subject
model.

Type kinds select their member validation context through the language. Most
interfaces and traits use contract-member profiles, while module- or
trait-backed concrete constructs such as Ruby modules and PHP traits retain
concrete member rules. The same language policy decides which type kinds may
carry an explicit abstract modifier.

For languages where `is_abstract` represents an abstract method, a concrete
type containing such a method must itself be marked abstract. C++ remains the
exception because a pure virtual member makes the class abstract structurally.

### Pass 0: Materialize

Declaration specs are validated and converted to `CodeBlock`s:
- `FileMember::Type(TypeSpec)` calls `type_spec.emit(&lang)` -> `Vec<CodeBlock>`
- `FileMember::Fun(FunSpec)` calls `fun_spec.emit(&lang, ctx)` -> `CodeBlock`
- `FileMember::Code(CodeBlock)` passes through unchanged
- `FileMember::RawContent(String)` passes through as-is

The public function, field, property, and owner-aware variant `emit` paths apply
crate-owned semantic validation, call the corresponding
`CodeLang::validate_*()` method for additional target-local checks, construct a
`ValidatedFunction`, `ValidatedFields`, `ValidatedProperty`, or
`ValidatedVariants`, and then call the matching `CodeLang::lower_*()` method.
The defaults delegate to frozen legacy-syntax compatibility modules so
pre-0.6.8 external adapters remain source compatible. Built-in complete
lowerers do not consume deprecated declaration configuration for the migrated
family.

`TypeMembersIntent` is validation evidence only. Its pass runs after the
per-family checks and creates neither a validated wrapper nor a `CodeBlock`.

Language lowering composes structured child blocks and preserves every
`TypeName` as a `TypeRef`. Construction errors propagate from this pass; they
are never converted to empty output. After materialization, everything is
either a `CodeBlock` or explicitly raw content.

### Pass 1: Collect Imports

`import_collector` walks every CodeBlock tree. For each `CodeNode::TypeRef` in any block, it calls `type_name.collect_imports()` to extract `ImportRef` structs (module + name + optional alias).

Nested CodeBlocks (`CodeNode::Nested`) are walked recursively. `RawContentWithImports` members have their type list walked for imports even though the content itself is opaque.

### Import Resolution

`ImportGroup::resolve()` takes the collected `ImportRef` list and:

1. **Deduplicates**: Same module + same name = one import
2. **Detects conflicts**: Two different modules exporting the same name (e.g., `User` from `./models` and `User` from `./legacy`)
3. **Assigns aliases**: First-encountered `User` wins the simple name. The second gets aliased using a module-derived prefix (e.g., `LegacyUser`)
4. **Merges explicit imports**: `ImportSpec` entries (aliased, side-effect, wildcard) are merged into the resolved set

The result is an `ImportGroup` that maps each module to its resolved names with aliases.

`qualify_import_name()` receives the module, original symbol, and resolved
alias. Go uses it to render `http.Server` with a package-level import of
`"net/http"`. Haskell uses the same hook to turn an assigned symbol alias into
a module-qualified reference and renders the corresponding import as
`qualified`.

### Pass 2: Render

`CodeRenderer` walks each CodeBlock's `CodeNode` sequence:

| Node | Action |
|------|--------|
| `Literal(s)` | Emit string directly |
| `TypeRef(tn)` | Resolve import name via ImportGroup, emit |
| `NameRef(s)` | Emit identifier |
| `StringLit(s)` | Call `lang.render_string_literal()` |
| `VerbatimStr(s)` | Call `lang.render_verbatim_string()` |
| `InlineLiteral(s)` | Emit raw literal |
| `Nested(block)` | Recursively render the inner CodeBlock |
| `Comment(s)` | Emit with `lang.line_comment_prefix()` |
| `SoftBreak` | Pretty-print decision point |
| `Indent` / `Dedent` | Adjust indent level |
| `StatementBegin` / `StatementEnd` | Statement boundaries (`;` if applicable) |
| `Newline` | Emit newline + indent |
| `BlockOpenIntent` / `BlockCloseIntent` | Map `BlockIntent` + condition through `lang.block_open_for_intent()` / `block_close_for_intent()` |
| `BranchCloseIntent` | Transition close + space when `close_on_transition` is set |
| `BlockOpen` / `BlockClose` / `BranchClose` | Deprecated legacy string-only nodes for old serialized blocks and external adapters |
| `Sequence(children)` | Recursively render a sub-sequence of nodes |

**Width-aware rendering**: One semantic walker interprets every rewritten
`CodeNode`. CodeBlocks without `SoftBreak` use a direct string adapter. When a
`SoftBreak` exists anywhere in the tree, the same walker uses a `pretty::BoxDoc`
adapter for the whole tree so the Wadler-Lindig algorithm can choose between a
space and an indented line break. `Nested` and `Sequence` nodes form layout
groups without resetting renderer state. Both adapters preserve the language's
configured indentation string exactly.

## Import Conflict Resolution

A concrete example of the conflict resolution:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let user_a = TypeName::importable_type("./models", "User");
let user_b = TypeName::importable_type("./legacy", "User");

let mut cb = CodeBlock::builder();
cb.add_statement("const a: %T = getA()", (user_a,));
cb.add_statement("const b: %T = getB()", (user_b,));
let body = cb.build().unwrap();

let output = FileSpec::builder("test.ts")
    .add_code(body)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

The output would contain:
```typescript
import type { User } from './models'
import type { User as LegacyUser } from './legacy'

const a: User = getA();
const b: LegacyUser = getB();
```

The first `User` (from `./models`) wins the simple name. The second (from `./legacy`) gets the alias `LegacyUser`, derived from the module path.

## Language-Independent Containers and Target-Specific Payloads

Public types such as `CodeBlock`, `TypeName`, `TypeSpec`, and `FunSpec` have no
target-language generic parameter. The target is supplied through
`&dyn RendererLang` or `&dyn CodeLang` when a block or declaration is
materialized and rendered.

The distinction is about the Rust interface, not automatic portability of all
values:

- `TypeName::Array(T)` and a `FunSpec` type-parameter list are semantic and can
  be lowered for different targets.
- A `CodeBlock` containing the literal `const u = ...` is already
  target-language source, even though the `CodeBlock` type itself is shared.
- `TypeRef`, `StringLit`, comments, layout intent, and import references remain
  structured until the renderer applies target policy.

`FileSpec::builder("user.ts")` auto-detects the adapter from the file
extension. `FileSpec::builder_with(...)` selects one explicitly. In both cases,
the adapter must validate declaration intent and own its concrete grammar.
