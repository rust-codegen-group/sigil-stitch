# Building Functions & Fields

Specs are structural builders that produce `Vec<CodeBlock<L>>`. They encapsulate common declaration patterns -- classes, functions, fields, enums -- so you work with named concepts instead of raw format strings. Every spec takes a `&L` language reference at emit time, which means the same builder definition renders correctly for any target language.

All spec types live in `src/spec/`. They follow a consistent builder pattern:

- **`&mut Self` for setters** -- chainable configuration methods
- **`self` for `.build()`** -- consumes the builder and returns `Result<Spec, SigilStitchError>`
- **Never chain `.build()` after setters** -- use a `let mut` binding instead

```rust,ignore
// Correct:
let mut fb = FunSpec::<TypeScript>::builder("greet");
fb.returns(TypeName::primitive("string"));
fb.body(body);
let fun = fb.build().unwrap();

// Wrong -- .build() consumes self, so you can't chain it after &mut Self setters
```

Every spec type (including `CodeBlock`, `TypeName`, `FileSpec`, and `ProjectSpec`) derives `serde::Serialize` and `serde::Deserialize`, so you can round-trip specs through JSON, YAML, or any other serde format. This is useful for caching materialized specs, shipping them across process boundaries, or diffing them in tests.

## ParameterSpec

A single function parameter: name, type, optional default value, and variadic flag.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

// Simple parameter
let p = ParameterSpec::<TypeScript>::new("name", TypeName::primitive("string")).unwrap();

// Parameter with default value
let mut pb = ParameterSpec::builder("count", TypeName::<TypeScript>::primitive("number"));
pb.default_value(CodeBlock::<TypeScript>::of("0", ()).unwrap());
let p = pb.build().unwrap();
// Output: count: number = 0

// Variadic parameter
let mut pb = ParameterSpec::builder("args", TypeName::<TypeScript>::primitive("string"));
pb.variadic();
let p = pb.build().unwrap();
// Output: ...args: string
```

`ParameterSpec` adapts to the target language. TypeScript emits `name: type`, C emits `type name`, and Python omits the type annotation when the type is empty.

## FieldSpec

A struct field or class property: name, type, visibility, static/readonly flags, initializer, annotations, and doc comments.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::lang::rust_lang::RustLang;

let mut fb = FieldSpec::builder("name", TypeName::<TypeScript>::primitive("string"));
fb.visibility(Visibility::Private);
fb.is_readonly();
let field = fb.build().unwrap();
// TypeScript: private readonly name: string;

let mut fb = FieldSpec::builder("name", TypeName::<RustLang>::primitive("String"));
fb.visibility(Visibility::Public);
let field = fb.build().unwrap();
// Rust: pub name: String,
```

Fields support initializers for default values:

```rust,ignore
let mut fb = FieldSpec::builder("count", TypeName::<TypeScript>::primitive("number"));
fb.initializer(CodeBlock::<TypeScript>::of("0", ()).unwrap());
let field = fb.build().unwrap();
// TypeScript: count: number = 0;
```

For Go, use `.tag()` to attach struct tags:

```rust,ignore
let mut fb = FieldSpec::builder("Name", TypeName::<Go>::primitive("string"));
fb.tag("json:\"name\" db:\"name\"");
let field = fb.build().unwrap();
// Go: Name string `json:"name" db:"name"`
```

### Optional fields

`is_optional()` marks a field whose key may be absent (distinct from a value that
can be `null`). Rendering is language-specific, delegated to
`CodeLang::optional_field_style()`:

```rust,ignore
let mut fb = FieldSpec::builder("email", TypeName::<TypeScript>::primitive("string"));
fb.is_optional();
let field = fb.build().unwrap();
// TypeScript:  email?: string;
// JavaScript:  email;                (marker stripped — no optionality in JS)
// Rust:        email: Option<String>,
// Go:          Email *string
// Python:      email: str | None
// Java:        Optional<String> email;   (caller must import java.util.Optional)
// Kotlin:      name: String?
// Swift:       name: String?
// Dart:        String? name;
// C:           string *email;
// C++:         std::optional<string> email;   (caller must #include <optional>)
```

Use `is_optional()` for "the key might not be there" (e.g., an OpenAPI property
not listed in `required`). Use `TypeName::optional(...)` for "the value might be
null" at the type level.

## FunSpec

A function or method: parameters, return type, body, modifiers (async, static, abstract, constructor, override), type parameters, annotations, and doc comments.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let body = CodeBlock::<TypeScript>::of("return this.name", ()).unwrap();

let mut fb = FunSpec::<TypeScript>::builder("getName");
fb.returns(TypeName::primitive("string"));
fb.body(body);
let fun = fb.build().unwrap();
// function getName(): string {
//     return this.name
// }
```

### Async methods

```rust,ignore
let mut fb = FunSpec::<TypeScript>::builder("fetchUser");
fb.is_async();
fb.visibility(Visibility::Public);
fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
fb.returns(TypeName::generic(
    TypeName::primitive("Promise"),
    vec![TypeName::primitive("User")],
));
let body = CodeBlock::<TypeScript>::of("return await db.find(id)", ()).unwrap();
fb.body(body);
let fun = fb.build().unwrap();
// public async fetchUser(id: string): Promise<User> {
//     return await db.find(id)
// }
```

### Type parameters

```rust,ignore
let tp = TypeParamSpec::<TypeScript>::new("T")
    .with_bound(TypeName::primitive("Serializable"));

let mut fb = FunSpec::<TypeScript>::builder("serialize");
fb.add_type_param(tp);
fb.add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap());
fb.returns(TypeName::primitive("string"));
let body = CodeBlock::<TypeScript>::of("return JSON.stringify(value)", ()).unwrap();
fb.body(body);
let fun = fb.build().unwrap();
// function serialize<T extends Serializable>(value: T): string {
//     return JSON.stringify(value)
// }
```

### Abstract methods

When no body is provided, the function renders as a declaration. Combined with `is_abstract()`, this produces abstract method signatures:

```rust,ignore
let mut fb = FunSpec::<TypeScript>::builder("validate");
fb.is_abstract();
fb.returns(TypeName::primitive("boolean"));
let fun = fb.build().unwrap();
// abstract validate(): boolean;
```

### Constructor delegation

Use `.delegation()` to emit `super(...)` or `this(...)` calls. The placement is language-dependent: body-style (TS, Java, Dart, Swift) emits it as the first statement; signature-style (Kotlin) emits it after the parameter list.

```rust,ignore
let mut fb = FunSpec::<TypeScript>::builder("constructor");
fb.is_constructor();
fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
fb.delegation(CodeBlock::<TypeScript>::of("super(name)", ()).unwrap());
let body = CodeBlock::<TypeScript>::of("this.name = name", ()).unwrap();
fb.body(body);
let fun = fb.build().unwrap();
// constructor(name: string) {
//     super(name);
//     this.name = name
// }
```
