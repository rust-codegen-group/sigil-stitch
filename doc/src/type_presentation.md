# Type Presentation

This chapter describes how sigil-stitch renders `TypeName` variants across different languages using a data-driven presentation layer.

## The Problem

`TypeName` is a *semantic* type algebra — `Array(T)` means "array of T" regardless of target language. But the surface syntax varies widely:

| TypeName | TypeScript | Rust | Go | Python | C++ |
|----------|-----------|------|----|--------|-----|
| `Array(T)` | `T[]` | `Vec<T>` | `[]T` | `list[T]` | `std::vector<T>` |
| `Optional(T)` | `T \| null` | `Option<T>` | `*T` | `T \| None` | `std::optional<T>` |
| `Map(K, V)` | `Record<K, V>` | `HashMap<K, V>` | `map[K]V` | `dict[K, V]` | `std::map<K, V>` |
| `Pointer(T)` | n/a | `*const T` | `*T` | n/a | `T*` |

Each variant needs language-specific rendering, but the rendering follows a small set of structural patterns. Rather than writing per-language rendering code for every variant, we identify these patterns and let languages declare which pattern to use.

## Architecture

```
              ┌──────────────┐
              │   TypeName   │  Semantic type algebra
              │  (unchanged) │  Array, Optional, Map, ...
              └──────┬───────┘
                     │ to_doc_with_lang(resolve, lang)
                     ▼
          ┌─────────────────────┐
          │  lang.present_*(…)  │  CodeLang returns TypePresentation (DATA)
          └──────────┬──────────┘
                     ▼
          ┌─────────────────────┐
          │  Rendering engine   │  Single function: (TypePresentation, inner docs) → BoxDoc
          │  (one place)        │  Lives in type_name.rs, NEVER in CodeLang impls
          └──────────┬──────────┘
                     ▼
                BoxDoc output
```

The key invariant: **`BoxDoc` never appears in the `CodeLang` trait.** Languages declare data (which syntactic pattern to use). The rendering engine — a single function in `type_name.rs` — interprets that data into `BoxDoc` output.

This separates three concerns that were previously tangled:

1. **What a type means** — `TypeName` variants (semantic, language-independent)
2. **How a language spells it** — `TypePresentation` data (per-language, no rendering logic)
3. **How to assemble output** — rendering engine (one place, all patterns)

## TypePresentation

`TypePresentation` is an enum of syntactic patterns. Each variant describes a structural template for assembling already-rendered inner type docs:

```rust,ignore
pub enum TypePresentation<'a> {
    /// `name<P1, P2>` — uses generic_open()/generic_close().
    /// Vec<T>, Option<T>, HashMap<K,V>, List<T>.
    GenericWrap { name: &'a str },

    /// `prefix inner` — *T, &T, []T, &mut T.
    Prefix { prefix: &'a str },

    /// `inner suffix` — T[], T?, T*.
    Postfix { suffix: &'a str },

    /// `open P1 sep P2 sep ... close` — (A, B), [T], [K: V], dict[K, V].
    Delimited {
        open: &'a str,
        sep: &'a str,
        close: &'a str,
    },

    /// `P1 sep P2 sep ... Pn` — A | B, A & B, A + B.
    Infix { sep: &'a str },
}
```

Five patterns cover every type rendering need across all supported languages. A language implementation never builds `BoxDoc` — it returns one of these variants with the appropriate strings filled in.

## FunctionPresentation

Function types are too complex for a single `TypePresentation` variant — they have parameter lists, return types, arrows, optional keywords, and wrappers that combine in language-specific ways. They get their own struct:

```rust,ignore
pub struct FunctionPresentation<'a> {
    pub keyword: &'a str,        // "fn", "func", ""
    pub params_open: &'a str,    // "(", "Callable[["
    pub params_sep: &'a str,     // ", "
    pub params_close: &'a str,   // ")", "]]"
    pub arrow: &'a str,          // " -> ", " => ", ", "
    pub return_first: bool,      // Dart: R Function(A, B)
    pub curried: bool,           // Haskell: A -> B -> R
    pub wrapper_open: &'a str,   // C++: "std::function<"
    pub wrapper_close: &'a str,  // C++: ">"
}
```

This declaratively covers TypeScript `(A, B) => R`, Rust `fn(A, B) -> R`, Python `Callable[[A, B], R]`, C++ `std::function<R(A, B)>`, Dart `R Function(A, B)`, and Haskell `A -> B -> R` — all from a single rendering engine interpreting the data.

## CodeLang Trait Methods

Languages declare their type syntax through `present_*` methods on the `CodeLang` trait. Each method returns data — never `BoxDoc`:

```rust,ignore
trait CodeLang {
    fn present_array(&self) -> TypePresentation<'_>;
    fn present_readonly_array(&self) -> Option<TypePresentation<'_>>;
    fn present_optional(&self) -> TypePresentation<'_>;
    fn optional_absent_literal(&self) -> &str;
    fn present_map(&self) -> TypePresentation<'_>;
    fn present_union(&self) -> TypePresentation<'_>;
    fn present_intersection(&self) -> TypePresentation<'_>;
    fn present_pointer(&self) -> TypePresentation<'_>;
    fn present_slice(&self) -> TypePresentation<'_>;
    fn present_function(&self) -> FunctionPresentation<'_>;
}
```

Every method has a sensible default. TypeScript needs almost no overrides. Most languages override 3–5 methods with one-line data returns.

## Rendering Engine

A single private function in `type_name.rs` interprets presentations:

```rust,ignore
fn render_presentation(
    pres: &TypePresentation<'_>,
    inner_docs: Vec<BoxDoc<'static, ()>>,
    lang: &impl CodeLang,
) -> BoxDoc<'static, ()> {
    match pres {
        TypePresentation::GenericWrap { name } => {
            // name<P1, P2> using lang.generic_open()/generic_close()
        }
        TypePresentation::Prefix { prefix } => {
            // prefix inner
        }
        TypePresentation::Postfix { suffix } => {
            // inner suffix
        }
        TypePresentation::Delimited { open, sep, close } => {
            // open P1 sep P2 close
        }
        TypePresentation::Infix { sep } => {
            // P1 sep P2 sep P3
        }
    }
}
```

Each `TypeName` variant in `to_doc_with_lang` becomes a three-step process:

1. Recursively render inner types to `BoxDoc`
2. Ask the language for a `TypePresentation`
3. Pass both to `render_presentation`

```rust,ignore
TypeName::Array(inner) => {
    let inner_doc = inner.to_doc_with_lang(resolve, lang);
    render_presentation(&lang.present_array(), vec![inner_doc], lang)
}
```

## Per-Language Examples

### TypeScript (mostly defaults)

```rust,ignore
fn present_map(&self) -> TypePresentation<'_> {
    TypePresentation::GenericWrap { name: "Record" }
}
```

Everything else uses defaults: `Array` → `Postfix { suffix: "[]" }`, `Optional` → `Infix { sep: " | " }` with `optional_absent_literal()` returning `"null"`.

### Rust

```rust,ignore
fn present_array(&self) -> TypePresentation<'_> { GenericWrap { name: "Vec" } }
fn present_optional(&self) -> TypePresentation<'_> { GenericWrap { name: "Option" } }
fn present_map(&self) -> TypePresentation<'_> { GenericWrap { name: "HashMap" } }
fn present_intersection(&self) -> TypePresentation<'_> { Infix { sep: " + " } }
fn present_pointer(&self) -> TypePresentation<'_> { Prefix { prefix: "*const " } }
fn present_slice(&self) -> TypePresentation<'_> {
    Delimited { open: "&[", sep: "", close: "]" }
}
```

### Go

```rust,ignore
fn present_array(&self) -> TypePresentation<'_> { Prefix { prefix: "[]" } }
fn present_map(&self) -> TypePresentation<'_> {
    Delimited { open: "map[", sep: "]", close: "" }
}
```

Note that `GenericWrap` reuses `generic_open()`/`generic_close()`, so Go's `List[T]` works automatically because Go already sets `generic_open()` to `"["`.

### Swift

```rust,ignore
fn present_array(&self) -> TypePresentation<'_> {
    Delimited { open: "[", sep: "", close: "]" }
}
fn present_optional(&self) -> TypePresentation<'_> { Postfix { suffix: "?" } }
fn present_map(&self) -> TypePresentation<'_> {
    Delimited { open: "[", sep: ": ", close: "]" }
}
```

## Design Properties

1. **`BoxDoc` never appears in `CodeLang`** — languages declare data, the engine renders.
2. **Adding a `TypeName` variant** requires one new `present_*` method returning data. No per-language render code needed.
3. **~10 methods** replace what would otherwise be ~17+ render methods. Each is a 1-line return.
4. **One rendering engine** in `type_name.rs` handles all patterns uniformly.
5. **Semantic types are preserved** — `Array(T)` stays `Array(T)`. The language says "render Array as GenericWrap(Vec)" not "rewrite Array to Generic('Vec', [T])".
6. **`GenericWrap` reuses `generic_open()`/`generic_close()`** — languages that already configure these delimiters get correct rendering automatically.
