# C Cookbook

Practical, copy-paste-ready recipes for C code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Typedef

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
    .extends(TypeName::primitive("double"))
    .build()
    .unwrap();
```

```c
typedef double Meters;
```

## Struct with fields

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Config", TypeKind::Struct)
    .doc("Application configuration.")
    .add_field(
        FieldSpec::builder("timeout", TypeName::primitive("int"))
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("name", TypeName::primitive("char*"))
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("verbose", TypeName::primitive("int"))
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let file = FileSpec::builder("config.h")
    .header(CodeBlock::of("#pragma once", ()).unwrap())
    .add_type(type_spec)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```c
#pragma once

/* Application configuration. */
struct Config {
    int timeout;
    char* name;
    int verbose;
};
```

## Function declaration

```rust,ignore
use sigil_stitch::prelude::*;

let fun = FunSpec::builder("process")
    .add_param(ParameterSpec::new("data", TypeName::primitive("const char*")).unwrap())
    .add_param(ParameterSpec::new("len", TypeName::primitive("size_t")).unwrap())
    .returns(TypeName::primitive("int"))
    .build()
    .unwrap();

let file = FileSpec::builder("api.h")
    .add_function(fun)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```c
int process(const char* data, size_t len);
```

## Enum

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Direction", TypeKind::Enum)
    .doc("Cardinal directions.")
    .add_variant(EnumVariantSpec::new("UP").unwrap())
    .add_variant(EnumVariantSpec::new("DOWN").unwrap())
    .add_variant(EnumVariantSpec::new("LEFT").unwrap())
    .add_variant(EnumVariantSpec::new("RIGHT").unwrap())
    .build()
    .unwrap();
```

```c
/* Cardinal directions. */
enum Direction {
    UP,
    DOWN,
    LEFT,
    RIGHT
};
```
