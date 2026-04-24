# Scala Cookbook

Practical, copy-paste-ready recipes for Scala code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Case class

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("User", TypeKind::Struct)
    .doc("A user case class.")
    .add_primary_constructor_param(
        ParameterSpec::new("name", TypeName::primitive("String")).unwrap(),
    )
    .add_primary_constructor_param(
        ParameterSpec::new("age", TypeName::primitive("Int")).unwrap(),
    )
    .add_primary_constructor_param(
        ParameterSpec::new("email", TypeName::primitive("String")).unwrap(),
    )
    .build()
    .unwrap();
```

```scala
/**
 * A user case class.
 */
case class User(name: String, age: Int, email: String) {
}
```

## Trait with type parameter

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Repository", TypeKind::Trait)
    .add_type_param(TypeParamSpec::new("T"))
    .doc("Generic data repository.")
    .add_method(
        FunSpec::builder("findById")
            .returns(TypeName::primitive("Option[T]"))
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
    .build()
    .unwrap();
```

```scala
/**
 * Generic data repository.
 */
trait Repository[T] {
  def findById(id: String): Option[T]

  def save(entity: T)
}
```

## Enum

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;

let type_spec = TypeSpec::builder("Color", TypeKind::Enum)
    .doc("Supported colors.")
    .add_variant(EnumVariantSpec::new("Red").unwrap())
    .add_variant(EnumVariantSpec::new("Green").unwrap())
    .add_variant(EnumVariantSpec::new("Blue").unwrap())
    .build()
    .unwrap();
```

```scala
/**
 * Supported colors.
 */
enum Color {
  Red,
  Green,
  Blue
}
```

## Bounded type parameter

```rust,ignore
use sigil_stitch::prelude::*;

let body = CodeBlock::of("if (a.compareTo(b) >= 0) a else b", ()).unwrap();

let fun = FunSpec::builder("max")
    .add_type_param(
        TypeParamSpec::new("T").with_bound(TypeName::primitive("Comparable[T]")),
    )
    .returns(TypeName::primitive("T"))
    .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
    .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
    .body(body)
    .build()
    .unwrap();
```

```scala
def max[T <: Comparable[T]](a: T, b: T): T = {
  if (a.compareTo(b) >= 0) a else b
}
```

## Newtype

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .extends(TypeName::primitive("Double"))
    .build()
    .unwrap();
```

```scala
class Meters(val value: Double)
```
