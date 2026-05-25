# C++ Cookbook

Practical, copy-paste-ready recipes for C++ code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Class with template

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let body = CodeBlock::of("data_.push_back(value)", ()).unwrap();

let type_spec = TypeSpec::builder("Stack", TypeKind::Class)
    .add_type_param(TypeParamSpec::new("T"))
    .add_field(
        FieldSpec::builder("data_", TypeName::generic(
            TypeName::primitive("std::vector"),
            vec![TypeName::primitive("T")],
        ))
            .visibility(Visibility::Private)
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("push")
            .visibility(Visibility::Public)
            .add_param(ParameterSpec::new("value", TypeName::reference(TypeName::primitive("T"))).unwrap())
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
# }
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

## Using alias (C++ type alias)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("StringVec", TypeKind::TypeAlias)
    .extends(TypeName::generic(
        TypeName::primitive("std::vector"),
        vec![TypeName::primitive("std::string")],
    ))
    .build()
    .unwrap();
# }
```

```cpp
using StringVec = std::vector<std::string>;
```

## Enum class

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let type_spec = TypeSpec::builder("Color", TypeKind::Enum)
    .doc("Available colors.")
    .add_variant(EnumVariantSpec::new("Red").unwrap())
    .add_variant(EnumVariantSpec::new("Green").unwrap())
    .add_variant(EnumVariantSpec::new("Blue").unwrap())
    .build()
    .unwrap();
# }
```

```cpp
/// Available colors.
enum class Color {
    Red,
    Green,
    Blue
};
```

## Virtual method

C++ abstract classes with pure virtual methods require the `extra_member` escape hatch. Use `FunSpec::emit()` to render each method signature as a `CodeBlock`, then attach it to the type via `extra_member`.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::cpp::Cpp;
# fn main() {
fn emit_fun(fun: &FunSpec) -> CodeBlock {
    let lang = Cpp::new();
    fun.emit(&lang, DeclarationContext::Member).unwrap()
}

let mut pub_section = CodeBlock::builder();
pub_section.add("%<", ());
pub_section.add("public:", ());
pub_section.add_line();
pub_section.add("%>", ());

pub_section.add_code(emit_fun(
    &FunSpec::builder("area")
        .is_abstract()
        .returns(TypeName::primitive("double"))
        .suffix("const")
        .suffix("= 0")
        .build()
        .unwrap(),
));

pub_section.add_line();
pub_section.add_code(emit_fun(
    &FunSpec::builder("~Shape")
        .is_abstract()
        .suffix("= default")
        .build()
        .unwrap(),
));

let type_spec = TypeSpec::builder("Shape", TypeKind::Class)
    .doc("Abstract shape base class.")
    .extra_member(pub_section.build().unwrap())
    .build()
    .unwrap();
# }
```

```cpp
/// Abstract shape base class.
class Shape {
public:
    virtual double area() const = 0;

    virtual ~Shape() = default;
};
```

## Namespace wrapping

Use `FileSpec::add_raw` to wrap generated code in a namespace block.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# fn main() {
let mut b = CodeBlock::builder();
b.add("int square(int x) {", ());
b.add_line();
b.add("%>", ());
b.add("return x * x;", ());
b.add_line();
b.add("%<", ());
b.add("}", ());
b.add_line();
let block = b.build().unwrap();

let file = FileSpec::builder("math.hpp")
    .header(CodeBlock::of("#pragma once", ()).unwrap())
    .add_raw("namespace math {\n")
    .add_code(block)
    .add_raw("\n} // namespace math\n")
    .build()
    .unwrap();
let output = file.render(80).unwrap();
# }
```

```cpp
#pragma once

namespace math {

int square(int x) {
    return x * x;
}


} // namespace math
```
