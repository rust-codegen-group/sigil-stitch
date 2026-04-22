# Language Cookbook

This chapter collects practical, copy-paste-ready recipes for each supported language. Each example shows the builder calls and the rendered output. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## TypeScript

### Class with imports

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let user_type = TypeName::<TypeScript>::importable_type("./models", "User");
let repo_type = TypeName::<TypeScript>::importable("./repository", "UserRepository");

let mut tb = TypeSpec::<TypeScript>::builder("UserService", TypeKind::Class);
tb.visibility(Visibility::Public);

let mut field_b = FieldSpec::builder("repo", repo_type.clone());
field_b.visibility(Visibility::Private);
field_b.is_readonly();
tb.add_field(field_b.build().unwrap());

let body = CodeBlock::<TypeScript>::of("return this.repo.findById(id)", ()).unwrap();
let mut fb = FunSpec::builder("getUser");
fb.is_async();
fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
fb.returns(TypeName::generic(TypeName::primitive("Promise"), vec![user_type]));
fb.body(body);
tb.add_method(fb.build().unwrap());

let type_spec = tb.build().unwrap();
let mut file = FileSpec::<TypeScript>::builder("user_service.ts");
file.add_type(type_spec);
let output = file.build().unwrap().render(80).unwrap();
```

```typescript
import type { User } from './models'
import { UserRepository } from './repository'

export class UserService {
    private readonly repo: UserRepository;

    async getUser(id: string): Promise<User> {
        return this.repo.findById(id)
    }
}
```

### Interface with generics

```rust,ignore
let mut tb = TypeSpec::<TypeScript>::builder("Repository", TypeKind::Interface);
tb.visibility(Visibility::Public);
tb.add_type_param(TypeParamSpec::new("T"));

let mut find = FunSpec::builder("findById");
find.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
find.returns(TypeName::generic(TypeName::primitive("Promise"), vec![TypeName::primitive("T")]));
tb.add_method(find.build().unwrap());

let mut save = FunSpec::builder("save");
save.add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap());
save.returns(TypeName::generic(TypeName::primitive("Promise"), vec![TypeName::primitive("void")]));
tb.add_method(save.build().unwrap());

let type_spec = tb.build().unwrap();
```

```typescript
export interface Repository<T> {
    findById(id: string): Promise<T>;
    save(entity: T): Promise<void>;
}
```

### Type alias

```rust,ignore
let mut tb = TypeSpec::<TypeScript>::builder("UserId", TypeKind::TypeAlias);
tb.visibility(Visibility::Public);
tb.extends(TypeName::primitive("string"));
let type_spec = tb.build().unwrap();
```

```typescript
export type UserId = string;
```

## Rust

### Struct with impl

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::rust_lang::RustLang;

let mut tb = TypeSpec::<RustLang>::builder("Config", TypeKind::Struct);
tb.visibility(Visibility::Public);

let mut field_b = FieldSpec::builder("name", TypeName::primitive("String"));
field_b.visibility(Visibility::Public);
tb.add_field(field_b.build().unwrap());

let mut field_b = FieldSpec::builder("port", TypeName::primitive("u16"));
field_b.visibility(Visibility::Public);
tb.add_field(field_b.build().unwrap());

let body = CodeBlock::<RustLang>::of("Self { name: name.into(), port }", ()).unwrap();
let mut fb = FunSpec::builder("new");
fb.visibility(Visibility::Public);
fb.add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap());
fb.add_param(ParameterSpec::new("port", TypeName::primitive("u16")).unwrap());
fb.returns(TypeName::primitive("Self"));
fb.body(body);
tb.add_method(fb.build().unwrap());

let type_spec = tb.build().unwrap();
```

```rust
pub struct Config {
    pub name: String,
    pub port: u16,
}

impl Config {
    pub fn new(name: &str, port: u16) -> Self {
        Self { name: name.into(), port }
    }
}
```

### Enum with variants

```rust,ignore
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;

let mut tb = TypeSpec::<RustLang>::builder("Expr", TypeKind::Enum);
tb.visibility(Visibility::Public);

tb.add_variant(EnumVariantSpec::new("Nil").unwrap());

let mut v = EnumVariantSpec::builder("Literal");
v.associated_type(TypeName::primitive("i64"));
tb.add_variant(v.build().unwrap());

let mut v = EnumVariantSpec::builder("Binary");
v.add_field(FieldSpec::builder("left", TypeName::primitive("Box<Expr>")).build().unwrap());
v.add_field(FieldSpec::builder("op", TypeName::primitive("Op")).build().unwrap());
v.add_field(FieldSpec::builder("right", TypeName::primitive("Box<Expr>")).build().unwrap());
tb.add_variant(v.build().unwrap());

let type_spec = tb.build().unwrap();
```

```rust
pub enum Expr {
    Nil,
    Literal(i64),
    Binary {
        left: Box<Expr>,
        op: Op,
        right: Box<Expr>,
    },
}
```

### Newtype

```rust,ignore
let mut tb = TypeSpec::<RustLang>::builder("Meters", TypeKind::Newtype);
tb.visibility(Visibility::Public);
tb.extends(TypeName::primitive("f64"));
let type_spec = tb.build().unwrap();
```

```rust
pub struct Meters(f64);
```

## Go

### Struct with tags

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::go_lang::GoLang;

let mut tb = TypeSpec::<GoLang>::builder("User", TypeKind::Struct);

let mut fb = FieldSpec::builder("Name", TypeName::primitive("string"));
fb.tag("json:\"name\" db:\"name\"");
tb.add_field(fb.build().unwrap());

let mut fb = FieldSpec::builder("Email", TypeName::primitive("string"));
fb.tag("json:\"email\" db:\"email\"");
tb.add_field(fb.build().unwrap());

let mut fb = FieldSpec::builder("Age", TypeName::primitive("int"));
fb.tag("json:\"age,omitempty\"");
tb.add_field(fb.build().unwrap());

let type_spec = tb.build().unwrap();
```

```go
type User struct {
    Name string `json:"name" db:"name"`
    Email string `json:"email" db:"email"`
    Age int `json:"age,omitempty"`
}
```

### Newtype (distinct type)

```rust,ignore
let mut tb = TypeSpec::<GoLang>::builder("Meters", TypeKind::Newtype);
tb.extends(TypeName::primitive("float64"));
let type_spec = tb.build().unwrap();
```

```go
type Meters float64
```

## Python

### Function with type hints

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::python::Python;

let user_type = TypeName::<Python>::importable("models", "User");

let mut fb = FunSpec::<Python>::builder("get_active_users");
fb.is_async();
fb.add_param(ParameterSpec::new("db", TypeName::primitive("Database")).unwrap());
fb.returns(TypeName::generic(
    TypeName::primitive("list"),
    vec![user_type],
));
let body = CodeBlock::<Python>::of("return await db.query(User).filter(active=True)", ()).unwrap();
fb.body(body);
let fun = fb.build().unwrap();
```

```python
async def get_active_users(db: Database) -> list[User]:
    return await db.query(User).filter(active=True)
```

### Type alias

```rust,ignore
let mut tb = TypeSpec::<Python>::builder("UserId", TypeKind::TypeAlias);
tb.extends(TypeName::primitive("str"));
let type_spec = tb.build().unwrap();
```

```python
type UserId = str
```

## Java

### Class with annotations

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::lang::java_lang::JavaLang;

let mut tb = TypeSpec::<JavaLang>::builder("UserService", TypeKind::Class);
tb.visibility(Visibility::Public);

let inject = AnnotationSpec::<JavaLang>::new("Inject");

let mut field_b = FieldSpec::builder("repository", TypeName::primitive("UserRepository"));
field_b.visibility(Visibility::Private);
field_b.add_annotation(inject);
tb.add_field(field_b.build().unwrap());

let body = CodeBlock::<JavaLang>::of("return repository.findById(id)", ()).unwrap();
let mut fb = FunSpec::builder("getUser");
fb.visibility(Visibility::Public);
fb.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
fb.returns(TypeName::primitive("User"));
fb.body(body);
tb.add_method(fb.build().unwrap());

let type_spec = tb.build().unwrap();
```

```java
public class UserService {
    @Inject
    private UserRepository repository;

    public User getUser(String id) {
        return repository.findById(id);
    }
}
```

## Kotlin

### Data class

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::kotlin::Kotlin;

let mut tb = TypeSpec::<Kotlin>::builder("User", TypeKind::Class);
tb.visibility(Visibility::Public);
tb.add_modifier("data");

let mut fb = FieldSpec::builder("name", TypeName::primitive("String"));
tb.add_field(fb.build().unwrap());

let mut fb = FieldSpec::builder("email", TypeName::primitive("String"));
tb.add_field(fb.build().unwrap());

let type_spec = tb.build().unwrap();
```

```kotlin
data class User(
    val name: String,
    val email: String,
)
```

## Swift

### Struct with protocol conformance

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::swift::Swift;

let mut tb = TypeSpec::<Swift>::builder("Point", TypeKind::Struct);
tb.implements(TypeName::primitive("Codable"));

let mut fb = FieldSpec::builder("x", TypeName::primitive("Double"));
tb.add_field(fb.build().unwrap());

let mut fb = FieldSpec::builder("y", TypeName::primitive("Double"));
tb.add_field(fb.build().unwrap());

let type_spec = tb.build().unwrap();
```

```swift
struct Point: Codable {
    var x: Double
    var y: Double
}
```

## C++

### Class with template

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::cpp_lang::CppLang;

let mut tb = TypeSpec::<CppLang>::builder("Stack", TypeKind::Class);
tb.add_type_param(TypeParamSpec::new("T"));

let mut fb = FieldSpec::builder("data_", TypeName::generic(
    TypeName::primitive("std::vector"),
    vec![TypeName::primitive("T")],
));
fb.visibility(Visibility::Private);
tb.add_field(fb.build().unwrap());

let body = CodeBlock::<CppLang>::of("data_.push_back(value)", ()).unwrap();
let mut push = FunSpec::builder("push");
push.visibility(Visibility::Public);
push.add_param(ParameterSpec::new("value", TypeName::reference(TypeName::primitive("T"))).unwrap());
push.body(body);
tb.add_method(push.build().unwrap());

let type_spec = tb.build().unwrap();
```

```cpp
template <typename T>
class Stack {
private:
    std::vector<T> data_;

public:
    void push(const T& value) {
        data_.push_back(value);
    }
};
```

### Using alias (C++ type alias)

```rust,ignore
let mut tb = TypeSpec::<CppLang>::builder("StringVec", TypeKind::TypeAlias);
tb.extends(TypeName::generic(
    TypeName::primitive("std::vector"),
    vec![TypeName::primitive("std::string")],
));
let type_spec = tb.build().unwrap();
```

```cpp
using StringVec = std::vector<std::string>;
```

## C

### Typedef

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::c_lang::CLang;

let mut tb = TypeSpec::<CLang>::builder("Meters", TypeKind::TypeAlias);
tb.extends(TypeName::primitive("double"));
let type_spec = tb.build().unwrap();
```

```c
typedef double Meters;
```

## Cross-language comparison

The same logical concept -- a simple data type with two fields -- rendered across four languages from the same builder structure:

| Language   | Output |
|------------|--------|
| TypeScript | `export class Point { x: number; y: number; }` |
| Rust       | `pub struct Point { pub x: f64, pub y: f64, }` + separate `impl` block |
| Go         | `type Point struct { X float64; Y float64 }` |
| Python     | `class Point: x: float; y: float` |

The language's `CodeLang` trait controls every syntax detail: keywords, delimiters, field ordering, visibility rendering, and whether methods live inside the type body or in a separate `impl` block. You build the spec once and the language parameter does the rest.
