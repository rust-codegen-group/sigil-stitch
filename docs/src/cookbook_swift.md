# Swift Cookbook

Practical, copy-paste-ready recipes for Swift code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Struct with protocol conformance

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Point", TypeKind::Struct)
    .implements(TypeName::primitive("Codable"))
    .add_field(FieldSpec::builder("x", TypeName::primitive("Double")).build().unwrap())
    .add_field(FieldSpec::builder("y", TypeName::primitive("Double")).build().unwrap())
    .build()
    .unwrap();
```

```swift
struct Point: Codable {
    var x: Double
    var y: Double
}
```

## Enum

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Color", TypeKind::Enum)
    .visibility(Visibility::Public)
    .doc("Supported colors.")
    .add_variant(EnumVariantSpec::new("red").unwrap())
    .add_variant(EnumVariantSpec::new("green").unwrap())
    .add_variant(EnumVariantSpec::new("blue").unwrap())
    .build()
    .unwrap();
```

```swift
/// Supported colors.
public enum Color {
    case red
    case green
    case blue
}
```

## Enum with associated values

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("NetworkResult", TypeKind::Enum)
    .visibility(Visibility::Public)
    .doc("Result of a network request.")
    .add_variant(
        EnumVariantSpec::builder("success")
            .associated_type(TypeName::primitive("Data"))
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("failure")
            .associated_type(TypeName::primitive("Error"))
            .associated_type(TypeName::primitive("Int"))
            .build()
            .unwrap(),
    )
    .add_variant(EnumVariantSpec::new("loading").unwrap())
    .build()
    .unwrap();
```

```swift
/// Result of a network request.
public enum NetworkResult {
    case success(Data)
    case failure(Error, Int)
    case loading
}
```

## Protocol

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

```swift
/// Generic data repository.
protocol Repository<T> {
    func findById(id: String) -> T?

    func save(entity: T)

    func delete(id: String)
}
```
