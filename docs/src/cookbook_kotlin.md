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

## Enum

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Color", TypeKind::Enum)
    .doc("Supported colors.")
    .add_variant(EnumVariantSpec::new("RED").unwrap())
    .add_variant(EnumVariantSpec::new("GREEN").unwrap())
    .add_variant(EnumVariantSpec::new("BLUE").unwrap())
    .build()
    .unwrap();
```

```kotlin
/**
 * Supported colors.
 */
internal enum class Color {
    RED,
    GREEN,
    BLUE
}
```

## Interface

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Repository", TypeKind::Interface)
    .add_type_param(TypeParamSpec::new("T"))
    .doc("Generic data repository.")
    .add_method(
        FunSpec::builder("findById")
            .returns(TypeName::primitive("T?"))
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("save")
            .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("delete")
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```kotlin
/**
 * Generic data repository.
 */
internal interface Repository<T> {
    internal fun findById(id: String): T?

    internal fun save(entity: T)

    internal fun delete(id: String)
}
```

## Suspend function

```rust,ignore
use sigil_stitch::prelude::*;

let user = TypeName::importable("com.example.model", "User");

let body = CodeBlock::of("return api.fetchUser(id)", ()).unwrap();

let fun = FunSpec::builder("fetchUser")
    .is_async()
    .returns(user)
    .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
    .body(body)
    .build()
    .unwrap();

let file = FileSpec::builder("Api.kt")
    .add_function(fun)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```kotlin
import com.example.model.User

internal suspend fun fetchUser(id: String): User {
    return api.fetchUser(id)
}
```
