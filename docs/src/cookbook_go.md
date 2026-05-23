# Go Cookbook

Practical, copy-paste-ready recipes for Go code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Struct with tags

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("User", TypeKind::Struct)
    .add_field(
        FieldSpec::builder("Name", TypeName::primitive("string"))
            .tag("json:\"name\" db:\"name\"")
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("Email", TypeName::primitive("string"))
            .tag("json:\"email\" db:\"email\"")
            .build()
            .unwrap(),
    )
    .add_field(
        FieldSpec::builder("Age", TypeName::primitive("int"))
            .tag("json:\"age,omitempty\"")
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
# }
```

```go
type User struct {
    Name string `json:"name" db:"name"`
    Email string `json:"email" db:"email"`
    Age int `json:"age,omitempty"`
}
```

## Newtype (distinct type)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("Meters", TypeKind::Newtype)
    .extends(TypeName::primitive("float64"))
    .build()
    .unwrap();
# }
```

```go
type Meters float64
```

## Interface

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("Repository", TypeKind::Interface)
    .doc("Repository defines data access methods.")
    .add_method(
        FunSpec::builder("FindByID")
            .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
            .returns(TypeName::raw("(Entity, error)"))
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("Save")
            .add_param(ParameterSpec::new("entity", TypeName::primitive("Entity")).unwrap())
            .returns(TypeName::primitive("error"))
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();

let file = FileSpec::builder("repo.go")
    .header(CodeBlock::of("package repo", ()).unwrap())
    .add_type(type_spec)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
# }
```

```go
package repo

// Repository defines data access methods.
type Repository interface {
	FindByID(id string) (Entity, error)

	Save(entity Entity) error
}
```

## Generic function

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("comparable"));

let mut body_b = CodeBlock::builder();
body_b.begin_control_flow("if a > b", ());
body_b.add_statement("return a", ());
body_b.end_control_flow();
body_b.add_statement("return b", ());
let body = body_b.build().unwrap();

let fun = FunSpec::builder("Max")
    .add_type_param(tp)
    .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
    .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
    .returns(TypeName::primitive("T"))
    .body(body)
    .build()
    .unwrap();

let file = FileSpec::builder("max.go")
    .header(CodeBlock::of("package main", ()).unwrap())
    .add_function(fun)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
# }
```

```go
package main

func Max[T comparable](a T, b T) T {
	if a > b {
		return a
	}
	return b
}
```

## Const block with enum generation

Use `sigil_quote!` with a `$for` inside `const ( ... )` to generate enum-like
const blocks:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::go_lang::GoLang;
# fn main() {
let variants = vec!["Alpha", "Beta", "Gamma"];

let const_block = sigil_quote!(GoLang {
    const (
    $for(v in &variants) {
        $L("@{v}Kind @{v} = \"@{v}\"");
    }
    )
}).unwrap();

let file = FileSpec::builder("kind.go")
    .header(CodeBlock::of("package main", ()).unwrap())
    .add_code(const_block)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
# }
```

```go
package main

const (
	AlphaKind Alpha = "Alpha"
	BetaKind Beta = "Beta"
	GammaKind Gamma = "Gamma"
)
```

The parser recognizes `const (`, `var (`, `import (`, and `type (` as structural
blocks in GoLang, so `$for`, `$if`, `$C_each`, and interpolation markers all
work inside the parentheses. The body is auto-indented with tabs.
