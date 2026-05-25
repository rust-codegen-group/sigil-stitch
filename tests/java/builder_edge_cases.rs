use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java::Java;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_static_final_field() {
    let ts = TypeSpec::builder("Constants", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("MAX_SIZE", TypeName::primitive("int"))
                .visibility(Visibility::Public)
                .is_static()
                .is_readonly()
                .initializer(CodeBlock::of("100", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("APP_NAME", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .is_static()
                .is_readonly()
                .initializer(
                    CodeBlock::of(
                        "%S",
                        (sigil_stitch::code_block::StringLitArg("MyApp".to_string()),),
                    )
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Constants.java", Java::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/static_final_field.java", &output);
}

#[test]
fn test_annotated_method() {
    let body = CodeBlock::of("return \"Woof!\";", ()).unwrap();
    let ts = TypeSpec::builder("Dog", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Animal"))
        .add_method(
            FunSpec::builder("speak")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .annotation(CodeBlock::of("@Override", ()).unwrap())
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Dog.java", Java::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/annotated_method.java", &output);
}

#[test]
fn test_full_module() {
    let list = TypeName::importable("java.util", "List");
    let list_user = TypeName::generic(list, vec![TypeName::primitive("User")]);
    let array_list = TypeName::importable("java.util", "ArrayList");
    let nullable = TypeName::importable("javax.annotation", "Nullable");

    // Interface.
    let iface_spec = TypeSpec::builder("UserRepository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::primitive("User"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .annotation(CodeBlock::of("@%T", (nullable.clone(),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findAll")
                .returns(list_user.clone())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // Implementation class.
    let ctor_body = CodeBlock::of("this.users = new %T<>();", (array_list.clone(),)).unwrap();
    let find_body = CodeBlock::of(
        "return this.users.stream()\n    .filter(u -> u.getId().equals(id))\n    .findFirst()\n    .orElse(null);",
        (),
    )
    .unwrap();
    let find_all_body = CodeBlock::of("return new %T<>(this.users);", (array_list,)).unwrap();

    let cls_spec = TypeSpec::builder("InMemoryUserRepository", TypeKind::Class)
        .visibility(Visibility::Public)
        .implements(TypeName::primitive("UserRepository"))
        .doc("In-memory implementation of UserRepository.")
        .add_field(
            FieldSpec::builder("users", list_user.clone())
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("InMemoryUserRepository")
                .visibility(Visibility::Public)
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findById")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("User"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .annotation(CodeBlock::of("@Override", ()).unwrap())
                .annotation(CodeBlock::of("@%T", (nullable,)).unwrap())
                .body(find_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findAll")
                .visibility(Visibility::Public)
                .returns(list_user)
                .annotation(CodeBlock::of("@Override", ()).unwrap())
                .body(find_all_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserRepo.java", Java::new())
        .add_type(iface_spec)
        .add_type(cls_spec)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/full_module.java", &output);
}
