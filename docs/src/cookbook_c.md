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
