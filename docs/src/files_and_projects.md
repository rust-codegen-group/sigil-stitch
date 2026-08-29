# Files & Projects

This chapter covers the import system, file rendering, and multi-file project generation. These specs follow the same builder pattern described in [Building Functions & Fields](functions_and_fields.md).

## ImportSpec

Explicit import control for cases where `%T` / `TypeName::Importable` is not sufficient. Add to a FileSpec via `add_import()`.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::spec::import_spec::ImportSpec;
# use sigil_stitch::lang::typescript::TypeScript;
# use sigil_stitch::prelude::*;
# fn main() {
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
# }
```

Most of the time you do not need `ImportSpec` -- imports driven by `%T` and `TypeName::importable()` handle the common case. Use `ImportSpec` for forced imports, side-effect imports, and wildcard imports.

## FileSpec

The top-level file orchestrator combines code blocks and declaration specs.

`FileSpec::render()` owns the complete render-preparation pipeline:

1. **Lower declarations** -- Validate declaration specs and ask the language
   adapter to lower them to source `CodeBlock`s.
2. **Prepare blocks** -- Rewrite each source block exactly once, validate its
   structure, lower every `%T` type, and validate the lowered type blocks.
3. **Resolve imports** -- Collect imports only from the prepared blocks, merge
   explicit imports, then deduplicate them and assign every peer conflict set
   atomically.
4. **Render** -- Emit the import header and prepared body with no further
   rewrite or type lowering.

No import header or body text is returned until every preparation and
resolution operation succeeds. `FileSpec::validate()` remains model-only
validation; rewrite, type-name lowering, and import resolution run only during
render preparation.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
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
# }
```

### Custom import conflict resolution

`render()` uses the built-in deterministic module-prefix policy. For
project-specific naming, implement `ImportAliasConflictResolver` and pass a
borrowed value to `render_with_import_alias_resolver()`. One call receives all
ambiguous peer classes in that file. It must return exactly one assignment for
every claim; exact `ImportSpec` bindings cannot change. Missing, duplicate,
unknown, unsafe, globally colliding, or target-invalid assignments abort the
render before source is returned.

`ProjectSpec::render_with_import_alias_resolver()` applies the same borrowed
policy independently to each file. Its matching
`write_to_with_import_alias_resolver()` renders every file successfully before
creating output, so a resolution failure cannot leave a partially written
project. The resolver is an execution dependency and is never stored or
serialized in a file or project spec.

Direct `ImportGroup::try_resolve()` and `try_resolve_with()` callers must invoke
the selected adapter's `CodeLang::validate_resolved_imports()` before passing
the group to `render_imports()`. `FileSpec` and `ProjectSpec` perform this
target-local validation automatically.

You can mix member types freely: `add_code()` for raw CodeBlocks, `add_type()` for TypeSpec, `add_function()` for FunSpec, `add_raw()` for escape-hatch strings with no import tracking.

A file header (license comment, package declaration) can be set with `.header()`:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
# let service_type = TypeSpec::builder("Service", TypeKind::Class).build().unwrap();
let mut header_b = CodeBlock::builder();
header_b.add("// License: MIT", ());
let header = header_b.build().unwrap();

let file = FileSpec::builder("service.ts")
    .header(header)
    .add_type(service_type)
    .build()
    .unwrap();
# }
```

## ProjectSpec

Multi-file generation. Wraps multiple FileSpecs, renders them all, and can optionally write to the filesystem.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
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
    .build()
    .unwrap();

// Render all files in memory
let rendered = project.render(80).unwrap();
for file in &rendered {
    println!("--- {} ---\n{}", file.path, file.content);
}

// Or write directly to disk
// project.write_to(Path::new("./output"), 80).unwrap();
# }
```

`ProjectSpec::validate()` checks every file in project order and returns one
`ProjectSpecValidation` error containing each invalid file's complete
`FileSpec::validate()` failure. Member errors remain grouped inside their
`FileSpecValidation` error. `render()` performs this complete validation before
rendering any file, and `write_to()` renders the whole project in memory before
creating directories or files. A validation failure therefore returns all
known file diagnostics and performs no writes.

After validation, each file resolves imports independently. `render()` returns
`Vec<RenderedFile>` with `path` and `content` fields. `write_to()` creates
parent directories as needed only after every file renders successfully.

## End-to-End Example

A complete TypeScript class with imports, fields, a constructor, and a method -- from builder calls to rendered output.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::typescript::TypeScript;
# fn main() {
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
# }
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
