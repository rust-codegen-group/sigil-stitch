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
