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

## Interface

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Repository", TypeKind::Interface)
    .visibility(Visibility::Public)
    .add_type_param(TypeParamSpec::new("T"))
    .doc("Generic data repository.")
    .add_method(
        FunSpec::builder("findById")
            .returns(TypeName::primitive("T"))
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("save")
            .returns(TypeName::primitive("void"))
            .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("delete")
            .returns(TypeName::primitive("void"))
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .build()
            .unwrap(),
    )
    .build()
    .unwrap();
```

```java
/**
 * Generic data repository.
 */
public interface Repository<T> {
    T findById(String id);

    void save(T entity);

    void delete(String id);
}
```

## Enum

```rust,ignore
use sigil_stitch::prelude::*;

let type_spec = TypeSpec::builder("Color", TypeKind::Enum)
    .visibility(Visibility::Public)
    .doc("Supported colors.")
    .add_variant(EnumVariantSpec::new("RED").unwrap())
    .add_variant(EnumVariantSpec::new("GREEN").unwrap())
    .add_variant(EnumVariantSpec::new("BLUE").unwrap())
    .build()
    .unwrap();
```

```java
/**
 * Supported colors.
 */
public enum Color {
    RED,
    GREEN,
    BLUE
}
```

## Abstract class

```rust,ignore
use sigil_stitch::prelude::*;

let desc_body = CodeBlock::of("return this.getClass().getSimpleName();", ()).unwrap();

let type_spec = TypeSpec::builder("Shape", TypeKind::Class)
    .visibility(Visibility::Public)
    .doc("Abstract shape.")
    .add_method(
        FunSpec::builder("describe")
            .visibility(Visibility::Public)
            .returns(TypeName::primitive("String"))
            .body(desc_body)
            .build()
            .unwrap(),
    )
    .add_method(
        FunSpec::builder("area")
            .visibility(Visibility::Public)
            .is_abstract()
            .returns(TypeName::primitive("double"))
            .build()
            .unwrap(),
    )
    .is_abstract()
    .build()
    .unwrap();
```

```java
/**
 * Abstract shape.
 */
public abstract class Shape {
    public String describe() {
        return this.getClass().getSimpleName();
    }

    public abstract double area();
}
```
