mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_swift_class_with_properties() {
    let mut tb = TypeSpec::<Swift>::builder("UserService", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.doc("Service for managing users.");

    // Properties.
    let mut repo_field = FieldSpec::builder("repo", TypeName::primitive("UserRepository"));
    repo_field.visibility(Visibility::Private);
    tb.add_field(repo_field.build().unwrap());

    let mut logger_field = FieldSpec::builder("logger", TypeName::primitive("Logger"));
    logger_field.visibility(Visibility::Private);
    logger_field.is_readonly();
    tb.add_field(logger_field.build().unwrap());

    // Method.
    let find_body = CodeBlock::<Swift>::of("return repo.find(by: id)", ()).unwrap();
    let mut find = FunSpec::<Swift>::builder("findUser");
    find.visibility(Visibility::Public);
    find.returns(TypeName::primitive("User?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    find.body(find_body);
    tb.add_method(find.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("UserService.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/class_with_properties.swift", &output);
}

#[test]
fn test_swift_struct() {
    let mut tb = TypeSpec::<Swift>::builder("User", TypeKind::Struct);
    tb.visibility(Visibility::Public);
    tb.doc("A user value type.");

    let mut name_field = FieldSpec::builder("name", TypeName::primitive("String"));
    name_field.visibility(Visibility::Public);
    name_field.is_readonly();
    tb.add_field(name_field.build().unwrap());

    let mut age_field = FieldSpec::builder("age", TypeName::primitive("Int"));
    age_field.visibility(Visibility::Public);
    age_field.is_readonly();
    tb.add_field(age_field.build().unwrap());

    let mut email_field = FieldSpec::builder("email", TypeName::primitive("String?"));
    email_field.visibility(Visibility::Public);
    tb.add_field(email_field.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("User.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/struct.swift", &output);
}

#[test]
fn test_swift_protocol() {
    let tp = TypeParamSpec::<Swift>::new("T");

    let mut tb = TypeSpec::<Swift>::builder("Repository", TypeKind::Interface);
    tb.add_type_param(tp);
    tb.doc("Generic data repository.");

    // Protocol method requirements (no body).
    let mut find = FunSpec::<Swift>::builder("findById");
    find.returns(TypeName::primitive("T?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    tb.add_method(find.build().unwrap());

    let mut save = FunSpec::<Swift>::builder("save");
    save.add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap());
    tb.add_method(save.build().unwrap());

    let mut delete = FunSpec::<Swift>::builder("delete");
    delete.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    tb.add_method(delete.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("Repository.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/protocol.swift", &output);
}

#[test]
fn test_swift_abstract_class() {
    let mut tb = TypeSpec::<Swift>::builder("Shape", TypeKind::Class);
    tb.doc("Abstract shape base class.");

    // Concrete method.
    let desc_body =
        CodeBlock::<Swift>::of("return String(describing: type(of: self))", ()).unwrap();
    let mut desc = FunSpec::<Swift>::builder("describe");
    desc.returns(TypeName::primitive("String"));
    desc.body(desc_body);
    tb.add_method(desc.build().unwrap());

    // Abstract-like method (fatalError convention).
    let area_body = CodeBlock::<Swift>::of("fatalError(\"Subclasses must override\")", ()).unwrap();
    let mut area = FunSpec::<Swift>::builder("area");
    area.returns(TypeName::primitive("Double"));
    area.body(area_body);
    tb.add_method(area.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("Shape.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/abstract_class.swift", &output);
}

#[test]
fn test_swift_class_extends_implements() {
    let base = TypeName::<Swift>::importable("MyModule", "BaseService");
    let codable = TypeName::<Swift>::importable("Foundation", "Codable");
    let hashable = TypeName::<Swift>::primitive("Hashable");

    // Swift uses `:` for both superclass and protocol conformance.
    let mut tb = TypeSpec::<Swift>::builder("AdminService", TypeKind::Class);
    tb.extends(base);
    tb.extends(codable);
    tb.extends(hashable);

    let body = CodeBlock::<Swift>::of("return true", ()).unwrap();
    let mut is_admin = FunSpec::<Swift>::builder("isAdmin");
    is_admin.returns(TypeName::primitive("Bool"));
    is_admin.body(body);
    tb.add_method(is_admin.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("AdminService.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/class_extends_implements.swift", &output);
}

#[test]
fn test_swift_enum() {
    let mut tb = TypeSpec::<Swift>::builder("Color", TypeKind::Enum);
    tb.visibility(Visibility::Public);
    tb.doc("Supported colors.");

    tb.add_variant(EnumVariantSpec::new("red").unwrap());
    tb.add_variant(EnumVariantSpec::new("green").unwrap());
    tb.add_variant(EnumVariantSpec::new("blue").unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("Color.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/enum.swift", &output);
}

#[test]
fn test_swift_async_function() {
    let user = TypeName::<Swift>::importable("MyModule", "User");

    let body = CodeBlock::<Swift>::of("return try await api.fetchUser(id: id)", ()).unwrap();
    let mut fb_fun = FunSpec::<Swift>::builder("fetchUser");
    fb_fun.is_async();
    fb_fun.returns(user);
    fb_fun.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    fb_fun.body(body);
    let fun = fb_fun.build().unwrap();

    let mut fb = FileSpec::builder_with("Api.swift", Swift::new());
    fb.add_function(fun);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/async_function.swift", &output);
}

#[test]
fn test_swift_override_method() {
    let mut tb = TypeSpec::<Swift>::builder("Dog", TypeKind::Class);
    tb.extends(TypeName::primitive("Animal"));

    let body = CodeBlock::<Swift>::of(
        "return %S",
        (sigil_stitch::code_block::StringLitArg("Woof!".to_string()),),
    )
    .unwrap();
    let mut speak = FunSpec::<Swift>::builder("speak");
    speak.returns(TypeName::primitive("String"));
    speak.is_override();
    speak.body(body);
    tb.add_method(speak.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("Dog.swift", Swift::new());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/override_method.swift", &output);
}

#[test]
fn test_swift_full_module() {
    let url = TypeName::<Swift>::importable("Foundation", "URL");
    let data = TypeName::<Swift>::importable("Foundation", "Data");

    // Protocol.
    let mut proto = TypeSpec::<Swift>::builder("DataFetcher", TypeKind::Interface);

    let mut fetch = FunSpec::<Swift>::builder("fetchData");
    fetch.is_async();
    fetch.returns(data.clone());
    fetch.add_param(ParameterSpec::new("from", TypeName::primitive("URL")).unwrap());
    proto.add_method(fetch.build().unwrap());

    let proto_spec = proto.build().unwrap();

    // Struct.
    let mut model = TypeSpec::<Swift>::builder("Response", TypeKind::Struct);
    model.doc("API response model.");

    let mut status_field = FieldSpec::builder("statusCode", TypeName::primitive("Int"));
    status_field.is_readonly();
    model.add_field(status_field.build().unwrap());

    let mut body_field = FieldSpec::builder("body", TypeName::primitive("Data"));
    body_field.is_readonly();
    model.add_field(body_field.build().unwrap());

    let model_spec = model.build().unwrap();

    // Implementation class.
    let mut cls = TypeSpec::<Swift>::builder("NetworkFetcher", TypeKind::Class);
    cls.extends(TypeName::primitive("DataFetcher"));
    cls.doc("Network-based data fetcher.");

    let mut session_field = FieldSpec::builder("session", TypeName::primitive("URLSession"));
    session_field.visibility(Visibility::Private);
    session_field.is_readonly();
    cls.add_field(session_field.build().unwrap());

    // fetchData implementation.
    let fetch_body = CodeBlock::<Swift>::of(
        "let (data, _) = try await session.data(from: from)\nreturn data",
        (),
    )
    .unwrap();
    let mut fetch_impl = FunSpec::<Swift>::builder("fetchData");
    fetch_impl.is_async();
    fetch_impl.returns(data);
    fetch_impl.add_param(ParameterSpec::new("from", TypeName::primitive("URL")).unwrap());
    fetch_impl.body(fetch_body);
    cls.add_method(fetch_impl.build().unwrap());

    let cls_spec = cls.build().unwrap();

    // Standalone function using URL import.
    let make_body = CodeBlock::<Swift>::of("return %T(string: urlString)!", (url,)).unwrap();
    let mut make_fn = FunSpec::<Swift>::builder("makeURL");
    make_fn.returns(TypeName::primitive("URL"));
    make_fn.add_param(ParameterSpec::new("urlString", TypeName::primitive("String")).unwrap());
    make_fn.body(make_body);
    let make_url = make_fn.build().unwrap();

    let mut fb = FileSpec::builder_with("Network.swift", Swift::new());
    fb.add_type(proto_spec);
    fb.add_type(model_spec);
    fb.add_type(cls_spec);
    fb.add_function(make_url);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/full_module.swift", &output);
}

#[test]
fn test_swift_enum_associated_values() {
    let mut tb = TypeSpec::<Swift>::builder("NetworkResult", TypeKind::Enum);
    tb.visibility(Visibility::Public);
    tb.doc("Result of a network request.");

    // case success(Data)
    let mut v_success = EnumVariantSpec::<Swift>::builder("success");
    v_success.associated_type(TypeName::primitive("Data"));
    tb.add_variant(v_success.build().unwrap());

    // case failure(Error, Int) — multi-element associated value
    let mut v_failure = EnumVariantSpec::<Swift>::builder("failure");
    v_failure.associated_type(TypeName::primitive("Error"));
    v_failure.associated_type(TypeName::primitive("Int"));
    tb.add_variant(v_failure.build().unwrap());

    // case loading — simple variant
    tb.add_variant(EnumVariantSpec::new("loading").unwrap());

    let mut fb = FileSpec::builder_with("NetworkResult.swift", Swift::new());
    fb.add_type(tb.build().unwrap());
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/enum_associated.swift", &output);
}
