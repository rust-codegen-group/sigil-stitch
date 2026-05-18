# Architecture

This chapter describes how sigil-stitch works internally. It covers the four-layer design, the three-pass rendering pipeline, and the import resolution system.

## Four Layers

The library is organized in four layers, each building on the one below:

```text
┌─────────────────────────────────────┐
│  Spec Layer (TypeSpec, FunSpec, ...) │  Structural builders
├─────────────────────────────────────┤
│  CodeBlock + Format Specifiers       │  Composable code fragments
├─────────────────────────────────────┤
│  TypeName                            │  Type references with import tracking
├─────────────────────────────────────┤
│  CodeLang Trait                      │  Language abstraction
└─────────────────────────────────────┘
```

### Layer 1: CodeLang

`src/lang/mod.rs` defines the `CodeLang` trait with 36 methods, including 6 config struct accessors (`type_presentation()`, `generic_syntax()`, `block_syntax()`, `function_syntax()`, `type_decl_syntax()`, `enum_and_annotation()`) that return data structs with sensible defaults. Each supported language implements this trait in its own module (`src/lang/typescript.rs`, etc.). Languages can also implement `rewrite_nodes()` to transform the CodeNode tree after macro expansion for language-specific fixups (e.g., Go IIFE `}()` fusion, C++ lambda `};` semicolons).

At the macro level, the `MacroLang` enum (`macros/src/parse/types.rs`) provides compile-time language-aware tokenizer annotations. Languages like Bash, Zsh, Go, and Haskell get specialized spacing rules in `sigil_quote!` without runtime overhead. See [Language-Aware Tokenizer](macrolang.md).

Public types are language-agnostic — no generic parameter. The language enters as `&dyn CodeLang` at render time. `FileSpec` stores a `Box<dyn CodeLang>` internally; all other types (`CodeBlock`, `TypeName`, specs) are language-independent.

### Layer 2: TypeName

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
2. A single rendering engine in `type_name.rs` interprets the pattern into `BoxDoc` output.

`BoxDoc` never appears in the `CodeLang` trait. Languages return pure data; the engine does all rendering. See [Type Presentation](type_presentation.md) for the full design.

### Layer 3: CodeBlock

A `CodeBlock` stores `nodes: Vec<CodeNode>` — a tree of self-contained nodes (`Literal`, `TypeRef`, `NameRef`, `StringLit`, `Comment`, `Nested`, etc.). Format strings are parsed at build time and immediately converted to `CodeNode` nodes. Each node is self-contained: `TypeRef(TypeName)` carries its type reference directly, with no separate arg-index lookup.

CodeBlocks are immutable after construction. The builder (`CodeBlockBuilder`) validates argument counts and indent balance before producing a block.

### Layer 4: Spec Layer

`src/spec/` contains structural builders that emit `Vec<CodeBlock>`. TypeSpec emits one or two blocks depending on `methods_inside_type_body()`. FunSpec emits one block. FileSpec orchestrates the full rendering pipeline.

The key design decision: specs emit CodeBlocks, never raw strings. This means the renderer and import system never need to change when new spec types are added. A new `WidgetSpec` would just emit CodeBlocks with `%T` references, and imports would work automatically.

## Three-Pass Rendering Pipeline

`FileSpec::render(width)` drives everything. It runs three passes over the file's members.

### Pass 0: Materialize

Specs are converted to CodeBlocks:
- `FileMember::Type(TypeSpec)` calls `type_spec.emit(&lang)` -> `Vec<CodeBlock>`
- `FileMember::Fun(FunSpec)` calls `fun_spec.emit(&lang, ctx)` -> `CodeBlock`
- `FileMember::Code(CodeBlock)` passes through unchanged
- `FileMember::RawContent(String)` passes through as-is

After this phase, everything is either a CodeBlock or raw content.

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

Go's `qualify_import_name()` adds another layer: instead of importing `Server` directly, it renders as `http.Server` in code, with a package-level import of `"net/http"`.

### Pass 2: Render

`CodeRenderer` walks each CodeBlock's `CodeNode` sequence:

| Node | Action |
|------|--------|
| `Literal(s)` | Emit string directly |
| `TypeRef(tn)` | Resolve import name via ImportGroup, emit |
| `NameRef(s)` | Emit identifier |
| `StringLit(s)` | Call `lang.render_string_literal()` |
| `InlineLiteral(s)` | Emit raw literal |
| `Nested(block)` | Recursively render the inner CodeBlock |
| `Comment(s)` | Emit with `lang.line_comment_prefix()` |
| `SoftBreak` | Pretty-print decision point |
| `Indent` / `Dedent` | Adjust indent level |
| `StatementBegin` / `StatementEnd` | Statement boundaries (`;` if applicable) |
| `Newline` | Emit newline + indent |
| `BlockOpen` / `BlockClose` | Block delimiters from `lang.block_syntax()` |
| `BlockOpenOverride(s)` | Emit custom block opener (e.g. `" where"`) |
| `BlockCloseTransition` | Close delimiter + space (for `} else {` chains) |
| `Sequence(children)` | Recursively render a sub-sequence of nodes |

**Width-aware rendering**: When a CodeBlock contains `SoftBreak` nodes, the renderer builds a `pretty::BoxDoc` tree (Send + Sync) via `nodes_to_doc` instead of doing direct string concatenation. The Wadler-Lindig algorithm then decides at each `SoftBreak` point whether to insert a line break or a space, based on the target width. CodeBlocks without `SoftBreak` use the simpler direct-concat path for efficiency.

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

## Language-Agnostic Types

All public types (`CodeBlock`, `TypeName`, `TypeSpec`, `FunSpec`, etc.) are language-agnostic. The language is supplied at render time via `&dyn CodeLang`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let user = TypeName::importable_type("./models", "User");
let mut cb = CodeBlock::builder();
cb.add("const u: %T = getUser()", (user,));
let block = cb.build().unwrap();
// Render for any language:
let output_ts = FileSpec::builder("user.ts")
    .add_code(block.clone())
    .build()
    .unwrap()
    .render(80)
    .unwrap();

let output_rs = FileSpec::builder("user.rs")
    .add_code(block)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

`FileSpec::builder("user.ts")` auto-detects the language from the file extension. Use `FileSpec::builder_with("user.ts", TypeScript::new())` for explicit control.
