use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::python::Python;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_dataclass() {
    let file = FileSpec::builder_with("config.py", Python::new())
        .add_type(
            TypeSpec::builder("Config", TypeKind::Class)
                .doc("Application configuration.")
                .annotation(CodeBlock::of("@dataclass", ()).unwrap())
                .add_field(
                    FieldSpec::builder("name", TypeName::primitive("str"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("port", TypeName::primitive("int"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("debug", TypeName::primitive("bool"))
                        .initializer(CodeBlock::of("False", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/dataclass.py", &output);
}

#[test]
fn test_class_with_methods() {
    let file = FileSpec::builder_with("service.py", Python::new())
        .add_type(
            TypeSpec::builder("UserService", TypeKind::Class)
                .doc("Service for managing users.")
                .add_field(
                    FieldSpec::builder("_repo", TypeName::primitive("UserRepository"))
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("get_user")
                        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
                        .add_param(
                            ParameterSpec::new("user_id", TypeName::primitive("str")).unwrap(),
                        )
                        .returns(TypeName::primitive("User"))
                        .body(CodeBlock::of("return self._repo.find(user_id)", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("save_user")
                        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
                        .add_param(ParameterSpec::new("user", TypeName::primitive("User")).unwrap())
                        .returns(TypeName::primitive("None"))
                        .body(CodeBlock::of("self._repo.save(user)", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/class_with_methods.py", &output);
}

#[test]
fn test_class_with_bases() {
    let base = TypeName::primitive("BaseService");
    let auth = TypeName::primitive("Authenticatable");

    let file = FileSpec::builder_with("admin.py", Python::new())
        .add_type(
            TypeSpec::builder("AdminService", TypeKind::Class)
                .extends(base)
                .implements(auth)
                .add_method(
                    FunSpec::builder("is_admin")
                        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
                        .returns(TypeName::primitive("bool"))
                        .body(CodeBlock::of("return True", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/class_with_bases.py", &output);
}

#[test]
fn test_protocol() {
    let protocol = TypeName::importable("typing", "Protocol");

    let file = FileSpec::builder_with("repo.py", Python::new())
        .add_type(
            TypeSpec::builder("Repository", TypeKind::Interface)
                .doc("Repository defines data access methods.")
                .extends(protocol)
                .add_method(
                    FunSpec::builder("find_by_id")
                        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
                        .add_param(ParameterSpec::new("id", TypeName::primitive("str")).unwrap())
                        .returns(TypeName::primitive("Entity"))
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("save")
                        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
                        .add_param(
                            ParameterSpec::new("entity", TypeName::primitive("Entity")).unwrap(),
                        )
                        .returns(TypeName::primitive("None"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/protocol.py", &output);
}

#[test]
fn test_enum() {
    let enum_base = TypeName::importable("enum", "Enum");

    let tb = TypeSpec::builder("Direction", TypeKind::Enum)
        .extends(enum_base)
        .add_variant(
            EnumVariantSpec::builder("UP")
                .discriminant(CodeBlock::of("'UP'", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("DOWN")
                .discriminant(CodeBlock::of("'DOWN'", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("LEFT")
                .discriminant(CodeBlock::of("'LEFT'", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("RIGHT")
                .discriminant(CodeBlock::of("'RIGHT'", ()).unwrap())
                .build()
                .unwrap(),
        );

    let file = FileSpec::builder_with("direction.py", Python::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/enum.py", &output);
}
