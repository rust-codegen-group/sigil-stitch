# TypeScript Cookbook

Practical, copy-paste-ready recipes for TypeScript code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Class with imports

```rust,ignore
use sigil_stitch::prelude::*;

let user_type = TypeName::importable_type("./models", "User");
let repo_type = TypeName::importable("./repository", "UserRepository");

let body = CodeBlock::of("return this.repo.findById(id)", ()).unwrap();

let type_spec = TypeSpec::builder("UserService", TypeKind::Class)
    .visibility(Visibility::Public)
    .add_field(
        FieldSpec::builder("repo", repo_type.clone())
            .visibility(Visibility::Private)
            .is_readonly()
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("getUser")
            .is_async()
            .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
            .returns(TypeName::generic(TypeName::primitive("Promise"), vec![user_type]))
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let output = FileSpec::builder("user_service.ts")
    .add_type(type_spec)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
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

## Interface with generics

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Repository", TypeKind::Interface)
    .visibility(Visibility::Public)
    .add_type_param(TypeParamSpec::new("T"))
    .add_method(
        FunSpec::builder("findById")
            .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
            .returns(TypeName::generic(TypeName::primitive("Promise"), vec![TypeName::primitive("T")]))
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("save")
            .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
            .returns(TypeName::generic(TypeName::primitive("Promise"), vec![TypeName::primitive("void")]))
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```typescript
export interface Repository<T> {
    findById(id: string): Promise<T>;
    save(entity: T): Promise<void>;
}
```

## Type alias

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("UserId", TypeKind::TypeAlias)
    .visibility(Visibility::Public)
    .extends(TypeName::primitive("string"))
    .build()
    .unwrap();
```

```typescript
export type UserId = string;
```
