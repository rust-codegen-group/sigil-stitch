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
    let user = TypeName::<Kotlin>::primitive("User");
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
    let mut iface = TypeSpec::<Kotlin>::builder("UserRepository", TypeKind::Interface);

    let mut find = FunSpec::<Kotlin>::builder("findById");
    find.returns(TypeName::primitive("User?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    iface.add_method(find.build().unwrap());

    let mut find_all = FunSpec::<Kotlin>::builder("findAll");
    find_all.returns(list.clone());
    iface.add_method(find_all.build().unwrap());

    let iface_spec = iface.build().unwrap();

    // Implementation class.
    let mut cls = TypeSpec::<Kotlin>::builder("InMemoryUserRepository", TypeKind::Class);
    cls.extends(TypeName::primitive("UserRepository"));
    cls.doc("In-memory implementation of UserRepository.");

    let mut users_field = FieldSpec::builder("users", mutable_list);
    users_field.visibility(Visibility::Private);
    users_field.is_readonly();
    cls.add_field(users_field.build().unwrap());

    // findById override.
    let find_body =
        CodeBlock::<Kotlin>::of("return users.firstOrNull { it.id == id }", ()).unwrap();
    let mut find_impl = FunSpec::<Kotlin>::builder("findById");
    find_impl.returns(TypeName::primitive("User?"));
    find_impl.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    find_impl.is_override();
    find_impl.body(find_body);
    cls.add_method(find_impl.build().unwrap());

    // findAll override.
    let find_all_body = CodeBlock::<Kotlin>::of("return %T(users)", (array_list,)).unwrap();
    let mut find_all_impl = FunSpec::<Kotlin>::builder("findAll");
    find_all_impl.returns(list);
    find_all_impl.is_override();
    find_all_impl.body(find_all_body);
    cls.add_method(find_all_impl.build().unwrap());

    let cls_spec = cls.build().unwrap();

    let mut fb = FileSpec::builder_with("UserRepo.kt", Kotlin::new());
    fb.add_type(iface_spec);
    fb.add_type(cls_spec);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/full_module.kt", &output);
}

#[test]
fn test_string_dollar_escape() {
    let body = CodeBlock::<Kotlin>::of(
        "val greeting = %S\nval template = %S\nprintln(greeting)",
        (
            StringLitArg("Hello ${name}!".into()),
            StringLitArg("Price: $100".into()),
        ),
    )
    .unwrap();
    let mut fb = FunSpec::<Kotlin>::builder("greet");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("greet.kt", Kotlin::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/string_dollar_escape.kt", &output);
}
