use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

/// Shorthand for a JS parameter (no type annotation).
fn param(name: &str) -> ParameterSpec {
    ParameterSpec::new(name, TypeName::primitive("")).unwrap()
}

/// Shorthand for a JS field (no type annotation).
fn field(name: &str) -> FieldSpec {
    FieldSpec::builder(name, TypeName::primitive(""))
        .build()
        .unwrap()
}

#[test]
fn test_class_with_methods() {
    // Constructor.
    let ctor_body = CodeBlock::of("this.count = 0;", ()).unwrap();
    // increment method.
    let inc_body = CodeBlock::of("this.count++;", ()).unwrap();
    // getCount method.
    let get_body = CodeBlock::of("return this.count;", ()).unwrap();

    let ts = TypeSpec::builder("Counter", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("A simple counter.")
        .add_field(field("count"))
        .add_method(
            FunSpec::builder("constructor")
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("increment")
                .body(inc_body)
                .build()
                .unwrap(),
        )
        .add_method(FunSpec::builder("getCount").body(get_body).build().unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("counter.js", JavaScript::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/class_with_methods.js", &output);
}

#[test]
fn test_class_with_constructor() {
    let ctor_body = CodeBlock::of("this.name = name;\nthis.email = email;", ()).unwrap();

    let ts = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_field(field("name"))
        .add_field(field("email"))
        .add_method(
            FunSpec::builder("constructor")
                .add_param(param("name"))
                .add_param(param("email"))
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("user.js", JavaScript::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/class_with_constructor.js", &output);
}

#[test]
fn test_class_extends() {
    let animal_import = TypeName::importable("./animal", "Animal");

    let ctor_body = CodeBlock::of("super(name);\nthis.breed = breed;", ()).unwrap();
    let speak_body = CodeBlock::of("return 'Woof!';", ()).unwrap();

    let ts = TypeSpec::builder("Dog", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Animal"))
        .add_method(
            FunSpec::builder("constructor")
                .add_param(param("name"))
                .add_param(param("breed"))
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(FunSpec::builder("speak").body(speak_body).build().unwrap())
        .build()
        .unwrap();

    // Trigger import via code block.
    let import_trigger = CodeBlock::of("// Uses %T", (animal_import,)).unwrap();

    let file = FileSpec::builder_with("dog.js", JavaScript::new())
        .add_code(import_trigger)
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/class_extends.js", &output);
}

#[test]
fn test_static_method() {
    let body = CodeBlock::of("return a + b;", ()).unwrap();
    let ts = TypeSpec::builder("MathUtils", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("add")
                .is_static()
                .add_param(param("a"))
                .add_param(param("b"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("math.js", JavaScript::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/static_method.js", &output);
}

#[test]
fn test_private_field() {
    let ctor_body = CodeBlock::of("this.#balance = initialBalance;", ()).unwrap();
    let get_body = CodeBlock::of("return this.#balance;", ()).unwrap();

    let ts = TypeSpec::builder("BankAccount", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_field(field("#balance"))
        .add_method(
            FunSpec::builder("constructor")
                .add_param(param("initialBalance"))
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getBalance")
                .body(get_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("account.js", JavaScript::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/private_field.js", &output);
}

#[test]
fn test_enum() {
    // JavaScript has no native enums. TypeKind::Enum maps to `class`,
    // producing a class with constant-like variant members.
    let file = FileSpec::builder_with("direction.js", JavaScript::new())
        .add_type(
            TypeSpec::builder("Direction", TypeKind::Enum)
                .visibility(Visibility::Public)
                .doc("Cardinal directions.")
                .add_variant(
                    EnumVariantSpec::builder("Up")
                        .discriminant(CodeBlock::of("'UP'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Down")
                        .discriminant(CodeBlock::of("'DOWN'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Left")
                        .discriminant(CodeBlock::of("'LEFT'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .add_variant(
                    EnumVariantSpec::builder("Right")
                        .discriminant(CodeBlock::of("'RIGHT'", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    assert!(output.contains("static Up = 'UP'"));
    assert!(!output.contains("\n  Up = 'UP'"));
    golden::assert_golden("javascript/enum.js", &output);
}
