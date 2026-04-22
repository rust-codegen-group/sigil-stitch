# TypeName

`TypeName<L>` is the type reference enum at the heart of sigil-stitch's import tracking. When you use a `TypeName` with the `%T` format specifier in a `CodeBlock`, the library renders the type name in the output *and* records the import. At render time, `FileSpec` collects all recorded imports, deduplicates them, resolves naming conflicts, and emits the import header automatically.

Every `TypeName` is parameterized by `L: CodeLang`, so a `TypeName<TypeScript>` cannot be mixed with a `TypeName<Rust>`. The compiler enforces this -- no runtime checks needed.

## Import tracking

The two `Importable` constructors are the primary way to create types that generate import statements:

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

// Value import: import { User } from './models'
let user = TypeName::<TypeScript>::importable("./models", "User");

// Type-only import: import type { User } from './models'
let user = TypeName::<TypeScript>::importable_type("./models", "User");
```

When these types appear in a `CodeBlock` via `%T`, the import is tracked automatically. At file render time, all imports are collected, deduplicated, and emitted. If two modules export the same name, the first keeps the simple name and the second gets an auto-generated alias.

You can also set an explicit alias:

```rust,ignore
let user = TypeName::<TypeScript>::importable("./other", "User")
    .with_alias("OtherUser");
// import { User as OtherUser } from './other'
// Rendered as: OtherUser
```

## Primitives

Types that don't need imports -- built-in language types, type parameters, or any name that's already in scope:

```rust,ignore
let s = TypeName::<TypeScript>::primitive("string");
let n = TypeName::<TypeScript>::primitive("number");
let t = TypeName::<TypeScript>::primitive("T");  // type parameter
```

## Collections

### Arrays

```rust,ignore
// TypeScript: string[]
// Rust:       Vec<String>  (via present_array)
// Go:         []string
let arr = TypeName::<TypeScript>::array(TypeName::primitive("string"));

// TypeScript: readonly number[]
let ro = TypeName::<TypeScript>::readonly_array(TypeName::primitive("number"));
```

### Maps

```rust,ignore
// Go:         map[string]User
// TypeScript: Record<string, User>  (via present_map)
let m = TypeName::<TypeScript>::map(
    TypeName::primitive("string"),
    TypeName::importable("./models", "User"),
);
```

### Tuples

```rust,ignore
// Rust:   (String, i32)
// TS:     [string, number]
// Python: tuple[str, int]
// C++:    std::tuple<string, int>
let t = TypeName::<TypeScript>::tuple(vec![
    TypeName::primitive("string"),
    TypeName::primitive("number"),
]);

// Unit type (empty tuple): Rust ()
let unit = TypeName::<TypeScript>::unit();
```

### Slices

```rust,ignore
// Go: []User
let s = TypeName::<TypeScript>::slice(TypeName::primitive("User"));
```

## Generics

Wrap a base type with type parameters:

```rust,ignore
// TypeScript: Promise<User>
let promise = TypeName::<TypeScript>::generic(
    TypeName::primitive("Promise"),
    vec![TypeName::importable("./models", "User")],
);

// Rust: HashMap<String, Vec<User>>
let map = TypeName::<TypeScript>::generic(
    TypeName::primitive("HashMap"),
    vec![
        TypeName::primitive("String"),
        TypeName::generic(
            TypeName::primitive("Vec"),
            vec![TypeName::primitive("User")],
        ),
    ],
);
```

Nesting works to any depth. Imports are collected recursively -- every `Importable` type anywhere in the tree gets tracked.

## Union and intersection types

```rust,ignore
// TypeScript: string | number | boolean
let u = TypeName::<TypeScript>::union(vec![
    TypeName::primitive("string"),
    TypeName::primitive("number"),
    TypeName::primitive("boolean"),
]);

// TypeScript: Serializable & Loggable
let i = TypeName::<TypeScript>::intersection(vec![
    TypeName::primitive("Serializable"),
    TypeName::primitive("Loggable"),
]);
```

These are primarily useful for TypeScript. Other languages render them using their closest equivalent (e.g., Python uses `X | Y` for unions).

## Optional types

```rust,ignore
// TypeScript: string | null
// Rust:       Option<String>
// Go:         *string
// Kotlin:     String?
// Swift:      String?
let opt = TypeName::<TypeScript>::optional(TypeName::primitive("string"));
```

The rendering adapts per language through `present_optional()` in the `CodeLang` trait.

## Pointer and reference types

```rust,ignore
// Go: *User
let ptr = TypeName::<TypeScript>::pointer(TypeName::primitive("User"));

// Rust: &str
let r = TypeName::<TypeScript>::reference(TypeName::primitive("str"));

// Rust: &mut Vec<i32>
let rm = TypeName::<TypeScript>::reference_mut(TypeName::primitive("Vec<i32>"));
```

Reference rendering is language-aware:
- Rust: `&T` / `&mut T`
- C++: `const T&` / `T&`
- C: `const T*` / `T*`
- Go: shared reference is a no-op, mutable reference renders as `*T`
- TypeScript: references are a no-op (everything is by reference)

## Function types

```rust,ignore
// TypeScript: (string, number) => boolean
// Rust:       fn(String, i32) -> bool
// Python:     Callable[[str, int], bool]
// C++:        std::function<bool(string, int)>
// Dart:       bool Function(String, int)
let f = TypeName::<TypeScript>::function(
    vec![TypeName::primitive("string"), TypeName::primitive("number")],
    TypeName::primitive("boolean"),
);
```

Function type rendering varies significantly across languages. The `present_function()` trait method returns a `FunctionPresentation` struct that controls keyword, delimiters, arrow syntax, parameter order, and optional outer wrappers.

## Raw escape hatch

For type expressions not covered by the built-in variants:

```rust,ignore
let t = TypeName::<TypeScript>::raw("keyof User");
```

`Raw` emits the string verbatim with no import tracking. Use it sparingly -- prefer the structured variants when possible.

## Cross-language rendering

The same `TypeName` variant renders differently per language. This is powered by the `TypePresentation` system -- each language returns a rendering pattern (prefix, postfix, surround, delimited, generic-wrap, or infix) for each type construct, and the rendering engine in `type_name.rs` interprets the pattern into formatted output. Language implementations never build `BoxDoc` directly.

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

```rust,ignore
// Check if a type renders to empty string (used internally by ParameterSpec)
let empty = TypeName::<TypeScript>::primitive("");
assert!(empty.is_empty());

// Get the simple name (for import resolution lookups)
let t = TypeName::<TypeScript>::importable("./models", "User");
assert_eq!(t.simple_name(), Some("User"));
```
