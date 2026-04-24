# OCaml Cookbook

Practical, copy-paste-ready recipes for OCaml code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Record type

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("person", TypeKind::Struct)
    .doc("A person record.")
    .add_field(
        FieldSpec::builder("name", TypeName::primitive("string")).build().unwrap(),
    )
    .add_field(
        FieldSpec::builder("age", TypeName::primitive("int")).build().unwrap(),
    )
    .add_field(
        FieldSpec::builder("email", TypeName::primitive("string")).build().unwrap(),
    )
    .build()
    .unwrap();
```

```ocaml
(** A person record. *)
type person =
  {
    name : string;
    age : int;
    email : string;
  }
```

## Function with curried params

```rust,ignore
use sigil_stitch::prelude::*;

let body = CodeBlock::of("List.map f xs", ()).unwrap();

let fun = FunSpec::builder("transform")
    .add_param(ParameterSpec::new("f", TypeName::primitive("'a -> 'b")).unwrap())
    .add_param(ParameterSpec::new("xs", TypeName::primitive("'a list")).unwrap())
    .returns(TypeName::primitive("'b list"))
    .body(body)
    .build()
    .unwrap();
```

```ocaml
let transform (f : 'a -> 'b) (xs : 'a list) : 'b list =
  List.map f xs
```

## Module block

OCaml modules are structurally different from types -- they can contain multiple types and values. Use the `OCaml::module_block` helper to build a `module Name = struct ... end` block as a raw `CodeBlock`.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::ocaml::OCaml;

let mut inner = CodeBlock::builder();
inner.add_statement("let greeting = \"hello\"", ());
inner.add_statement("let farewell = \"goodbye\"", ());
let body = inner.build().unwrap();

let module = OCaml::module_block("MyModule", body).unwrap();
```

```ocaml
module MyModule = struct
  let greeting = "hello"
  let farewell = "goodbye"
end
```

## Type alias

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("string_list", TypeKind::TypeAlias)
    .extends(TypeName::primitive("string list"))
    .build()
    .unwrap();
```

```ocaml
type string_list = string list
```

## Pattern match

Pattern matching is built using `CodeBlock` control-flow methods. Use `begin_control_flow` for the outer binding, then `begin_control_flow_with_open` to open the `match` expression with no trailing brace.

```rust,ignore
use sigil_stitch::code_block::CodeBlock;

let mut b = CodeBlock::builder();
b.begin_control_flow("let describe color", ());
b.begin_control_flow_with_open("match color with", (), "");
b.add("| Red -> \"red\"", ());
b.add_line();
b.add("| Green -> \"green\"", ());
b.add_line();
b.add("| Blue -> \"blue\"", ());
b.add_line();
b.end_control_flow();
b.end_control_flow();
let block = b.build().unwrap();
```

```ocaml
let describe color =
  match color with
    | Red -> "red"
    | Green -> "green"
    | Blue -> "blue"
```
