# TypeName

This chapter describes the implemented 0.7 type-name-lowering contract.

`TypeName` is the type reference enum at the heart of sigil-stitch's import tracking. When you use a `TypeName` with the `%T` format specifier in a `CodeBlock`, the library renders the type name in the output *and* records the import. At render time, `FileSpec` collects all recorded imports, deduplicates them, resolves naming conflicts, and emits the import header automatically.

`TypeName` carries semantic type structure and has no language generic
parameter. At `FileSpec::render()` time, the selected `RendererLang` lowers one
complete type name into structured target-language output or rejects it.
`Primitive`, `Qualified`, and especially `Raw` values may still contain
target-specific names or syntax.

Public type rendering is always language-aware. For normal generation, place a
`TypeName` in a `CodeBlock` `%T` slot. `FileSpec` first applies the selected
adapter's source-tree rewrite, then lowers every type name, collects imports
from the lowered blocks, resolves aliases, and renders the target syntax with no
further rewrite or type lowering. Language-neutral rendering shortcuts are not
exposed because they would flatten type references before representability
checks and import resolution.

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

When these types appear in a `CodeBlock` via `%T`, the import is tracked
automatically. At file render time, all imports are collected, deduplicated,
and emitted. Imports requesting the same local name form a peer conflict set.
The default resolver uses encounter order only as a deterministic compatibility
tie-break; a custom fallible resolver can assign a different complete set.

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

The selected language lowerer owns the separator between module and name:
`"::"` for targets such as Rust and C++, and `"."` for targets such as Go,
Python, Java, Kotlin, Scala, Swift, Dart, Haskell, and OCaml. A language that
cannot preserve a qualified reference rejects it instead of silently dropping
the module.

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
// Rust:       Vec<String>
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
// TypeScript: Record<string, User>
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

These are primarily useful for languages with union or intersection type
syntax. Each adapter must preserve the requested meaning exactly or reject the
complete type; it cannot substitute a merely similar construct.

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

The selected language lowerer owns the complete optional-type grammar and
rejects the variant when the target has no exact representation.

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

Function type grammar varies significantly across languages. The selected
adapter owns the complete construct, including parameter order, delimiters,
arrows or keywords, wrapping, and any target-derived imports.

## String literal types

0.7 adds one focused singleton type:

```text
TypeName::StringLiteral("active".to_owned())
```

The stored string is the decoded semantic value, not source text with quotes
or escapes. Use `TypeName::string_literal(...)` when constructing one.
TypeScript lowers it to a string literal type, Python lowers it through
structured `typing.Literal`, and targets without an exact string singleton
type reject it.

Python lowers one singleton as `typing.Literal["active"]`. A non-empty direct
union containing only string singletons becomes one `typing.Literal[...]` in
the original order, including duplicate members. A mixed union or a union
nested inside another type lowers recursively through ordinary Python union
grammar; this special case does not flatten nested unions.

Several accepted values use ordinary union composition:

```text
TypeName::Union([
    TypeName::StringLiteral("active".to_owned()),
    TypeName::StringLiteral("inactive".to_owned()),
])
```

There is no separate string-enum or literal-set type. Numeric literal types
are not part of this extension.

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

## Language-owned lowering across targets

The same `TypeName` variant lowers differently per language. Each adapter
constructs a non-empty `CodeBlock` for the complete accepted type expression;
the core validates that block, collects its imports, resolves aliases, and then
uses the ordinary direct or pretty renderer. Type blocks are produced after
source rewrite and are not rewritten a second time.

| TypeName | TypeScript | Rust | Go | C++ |
|----------|-----------|------|-----|-----|
| `array(T)` | `T[]` | `Vec<T>` | `[]T` | `std::vector<T>` |
| `optional(T)` | `T \| null` | `Option<T>` | `*T` | `std::optional<T>` |
| `tuple(A, B)` | `[A, B]` | `(A, B)` | n/a | `std::tuple<A, B>` |
| `reference(T)` | `T` | `&T` | `T` | `const T&` |
| `reference_mut(T)` | `T` | `&mut T` | `*T` | `T&` |
| `map(K, V)` | `Record<K, V>` | `HashMap<K, V>` | `map[K]V` | `std::map<K, V>` |
| `function(A) -> R` | `(A) => R` | `fn(A) -> R` | `func(A) R` | `std::function<R(A)>` |

See [TypeName Validation and Lowering](type_name_lowering.md) for ownership,
output validation, compatibility, and import behavior.

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
