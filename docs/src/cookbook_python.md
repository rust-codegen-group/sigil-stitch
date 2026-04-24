# Python Cookbook

Practical, copy-paste-ready recipes for Python code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Function with type hints

```rust,ignore
use sigil_stitch::prelude::*;

let user_type = TypeName::importable("models", "User");

let body = CodeBlock::of("return await db.query(User).filter(active=True)", ()).unwrap();

let fun = FunSpec::builder("get_active_users")
    .is_async()
    .add_param(ParameterSpec::new("db", TypeName::primitive("Database")).unwrap())
    .returns(TypeName::generic(
        TypeName::primitive("list"),
        vec![user_type],
    ))
    .body(body)
    .build()
    .unwrap();
```

```python
async def get_active_users(db: Database) -> list[User]:
    return await db.query(User).filter(active=True)
```

## Type alias

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("UserId", TypeKind::TypeAlias)
    .extends(TypeName::primitive("str"))
    .build()
    .unwrap();
```

```python
type UserId = str
```

## Class with bases

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("AdminService", TypeKind::Class)
    .extends(TypeName::primitive("BaseService"))
    .implements(TypeName::primitive("Authenticatable"))
    .add_method(
        FunSpec::builder("is_admin")
            .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
            .returns(TypeName::primitive("bool"))
            .body(CodeBlock::of("return True", ()).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```python
class AdminService(BaseService, Authenticatable):
    def is_admin(self) -> bool:
        return True
```

## Dataclass

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Config", TypeKind::Class)
    .doc("Application configuration.")
    .annotation(CodeBlock::of("@dataclass", ()).unwrap())
    .add_field(
        FieldSpec::builder("name", TypeName::primitive("str"))
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("port", TypeName::primitive("int"))
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("debug", TypeName::primitive("bool"))
            .initializer(CodeBlock::of("False", ()).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```python
@dataclass
class Config:
    """Application configuration."""
    name: str
    port: int
    debug: bool = False
```

## Enum

```rust,ignore
use sigil_stitch::prelude::*;

let enum_base = TypeName::importable("enum", "Enum");

let type_spec = TypeSpec::builder("Direction", TypeKind::Enum)
    .extends(enum_base)
    .add_variant(
        EnumVariantSpec::builder("UP")
            .value(CodeBlock::of("'UP'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("DOWN")
            .value(CodeBlock::of("'DOWN'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("LEFT")
            .value(CodeBlock::of("'LEFT'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("RIGHT")
            .value(CodeBlock::of("'RIGHT'", ()).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let file = FileSpec::builder("direction.py")
    .add_type(type_spec)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```python
from enum import Enum

class Direction(Enum):
    UP = 'UP'
    DOWN = 'DOWN'
    LEFT = 'LEFT'
    RIGHT = 'RIGHT'
```
