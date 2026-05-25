# Building Types & Enums

This chapter covers type declarations (classes, structs, interfaces, enums, type aliases, newtypes), computed properties, annotations, and enum variants. These specs follow the same builder pattern described in [Building Functions & Fields](functions_and_fields.md): `mut self` for setters that return `Self`, `self` for `.build()`, and fluent chaining: `Builder::new(...).method().method().build()`.

## TypeSpec

The largest spec. Models type declarations: struct, class, interface, trait, enum, type alias, or newtype wrapper. Takes a `TypeKind` to select the declaration form.

`.build()` returns `Err(SigilStitchError::DuplicateFieldName { type_name, field_name })` when two fields in the same type share a name.

### Single-block output (TypeScript class)

When `lang.methods_inside_type_body(kind)` returns `true`, TypeSpec emits a single CodeBlock with fields and methods inside the body:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let body = CodeBlock::of("return this.name", ()).unwrap();

let type_spec = TypeSpec::builder("UserService", TypeKind::Class)
    .visibility(Visibility::Public)
    .add_field(
        FieldSpec::builder("name", TypeName::primitive("string"))
            .visibility(Visibility::Private)
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("getName")
            .returns(TypeName::primitive("string"))
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
let blocks = type_spec.emit(&TypeScript::new()).unwrap();
// blocks.len() == 1
//
// export class UserService {
//     private name: string;
//
//     getName(): string {
//         return this.name
//     }
// }
# }
```

### Two-block output (Rust struct + impl)

When `methods_inside_type_body(kind)` returns `false` (Rust structs and enums), TypeSpec emits two separate CodeBlocks: one for the data definition, one for the `impl` block:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::rust::Rust;
# fn main() {
let body = CodeBlock::of("Self { name: name.to_string() }", ()).unwrap();

let type_spec = TypeSpec::builder("Config", TypeKind::Struct)
    .visibility(Visibility::Public)
    .add_field(
        FieldSpec::builder("name", TypeName::primitive("String"))
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("new")
            .visibility(Visibility::Public)
            .add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap())
            .returns(TypeName::primitive("Self"))
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
let blocks = type_spec.emit(&Rust::new()).unwrap();
// blocks.len() == 2
//
// Block 0:
// pub struct Config {
//     pub name: String,
// }
//
// Block 1:
// impl Config {
//     pub fn new(name: &str) -> Self {
//         Self { name: name.to_string() }
//     }
// }
# }
```

This split is the key structural decision. It is fully automatic -- you build one TypeSpec, and the language's `methods_inside_type_body()` determines whether the output is one block or two.

### Extends and implements

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("AdminService", TypeKind::Class)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("BaseService"))
    .implements(TypeName::primitive("Serializable"))
    .build()
    .unwrap();
// export class AdminService extends BaseService implements Serializable {
// }
# }
```

### Embedded types (Go struct composition)

Use `add_embedded(TypeName)` for unnamed type references inside a struct body. This models Go's embedded field pattern where a type is included by name without a field identifier:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::go::Go;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("UserAdmin", TypeKind::Struct)
    .add_embedded(TypeName::primitive("User"))
    .add_embedded(TypeName::primitive("Admin"))
    .add_field(
        FieldSpec::builder("Role", TypeName::primitive("string"))
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
// type UserAdmin struct {
//     User
//     Admin
//     Role string
// }
# }
```

Embedded types render before regular fields. If the embedded type is `TypeName::importable(...)`, its import is tracked automatically via `%T`. This works across languages — for Go interfaces, embedded types produce interface composition:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::lang::go::Go;
# use sigil_stitch::prelude::*;
# fn main() {
let io_reader = TypeName::importable("io", "Reader");
let io_writer = TypeName::importable("io", "Writer");

let type_spec = TypeSpec::builder("ReadWriter", TypeKind::Interface)
    .add_embedded(io_reader)
    .add_embedded(io_writer)
    .build()
    .unwrap();
// type ReadWriter interface {
//     io.Reader
//     io.Writer
// }
# }
```

### Type aliases

`TypeKind::TypeAlias` emits a single-line type alias declaration with no body. The aliased target is set via `.extends()` (exactly one required). No fields, methods, or variants are allowed.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::lang::rust::Rust;
# fn main() {
// TypeScript: export type UserId = string;
let type_spec = TypeSpec::builder("UserId", TypeKind::TypeAlias)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("string"))
    .build()
    .unwrap();

// Rust: pub type Meters = f64;
let type_spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("f64"))
    .build()
    .unwrap();
# }
```

Per-language rendering is controlled by `type_keyword(TypeKind::TypeAlias)`:
- TypeScript/Rust: `type Foo = Bar;`
- C++: `using Foo = Bar;`
- C: `typedef Bar Foo;` (target-first, via `type_decl_syntax().type_alias_target_first`)
- Go: `type Foo = Bar`
- Kotlin: `typealias Foo = Bar`
- Python: `type Foo = Bar`

Type aliases support type parameters:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
// Rust: pub type Result<T> = std::result::Result<T, MyError>;
let type_spec = TypeSpec::builder("Result", TypeKind::TypeAlias)
    .visibility(Visibility::Public)
    .add_type_param(TypeParamSpec::new("T"))
    .extends(TypeName::generic(
        TypeName::primitive("std::result::Result"),
        vec![TypeName::primitive("T"), TypeName::primitive("MyError")],
    ))
    .build()
    .unwrap();
# }
```

### Newtype wrappers

`TypeKind::Newtype` emits a single-line newtype wrapper. Like type aliases, the inner type is set via `.extends()` (exactly one required).

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::lang::go::Go;
# fn main() {
// Rust: pub struct Meters(f64);
let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("f64"))
    .build()
    .unwrap();

// Go: type Meters float64
let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .extends(TypeName::primitive("float64"))
    .build()
    .unwrap();
# }
```

Newtype syntax varies across languages and is controlled by `render_newtype_line()`:
- Rust: `struct Meters(f64);` (tuple struct)
- Go: `type Meters float64` (distinct type)
- Kotlin: `value class Meters(val value: f64)` (inline class)
- Python: `Meters = NewType("Meters", float)` (typing.NewType)
- C: `typedef float Meters;` (typedef)

### Enums with EnumVariantSpec

TypeSpec with `TypeKind::Enum` uses `add_variant()` instead of `add_field()`. See the [EnumVariantSpec](#enumvariantspec) section below for variant forms.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let type_spec = TypeSpec::builder("Direction", TypeKind::Enum)
    .add_variant(
        EnumVariantSpec::builder("Up")
            .value(CodeBlock::of("'UP'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("Down")
            .value(CodeBlock::of("'DOWN'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
// enum Direction {
//     Up = 'UP',
//     Down = 'DOWN',
// }
# }
```

## PropertySpec

Computed properties with getter and/or setter. Rendering depends on `lang.property_style()`:

- **Accessor** (TypeScript, JavaScript): emits separate `get name(): T { ... }` and `set name(v: T) { ... }` methods
- **Field** (Swift, Kotlin): emits a field with inline `get`/`set` blocks

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::spec::property_spec::PropertySpec;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let getter_body = CodeBlock::of("return this._name", ()).unwrap();
let setter_body = CodeBlock::of("this._name = value", ()).unwrap();

let prop = PropertySpec::builder("name", TypeName::primitive("string"))
    .getter(getter_body)
    .setter("value", setter_body)
    .build()
    .unwrap();
// TypeScript (Accessor style):
// get name(): string {
//     return this._name
// }
// set name(value: string) {
//     this._name = value
// }
# }
```

For Swift and Kotlin, the same PropertySpec renders as a field with inline body blocks instead.

## AnnotationSpec

Structured annotations that render with language-appropriate syntax. The prefix and suffix adapt automatically:

| Language       | Syntax                          |
|----------------|---------------------------------|
| Java, Kotlin, TS | `@Name(args)`                |
| Rust           | `#[name(args)]`                 |
| C++            | `[[name(args)]]`                |
| C              | `__attribute__((name(args)))`   |

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::annotation_spec::AnnotationSpec;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() {
// Simple annotation: #[allow(dead_code)]
let ann = AnnotationSpec::new("allow").arg("dead_code");

// Multiple arguments: #[cfg(test, feature = "nightly")]
let ann = AnnotationSpec::new("cfg")
    .arg("test")
    .arg("feature = \"nightly\"");

// Bulk arguments from an iterator: #[derive(Debug, Clone, Serialize)]
let ann = AnnotationSpec::new("derive")
    .args(["Debug", "Clone", "Serialize"]);
# }
```

For import-tracked annotations, use `importable()` with a `TypeName`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::annotation_spec::AnnotationSpec;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::type_name::TypeName;
# use sigil_stitch::prelude::*;
# fn main() {
let type_name = TypeName::importable("./decorators", "Component");
let ann = AnnotationSpec::importable(type_name);
// TS: @Component (with import { Component } from './decorators')
# }
```

If `AnnotationSpec` does not cover your annotation format, every builder also has an `.annotation(CodeBlock)` escape hatch that accepts a raw CodeBlock.

## EnumVariantSpec

Individual enum variants. Four forms are supported:

### Simple variant

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() {
let v = EnumVariantSpec::new("Red").unwrap();
// Rust: Red,
# }
```

### Valued variant

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("Up")
    .value(CodeBlock::of("'UP'", ()).unwrap())
    .build()
    .unwrap();
// TypeScript: Up = 'UP',
# }
```

### Tuple variant (Rust, Swift)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("Literal")
    .associated_type(TypeName::primitive("i64"))
    .build()
    .unwrap();
// Rust: Literal(i64),

// Multi-element tuple
let variant = EnumVariantSpec::builder("Pair")
    .associated_type(TypeName::primitive("String"))
    .associated_type(TypeName::primitive("i32"))
    .build()
    .unwrap();
// Rust: Pair(String, i32),
# }
```

### Struct variant (Rust)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::spec::field_spec::FieldSpec;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("Move")
    .add_field(
        FieldSpec::builder("x", TypeName::primitive("i32")).build().unwrap(),
    )
    .add_field(
        FieldSpec::builder("y", TypeName::primitive("i32")).build().unwrap(),
    )
    .build()
    .unwrap();
// Rust:
// Move {
//     x: i32,
//     y: i32,
// },
# }
```

Variants are added to a TypeSpec via `add_variant()`. The language controls separators (`enum_and_annotation().variant_separator`), trailing separators (`enum_and_annotation().variant_trailing_separator`), and prefixes (Swift's `case`).
