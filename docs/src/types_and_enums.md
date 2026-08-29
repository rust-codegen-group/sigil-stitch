# Building Types & Enums

This chapter covers type declarations (classes, structs, interfaces, enums, type aliases, newtypes), computed properties, annotations, and enum variants. These specs follow the same builder pattern described in [Building Functions & Fields](functions_and_fields.md): `mut self` for setters that return `Self`, `self` for `.build()`, and fluent chaining: `Builder::new(...).method().method().build()`.

## TypeSpec

The largest spec. Models type declarations: struct, class, interface, trait,
enum, type alias, or newtype wrapper. Takes a `TypeKind` to select the semantic
declaration kind. At emission, sigil-stitch validates the complete type and its
children, constructs `ValidatedType`, and delegates the entire declaration to
the selected adapter's `lower_type()` implementation.

`.build()` returns `Err(SigilStitchError::DuplicateFieldName { type_name, field_name })` when two fields in the same type share a name.

### Single-block output (TypeScript class)

The TypeScript adapter lowers a class and its members into one `CodeBlock`:

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

The Rust adapter lowers a struct with methods into two `CodeBlock`s: one for the
data definition and one for the `impl` block:

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

The split is target grammar owned by the adapter. The `TypeSpec` records the
same declaration intent without describing whether members are nested in the
type or emitted in a separate implementation block.

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

Keep nominal inheritance in `.extends()` and implemented contracts in
`.implements()` even when the target writes both in one punctuation-delimited
list. Single-inheritance adapters reject a second nominal superclass instead
of silently reinterpreting or dropping it.

Kotlin initializes a superclass in the type header with a zero-argument call
when the declaration has an implicit or explicit primary constructor, so
`.extends(BaseService)` becomes `: BaseService()`. A class with only secondary
constructors keeps the bare superclass in the header and each secondary
constructor must provide a `this(...)` or `super(...)` delegation. Superclass
constructor arguments for a primary constructor are not part of the current
semantic vocabulary; use a target-local declaration when a nonzero-argument
header call is required.

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

The Go adapter renders embedded types before regular fields. If an embedded
type is `TypeName::importable(...)`, its import is tracked automatically via
`%T`. Go interfaces use the same semantic input for interface composition:

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

Go is currently the built-in adapter that advertises structural embedding.
Python, Rust, and TypeScript reject this capability because their previous
generic output was invalid or did not preserve composition semantics. Use a
nominal supertype, implemented contract, named field, or explicit target-local
member instead.

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

Each language adapter owns the complete type-alias form:
- TypeScript/Rust: `type Foo = Bar;`
- C++: `using Foo = Bar;`
- C: `typedef Bar Foo;`
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

Newtype syntax varies across languages and is owned by each adapter's
declaration lowering. Lowering preserves the inner `TypeName` as a structured
reference, so imports and aliases work inside newtype declarations just as they
do in ordinary `%T` slots:

- Rust: `struct Meters(f64);` (tuple struct)
- Go: `type Meters float64` (distinct type)
- Kotlin: `value class Meters(val value: f64)` (inline class)
- Python: `Meters = NewType("Meters", float)` (typing.NewType)

Rust, Go, Haskell, Kotlin, and Scala adapters emit supported type parameters
and bounds. C, PHP, and Python reject generic newtype intent because their
supported wrapper forms do not preserve declaration-site generic parameters.

### Primary constructors

Kotlin and Scala accept primary-constructor parameters on the type declaration.
Pass the identifier as the parameter name and use semantic promotion flags:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("User", TypeKind::Struct)
    .add_primary_constructor_param(
        ParameterSpec::builder("name", TypeName::primitive("String"))
            .is_property()
            .build()
            .unwrap(),
    )
    .add_primary_constructor_param(
        ParameterSpec::builder("age", TypeName::primitive("Int"))
            .is_mutable_property()
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
// Kotlin: data class User(val name: String, var age: Int) { ... }
# }
```

Do not put `val` or `var` in the name. Strict adapters reject such syntax in an
identifier. A Kotlin `TypeKind::Struct` is a data class, so it requires at
least one primary-constructor parameter and every such parameter must request
an immutable or mutable property. Haskell and OCaml algebraic constructor data
uses variant positional or record payloads instead; it is not modeled as a
primary constructor.

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
            .discriminant(CodeBlock::of("'UP'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("Down")
            .discriminant(CodeBlock::of("'DOWN'", ()).unwrap())
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

### Closed sums

Use `TypeSpec::closed_sum()` when the variants are a complete set of cases
rather than value-enum entries. Cases may be unit-shaped, carry positional
types, or carry named record fields. This is declaration intent: each adapter
chooses native enum, algebraic-data-type, nested sealed-hierarchy, or sibling
case syntax locally.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::spec::field_spec::FieldSpec;
# fn main() {
let outcome = TypeSpec::closed_sum("Outcome")
    .add_variant(EnumVariantSpec::new("Empty").unwrap())
    .add_variant(
        EnumVariantSpec::builder("Value")
            .positional_payload(TypeName::primitive("Payload"))
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("Failure")
            .record_payload_field(FieldSpec::of(
                "code",
                TypeName::primitive("FailureCode"),
            ))
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

assert!(outcome.is_closed_sum());
assert_eq!(outcome.kind(), TypeKind::Enum);
# }
```

`TypeSpec::builder(name, TypeKind::Enum)` remains the ordinary value-enum
entry point. Closed sums reject discriminants, deprecated variant values, enum
constructor arguments, and opaque members that could add unvalidated cases.
Wire discriminator values and serialization tags remain caller data or
annotations; they do not change which case declaration is generated.

The built-in support matrix is:

| Target | Representation | Empty sum |
|--------|----------------|-----------|
| Rust | Native enum | Native empty enum |
| Swift | Native enum | Native empty enum |
| Haskell | Data declaration | Rejected without an `EmptyDataDecls` file contract |
| OCaml | Native variant | Native `type name = |` declaration |
| Scala | Scala 3 enum | Rejected |
| Java | Sealed interface with nested singleton and record cases | Rejected |
| Kotlin | Private-constructor sealed class with nested data cases | Supported |
| Dart | Sealed root with root-qualified final sibling cases | Rejected |

Other built-ins reject closed-sum intent instead of widening it to `Object`,
`Any`, an open hierarchy, or an ordinary value enum. Root features such as
methods, contracts, attributes, and type parameters still require the
selected target's ordinary type capability. Rust, Haskell, and OCaml preserve
the supported generic forms; Scala rejects generic closed sums until every
case can preserve the root type arguments, and the other targets reject any
generic combination not present in their enum capability profile.

Calling `TypeSpec::closed_sum(name).build()` with no cases requests a named
empty sum. It is not the unit type and does not add a `TypeName::Never`
reference. A target accepts this form only when it can emit that named
uninhabited declaration exactly.

## PropertySpec

`PropertySpec` describes a computed value with read and/or write behavior. It
records the value type, accessor bodies, visibility, static intent,
documentation, and annotations without choosing a target syntax. At emission,
the selected adapter validates `PropertyIntent` against its context-specific
profile and completely lowers the accepted declaration:

- TypeScript and JavaScript emit native accessor declarations.
- Swift emits a `var` computed property, including getter-only properties.
- Kotlin emits a `val` or `var` followed directly by its indented accessors;
  there is no outer property brace.
- PHP emits `getName()` and `setName()` methods.
- Scala emits `def name` and `def name_=` methods.

Other built-ins reject the unsupported property context instead of falling
back to plausible target text. Swift and Kotlin require an explicit value type
and read accessor. Kotlin and Scala reject static property intent.
TypeScript interfaces and Swift protocols also reject `PropertySpec`: those
targets support bodyless property requirements, while this spec carries
concrete accessor bodies and never discards them.

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
// TypeScript:
// get name(): string {
//     return this._name
// }
// set name(value: string) {
//     this._name = value
// }
# }
```

`PropertySpec::emit()` retains its pre-0.6.8 direct facade and accepts a
`DeclarationContext`. Adding the property to `TypeSpec` supplies the owning
`TypeKind`, which lets the adapter reject invalid contract or declaration
contexts before lowering. External adapters written against 0.6.8 retain the
deprecated `PropertyStyle` compatibility behavior; new adapters implement
`validate_property()` and `lower_property()` instead. The complete deprecated
surface and migration paths are listed in [0.6.8 Legacy Compatibility and
Migration](legacy_compatibility_and_migration.md).

When a target lowers properties into a namespace shared with other members,
`TypeSpec` also supplies one validation-only `TypeMembersIntent` after all
per-family checks. Exact duplicate property names are rejected by the crate;
the adapter rejects names that collide only after its own lowering. PHP uses
this pass because method names are case-insensitive and generated `getName()`
or `setName()` accessors can collide with accessors from another property or
with an explicit method. TypeScript, Kotlin, Swift, and Scala use the same seam
for their own field/property namespaces; only declarations in the same
target-local namespace collide, so TypeScript private names and TypeScript and
Swift static members stay distinct from their instance counterparts.
TypeScript, Swift, and Scala also reject explicit methods that occupy the same
emitted member name in that namespace. These are separate adapter rules, not
one general namespace abstraction. This owner-wide view does not change the
per-property `PropertyIntent -> ValidatedProperty -> lower_property()` path.

## AnnotationSpec

Structured annotations that render with language-appropriate syntax. The prefix and suffix adapt automatically:

| Language       | Syntax                          |
|----------------|---------------------------------|
| Java, Kotlin, TS | `@Name(args)`                |
| Rust           | `#[name(args)]`                 |
| C++            | `[[name(args)]]`                |
| C              | `__attribute__((name(args)))`   |

Attribute support is declaration-kind specific. For example, TypeScript
decorators are accepted on class-backed declarations but rejected on
interfaces, where decorator syntax cannot be emitted.

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

Variants are validated and lowered as one owner-aware sequence through
`TypeSpec`. This lets the selected language derive first/last position, choose
valid separators, and terminate the variant section when fields or methods
follow. Direct positional emission with `VariantContext` is deprecated and is
rejected by strict built-in adapters. See [0.6.8 Legacy Compatibility and
Migration](legacy_compatibility_and_migration.md) for direct-facade and builder
replacements.

Individual enum variants. Five forms are supported:

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

### Discriminated variant

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("Up")
    .discriminant(CodeBlock::of("'UP'", ()).unwrap())
    .build()
    .unwrap();
// TypeScript: Up = 'UP',
# }
```

Use `.constructor_argument(...)` instead when an enum entry invokes its
declaring enum's constructor, as in Java or Kotlin. Discriminants, constructor
arguments, positional payload types, and record payload fields are distinct
semantic forms and cannot be combined on one variant. The deprecated
`.value(...)` builder remains only for 0.6.8 compatibility and is rejected when
the selected language cannot give it one validity-preserving meaning.

### Enum-entry constructor arguments (Java, Kotlin)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("ACTIVE")
    .constructor_argument(CodeBlock::of("\"active\"", ()).unwrap())
    .build()
    .unwrap();
// Java/Kotlin: ACTIVE("active")
# }
```

The owning enum must also declare a compatible structured constructor (or
Kotlin primary constructor). sigil-stitch checks every enum entry against the
accepted argument-count ranges of structured constructors, including overloads,
defaulted parameters, and variadic parameters. Opaque extra members remain an
escape hatch whose target-language constructor signatures cannot be inferred.

Structured variant annotations are accepted only when the adapter can preserve
declaration-metadata semantics. Ruby therefore rejects `AnnotationSpec` on enum
constants instead of rendering it as a comment; `.annotation(CodeBlock)` remains
an explicit escape hatch for target-specific Ruby code.

### Positional payload (Rust, Swift)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("Literal")
    .positional_payload(TypeName::primitive("i64"))
    .build()
    .unwrap();
// Rust: Literal(i64),

// Multi-element tuple
let variant = EnumVariantSpec::builder("Pair")
    .positional_payload(TypeName::primitive("String"))
    .positional_payload(TypeName::primitive("i32"))
    .build()
    .unwrap();
// Rust: Pair(String, i32),
# }
```

### Record payload (Rust)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# use sigil_stitch::spec::field_spec::FieldSpec;
# use sigil_stitch::lang::rust::Rust;
# use sigil_stitch::prelude::*;
# fn main() {
let variant = EnumVariantSpec::builder("Move")
    .record_payload_field(
        FieldSpec::builder("x", TypeName::primitive("i32")).build().unwrap(),
    )
    .record_payload_field(
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

Variants are added to a `TypeSpec` via `add_variant()`. The language adapter
owns their complete grammar, including separators, trailing punctuation, and
prefixes such as Swift's `case`. The pre-0.6.8 builder names
`.associated_type(...)` and `.add_field(...)` remain as deprecated aliases for
`.positional_payload(...)` and `.record_payload_field(...)`, respectively.
