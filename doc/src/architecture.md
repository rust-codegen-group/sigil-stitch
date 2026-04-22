# Architecture

This chapter describes how sigil-stitch works internally. It covers the four-layer design, the three-pass rendering pipeline, and the import resolution system.

## Four Layers

The library is organized in four layers, each building on the one below:

```
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

`src/lang/mod.rs` defines the `CodeLang` trait with 45 methods covering syntax, formatting, and import rendering. Each supported language implements this trait in its own module (`src/lang/typescript.rs`, etc.).

All types in the library are parameterized by `L: CodeLang`. This phantom type parameter prevents cross-language mixing at compile time. You can't accidentally pass a `TypeName<TypeScript>` to a `CodeBlock<RustLang>`.

The trait is `Sized + Clone + 'static` to allow language instances to be stored inside specs and cloned freely.

### Layer 2: TypeName

`src/type_name.rs` defines type references. Key variants:

| Variant | Example | Import Tracked? |
|---------|---------|-----------------|
| `Primitive` | `string`, `i32` | No |
| `Importable` | `User` from `./models` | Yes |
| `Generic` | `Promise<User>` | Recursively |
| `Array` | `User[]`, `Vec<User>` | Inner type tracked |
| `Optional` | `User?`, `Option<User>` | Inner type tracked |
| `Union` | `string \| number` | All members tracked |
| `Function` | `(x: string) => void` | Params + return tracked |
| `Map` | `Map<string, User>` | Key + value tracked |
| `Pointer` / `Slice` | `*const T`, `&[T]` | Inner type tracked |
| `Raw` | any string | No |

Every variant that contains other types recursively collects imports via `collect_imports()`. This means `Generic(Promise, [Importable(User)])` tracks the `User` import even though `Promise` is a primitive.

TypeName also renders to `pretty::BoxDoc` for width-aware output of complex type signatures. `BoxDoc` is used (rather than `RcDoc`) so rendered documents are `Send + Sync` and can cross thread boundaries.

#### Type Presentation Layer

`TypeName` variants are *semantic* — `Array(T)` means "array of T" regardless of language. Cross-language rendering is handled by a **data-driven presentation layer**:

1. Each `TypeName` variant asks the language for a `TypePresentation` — a data enum describing the syntactic pattern (e.g., `GenericWrap`, `Prefix`, `Postfix`, `Delimited`, `Infix`).
2. A single rendering engine in `type_name.rs` interprets the pattern into `BoxDoc` output.

`BoxDoc` never appears in the `CodeLang` trait. Languages return pure data; the engine does all rendering. See [Type Presentation](type_presentation.md) for the full design.

### Layer 3: CodeBlock

`src/code_block.rs` is the core composition primitive. A `CodeBlock<L>` stores:
- `parts: Vec<FormatPart>` -- parsed format specifiers
- `args: Vec<Arg<L>>` -- the corresponding arguments

Format strings are parsed at build time: `"const u: %T = getUser()"` becomes `[Literal("const u: "), Type, Literal(" = getUser()")]`. The `Type` part consumes one `Arg::TypeName` from the args vector.

CodeBlocks are immutable after construction. The builder (`CodeBlockBuilder`) validates argument counts and indent balance before producing a block.

### Layer 4: Spec Layer

`src/spec/` contains structural builders that emit `Vec<CodeBlock<L>>`. TypeSpec emits one or two blocks depending on `methods_inside_type_body()`. FunSpec emits one block. FileSpec orchestrates the full rendering pipeline.

The key design decision: specs emit CodeBlocks, never raw strings. This means the renderer and import system never need to change when new spec types are added. A new `WidgetSpec` would just emit CodeBlocks with `%T` references, and imports would work automatically.

## Three-Pass Rendering Pipeline

`FileSpec::render(width)` drives everything. It runs three passes over the file's members.

### Phase 0: Materialize

Specs are converted to CodeBlocks:
- `FileMember::Type(TypeSpec)` calls `type_spec.emit(&lang)` -> `Vec<CodeBlock<L>>`
- `FileMember::Fun(FunSpec)` calls `fun_spec.emit(&lang, ctx)` -> `CodeBlock<L>`
- `FileMember::Code(CodeBlock)` passes through unchanged
- `FileMember::RawContent(String)` passes through as-is

After this phase, everything is either a CodeBlock or raw content.

### Pass 1: Collect Imports

`import_collector` walks every CodeBlock tree. For each `Arg::TypeName` in any block, it calls `type_name.collect_imports()` to extract `ImportRef` structs (module + name + optional alias).

Nested CodeBlocks (from `%L` with `Arg::Code`) are walked recursively. `RawContentWithImports` members have their type list walked for imports even though the content itself is opaque.

### Import Resolution

`ImportGroup::resolve()` takes the collected `ImportRef` list and:

1. **Deduplicates**: Same module + same name = one import
2. **Detects conflicts**: Two different modules exporting the same name (e.g., `User` from `./models` and `User` from `./legacy`)
3. **Assigns aliases**: First-encountered `User` wins the simple name. The second gets aliased using a module-derived prefix (e.g., `LegacyUser`)
4. **Merges explicit imports**: `ImportSpec` entries (aliased, side-effect, wildcard) are merged into the resolved set

The result is an `ImportGroup` that maps each module to its resolved names with aliases.

Go's `qualify_import_name()` adds another layer: instead of importing `Server` directly, it renders as `http.Server` in code, with a package-level import of `"net/http"`.

### Pass 2: Render

`CodeRenderer` walks each CodeBlock's `FormatPart` sequence:

| Part | Action |
|------|--------|
| `Literal(s)` | Emit string directly |
| `Type` | Look up the TypeName's resolved name in ImportGroup, emit it |
| `Name` | Emit the name string |
| `StringLit` | Call `lang.render_string_literal()` |
| `Literal_` | Emit the literal or recursively render a nested CodeBlock |
| `Newline` | Emit newline + current indent |
| `Indent` | Increase indent level |
| `Dedent` | Decrease indent level |
| `StatementBegin` | Mark statement start |
| `StatementEnd` | Append `;` if `lang.uses_semicolons()` |
| `BlockOpen` | Emit `lang.block_open()` (` {` or `:`) |
| `BlockClose` | Emit `lang.block_close()` (`}` or nothing) |
| `Wrap` | Pretty-print decision point (see below) |

**Width-aware rendering**: When a CodeBlock contains `%W` (Wrap) parts, the renderer builds a `pretty::BoxDoc` tree (Send + Sync) instead of doing direct string concatenation. The Wadler-Lindig algorithm then decides at each `%W` point whether to insert a line break or a space, based on the target width. CodeBlocks without `%W` use the simpler direct-concat path for efficiency.

## Import Conflict Resolution

A concrete example of the conflict resolution:

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let user_a = TypeName::<TypeScript>::importable_type("./models", "User");
let user_b = TypeName::<TypeScript>::importable_type("./legacy", "User");

let mut cb = CodeBlock::<TypeScript>::builder();
cb.add_statement("const a: %T = getA()", (user_a,));
cb.add_statement("const b: %T = getB()", (user_b,));
let body = cb.build().unwrap();

let mut fb = FileSpec::<TypeScript>::builder("test.ts");
fb.add_code(body);
let file = fb.build().unwrap();
let output = file.render(80).unwrap();
```

The output would contain:
```typescript
import type { User } from './models'
import type { User as LegacyUser } from './legacy'

const a: User = getA();
const b: LegacyUser = getB();
```

The first `User` (from `./models`) wins the simple name. The second (from `./legacy`) gets the alias `LegacyUser`, derived from the module path.

## The Phantom Type Parameter

Every type in the library carries `L: CodeLang`:

- `CodeBlock<L>`, `CodeBlockBuilder<L>`
- `TypeName<L>`
- `FileSpec<L>`, `TypeSpec<L>`, `FunSpec<L>`, `FieldSpec<L>`, etc.
- `Arg<L>`, `ImportRef` (module-level, not parameterized, but used within `L`-parameterized contexts)

This design means the compiler rejects cross-language mistakes:

```rust,ignore
let ts_type = TypeName::<TypeScript>::primitive("string");
let mut cb = CodeBlock::<RustLang>::builder();
cb.add("let x: %T", (ts_type,));  // Compile error: TypeScript != RustLang
```

The language parameter enters at construction time (`TypeName::<TypeScript>::...`) and flows through every operation. There's no runtime check needed because the type system handles it.
