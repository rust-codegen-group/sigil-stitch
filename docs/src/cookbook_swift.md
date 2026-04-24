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
