use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::dart::Dart;
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
    let future = TypeName::importable("dart:async", "Future");
    let convert = TypeName::importable("dart:convert", "jsonDecode");

    // Abstract class (interface).
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
                .returns(TypeName::primitive("List<User>"))
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
        FieldSpec::builder("_users", TypeName::primitive("List<User>"))
            .is_readonly()
            .initializer(CodeBlock::of("[]", ()).unwrap())
            .build()
            .unwrap(),
    );

    // findById with @override.
    let find_body = CodeBlock::of(
        "return _users.cast<User?>().firstWhere(\n  (u) => u?.id == id,\n  orElse: () => null,\n);",
        (),
    )
    .unwrap();
    let cls = cls.add_method(
        FunSpec::builder("findById")
            .returns(TypeName::primitive("User?"))
            .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
            .annotation(CodeBlock::of("@override", ()).unwrap())
            .body(find_body)
            .build()
            .unwrap(),
    );

    // findAll with @override.
    let find_all_body = CodeBlock::of("return List.unmodifiable(_users);", ()).unwrap();
    let cls = cls.add_method(
        FunSpec::builder("findAll")
            .returns(TypeName::primitive("List<User>"))
            .annotation(CodeBlock::of("@override", ()).unwrap())
            .body(find_all_body)
            .build()
            .unwrap(),
    );

    let cls_spec = cls.build().unwrap();

    // Standalone function using Future + convert imports.
    let parse_body = CodeBlock::of(
        "final data = %T(json);\nreturn User.fromMap(data);",
        (convert,),
    )
    .unwrap();
    let parse_user = FunSpec::builder("parseUser")
        .returns(TypeName::primitive("User"))
        .add_param(ParameterSpec::new("json", TypeName::primitive("String")).unwrap())
        .body(parse_body)
        .build()
        .unwrap();

    // Trigger Future import.
    let future_trigger = CodeBlock::of("// %T", (future,)).unwrap();

    let file = FileSpec::builder_with("user_repo.dart", Dart::new())
        .add_code(future_trigger)
        .add_type(iface_spec)
        .add_type(cls_spec)
        .add_function(parse_user)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/full_module.dart", &output);
}

#[test]
fn test_string_dollar_escape() {
    let body = CodeBlock::of(
        "final greeting = %S;\nfinal template = %S;\nprint(greeting);",
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

    let file = FileSpec::builder_with("greet.dart", Dart::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/string_dollar_escape.dart", &output);
}
