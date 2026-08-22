# Rust Cookbook

Practical, copy-paste-ready recipes for Rust code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Struct with impl

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let body = CodeBlock::of("Self { name: name.into(), port }", ()).unwrap();

let type_spec = TypeSpec::builder("Config", TypeKind::Struct)
    .visibility(Visibility::Public)
    .add_field(
        FieldSpec::builder("name", TypeName::primitive("String"))
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("port", TypeName::primitive("u16"))
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("new")
            .visibility(Visibility::Public)
            .add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap())
            .add_param(ParameterSpec::new("port", TypeName::primitive("u16")).unwrap())
            .returns(TypeName::primitive("Self"))
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
# }
```

```rust,ignore
pub struct Config {
    pub name: String,
    pub port: u16,
}

impl Config {
    pub fn new(name: &str, port: u16) -> Self {
        Self { name: name.into(), port }
    }
}
```

## Enum with variants

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
# fn main() {
let type_spec = TypeSpec::builder("Expr", TypeKind::Enum)
    .visibility(Visibility::Public)
    .add_variant(EnumVariantSpec::new("Nil").unwrap())
    .add_variant(
        EnumVariantSpec::builder("Literal")
            .positional_payload(TypeName::primitive("i64"))
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("Binary")
            .record_payload_field(FieldSpec::builder("left", TypeName::primitive("Box<Expr>")).build().unwrap())
            .record_payload_field(FieldSpec::builder("op", TypeName::primitive("Op")).build().unwrap())
            .record_payload_field(FieldSpec::builder("right", TypeName::primitive("Box<Expr>")).build().unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
# }
```

```rust,ignore
pub enum Expr {
    Nil,
    Literal(i64),
    Binary {
        left: Box<Expr>,
        op: Op,
        right: Box<Expr>,
    },
}
```

## Newtype

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("f64"))
    .build()
    .unwrap();
# }
```

```rust,ignore
pub struct Meters(f64);
```

## Trait

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("Summary", TypeKind::Trait)
    .visibility(Visibility::Public)
    .add_method(
        FunSpec::builder("summarize")
            .add_param(ParameterSpec::new("&self", TypeName::primitive("")).unwrap())
            .returns(TypeName::primitive("String"))
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("preview")
            .add_param(ParameterSpec::new("&self", TypeName::primitive("")).unwrap())
            .returns(TypeName::primitive("String"))
            .body(CodeBlock::of("self.summarize()[..50].to_string()", ()).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
# }
```

```rust,ignore
pub trait Summary {
    fn summarize(&self) -> String;

    fn preview(&self) -> String {
        self.summarize()[..50].to_string()
    }
}
```

## Type alias

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
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

```rust,ignore
pub type Result<T> = std::result::Result<T, MyError>;
```

## Qualified paths (no import)

Use `TypeName::qualified()` to render types with their full module path inline without generating a `use` statement:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let field = FieldSpec::builder("data", TypeName::qualified("serde_json", "Value"))
    .visibility(Visibility::Public)
    .build()
    .unwrap();

// In a generic:
let map_type = TypeName::generic(
    TypeName::qualified("std::collections", "HashMap"),
    vec![
        TypeName::primitive("String"),
        TypeName::qualified("serde_json", "Value"),
    ],
);
# }
```

```rust,ignore
pub data: serde_json::Value

std::collections::HashMap<String, serde_json::Value>
```
