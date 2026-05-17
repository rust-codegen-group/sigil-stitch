use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_class_with_properties() {
    let find_body = CodeBlock::of("return repo.findById(id)", ()).unwrap();

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
                .returns(TypeName::primitive("User"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .body(find_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserService.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/class_with_properties.kt", &output);
}

#[test]
fn test_data_class() {
    let ts = TypeSpec::builder("User", TypeKind::Struct)
        .visibility(Visibility::Public)
        .doc("A user data class.")
        .add_primary_constructor_param(
            ParameterSpec::builder("name", TypeName::primitive("String"))
                .is_property()
                .build()
                .unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::builder("age", TypeName::primitive("Int"))
                .is_property()
                .build()
                .unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::builder("email", TypeName::primitive("String"))
                .is_property()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("User.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/data_class.kt", &output);
}

#[test]
fn test_interface() {
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

    let file = FileSpec::builder_with("Repository.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/interface.kt", &output);
}

#[test]
fn test_abstract_class() {
    let desc_body = CodeBlock::of("return this::class.simpleName ?: \"Shape\"", ()).unwrap();

    let ts = TypeSpec::builder("Shape", TypeKind::Class)
        .doc("Abstract shape.")
        .is_abstract()
        .add_method(
            FunSpec::builder("describe")
                .returns(TypeName::primitive("String"))
                .body(desc_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("area")
                .is_abstract()
                .returns(TypeName::primitive("Double"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Shape.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/abstract_class.kt", &output);
}

#[test]
fn test_class_extends_implements() {
    let base = TypeName::importable("com.example.base", "BaseService");
    let auth = TypeName::importable("com.example.auth", "Authenticatable");
    let serial = TypeName::importable("com.example.serial", "Serializable");

    let body = CodeBlock::of("return true", ()).unwrap();
    // Kotlin uses `:` for both extends and implements.
    let ts = TypeSpec::builder("AdminService", TypeKind::Class)
        .extends(base)
        .extends(auth)
        .extends(serial)
        .add_method(
            FunSpec::builder("isAdmin")
                .returns(TypeName::primitive("Boolean"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("AdminService.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/class_extends_implements.kt", &output);
}

#[test]
fn test_enum_class() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .doc("Supported colors.")
        .add_variant(EnumVariantSpec::new("RED").unwrap())
        .add_variant(EnumVariantSpec::new("GREEN").unwrap())
        .add_variant(EnumVariantSpec::new("BLUE").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Color.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/enum_class.kt", &output);
}

#[test]
fn test_enum_with_values() {
    let ts = TypeSpec::builder("Status", TypeKind::Enum)
        .add_primary_constructor_param(
            ParameterSpec::builder("value", TypeName::primitive("String"))
                .is_property()
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("ACTIVE")
                .value(CodeBlock::of("\"active\"", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("INACTIVE")
                .value(CodeBlock::of("\"inactive\"", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getValue")
                .returns(TypeName::primitive("String"))
                .body(CodeBlock::of("return value", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Status.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/enum_with_values.kt", &output);
}

#[test]
fn test_override_method() {
    let body = CodeBlock::of(
        "return %S",
        (sigil_stitch::code_block::StringLitArg("Woof!".to_string()),),
    )
    .unwrap();

    let ts = TypeSpec::builder("Dog", TypeKind::Class)
        .extends(TypeName::primitive("Animal"))
        .add_method(
            FunSpec::builder("speak")
                .returns(TypeName::primitive("String"))
                .is_override()
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Dog.kt", Kotlin::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("kotlin/override_method.kt", &output);
}
