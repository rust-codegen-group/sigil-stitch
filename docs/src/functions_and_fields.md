# Building Functions & Fields

Specs are builders for declaration intent. They let you work with semantic
concepts such as functions, parameters, fields, and modifiers instead of
assembling declaration grammar from raw format strings. At emit time the
selected `CodeLang` validates whether the target can represent that intent and
lowers it to structured `CodeBlock`s. The same builder can be reused across
targets that support the requested semantics; unsupported combinations fail
closed.

All spec types live in `src/spec/`. They follow a consistent builder pattern:

- **`mut self` for setters** -- owning chainable configuration methods that return `Self`
- **`self` for `.build()`** -- consumes the builder and returns `Result<Spec, SigilStitchError>`
- **Chain calls fluently** -- `Builder::new(...).method().method().build()`

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
# let body = CodeBlock::of("todo!()", ()).unwrap();
// Correct:
let fun = FunSpec::builder("greet")
    .returns(TypeName::primitive("string"))
    .body(body)
    .build()
    .unwrap();
# }
```

(`CodeBlockBuilder` is different: it uses `&mut self`, so you keep it in a `let mut` binding and call methods on it.)

Every spec type (including `CodeBlock`, `TypeName`, `FileSpec`, and `ProjectSpec`) derives `serde::Serialize` and `serde::Deserialize`, so you can round-trip specs through JSON, YAML, or any other serde format. This is useful for caching materialized specs, shipping them across process boundaries, or diffing them in tests.

## ParameterSpec

A single function parameter: name, type, optional default value, variadic flag, and property mode.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
// Simple parameter
let p = ParameterSpec::new("name", TypeName::primitive("string")).unwrap();

// Parameter with default value
let p = ParameterSpec::builder("count", TypeName::primitive("number"))
    .default_value(CodeBlock::of("0", ()).unwrap())
    .build()
    .unwrap();
// Output: count: number = 0

// Variadic parameter
let p = ParameterSpec::builder("args", TypeName::primitive("string"))
    .variadic()
    .build()
    .unwrap();
// Output: ...args: string

// Readonly property parameter (Kotlin: val name: String)
let p = ParameterSpec::builder("name", TypeName::primitive("String"))
    .is_property()
    .build()
    .unwrap();

// Mutable property parameter (Kotlin: var name: String)
let p = ParameterSpec::builder("name", TypeName::primitive("String"))
    .is_mutable_property()
    .build()
    .unwrap();
# }
```

`ParameterSpec` records parameter intent. The selected adapter may lower it as
`name: type` in TypeScript, `type name` in C, or without an annotation in Python
when the type is empty. Likewise, `is_property()` and
`is_mutable_property()` record constructor-property intent; the adapter chooses
spellings such as `val`/`var` in Kotlin or `readonly` in C#.

## FieldSpec

A struct field or class property: name, type, visibility, static/readonly flags, initializer, annotations, and doc comments.

Fields are validated and lowered as a complete ordered sequence. The selected
adapter receives a semantic context for direct emission, ordinary type members,
an ordinary variant record payload, or a closed-sum case record payload. The
payload contexts stay separate so supporting a generated closed-sum case does
not grant that shape to an ordinary enum. The adapter can therefore validate
sibling name collisions and own sequence-level grammar such as access sections
and separators. Every built-in declares explicit field capability profiles; an
unsupported context, modifier, annotation form, tag, or type requirement
returns an error before lowering.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::lang::rust::Rust;
# fn main() {
let field = FieldSpec::builder("name", TypeName::primitive("string"))
    .visibility(Visibility::Private)
    .is_readonly()
    .build()
    .unwrap();
// TypeScript: private readonly name: string;

let field = FieldSpec::builder("name", TypeName::primitive("String"))
    .visibility(Visibility::Public)
    .build()
    .unwrap();
// Rust: pub name: String,
# }
```

Fields support initializers for default values:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let field = FieldSpec::builder("count", TypeName::primitive("number"))
    .initializer(CodeBlock::of("0", ()).unwrap())
    .build()
    .unwrap();
// TypeScript: count: number = 0;
# }
```

For Go, use `.tag()` to attach struct tags:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let field = FieldSpec::builder("Name", TypeName::primitive("string"))
    .tag("json:\"name\" db:\"name\"")
    .build()
    .unwrap();
// Go: Name string `json:"name" db:"name"`
# }
```

### Optional fields

`is_optional()` marks a field whose key may be absent. This requests
`FieldCapability::OptionalPresence`, which is distinct from a present value
that can be `null` or option-like. The selected adapter must explicitly support
the capability in the current field context:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let field = FieldSpec::builder("email", TypeName::primitive("string"))
    .is_optional()
    .build()
    .unwrap();
// TypeScript:  email?: string;
// Other built-in adapters currently reject OptionalPresence rather than
// silently changing its meaning.
# }
```

Use `is_optional()` for "the key might not be there" (e.g., an OpenAPI property
not listed in `required`). Use `TypeName::optional(...)` for "the field is
present, but its value might be absent or null" at the type level:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let field = FieldSpec::builder(
    "email",
    TypeName::optional(TypeName::primitive("String")),
)
.build()
.unwrap();
// Rust:   email: Option<String>,
// Swift:  var email: String?
// Python: email: String | None
# }
```

The deprecated `OptionalFieldStyle` and `CodeLang::optional_field_style()` API
exists only so adapters written against 0.6.8 keep their frozen output through
the default compatibility lowerer. New adapters must use field capabilities,
`TypeName::Optional`, and complete `lower_fields()` implementations instead.
See [0.6.8 Legacy Compatibility and Migration](legacy_compatibility_and_migration.md)
for the compatibility boundary and adapter migration sequence.

## FunSpec

A function or method: parameters, return type, body, modifiers (async, static, abstract, constructor, override), type parameters, annotations, and doc comments.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
let body = CodeBlock::of("return this.name", ()).unwrap();

let fun = FunSpec::builder("getName")
    .returns(TypeName::primitive("string"))
    .body(body)
    .build()
    .unwrap();
// function getName(): string {
//     return this.name
// }
# }
```

### Async methods

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let body = CodeBlock::of("return await db.find(id)", ()).unwrap();
let fun = FunSpec::builder("fetchUser")
    .is_async()
    .visibility(Visibility::Public)
    .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
    .returns(TypeName::generic(
        TypeName::primitive("Promise"),
        vec![TypeName::primitive("User")],
    ))
    .body(body)
    .build()
    .unwrap();
// public async fetchUser(id: string): Promise<User> {
//     return await db.find(id)
// }
# }
```

### Type parameters

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let tp = TypeParamSpec::new("T")
    .with_bound(TypeName::primitive("Serializable"));

let body = CodeBlock::of("return JSON.stringify(value)", ()).unwrap();
let fun = FunSpec::builder("serialize")
    .add_type_param(tp)
    .add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap())
    .returns(TypeName::primitive("string"))
    .body(body)
    .build()
    .unwrap();
// function serialize<T extends Serializable>(value: T): string {
//     return JSON.stringify(value)
// }
# }
```

### Abstract methods

When no body is provided, the function renders as a declaration. Combined with `is_abstract()`, this produces abstract method signatures:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let fun = FunSpec::builder("validate")
    .is_abstract()
    .returns(TypeName::primitive("boolean"))
    .build()
    .unwrap();
// abstract validate(): boolean;
# }
```

### Constructor delegation

Use `.delegation()` to provide a `super(...)` or `this(...)` delegation payload.
The selected adapter owns its placement: TypeScript, Java, Dart, and Swift put
it first in the body, while Kotlin places it after the parameter list.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let body = CodeBlock::of("this.name = name", ()).unwrap();
let fun = FunSpec::builder("constructor")
    .is_constructor()
    .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
    .delegation(CodeBlock::of("super(name)", ()).unwrap())
    .body(body)
    .build()
    .unwrap();
// constructor(name: string) {
//     super(name);
//     this.name = name
// }
# }
```
