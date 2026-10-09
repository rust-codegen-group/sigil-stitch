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
    let url = TypeName::importable("Foundation", "URL");
    let data = TypeName::importable("Foundation", "Data");

    // Protocol.
    let proto_spec = TypeSpec::builder("DataFetcher", TypeKind::Interface)
        .add_method(
            FunSpec::builder("fetchData")
                .is_async()
                .returns(vec![data.clone()])
                .add_param(ParameterSpec::new("from", TypeName::primitive("URL")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // Struct.
    let model_spec = TypeSpec::builder("Response", TypeKind::Struct)
        .doc("API response model.")
        .add_field(
            FieldSpec::builder("statusCode", TypeName::primitive("Int"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("body", TypeName::primitive("Data"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // Implementation class.
    let cls = TypeSpec::builder("NetworkFetcher", TypeKind::Class);
    let cls = cls.extends(TypeName::primitive("DataFetcher"));
    let cls = cls.doc("Network-based data fetcher.");

    let cls = cls.add_field(
        FieldSpec::builder("session", TypeName::primitive("URLSession"))
            .visibility(Visibility::Private)
            .is_readonly()
            .build()
            .unwrap(),
    );

    // fetchData implementation.
    let fetch_body = CodeBlock::of(
        "let (data, _) = try await session.data(from: from)\nreturn data",
        (),
    )
    .unwrap();
    let cls = cls.add_method(
        FunSpec::builder("fetchData")
            .is_async()
            .returns(vec![data])
            .add_param(ParameterSpec::new("from", TypeName::primitive("URL")).unwrap())
            .body(fetch_body)
            .build()
            .unwrap(),
    );

    let cls_spec = cls.build().unwrap();

    // Standalone function using URL import.
    let make_body = CodeBlock::of("return %T(string: urlString)!", (url,)).unwrap();
    let make_url = FunSpec::builder("makeURL")
        .returns(vec![TypeName::primitive("URL")])
        .add_param(ParameterSpec::new("urlString", TypeName::primitive("String")).unwrap())
        .body(make_body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Network.swift", Swift::new())
        .add_type(proto_spec)
        .add_type(model_spec)
        .add_type(cls_spec)
        .add_function(make_url)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/full_module.swift", &output);
}

#[test]
fn test_string_interpolation_escape() {
    let body = CodeBlock::of(
        "let greeting = %S\nlet escaped = %S\nprint(greeting)",
        (
            StringLitArg("Hello \\(name)!".into()),
            StringLitArg("Use \\(expr) for interpolation".into()),
        ),
    )
    .unwrap();
    let fun = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("greet.swift", Swift::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/string_interpolation_escape.swift", &output);
}
