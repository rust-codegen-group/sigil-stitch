# Kotlin Cookbook

Practical, copy-paste-ready recipes for Kotlin code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Data class

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("User", TypeKind::Class)
    .visibility(Visibility::Public)
    .add_modifier("data")
    .add_field(FieldSpec::builder("name", TypeName::primitive("String")).build().unwrap())
    .add_field(FieldSpec::builder("email", TypeName::primitive("String")).build().unwrap())
    .build()
    .unwrap();
```

```kotlin
data class User(
    val name: String,
    val email: String,
)
```
