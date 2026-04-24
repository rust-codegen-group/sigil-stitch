# Java Cookbook

Practical, copy-paste-ready recipes for Java code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Class with annotations

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;

let inject = AnnotationSpec::new("Inject");

let body = CodeBlock::of("return repository.findById(id)", ()).unwrap();

let type_spec = TypeSpec::builder("UserService", TypeKind::Class)
    .visibility(Visibility::Public)
    .add_field(
        FieldSpec::builder("repository", TypeName::primitive("UserRepository"))
            .visibility(Visibility::Private)
            .annotate(inject)
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("getUser")
            .visibility(Visibility::Public)
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .returns(TypeName::primitive("User"))
            .body(body)
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
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
