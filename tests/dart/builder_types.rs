use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::dart::Dart;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_class_with_fields() {
    // Constructor.
    let ctor_body = CodeBlock::of("this.repo = repo;\nthis.logger = logger;", ()).unwrap();

    // Method.
    let find_body = CodeBlock::of("return repo.findById(id);", ()).unwrap();

    let ts = TypeSpec::builder("UserService", TypeKind::Class)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("repo", TypeName::primitive("UserRepository"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("logger", TypeName::primitive("Logger"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("UserService")
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
                .returns(TypeName::primitive("User?"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .body(find_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("user_service.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/class_with_fields.dart", &output);
}

#[test]
fn test_abstract_class() {
    // Concrete method.
    let desc_body = CodeBlock::of("return runtimeType.toString();", ()).unwrap();

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
                .returns(TypeName::primitive("double"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("shape.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/abstract_class.dart", &output);
}

#[test]
fn test_class_extends_implements() {
    let base = TypeName::importable("package:myapp/base.dart", "BaseService");
    let auth = TypeName::importable("package:myapp/auth.dart", "Authenticatable");
    let serial = TypeName::importable("package:myapp/serial.dart", "Serializable");

    let body = CodeBlock::of("return true;", ()).unwrap();
    let ts = TypeSpec::builder("AdminService", TypeKind::Class)
        .extends(base)
        .implements(auth)
        .implements(serial)
        .add_method(
            FunSpec::builder("isAdmin")
                .returns(TypeName::primitive("bool"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("admin_service.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/class_extends_implements.dart", &output);
}

#[test]
fn test_enum() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .doc("Supported colors.")
        .add_variant(EnumVariantSpec::new("red").unwrap())
        .add_variant(EnumVariantSpec::new("green").unwrap())
        .add_variant(EnumVariantSpec::new("blue").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("color.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/enum.dart", &output);
}

#[test]
fn test_generic_class() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("Comparable"));

    let add_body = CodeBlock::of("items.add(item);\nitems.sort();", ()).unwrap();
    let ts = TypeSpec::builder("SortedList", TypeKind::Class)
        .add_type_param(tp)
        .doc("A sorted list with bounded type parameter.")
        .add_field(
            FieldSpec::builder("items", TypeName::primitive("List<T>"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("add")
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("item", TypeName::primitive("T")).unwrap())
                .body(add_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("sorted_list.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/generic_class.dart", &output);
}

#[test]
fn test_static_final() {
    let ts = TypeSpec::builder("Constants", TypeKind::Class)
        .add_field(
            FieldSpec::builder("maxSize", TypeName::primitive("int"))
                .is_static()
                .is_readonly()
                .initializer(CodeBlock::of("100", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("appName", TypeName::primitive("String"))
                .is_static()
                .is_readonly()
                .initializer(
                    CodeBlock::of(
                        "%S",
                        (sigil_stitch::code_block::StringLitArg("MyApp".to_string()),),
                    )
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("constants.dart", Dart::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("dart/static_final.dart", &output);
}
