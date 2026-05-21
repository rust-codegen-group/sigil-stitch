use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_class_with_properties() {
    let find_body = CodeBlock::of("return repo.find(by: id)", ()).unwrap();

    let ts = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("repo", TypeName::primitive("UserRepository"))
                .visibility(Visibility::Private)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("logger", TypeName::primitive("Logger"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findUser")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("User?"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .body(find_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserService.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/class_with_properties.swift", &output);
}

#[test]
fn test_struct() {
    let ts = TypeSpec::builder("User", TypeKind::Struct)
        .visibility(Visibility::Public)
        .doc("A user value type.")
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("age", TypeName::primitive("Int"))
                .visibility(Visibility::Public)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("email", TypeName::primitive("String?"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("User.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/struct.swift", &output);
}

#[test]
fn test_protocol() {
    let tp = TypeParamSpec::new("T");

    let ts = TypeSpec::builder("Repository", TypeKind::Interface)
        .add_type_param(tp)
        .doc("Generic data repository.")
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::primitive("T?"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("delete")
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Repository.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/protocol.swift", &output);
}

#[test]
fn test_abstract_class() {
    let desc_body = CodeBlock::of("return String(describing: type(of: self))", ()).unwrap();
    let area_body = CodeBlock::of("fatalError(\"Subclasses must override\")", ()).unwrap();

    let ts = TypeSpec::builder("Shape", TypeKind::Class)
        .doc("Abstract shape base class.")
        .add_method(
            FunSpec::builder("describe")
                .returns(TypeName::primitive("String"))
                .body(desc_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("area")
                .returns(TypeName::primitive("Double"))
                .body(area_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Shape.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/abstract_class.swift", &output);
}

#[test]
fn test_class_extends_implements() {
    let base = TypeName::importable("MyModule", "BaseService");
    let codable = TypeName::importable("Foundation", "Codable");
    let hashable = TypeName::primitive("Hashable");

    let body = CodeBlock::of("return true", ()).unwrap();
    // Swift uses `:` for both superclass and protocol conformance.
    let ts = TypeSpec::builder("AdminService", TypeKind::Class)
        .extends(base)
        .extends(codable)
        .extends(hashable)
        .add_method(
            FunSpec::builder("isAdmin")
                .returns(TypeName::primitive("Bool"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("AdminService.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/class_extends_implements.swift", &output);
}

#[test]
fn test_enum() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .visibility(Visibility::Public)
        .doc("Supported colors.")
        .add_variant(EnumVariantSpec::new("red").unwrap())
        .add_variant(EnumVariantSpec::new("green").unwrap())
        .add_variant(EnumVariantSpec::new("blue").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Color.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/enum.swift", &output);
}

#[test]
fn test_enum_associated_values() {
    let ts = TypeSpec::builder("NetworkResult", TypeKind::Enum)
        .visibility(Visibility::Public)
        .doc("Result of a network request.")
        .add_variant(
            EnumVariantSpec::builder("success")
                .associated_type(TypeName::primitive("Data"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("failure")
                .associated_type(TypeName::primitive("Error"))
                .associated_type(TypeName::primitive("Int"))
                .build()
                .unwrap(),
        )
        .add_variant(EnumVariantSpec::new("loading").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("NetworkResult.swift", Swift::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/enum_associated.swift", &output);
}
