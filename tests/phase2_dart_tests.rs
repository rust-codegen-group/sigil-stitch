mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_dart_class_with_fields() {
    let mut tb = TypeSpec::<DartLang>::builder("UserService", TypeKind::Class);
    tb.doc("Service for managing users.");

    // Fields.
    let repo_field = FieldSpec::builder("repo", TypeName::primitive("UserRepository"));
    tb.add_field(repo_field.build().unwrap());

    let mut logger_field = FieldSpec::builder("logger", TypeName::primitive("Logger"));
    logger_field.is_readonly();
    tb.add_field(logger_field.build().unwrap());

    // Constructor.
    let ctor_body =
        CodeBlock::<DartLang>::of("this.repo = repo;\nthis.logger = logger;", ()).unwrap();
    let mut ctor = FunSpec::<DartLang>::builder("UserService");
    ctor.add_param(ParameterSpec::new("repo", TypeName::primitive("UserRepository")).unwrap());
    ctor.add_param(ParameterSpec::new("logger", TypeName::primitive("Logger")).unwrap());
    ctor.body(ctor_body);
    tb.add_method(ctor.build().unwrap());

    // Method.
    let find_body = CodeBlock::<DartLang>::of("return repo.findById(id);", ()).unwrap();
    let mut find = FunSpec::<DartLang>::builder("findUser");
    find.returns(TypeName::primitive("User?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    find.body(find_body);
    tb.add_method(find.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("user_service.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/class_with_fields.dart", &output);
}

#[test]
fn test_dart_abstract_class() {
    let mut tb = TypeSpec::<DartLang>::builder("Shape", TypeKind::Class);
    tb.doc("Abstract shape.");
    tb.is_abstract();

    // Concrete method.
    let desc_body = CodeBlock::<DartLang>::of("return runtimeType.toString();", ()).unwrap();
    let mut desc = FunSpec::<DartLang>::builder("describe");
    desc.returns(TypeName::primitive("String"));
    desc.body(desc_body);
    tb.add_method(desc.build().unwrap());

    // Abstract method.
    let mut area = FunSpec::<DartLang>::builder("area");
    area.is_abstract();
    area.returns(TypeName::primitive("double"));
    tb.add_method(area.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("shape.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/abstract_class.dart", &output);
}

#[test]
fn test_dart_class_extends_implements() {
    let base = TypeName::<DartLang>::importable("package:myapp/base.dart", "BaseService");
    let auth = TypeName::<DartLang>::importable("package:myapp/auth.dart", "Authenticatable");
    let serial = TypeName::<DartLang>::importable("package:myapp/serial.dart", "Serializable");

    let mut tb = TypeSpec::<DartLang>::builder("AdminService", TypeKind::Class);
    tb.extends(base);
    tb.implements(auth);
    tb.implements(serial);

    let body = CodeBlock::<DartLang>::of("return true;", ()).unwrap();
    let mut is_admin = FunSpec::<DartLang>::builder("isAdmin");
    is_admin.returns(TypeName::primitive("bool"));
    is_admin.body(body);
    tb.add_method(is_admin.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("admin_service.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/class_extends_implements.dart", &output);
}

#[test]
fn test_dart_enum() {
    let mut tb = TypeSpec::<DartLang>::builder("Color", TypeKind::Enum);
    tb.doc("Supported colors.");

    tb.add_variant(EnumVariantSpec::new("red").unwrap());
    tb.add_variant(EnumVariantSpec::new("green").unwrap());
    tb.add_variant(EnumVariantSpec::new("blue").unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("color.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/enum.dart", &output);
}

#[test]
fn test_dart_generic_class() {
    let tp = TypeParamSpec::<DartLang>::new("T").with_bound(TypeName::primitive("Comparable"));

    let mut tb = TypeSpec::<DartLang>::builder("SortedList", TypeKind::Class);
    tb.add_type_param(tp);
    tb.doc("A sorted list with bounded type parameter.");

    let mut items_field = FieldSpec::builder("items", TypeName::primitive("List<T>"));
    items_field.is_readonly();
    tb.add_field(items_field.build().unwrap());

    let add_body = CodeBlock::<DartLang>::of("items.add(item);\nitems.sort();", ()).unwrap();
    let mut add = FunSpec::<DartLang>::builder("add");
    add.returns(TypeName::primitive("void"));
    add.add_param(ParameterSpec::new("item", TypeName::primitive("T")).unwrap());
    add.body(add_body);
    tb.add_method(add.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("sorted_list.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/generic_class.dart", &output);
}

#[test]
fn test_dart_static_final() {
    let mut tb = TypeSpec::<DartLang>::builder("Constants", TypeKind::Class);

    let mut max_field = FieldSpec::builder("maxSize", TypeName::primitive("int"));
    max_field.is_static();
    max_field.is_readonly();
    max_field.initializer(CodeBlock::<DartLang>::of("100", ()).unwrap());
    tb.add_field(max_field.build().unwrap());

    let mut name_field = FieldSpec::builder("appName", TypeName::primitive("String"));
    name_field.is_static();
    name_field.is_readonly();
    name_field.initializer(
        CodeBlock::<DartLang>::of(
            "%S",
            (sigil_stitch::code_block::StringLitArg("MyApp".to_string()),),
        )
        .unwrap(),
    );
    tb.add_field(name_field.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("constants.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/static_final.dart", &output);
}

#[test]
fn test_dart_async_function() {
    let user = TypeName::<DartLang>::importable("package:myapp/models/user.dart", "User");

    let body = CodeBlock::<DartLang>::of("return await api.fetchUser(id);", ()).unwrap();
    let mut fb_fun = FunSpec::<DartLang>::builder("fetchUser");
    fb_fun.returns(TypeName::primitive("Future<User>"));
    fb_fun.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    fb_fun.body(body);
    let fun = fb_fun.build().unwrap();

    // Trigger User import.
    let trigger = CodeBlock::<DartLang>::of("// %T", (user,)).unwrap();

    let mut fb = FileSpec::builder_with("api.dart", DartLang::new());
    fb.add_code(trigger);
    fb.add_function(fun);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/async_function.dart", &output);
}

#[test]
fn test_dart_annotated_method() {
    let mut tb = TypeSpec::<DartLang>::builder("Dog", TypeKind::Class);
    tb.extends(TypeName::primitive("Animal"));

    let body = CodeBlock::<DartLang>::of(
        "return %S;",
        (sigil_stitch::code_block::StringLitArg("Woof!".to_string()),),
    )
    .unwrap();
    let mut speak = FunSpec::<DartLang>::builder("speak");
    speak.returns(TypeName::primitive("String"));
    speak.annotation(CodeBlock::<DartLang>::of("@override", ()).unwrap());
    speak.body(body);
    tb.add_method(speak.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("dog.dart", DartLang::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/annotated_method.dart", &output);
}

#[test]
fn test_dart_full_module() {
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
