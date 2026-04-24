# Haskell Cookbook

Practical, copy-paste-ready recipes for Haskell code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Data record with deriving

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Person", TypeKind::Struct)
    .add_field(
        FieldSpec::builder("personName", TypeName::primitive("String")).build().unwrap(),
    )
    .add_field(
        FieldSpec::builder("personAge", TypeName::primitive("Int")).build().unwrap(),
    )
    .implements(TypeName::primitive("Show"))
    .implements(TypeName::primitive("Eq"))
    .build()
    .unwrap();
```

```haskell
data Person =
  Person {
    personName :: String,
    personAge :: Int,
  }
  deriving (Show, Eq)
```

## Type class

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Printable", TypeKind::Trait)
    .doc("Things that can be printed.")
    .add_method(
        FunSpec::builder("prettyPrint")
            .add_param(ParameterSpec::new("a", TypeName::primitive("a")).unwrap())
            .returns(TypeName::primitive("String"))
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```haskell
-- | Things that can be printed.
class Printable where
  prettyPrint :: a -> String
```

## Function with split signature

```rust,ignore
use sigil_stitch::prelude::*;

let body = CodeBlock::of("x + y", ()).unwrap();

let fun = FunSpec::builder("add")
    .add_param(ParameterSpec::new("x", TypeName::primitive("Int")).unwrap())
    .add_param(ParameterSpec::new("y", TypeName::primitive("Int")).unwrap())
    .returns(TypeName::primitive("Int"))
    .body(body)
    .build()
    .unwrap();
```

```haskell
add :: Int -> Int -> Int
add x y =
  x + y
```

## Newtype

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .extends(TypeName::primitive("Int"))
    .build()
    .unwrap();
```

```haskell
newtype Meters = Meters Int
```

## Type alias

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Name", TypeKind::TypeAlias)
    .extends(TypeName::primitive("String"))
    .build()
    .unwrap();
```

```haskell
type Name = String
```
