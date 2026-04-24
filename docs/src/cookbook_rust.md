# Rust Cookbook

Practical, copy-paste-ready recipes for Rust code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Struct with impl

```rust,ignore
use sigil_stitch::prelude::*;

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
```

```rust
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

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;

let type_spec = TypeSpec::builder("Expr", TypeKind::Enum)
    .visibility(Visibility::Public)
    .add_variant(EnumVariantSpec::new("Nil").unwrap())
    .add_variant(
        EnumVariantSpec::builder("Literal")
            .associated_type(TypeName::primitive("i64"))
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("Binary")
            .add_field(FieldSpec::builder("left", TypeName::primitive("Box<Expr>")).build().unwrap())
            .add_field(FieldSpec::builder("op", TypeName::primitive("Op")).build().unwrap())
            .add_field(FieldSpec::builder("right", TypeName::primitive("Box<Expr>")).build().unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```rust
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

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("f64"))
    .build()
    .unwrap();
```

```rust
pub struct Meters(f64);
```
