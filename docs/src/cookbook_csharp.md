# C# Cookbook

Practical, copy-paste-ready recipes for C# code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Class with XML doc

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::csharp::CSharp;

let body = CodeBlock::of("return $\"Hello, {name}!\";", ()).unwrap();

let ts = TypeSpec::builder("Greeter", TypeKind::Class)
    .visibility(Visibility::Public)
    .add_method(
        FunSpec::builder("Greet")
            .visibility(Visibility::Public)
            .returns(TypeName::primitive("string"))
            .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
            .doc("<summary>\nGreets a user by name.\n</summary>\n<param name=\"name\">The name to greet.</param>\n<returns>A greeting string.</returns>")
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let file = FileSpec::builder_with("Greeter.cs", CSharp::new())
    .add_type(ts)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```csharp
public class Greeter {
    /// <summary>
    /// Greets a user by name.
    /// </summary>
    /// <param name="name">The name to greet.</param>
    /// <returns>A greeting string.</returns>
    public string Greet(string name) {
        return $"Hello, {name}!";
    }
}
```

## Interface

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::csharp::CSharp;

let ts = TypeSpec::builder("IRepository", TypeKind::Interface)
    .visibility(Visibility::Public)
    .add_type_param(TypeParamSpec::new("T"))
    .doc("Generic data repository.")
    .add_method(
        FunSpec::builder("FindById")
            .returns(TypeName::primitive("T"))
            .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("Save")
            .returns(TypeName::primitive("void"))
            .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let file = FileSpec::builder_with("IRepository.cs", CSharp::new())
    .add_type(ts)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```csharp
/// Generic data repository.
public interface IRepository<T> {
    internal T FindById(string id);

    internal void Save(T entity);
}
```

## Enum

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::csharp::CSharp;

let ts = TypeSpec::builder("Direction", TypeKind::Enum)
    .visibility(Visibility::Public)
    .add_variant(EnumVariantSpec::new("North").unwrap())
    .add_variant(
        EnumVariantSpec::builder("South")
            .value(CodeBlock::of("1", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("East")
            .value(CodeBlock::of("2", ()).unwrap())
            .build()
            .unwrap(),
    )
    .add_variant(
        EnumVariantSpec::builder("West")
            .value(CodeBlock::of("3", ()).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let file = FileSpec::builder_with("Direction.cs", CSharp::new())
    .add_type(ts)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```csharp
public enum Direction {
    North,
    South = 1,
    East = 2,
    West = 3
}
```

## Async method with imports

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::csharp::CSharp;

let task_user = TypeName::importable("System.Threading.Tasks", "Task<User>");
let user = TypeName::importable("MyApp.Models", "User");

let body = CodeBlock::of("return await repo.GetByIdAsync(id);", ()).unwrap();

let fun = FunSpec::builder("GetUserAsync")
    .visibility(Visibility::Public)
    .is_async()
    .returns(task_user)
    .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
    .body(body)
    .build()
    .unwrap();

let ts = TypeSpec::builder("UserService", TypeKind::Class)
    .visibility(Visibility::Public)
    .add_method(fun)
    .build()
    .unwrap();

let file = FileSpec::builder_with("UserService.cs", CSharp::new())
    .add_type(ts)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```csharp
using System.Threading.Tasks;

using MyApp.Models;

public class UserService {
    public async Task<User> GetUserAsync(string id) {
        return await repo.GetByIdAsync(id);
    }
}
```
