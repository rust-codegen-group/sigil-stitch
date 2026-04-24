# Go Cookbook

Practical, copy-paste-ready recipes for Go code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Struct with tags

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("User", TypeKind::Struct)
    .add_field(
        FieldSpec::builder("Name", TypeName::primitive("string"))
            .tag("json:\"name\" db:\"name\"")
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("Email", TypeName::primitive("string"))
            .tag("json:\"email\" db:\"email\"")
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("Age", TypeName::primitive("int"))
            .tag("json:\"age,omitempty\"")
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```go
type User struct {
    Name string `json:"name" db:"name"`
    Email string `json:"email" db:"email"`
    Age int `json:"age,omitempty"`
}
```

## Newtype (distinct type)

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .extends(TypeName::primitive("float64"))
    .build()
    .unwrap();
```

```go
type Meters float64
```
