# TypeName

`TypeName` is the type reference enum at the heart of sigil-stitch's import tracking. When you use a `TypeName` with the `%T` format specifier in a `CodeBlock`, the library renders the type name in the output *and* records the import. At render time, `FileSpec` collects all recorded imports, deduplicates them, resolves naming conflicts, and emits the import header automatically.

`TypeName` is language-agnostic — it carries no generic parameter. The same `TypeName` value can be rendered for any target language at `FileSpec::render()` time.

Public type rendering is always language-aware. For normal generation, place a
`TypeName` in a `CodeBlock` `%T` slot and let `FileSpec` collect imports, resolve
aliases, and choose the target syntax. `TypeName::to_doc_with_lang()` is for
custom final renderers that already have a target language and an alias
resolver. Language-neutral `TypeName::render()` and `TypeName::to_doc()`
shortcuts are not exposed because they encourage type references to be
flattened before import resolution.

## Import tracking

The two `Importable` constructors are the primary way to create types that generate import statements:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
// Value import: import { User } from './models'
let user = TypeName::importable("./models", "User");

// Type-only import: import type { User } from './models'
let user = TypeName::importable_type("./models", "User");
# }
```

When these types appear in a `CodeBlock` via `%T`, the import is tracked automatically. At file render time, all imports are collected, deduplicated, and emitted. If two modules export the same name, the first keeps the simple name and the second gets an auto-generated alias.

You can also set an explicit alias:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let user = TypeName::importable("./other", "User")
    .with_alias("OtherUser");
// import { User as OtherUser } from './other'
// Rendered as: OtherUser
# }
```

## Primitives

Types that don't need imports -- built-in language types, type parameters, or any name that's already in scope:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let s = TypeName::primitive("string");
let n = TypeName::primitive("number");
let t = TypeName::primitive("T");  // type parameter
# }
```

## Qualified types

For types that should render with their full module path inline *without* generating an import statement:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Rust: serde_json::Value  (no `use serde_json::Value;`)
let val = TypeName::qualified("serde_json", "Value");

// Rust: super::Foo
let foo = TypeName::qualified("super", "Foo");

// Java: java.util.HashMap
let map = TypeName::qualified("java.util", "HashMap");
# }
```

The separator between module and name comes from `CodeLang::module_separator()` — `"::"` for Rust/C++, `"."` for Go/Python/Java/Kotlin/Scala/Swift/Dart/Haskell/OCaml. Languages without module-qualified paths (TypeScript, JavaScript, C, Bash, Zsh) silently fall back to rendering just the name.

Qualified types work anywhere a `TypeName` is accepted, including inside generics:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Rust: std::collections::HashMap<String, serde_json::Value>
let map = TypeName::generic(
    TypeName::qualified("std::collections", "HashMap"),
    vec![
        TypeName::primitive("String"),
        TypeName::qualified("serde_json", "Value"),
    ],
);
# }
```

You can also convert an existing importable type to qualified rendering with `.qualify()`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Equivalent to TypeName::qualified("serde_json", "Value")
let val = TypeName::importable("serde_json", "Value").qualify();
# }
```

## Collections

### Arrays

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// TypeScript: string[]
// Rust:       Vec<String>  (via type_presentation().array)
// Go:         []string
let arr = TypeName::array(TypeName::primitive("string"));

// TypeScript: readonly number[]
let ro = TypeName::readonly_array(TypeName::primitive("number"));
# }
```

### Maps

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Go:         map[string]User
// TypeScript: Record<string, User>  (via type_presentation().map)
let m = TypeName::map(
    TypeName::primitive("string"),
    TypeName::importable("./models", "User"),
);
# }
```

### Tuples

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Rust:   (String, i32)
// TS:     [string, number]
// Python: tuple[str, int]
// C++:    std::tuple<string, int>
let t = TypeName::tuple(vec![
    TypeName::primitive("string"),
    TypeName::primitive("number"),
]);

// Unit type (empty tuple): Rust ()
let unit = TypeName::unit();
# }
```

### Slices

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Go: []User
let s = TypeName::slice(TypeName::primitive("User"));
# }
```

## Generics

Wrap a base type with type parameters:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// TypeScript: Promise<User>
let promise = TypeName::generic(
    TypeName::primitive("Promise"),
    vec![TypeName::importable("./models", "User")],
);

// Rust: HashMap<String, Vec<User>>
let map = TypeName::generic(
    TypeName::primitive("HashMap"),
    vec![
        TypeName::primitive("String"),
        TypeName::generic(
            TypeName::primitive("Vec"),
            vec![TypeName::primitive("User")],
        ),
    ],
);
# }
```

Nesting works to any depth. Imports are collected recursively -- every `Importable` type anywhere in the tree gets tracked.

## Union and intersection types

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// TypeScript: string | number | boolean
let u = TypeName::union(vec![
    TypeName::primitive("string"),
    TypeName::primitive("number"),
    TypeName::primitive("boolean"),
]);

// TypeScript: Serializable & Loggable
let i = TypeName::intersection(vec![
    TypeName::primitive("Serializable"),
    TypeName::primitive("Loggable"),
]);
# }
```

These are primarily useful for TypeScript. Other languages render them using their closest equivalent (e.g., Python uses `X | Y` for unions).

## Optional types

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// TypeScript: string | null
// Rust:       Option<String>
// Go:         *string
// Kotlin:     String?
// Swift:      String?
let opt = TypeName::optional(TypeName::primitive("string"));
# }
```

The rendering adapts per language through the `optional` field in `lang.type_presentation()`.

## Pointer and reference types

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Go: *User
let ptr = TypeName::pointer(TypeName::primitive("User"));

// Rust: &str
let r = TypeName::reference(TypeName::primitive("str"));

// Rust: &mut Vec<i32>
let rm = TypeName::reference_mut(TypeName::primitive("Vec<i32>"));
# }
```

Reference rendering is language-aware:
- Rust: `&T` / `&mut T`
- C++: `const T&` / `T&`
- C: `const T*` / `T*`
- Go: shared reference is a no-op, mutable reference renders as `*T`
- TypeScript: references are a no-op (everything is by reference)

## Function types

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// TypeScript: (string, number) => boolean
// Rust:       fn(String, i32) -> bool
// Python:     Callable[[str, int], bool]
// C++:        std::function<bool(string, int)>
// Dart:       bool Function(String, int)
let f = TypeName::function(
    vec![TypeName::primitive("string"), TypeName::primitive("number")],
    TypeName::primitive("boolean"),
);
# }
```

Function type rendering varies significantly across languages. The `function` field in `lang.type_presentation()` returns a `FunctionPresentation` struct that controls keyword, delimiters, arrow syntax, parameter order, and optional outer wrappers.

## Raw escape hatch

For type expressions not covered by the built-in variants:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let t = TypeName::raw("keyof User");
# }
```

`Raw` emits the string verbatim with no import tracking. Use it sparingly -- prefer the structured variants when possible.

## Cross-language rendering

The same `TypeName` variant renders differently per language. This is powered by the `TypePresentation` system -- each language returns a rendering pattern (prefix, postfix, surround, delimited, generic-wrap, or infix) for each type construct, and the rendering engine in `type_name_render.rs` interprets the pattern into formatted output. Language implementations never build `BoxDoc` directly.

| TypeName | TypeScript | Rust | Go | C++ |
|----------|-----------|------|-----|-----|
| `array(T)` | `T[]` | `Vec<T>` | `[]T` | `std::vector<T>` |
| `optional(T)` | `T \| null` | `Option<T>` | `*T` | `std::optional<T>` |
| `tuple(A, B)` | `[A, B]` | `(A, B)` | n/a | `std::tuple<A, B>` |
| `reference(T)` | `T` | `&T` | `T` | `const T&` |
| `reference_mut(T)` | `T` | `&mut T` | `*T` | `T&` |
| `map(K, V)` | `Record<K, V>` | `HashMap<K, V>` | `map[K]V` | `std::map<K, V>` |
| `function(A) -> R` | `(A) => R` | `fn(A) -> R` | `func(A) R` | `std::function<R(A)>` |

See [Type Presentation](type_presentation.md) for the full technical details of how this rendering system works.

## Inspection methods

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Check if a type renders to empty string (used internally by ParameterSpec)
let empty = TypeName::primitive("");
assert!(empty.is_empty());

// Get the simple name (for import resolution lookups)
let t = TypeName::importable("./models", "User");
assert_eq!(t.simple_name(), Some("User"));
# }
```
