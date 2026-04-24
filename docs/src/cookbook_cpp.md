# C++ Cookbook

Practical, copy-paste-ready recipes for C++ code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Class with template

```rust,ignore
use sigil_stitch::prelude::*;

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

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("StringVec", TypeKind::TypeAlias)
    .extends(TypeName::generic(
        TypeName::primitive("std::vector"),
        vec![TypeName::primitive("std::string")],
    ))
    .build()
    .unwrap();
```

```cpp
using StringVec = std::vector<std::string>;
```
