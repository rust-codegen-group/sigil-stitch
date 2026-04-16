//! Phase 2 integration tests: TypeScript structural specs.

mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_ts_class_with_fields_and_methods() {
    let mut tb = TypeSpec::<TypeScript>::builder("UserService", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.doc("Service for managing users.");

    let mut field_b = FieldSpec::builder("userRepo", TypeName::primitive("UserRepository"));
    field_b.visibility(Visibility::Private);
    tb.add_field(field_b.build().unwrap());

    let mut field_b2 = FieldSpec::builder("logger", TypeName::primitive("Logger"));
    field_b2.visibility(Visibility::Private);
    field_b2.is_readonly();
    tb.add_field(field_b2.build().unwrap());

    // Constructor-like method.
    let body = CodeBlock::<TypeScript>::of("return this.userRepo.findById(id)", ()).unwrap();
    let mut fb = FunSpec::builder("getUser");
    fb.is_async();
    fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
    fb.returns(TypeName::generic(
        TypeName::primitive("Promise"),
        vec![TypeName::importable_type("./models", "User")],
    ));
    fb.body(body);
    tb.add_method(fb.build().unwrap());

    let mut file = FileSpec::<TypeScript>::builder("UserService.ts");
    file.add_type(tb.build().unwrap());
    let output = file.build().unwrap().render(80).unwrap();

    golden::assert_golden("typescript/class_with_methods.ts", &output);
}

#[test]
fn test_ts_interface() {
    let mut tb = TypeSpec::<TypeScript>::builder("Repository", TypeKind::Interface);
    tb.visibility(Visibility::Public);

    let tp = TypeParamSpec::<TypeScript>::new("T");
    tb.add_type_param(tp);

    let mut fb = FunSpec::builder("findById");
    fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
    fb.returns(TypeName::generic(
        TypeName::primitive("Promise"),
        vec![TypeName::primitive("T")],
    ));
    tb.add_method(fb.build().unwrap());

    let mut fb2 = FunSpec::builder("save");
    fb2.add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap());
    fb2.returns(TypeName::generic(
        TypeName::primitive("Promise"),
        vec![TypeName::primitive("void")],
    ));
    tb.add_method(fb2.build().unwrap());

    let mut file = FileSpec::<TypeScript>::builder("Repository.ts");
    file.add_type(tb.build().unwrap());
    let output = file.build().unwrap().render(80).unwrap();

    golden::assert_golden("typescript/interface_generic.ts", &output);
}

#[test]
fn test_ts_abstract_class() {
    let mut tb = TypeSpec::<TypeScript>::builder("BaseController", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.is_abstract();

    let mut fb = FunSpec::builder("handleRequest");
    fb.is_abstract();
    fb.add_param(ParameterSpec::new("req", TypeName::primitive("Request")).unwrap());
    fb.returns(TypeName::primitive("Response"));
    tb.add_method(fb.build().unwrap());

    let body = CodeBlock::<TypeScript>::of("console.log('handled')", ()).unwrap();
    let mut fb2 = FunSpec::builder("log");
    fb2.visibility(Visibility::Protected);
    fb2.body(body);
    tb.add_method(fb2.build().unwrap());

    let mut file = FileSpec::<TypeScript>::builder("BaseController.ts");
    file.add_type(tb.build().unwrap());
    let output = file.build().unwrap().render(80).unwrap();

    golden::assert_golden("typescript/abstract_class.ts", &output);
}

#[test]
fn test_ts_class_extends_implements() {
    let mut tb = TypeSpec::<TypeScript>::builder("AdminService", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.extends(TypeName::importable("./base", "BaseService"));
    tb.implements(TypeName::importable("./auth", "Authenticatable"));
    tb.implements(TypeName::importable_type("./serial", "Serializable"));

    let body = CodeBlock::<TypeScript>::of("return true", ()).unwrap();
    let mut fb = FunSpec::builder("isAdmin");
    fb.returns(TypeName::primitive("boolean"));
    fb.body(body);
    tb.add_method(fb.build().unwrap());

    let mut file = FileSpec::<TypeScript>::builder("AdminService.ts");
    file.add_type(tb.build().unwrap());
    let output = file.build().unwrap().render(80).unwrap();

    golden::assert_golden("typescript/class_extends_implements.ts", &output);
}

#[test]
fn test_ts_top_level_function() {
    let tp = TypeParamSpec::<TypeScript>::new("T").with_bound(TypeName::primitive("Serializable"));

    let mut fb = FunSpec::<TypeScript>::builder("serialize");
    fb.visibility(Visibility::Public);
    fb.add_type_param(tp);
    fb.add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap());
    fb.returns(TypeName::primitive("string"));
    let body = CodeBlock::<TypeScript>::of("return JSON.stringify(value)", ()).unwrap();
    fb.body(body);

    let mut file = FileSpec::<TypeScript>::builder("serialize.ts");
    file.add_function(fb.build().unwrap());
    let output = file.build().unwrap().render(80).unwrap();

    golden::assert_golden("typescript/top_level_function.ts", &output);
}

#[test]
fn test_ts_enum() {
    let mut tb = TypeSpec::<TypeScript>::builder("Direction", TypeKind::Enum);
    tb.visibility(Visibility::Public);

    let mut v_up = EnumVariantSpec::<TypeScript>::builder("Up");
    v_up.value(CodeBlock::<TypeScript>::of("'UP'", ()).unwrap());
    tb.add_variant(v_up.build().unwrap());

    let mut v_down = EnumVariantSpec::<TypeScript>::builder("Down");
    v_down.value(CodeBlock::<TypeScript>::of("'DOWN'", ()).unwrap());
    tb.add_variant(v_down.build().unwrap());

    let mut v_left = EnumVariantSpec::<TypeScript>::builder("Left");
    v_left.value(CodeBlock::<TypeScript>::of("'LEFT'", ()).unwrap());
    tb.add_variant(v_left.build().unwrap());

    let mut v_right = EnumVariantSpec::<TypeScript>::builder("Right");
    v_right.value(CodeBlock::<TypeScript>::of("'RIGHT'", ()).unwrap());
    tb.add_variant(v_right.build().unwrap());

    let mut file = FileSpec::<TypeScript>::builder("Direction.ts");
    file.add_type(tb.build().unwrap());
    let output = file.build().unwrap().render(80).unwrap();

    golden::assert_golden("typescript/enum.ts", &output);
}
