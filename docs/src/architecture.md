# Architecture

This chapter describes how sigil-stitch carries declaration intent to source
text. It covers ownership, the materialization and rendering pipeline, and
import resolution.

The type-declaration, function, field, property, and enum-variant lowering
seams described here are implemented for every built-in language. External
adapters that retain permissive capabilities may still use frozen pre-0.6.8
compatibility lowerers. See [Declaration Specs and Language
Lowering](declaration_lowering.md) for the ownership decision and [0.6.8 Legacy
Compatibility and Migration](legacy_compatibility_and_migration.md) for the
versioned compatibility contract.

The type-name-lowering pass and complete-set fallible import resolver described
here are implemented. The direct renderer-event methods remain an accepted 0.7
migration documented before implementation; the compatibility appendix records
the shared renderer grammar still used before that cutover.

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
                         +-- rewrite each source tree exactly once
                         +-- validate rewritten structure
                         +-- fallible language-owned TypeName lowering
                         +-- validate lowered type blocks
                         +-- collect and resolve imports
                         +-- final render with no rewrite or type lowering
                         |
                         v
                     source text
```

The important seam is between declaration intent and target grammar. Specs own
the former; a language adapter owns the latter. `CodeBlock` is the shared
structured source container passed through source rewrite, type-name lowering,
import resolution, and final rendering. A block is associated with its selected
target and is not a portable cross-language program.

## Language Interfaces

`src/lang/mod.rs` defines two traits:

- **`RendererLang`** is the renderer-only interface used by
  `code_renderer.rs`. It covers file extensions, string literals, block
  rendering, one complete fallible `lower_type_name()` seam, and other stable
  final-rendering policy. Implementing it is sufficient for direct
  `CodeBlock` rendering. Built-in adapters lower complete `TypeName` values to
  non-empty `CodeBlock`s; the provided default retains only frozen pre-0.6.8
  compatibility behavior.
- **`CodeLang: RendererLang`** adds declaration representability, lowering,
  imports, and spec-level documentation. A complete type crosses
  `validate_type()` / `collect_type_validation_errors()` and then
  `lower_type(ValidatedType) -> Vec<CodeBlock>`. The result contains one or
  more non-empty blocks; an empty vector or block fails closed. The validated
  view contains crate-validated child wrappers, so the type adapter owns the
  declaration's preamble, header, relationships, body order, primary
  constructor, close, and output cardinality while reusing complete child
  lowerers for child grammar.
  After crate-owned validation against
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
  `validate_variants()` sees the owning declaration, complete ordered
  `VariantIntent`, and whether non-variant members exist; adapters with
  independent per-variant checks implement the
  additive `collect_variant_validation_errors()` seam. `lower_variants()`
  receives `ValidatedVariants` and owns preambles, payload spelling,
  separators, and section termination. Callers do not assemble target
  declaration grammar from fragments, and adapters cannot construct or bypass
  the validated wrappers.

Each supported language implements both traits in its own module
(`src/lang/typescript.rs`, etc.). Control-flow nodes carry a language-neutral
`BlockIntent`; in the accepted renderer-event design, each adapter maps that
intent locally through `render_block_open()`, `render_block_close()`, and
`render_branch_transition()`. The current source retains intent-aware and
legacy block hooks as the compatibility bridge; the accepted renderer-event
interface replaces current-path reads of those hooks. Languages can
implement `rewrite_nodes()` for structural or literal fixups such as Go IIFE
`}()` fusion or C++ lambda `};` semicolons. The core invokes this existing
source-tree seam once per source block after declaration lowering and before
type-name lowering. It validates the rewritten structure before continuing.
Type-name-lowering results and raw import metadata are not rewritten.

Deprecated grammar and type-presentation accessors remain only at
compatibility boundaries for external adapters and direct compatibility
facades. New adapters and new syntax dimensions use language-owned lowering.
The complete inventory and migration replacements are in [0.6.8 Legacy
Compatibility and Migration](legacy_compatibility_and_migration.md).

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
    -> private FormattedCode / QuoteArg / Statement parse forms
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
option boundary. No partial parse model reaches codegen. Direct ordinary and raw string
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
| `StringLiteral` | `'active'`, `Literal["active"]` | Target-derived imports tracked after lowering |
| `Raw` | any string | No |

Every variant that contains other types remains structured until the selected
adapter lowers the complete root. The lowering result retains importable leaf
references in its `CodeBlock`, so ordinary nested imports and target-derived
imports are collected together before alias resolution.

#### Type-Name Lowering

`TypeName` variants are semantic: `Array(T)` means an array type and
`StringLiteral(value)` means one decoded string singleton. They do not select a
shared prefix, delimiter, precedence, or fallback. `RendererLang` owns one
fallible `lower_type_name(&TypeName) -> Result<CodeBlock, _>` method.

The crate validates the semantic value before the call and validates the
returned block afterward. Successful blocks are non-empty, contain only type
expression structure, and leave no unresolved compound `TypeName`. Unsupported
forms fail before import collection instead of inheriting TypeScript-like
defaults or widening to a primitive. See [TypeName Validation and
Lowering](type_name_lowering.md) for the complete contract.

## Structured Source Container: CodeBlock

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
non-variant members exist, the accepted arity ranges of structured
constructors, and whether opaque members may provide target-specific
constructor syntax. The type lowerer chooses where the sequence appears. A
language profile distinguishes discriminants, enum-entry constructor
arguments, positional payloads, record payloads, and attributes.
`VariantContext` is only
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
source rewrite -> validate rewritten tree -> lower TypeRefs
        |
        v
collect imports -> resolve aliases -> final CodeRenderer -> source text
```

Raw bodies, annotations, suffixes, and file fragments are explicit escape
hatches. They may contain target-specific syntax, but remain opaque to generic
specs and shared lowerers; their existence does not move ownership of the
surrounding declaration grammar into the spec. A private Python validator
recognizes the documented 0.6.8 `is_static` plus decorator pattern solely as a
frozen adapter-local compatibility exception. New semantics must not extend
that recognizer or add a shared syntax hook.

## File Rendering Pipeline

`FileSpec::render(width)` owns one ordered preparation and rendering pipeline.
It does not emit an import header or body text until declaration lowering,
source rewrite, type-name lowering, lowered-block validation, and import
resolution have all succeeded.

Declaration validation checks every `TypeSpec` against the type, function,
field, property, and enum-variant profiles returned by
`CodeLang::capabilities()`. Public `FileSpec::validate()` exposes the stored
intent checks; render preparation performs the same declaration checks without
calling the public method and retains successful lowered output.
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

### Validate and Lower Declarations

Declaration specs are validated and converted to `CodeBlock`s:
- `FileMember::Type(TypeSpec)` calls `type_spec.emit(&lang)` -> `Vec<CodeBlock>`
- `FileMember::Fun(FunSpec)` calls `fun_spec.emit(&lang, ctx)` -> `CodeBlock`
- `FileMember::Code(CodeBlock)` is cloned into an owned source block
- `FileMember::RawContent(String)` remains opaque
- `FileMember::RawContentWithImports` retains opaque text plus separate type
  metadata

The public type, function, field, property, and owner-aware variant `emit`
paths apply crate-owned semantic validation, call the corresponding
`CodeLang::validate_*()` method for additional target-local checks, construct a
`ValidatedType`, `ValidatedFunction`, `ValidatedFields`,
`ValidatedProperty`, or `ValidatedVariants`, and then call the matching
`CodeLang::lower_*()` method. `ValidatedType` contains the validated child
wrappers produced against that same adapter and deliberately does not
dereference to unvalidated `TypeIntent`. The defaults delegate to frozen
legacy-syntax compatibility modules so pre-0.6.8 external adapters remain
source compatible. Built-in complete lowerers do not consume deprecated
declaration configuration.

`TypeMembersIntent` is validation evidence only. Its pass runs after the
per-family checks and creates neither a validated wrapper nor a `CodeBlock`.

Language lowering composes structured child blocks and preserves every
`TypeName` as a `TypeRef`. Construction errors propagate from this pass; they
are never converted to empty output, and complete type lowering rejects empty
vectors or blocks. After materialization, everything is either a `CodeBlock`
or explicitly raw content.

### Rewrite and Lower Source Blocks

The core calls `RendererLang::rewrite_nodes()` exactly once for every owned
source block: the header, each direct caller block, and each block returned by a
declaration lowerer. The adapter may recurse through `Nested` and `Sequence`
with the standard rewrite walker; the core does not call the hook again for
those children. The complete rewritten tree is then checked for structural
errors.

Rewrite sees semantic, unaliased `TypeRef` nodes. It is a target source
correction seam, not declaration or type grammar. The existing public hook can
change those nodes, so the next step always observes the rewritten result.

The core then walks every rewritten source tree and lowers each
`CodeNode::TypeRef` through the selected adapter. Intrinsic type-name validation
runs before `RendererLang::lower_type_name()`; the returned non-empty block is
validated afterward and replaces the original node. The validator recurses
through adapter-produced blocks and permits only terminal, import-aware type
references. Unresolved compound types, empty output, or statement and
control-flow nodes fail the complete file. Blocks returned by
`lower_type_name()` are not source-rewrite inputs and are not rewritten again.

Opaque raw content is neither rewritten nor type-lowered. The separate types
listed by `RawContentWithImports` are import metadata rather than source trees:
the core lowers and validates them to discover imports but never passes them
through `rewrite_nodes()` or substitutes their spelling into the raw text.

`FileSpec::validate()` checks stored declaration and type intent but does not
invoke source rewrite or emit dynamic blocks. Render preparation remains the
authoritative check for the actual rewritten output.

### Collect and Resolve Imports

`import_collector` then walks the fully lowered tree. Each remaining
`CodeNode::TypeRef` yields its `ImportRef` (module, name, and optional alias).
This includes target-derived imports introduced by type-name lowering, such as
Python's structured `typing.Literal` reference. Lowered raw-import metadata
contributes imports through the same collector without becoming source text.

Nested CodeBlocks (`CodeNode::Nested`) and sequences are walked recursively.

### Import Resolution

The accepted fallible path merges explicit imports, deduplicates identical
semantic imports, reserves names from non-conflicting bindings, and constructs
every ambiguous requested-name class before calling a resolver. Imports in one
class are peers: the public context has no incoming import, current owner,
winner, loser, or mutable claim table.

Each peer request is one of:

- **Exact** -- an explicit local binding that must be preserved because opaque
  caller source may refer to it;
- **Preferred** -- a soft alias requested through `TypeName::with_alias()`; or
- **Natural** -- the original simple name, also a soft request.

A resolver receives the complete ambiguous set once per file and returns an
atomic assignment for every peer. Core validation requires every peer exactly
once, preserves exact bindings, rejects blank or unsafe names, and enforces
global uniqueness before the selected language validates identifier grammar,
reserved words, alias support, and import form. Any failure aborts the file
before an import header or body is returned.

Ordinary `FileSpec` rendering uses a deterministic default resolver. Encounter
order is only that resolver's compatibility tie-break: it may give the natural
name to one peer and module-derived aliases to the others, but this does not
make that peer an owner in the model. A borrowed custom resolver can choose a
different complete assignment. It is supplied to the render call and is never
stored or serialized in `FileSpec` or `ProjectSpec`.

The fallible `ImportGroup::try_resolve*()` entry points implement the current
core contract. The pre-0.6.8 infallible `resolve()` and
`resolve_with_explicit()` implementations remain frozen deprecated
compatibility APIs rather than wrappers that discard fallible errors.

After assignment, `qualify_import_reference()` receives the module, original
symbol, and resolved binding. Go uses it to render `http.Server` with a
package-level import of `"net/http"`. Haskell uses the same hook to turn an
assigned symbol alias into a module-qualified reference and renders the
corresponding import as `qualified`. The old two-argument
`qualify_import_name()` remains only as the 0.6.8 compatibility hook.

### Final Render

After aliases are resolved, the private final-rendering entry point in
`CodeRenderer` walks each prepared `CodeBlock`'s `CodeNode` sequence. It does
not rewrite the tree or lower another type root:

| Node | Action |
|------|--------|
| `Literal(s)` | Emit string directly |
| `TypeRef(tn)` | Resolve and emit one already-lowered terminal type reference |
| `NameRef(s)` | Emit identifier |
| `StringLit(s)` | Call `lang.render_string_literal()` |
| `VerbatimStr(s)` | Call `lang.render_verbatim_string()` |
| `InlineLiteral(s)` | Emit raw literal |
| `Nested(block)` | Recursively render the inner CodeBlock |
| `Comment(s)` | Emit with `lang.line_comment_prefix()` |
| `SoftBreak` | Pretty-print decision point |
| `Indent` / `Dedent` | Adjust indent level |
| `StatementBegin` / `StatementEnd` | Statement boundaries; `render_statement_end()` supplies the complete suffix |
| `Newline` | Emit newline + indent |
| `BlockOpenIntent` / `BlockCloseIntent` | Map `BlockIntent` + condition through `render_block_open()` / `render_block_close()` |
| `BranchCloseIntent` | Ask `render_branch_transition()` for the complete outgoing closer and connector whitespace |
| `BlockOpen` / `BlockClose` / `BranchClose` | Deprecated legacy string-only nodes for old serialized blocks and external adapters |
| `Sequence(children)` | Recursively render a sub-sequence of nodes |

**Width-aware rendering**: One semantic walker interprets every prepared
`CodeNode`. CodeBlocks without `SoftBreak` use a direct string adapter. When a
`SoftBreak` exists anywhere in the tree, the same walker uses a `pretty::BoxDoc`
adapter for the whole tree so the Wadler-Lindig algorithm can choose between a
space and an indented line break. `Nested` and `Sequence` nodes form layout
groups without resetting renderer state. Both adapters preserve the language's
`indent_unit()` string exactly.

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

The two imports are peers in one conflict class. The default resolver uses
encounter order only as a deterministic compatibility tie-break, so this
example assigns `User` to `./models` and the module-derived `LegacyUser` to
`./legacy`. A custom complete-set resolver may assign both peers differently
while still satisfying exact bindings, uniqueness, and TypeScript identifier
rules.

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
