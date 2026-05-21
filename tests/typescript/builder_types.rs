use sigil_stitch::code_block::CodeBlock;
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
fn test_class_with_fields_and_methods() {
    let body = CodeBlock::of("return this.userRepo.findById(id)", ()).unwrap();
    let tb = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("userRepo", TypeName::primitive("UserRepository"))
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
            FunSpec::builder("getUser")
                .is_async()
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::importable_type("./models", "User")],
                ))
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("UserService.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/class_with_methods.ts", &output);
}

#[test]
fn test_interface_generic() {
    let tp = TypeParamSpec::new("T");
    let tb = TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .add_method(
            FunSpec::builder("findById")
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("T")],
                ))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("void")],
                ))
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("Repository.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/interface_generic.ts", &output);
}

#[test]
fn test_abstract_class() {
    let body = CodeBlock::of("console.log('handled')", ()).unwrap();
    let tb = TypeSpec::builder("BaseController", TypeKind::Class)
        .visibility(Visibility::Public)
        .is_abstract()
        .add_method(
            FunSpec::builder("handleRequest")
                .is_abstract()
                .add_param(ParameterSpec::new("req", TypeName::primitive("Request")).unwrap())
                .returns(TypeName::primitive("Response"))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("log")
                .visibility(Visibility::Protected)
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("BaseController.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/abstract_class.ts", &output);
}

#[test]
fn test_class_extends_implements() {
    let body = CodeBlock::of("return true", ()).unwrap();
    let tb = TypeSpec::builder("AdminService", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::importable("./base", "BaseService"))
        .implements(TypeName::importable("./auth", "Authenticatable"))
        .implements(TypeName::importable_type("./serial", "Serializable"))
        .add_method(
            FunSpec::builder("isAdmin")
                .returns(TypeName::primitive("boolean"))
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder("AdminService.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/class_extends_implements.ts", &output);
}

#[test]
fn test_enum() {
    let output = FileSpec::builder("Direction.ts")
        .add_type(
            TypeSpec::builder("Direction", TypeKind::Enum)
                .visibility(Visibility::Public)
                .add_variant(
                    EnumVariantSpec::builder("Up")
                        .value(CodeBlock::of("'UP'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Down")
                        .value(CodeBlock::of("'DOWN'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Left")
                        .value(CodeBlock::of("'LEFT'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Right")
                        .value(CodeBlock::of("'RIGHT'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/enum.ts", &output);
}

#[test]
fn test_readonly_array_field() {
    let tb = TypeSpec::builder("Pet", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "tags",
                TypeName::readonly_array(TypeName::primitive("string")),
            )
            .is_readonly()
            .build()
            .unwrap(),
        );

    let output = FileSpec::builder("Pet.ts")
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(
        output.contains("readonly tags: readonly string[];"),
        "expected readonly array field; got:\n{output}"
    );
}
