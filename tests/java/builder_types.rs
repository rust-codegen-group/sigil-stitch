use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java_lang::JavaLang;
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
fn test_class_with_methods() {
    // Constructor.
    let ctor_body = CodeBlock::of("this.repo = repo;\nthis.logger = logger;", ()).unwrap();

    // Public method.
    let find_body = CodeBlock::of("return this.repo.findById(id);", ()).unwrap();

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
            FunSpec::builder("UserService")
                .visibility(Visibility::Public)
                .add_param(
                    ParameterSpec::new("repo", TypeName::primitive("UserRepository")).unwrap(),
                )
                .add_param(ParameterSpec::new("logger", TypeName::primitive("Logger")).unwrap())
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findUser")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("User"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .body(find_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserService.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/class_with_methods.java", &output);
}

#[test]
fn test_interface() {
    let tp = TypeParamSpec::new("T");

    let ts = TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .doc("Generic data repository.")
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::primitive("T"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("delete")
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Repository.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/interface.java", &output);
}

#[test]
fn test_abstract_class() {
    // Concrete method.
    let desc_body = CodeBlock::of("return this.getClass().getSimpleName();", ()).unwrap();

    let ts = TypeSpec::builder("Shape", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Abstract shape.")
        .add_method(
            FunSpec::builder("describe")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .body(desc_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("area")
                .visibility(Visibility::Public)
                .is_abstract()
                .returns(TypeName::primitive("double"))
                .build()
                .unwrap(),
        )
        .is_abstract()
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Shape.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/abstract_class.java", &output);
}

#[test]
fn test_class_extends_implements() {
    let base = TypeName::importable("com.example.base", "BaseService");
    let auth = TypeName::importable("com.example.auth", "Authenticatable");
    let serial = TypeName::importable("com.example.serial", "Serializable");

    let body = CodeBlock::of("return true;", ()).unwrap();
    let ts = TypeSpec::builder("AdminService", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(base)
        .implements(auth)
        .implements(serial)
        .add_method(
            FunSpec::builder("isAdmin")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("boolean"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("AdminService.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/class_extends_implements.java", &output);
}

#[test]
fn test_enum() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .visibility(Visibility::Public)
        .doc("Supported colors.")
        .add_variant(EnumVariantSpec::new("RED").unwrap())
        .add_variant(EnumVariantSpec::new("GREEN").unwrap())
        .add_variant(EnumVariantSpec::new("BLUE").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Color.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/enum.java", &output);
}

#[test]
fn test_enum_with_values() {
    let ts = TypeSpec::builder("PetStatus", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(
            EnumVariantSpec::builder("AVAILABLE")
                .value(CodeBlock::of("\"available\"", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("PENDING")
                .value(CodeBlock::of("\"pending\"", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("SOLD")
                .value(CodeBlock::of("\"sold\"", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("String"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("PetStatus")
                .add_param(ParameterSpec::new("value", TypeName::primitive("String")).unwrap())
                .body(CodeBlock::of("this.value = value;", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getValue")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .body(CodeBlock::of("return this.value;", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("PetStatus.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/enum_with_values.java", &output);
}

#[test]
fn test_generic_class() {
    let tp = TypeParamSpec::new("T")
        .with_bound(TypeName::primitive("Comparable"))
        .with_bound(TypeName::primitive("Serializable"));

    let add_body = CodeBlock::of("this.items.add(item);", ()).unwrap();
    let ts = TypeSpec::builder("SortedContainer", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .doc("A sorted container with bounded type parameter.")
        .add_field(
            FieldSpec::builder("items", TypeName::primitive("List<T>"))
                .visibility(Visibility::Private)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("add")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("item", TypeName::primitive("T")).unwrap())
                .body(add_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("SortedContainer.java", JavaLang::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/generic_class.java", &output);
}
