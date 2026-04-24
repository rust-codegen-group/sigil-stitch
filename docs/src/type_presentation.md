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
| `Tuple(A, B)` | `[A, B]` | `(A, B)` | n/a | `tuple[A, B]` | `std::tuple<A, B>` |
| `Reference(T)` | (identity) | `&T` | (identity) | (identity) | `const T&` |
| `Reference(T, mut)` | (identity) | `&mut T` | `*T` | (identity) | `T&` |

Each variant needs language-specific rendering, but the rendering follows a small set of structural patterns. Rather than writing per-language rendering code for every variant, we identify these patterns and let languages declare which pattern to use.

## Architecture

```
              ┌──────────────┐
              │   TypeName   │  Semantic type algebra
              │  (unchanged) │  Array, Optional, Map, ...
              └──────┬───────┘
                     │ to_doc_with_lang(resolve, lang)
                     ▼
          ┌──────────────────────────────┐
          │  lang.type_presentation()    │  CodeLang returns TypePresentationConfig (DATA)
          └──────────────┬───────────────┘
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
    /// `name<P1, P2>` — delimiters from generic_syntax().open/.close.
    /// Vec<T>, Option<T>, HashMap<K,V>, List<T>.
    GenericWrap { name: &'a str },

    /// `prefix inner` — *T, &T, []T, &mut T.
    Prefix { prefix: &'a str },

    /// `inner suffix` — T[], T?, T*.
    Postfix { suffix: &'a str },

    /// `prefix inner suffix` — const T&, const T*.
    Surround { prefix: &'a str, suffix: &'a str },

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

Six patterns cover every type rendering need across all supported languages. A language implementation never builds `BoxDoc` — it returns one of these variants with the appropriate strings filled in.

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

## CodeLang Trait Method

Languages declare their type syntax by returning a `TypePresentationConfig` from a single method on the `CodeLang` trait:

```rust,ignore
trait CodeLang {
    fn type_presentation(&self) -> TypePresentationConfig<'_>;
}
```

`TypePresentationConfig` bundles every type-rendering decision into one struct — never `BoxDoc`:

```rust,ignore
pub struct TypePresentationConfig<'a> {
    pub array: TypePresentation<'a>,
    pub readonly_array: Option<TypePresentation<'a>>,
    pub optional: TypePresentation<'a>,
    pub optional_absent_literal: &'a str,
    pub map: TypePresentation<'a>,
    pub union: TypePresentation<'a>,
    pub intersection: TypePresentation<'a>,
    pub pointer: TypePresentation<'a>,
    pub slice: TypePresentation<'a>,
    pub tuple: TypePresentation<'a>,
    pub reference: TypePresentation<'a>,
    pub reference_mut: TypePresentation<'a>,
    pub function: FunctionPresentation<'a>,
    pub associated_type: AssociatedTypeStyle<'a>,
    pub impl_trait: BoundsPresentation<'a>,
    pub dyn_trait: BoundsPresentation<'a>,
    pub wildcard: WildcardPresentation<'a>,
}
```

Every field has a sensible default via `Default::default()`. TypeScript needs almost no overrides. Most languages override 3–5 fields with struct-update syntax (`..Default::default()`).

## Rendering Engine

A single private function in `type_name.rs` interprets presentations:

```rust,ignore
fn render_presentation(
    pres: &TypePresentation<'_>,
    inner_docs: Vec<BoxDoc<'static, ()>>,
    gs: &GenericSyntaxConfig<'_>,
) -> BoxDoc<'static, ()> {
    match pres {
        TypePresentation::GenericWrap { name } => {
            // name<P1, P2> using lang.generic_syntax().open / .close
        }
        TypePresentation::Prefix { prefix } => {
            // prefix inner
        }
        TypePresentation::Postfix { suffix } => {
            // inner suffix
        }
        TypePresentation::Surround { prefix, suffix } => {
            // prefix inner suffix
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
    let tp = lang.type_presentation();
    let gs = lang.generic_syntax();
    render_presentation(&tp.array, vec![inner_doc], &gs)
}
```

## Per-Language Examples

### TypeScript

TypeScript overrides five fields from the defaults:

```rust,ignore
fn type_presentation(&self) -> TypePresentationConfig<'_> {
    TypePresentationConfig {
        map: TypePresentation::GenericWrap { name: "Record" },
        tuple: TypePresentation::Delimited { open: "[", sep: ", ", close: "]" },
        associated_type: AssociatedTypeStyle::IndexAccess { open: "[\"", close: "\"]" },
        impl_trait: BoundsPresentation { keyword: "", separator: " & " },
        wildcard: WildcardPresentation { unbounded: "unknown", .. },
        ..Default::default()
    }
}
```

The remaining fields use defaults: `Array` → `Postfix { suffix: "[]" }`, `Optional` → `Infix { sep: " | " }` with `optional_absent_literal` set to `"null"`.

### Rust

```rust,ignore
fn type_presentation(&self) -> TypePresentationConfig<'_> {
    TypePresentationConfig {
        array: TypePresentation::GenericWrap { name: "Vec" },
        optional: TypePresentation::GenericWrap { name: "Option" },
        map: TypePresentation::GenericWrap { name: "HashMap" },
        intersection: TypePresentation::Infix { sep: " + " },
        pointer: TypePresentation::Prefix { prefix: "*const " },
        slice: TypePresentation::Delimited { open: "&[", sep: "", close: "]" },
        reference: TypePresentation::Prefix { prefix: "&" },
        reference_mut: TypePresentation::Prefix { prefix: "&mut " },
        ..Default::default()
    }
}
```

### C++

```rust,ignore
fn type_presentation(&self) -> TypePresentationConfig<'_> {
    TypePresentationConfig {
        array: TypePresentation::GenericWrap { name: "std::vector" },
        optional: TypePresentation::GenericWrap { name: "std::optional" },
        pointer: TypePresentation::Postfix { suffix: "*" },
        reference: TypePresentation::Surround { prefix: "const ", suffix: "&" },
        reference_mut: TypePresentation::Postfix { suffix: "&" },
        tuple: TypePresentation::GenericWrap { name: "std::tuple" },
        ..Default::default()
    }
}
```

The `Surround` variant was introduced specifically for C++'s `const T&` pattern, where a type needs both a prefix and a suffix. C uses it similarly for `const T*`.

### Go

```rust,ignore
fn type_presentation(&self) -> TypePresentationConfig<'_> {
    TypePresentationConfig {
        array: TypePresentation::Prefix { prefix: "[]" },
        map: TypePresentation::Delimited { open: "map[", sep: "]", close: "" },
        ..Default::default()
    }
}
```

Note that `GenericWrap` reuses `generic_syntax().open`/`.close`, so Go's `List[T]` works automatically because Go already sets `generic_syntax().open` to `"["`.

### Swift

```rust,ignore
fn type_presentation(&self) -> TypePresentationConfig<'_> {
    TypePresentationConfig {
        array: TypePresentation::Delimited { open: "[", sep: "", close: "]" },
        optional: TypePresentation::Postfix { suffix: "?" },
        map: TypePresentation::Delimited { open: "[", sep: ": ", close: "]" },
        ..Default::default()
    }
}
```

## Design Properties

1. **`BoxDoc` never appears in `CodeLang`** — languages declare data, the engine renders.
2. **Adding a `TypeName` variant** requires one new field on `TypePresentationConfig`. No per-language render code needed.
3. **17 fields** on `TypePresentationConfig` replace what would otherwise be ~20+ render methods. Each override is a single struct field.
4. **One rendering engine** in `type_name.rs` handles all patterns uniformly.
5. **Semantic types are preserved** — `Array(T)` stays `Array(T)`. The language says "render Array as GenericWrap(Vec)" not "rewrite Array to Generic('Vec', [T])".
6. **`GenericWrap` reuses `generic_syntax().open`/`.close`** — languages that already configure these delimiters get correct rendering automatically.
