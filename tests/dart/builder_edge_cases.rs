use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_full_module() {
    let future = TypeName::<DartLang>::importable("dart:async", "Future");
    let convert = TypeName::<DartLang>::importable("dart:convert", "jsonDecode");

    // Abstract class (interface).
    let mut iface = TypeSpec::<DartLang>::builder("UserRepository", TypeKind::Interface);

    let mut find = FunSpec::<DartLang>::builder("findById");
    find.returns(TypeName::primitive("User?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    iface.add_method(find.build().unwrap());

    let mut find_all = FunSpec::<DartLang>::builder("findAll");
    find_all.returns(TypeName::primitive("List<User>"));
    iface.add_method(find_all.build().unwrap());

    let iface_spec = iface.build().unwrap();

    // Implementation class.
    let mut cls = TypeSpec::<DartLang>::builder("InMemoryUserRepository", TypeKind::Class);
    cls.implements(TypeName::primitive("UserRepository"));
    cls.doc("In-memory implementation of UserRepository.");

    let mut users_field = FieldSpec::builder("_users", TypeName::primitive("List<User>"));
    users_field.is_readonly();
    users_field.initializer(CodeBlock::<DartLang>::of("[]", ()).unwrap());
    cls.add_field(users_field.build().unwrap());

    // findById with @override.
    let find_body = CodeBlock::<DartLang>::of(
        "return _users.cast<User?>().firstWhere(\n  (u) => u?.id == id,\n  orElse: () => null,\n);",
        (),
    )
    .unwrap();
    let mut find_impl = FunSpec::<DartLang>::builder("findById");
    find_impl.returns(TypeName::primitive("User?"));
    find_impl.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    find_impl.annotation(CodeBlock::<DartLang>::of("@override", ()).unwrap());
    find_impl.body(find_body);
    cls.add_method(find_impl.build().unwrap());

    // findAll with @override.
    let find_all_body = CodeBlock::<DartLang>::of("return List.unmodifiable(_users);", ()).unwrap();
    let mut find_all_impl = FunSpec::<DartLang>::builder("findAll");
    find_all_impl.returns(TypeName::primitive("List<User>"));
    find_all_impl.annotation(CodeBlock::<DartLang>::of("@override", ()).unwrap());
    find_all_impl.body(find_all_body);
    cls.add_method(find_all_impl.build().unwrap());

    let cls_spec = cls.build().unwrap();

    // Standalone function using Future + convert imports.
    let parse_body = CodeBlock::<DartLang>::of(
        "final data = %T(json);\nreturn User.fromMap(data);",
        (convert,),
    )
    .unwrap();
    let mut parse_fn = FunSpec::<DartLang>::builder("parseUser");
    parse_fn.returns(TypeName::primitive("User"));
    parse_fn.add_param(ParameterSpec::new("json", TypeName::primitive("String")).unwrap());
    parse_fn.body(parse_body);
    let parse_user = parse_fn.build().unwrap();

    // Trigger Future import.
    let future_trigger = CodeBlock::<DartLang>::of("// %T", (future,)).unwrap();

    let mut fb = FileSpec::builder_with("user_repo.dart", DartLang::new());
    fb.add_code(future_trigger);
    fb.add_type(iface_spec);
    fb.add_type(cls_spec);
    fb.add_function(parse_user);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/full_module.dart", &output);
}

#[test]
fn test_string_dollar_escape() {
    let body = CodeBlock::<DartLang>::of(
        "final greeting = %S;\nfinal template = %S;\nprint(greeting);",
        (
            StringLitArg("Hello ${name}!".into()),
            StringLitArg("Price: $100".into()),
        ),
    )
    .unwrap();
    let mut fb = FunSpec::<DartLang>::builder("greet");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("greet.dart", DartLang::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/string_dollar_escape.dart", &output);
}
