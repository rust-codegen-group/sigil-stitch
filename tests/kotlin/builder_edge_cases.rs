use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_full_module() {
    let user = TypeName::primitive("User");
    let list = TypeName::generic(
        TypeName::importable("kotlin.collections", "List"),
        vec![user.clone()],
    );
    let mutable_list = TypeName::generic(
        TypeName::importable("kotlin.collections", "MutableList"),
        vec![user.clone()],
    );
    let array_list = TypeName::generic(
        TypeName::importable("kotlin.collections", "ArrayList"),
        vec![user],
    );

    // Interface.
    let iface_spec = TypeSpec::builder("UserRepository", TypeKind::Interface)
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::primitive("User?"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findAll")
                .returns(list.clone())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // Implementation class.
    let cls = TypeSpec::builder("InMemoryUserRepository", TypeKind::Class);
    let cls = cls.implements(TypeName::primitive("UserRepository"));
    let cls = cls.doc("In-memory implementation of UserRepository.");

    let cls = cls.add_field(
        FieldSpec::builder("users", mutable_list)
            .visibility(Visibility::Private)
            .is_readonly()
            .build()
            .unwrap(),
    );

    // findById override.
    let find_body = CodeBlock::of("return users.firstOrNull { it.id == id }", ()).unwrap();
    let cls = cls.add_method(
        FunSpec::builder("findById")
            .returns(TypeName::primitive("User?"))
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .is_override()
            .body(find_body)
            .build()
            .unwrap(),
    );

    // findAll override.
    let find_all_body = CodeBlock::of("return %T(users)", (array_list,)).unwrap();
    let cls = cls.add_method(
        FunSpec::builder("findAll")
            .returns(list)
            .is_override()
            .body(find_all_body)
            .build()
            .unwrap(),
    );

    let cls_spec = cls.build().unwrap();

    let file = FileSpec::builder_with("UserRepo.kt", Kotlin::new())
        .add_type(iface_spec)
        .add_type(cls_spec)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/full_module.kt", &output);
}

#[test]
fn test_string_dollar_escape() {
    let body = CodeBlock::of(
        "val greeting = %S\nval template = %S\nprintln(greeting)",
        (
            StringLitArg("Hello ${name}!".into()),
            StringLitArg("Price: $100".into()),
        ),
    )
    .unwrap();
    let fun = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("greet.kt", Kotlin::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/string_dollar_escape.kt", &output);
}
