# Files & Projects

This chapter covers the import system, file rendering, and multi-file project generation. These specs follow the same builder pattern described in [Building Functions & Fields](functions_and_fields.md).

## ImportSpec

Explicit import control for cases where `%T` / `TypeName::Importable` is not sufficient. Add to a FileSpec via `add_import()`.

```rust,ignore
use sigil_stitch::spec::import_spec::ImportSpec;
use sigil_stitch::lang::typescript::TypeScript;

// Forced named import (even without %T usage in code)
let spec = ImportSpec::named("./models", "User");

// Aliased import: import { User as MyUser } from './models'
let spec = ImportSpec::named_as("./models", "User", "MyUser");

// Type-only import: import type { User } from './models'
let spec = ImportSpec::named_type("./models", "User");

// Side-effect import: import './polyfill'
let spec = ImportSpec::side_effect("./polyfill");

// Wildcard import: import * from './utils'
let spec = ImportSpec::wildcard("./utils");
```

Most of the time you do not need `ImportSpec` -- imports driven by `%T` and `TypeName::importable()` handle the common case. Use `ImportSpec` for forced imports, side-effect imports, and wildcard imports.

## FileSpec

The top-level file orchestrator. Combines code blocks, type declarations, and functions, then drives the three-pass render pipeline:

1. **Materialize** -- Specs (`TypeSpec`, `FunSpec`) emit CodeBlocks
2. **Collect imports** -- Walk all blocks, extract import references from `%T` types
3. **Render** -- Emit the import header, then the body with resolved names and pretty printing

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

let user = TypeName::importable_type("./models", "User");

let mut cb = CodeBlock::builder();
cb.add_statement("const u: %T = getUser()", (user,));
let block = cb.build().unwrap();

let file = FileSpec::builder("user.ts")
    .add_code(block)
    .build()
    .unwrap();

let output = file.render(80).unwrap();
// import type { User } from './models'
//
// const u: User = getUser();
```

You can mix member types freely: `add_code()` for raw CodeBlocks, `add_type()` for TypeSpec, `add_function()` for FunSpec, `add_raw()` for escape-hatch strings with no import tracking.

A file header (license comment, package declaration) can be set with `.header()`:

```rust,ignore
let mut header_b = CodeBlock::builder();
header_b.add("// License: MIT", ());
let header = header_b.build().unwrap();

let file = FileSpec::builder("service.ts")
    .header(header)
    .add_type(service_type)
    .build()
    .unwrap();
```

## ProjectSpec

Multi-file generation. Wraps multiple FileSpecs, renders them all, and can optionally write to the filesystem.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

// Build individual files
let models = FileSpec::builder("src/models.ts")
    .add_type(
        TypeSpec::builder("User", TypeKind::Interface).build().unwrap(),
    )
    .build()
    .unwrap();

let index = FileSpec::builder("src/index.ts")
    .add_code(CodeBlock::of("export {}", ()).unwrap())
    .build()
    .unwrap();

// Combine into a project
let project = ProjectSpec::builder()
    .add_file(models)
    .add_file(index)
    .build();

// Render all files in memory
let rendered = project.render(80).unwrap();
for file in &rendered {
    println!("--- {} ---\n{}", file.path, file.content);
}

// Or write directly to disk
// project.write_to(Path::new("./output"), 80).unwrap();
```

Each file resolves imports independently. `render()` returns `Vec<RenderedFile>` with `path` and `content` fields. `write_to()` creates parent directories as needed.

## End-to-End Example

A complete TypeScript class with imports, fields, a constructor, and a method -- from builder calls to rendered output.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::typescript::TypeScript;

// Define an imported type
let repo_type = TypeName::importable_type("./repository", "UserRepository");

// Build the class
let user_type = TypeName::importable_type("./models", "User");
let ctor_body = CodeBlock::of("this.repo = repo", ()).unwrap();
let method_body = CodeBlock::of("return this.repo.findById(id)", ()).unwrap();

let type_spec = TypeSpec::builder("UserService", TypeKind::Class)
    .visibility(Visibility::Public)
    // Field: private readonly repo: UserRepository;
    .add_field(
        FieldSpec::builder("repo", repo_type.clone())
            .visibility(Visibility::Private)
            .is_readonly()
            .build()
            .unwrap(),
    )
    // Constructor
    .add_method(
        FunSpec::builder("constructor")
            .is_constructor()
            .add_param(ParameterSpec::new("repo", repo_type.clone()).unwrap())
            .body(ctor_body)
            .build()
            .unwrap(),
    )
    // Method: async getUser(id: string): Promise<User>
    .add_method(
        FunSpec::builder("getUser")
            .is_async()
            .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
            .returns(TypeName::generic(
                TypeName::primitive("Promise"),
                vec![user_type],
            ))
            .body(method_body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

// Build the file
let file = FileSpec::builder("user_service.ts")
    .add_type(type_spec)
    .build()
    .unwrap();

let output = file.render(80).unwrap();
```

Rendered output:

```typescript
import type { User } from './models'
import { UserRepository } from './repository'

export class UserService {
    private readonly repo: UserRepository;

    constructor(repo: UserRepository) {
        this.repo = repo
    }

    async getUser(id: string): Promise<User> {
        return this.repo.findById(id)
    }
}
```

The import header is fully automatic. `UserRepository` and `User` are collected from the `%T` references inside the emitted CodeBlocks, deduplicated, and rendered as import statements. No manual import management required.
