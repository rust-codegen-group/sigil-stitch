use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::swift::Swift;
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
fn test_string_interpolation_escape() {
    let body = CodeBlock::<Swift>::of(
        "let greeting = %S\nlet escaped = %S\nprint(greeting)",
        (
            StringLitArg("Hello \\(name)!".into()),
            StringLitArg("Use \\(expr) for interpolation".into()),
        ),
    )
    .unwrap();
    let mut fb = FunSpec::<Swift>::builder("greet");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("greet.swift", Swift::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/string_interpolation_escape.swift", &output);
}
