use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

/// Helper: emit a FunSpec as a CodeBlock for embedding in extra_member.
fn emit_fun(fun: &FunSpec) -> CodeBlock {
    let lang = Cpp::new();
    fun.emit(&lang, DeclarationContext::Member).unwrap()
}

/// Helper: emit a FieldSpec as a CodeBlock for embedding in extra_member.
fn emit_field(field: &FieldSpec) -> CodeBlock {
    let lang = Cpp::new();
    field.emit(&lang, DeclarationContext::Member).unwrap()
}

#[test]
fn test_class_with_methods() {
    let tb = TypeSpec::builder("Counter", TypeKind::Class).doc("A simple counter class.");

    // Build class body with access specifiers via extra_member.
    // private: section
    let mut priv_section = CodeBlock::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("count_", TypeName::primitive("int"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    let tb = tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Constructor
    let ctor_body = CodeBlock::of("count_ = 0;", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("Counter").body(ctor_body).build().unwrap(),
    ));

    // increment method
    pub_section.add_line();
    let inc_body = CodeBlock::of("++count_;", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("increment")
            .returns(TypeName::primitive("void"))
            .body(inc_body)
            .build()
            .unwrap(),
    ));

    // get_count — const method
    pub_section.add_line();
    let get_body = CodeBlock::of("return count_;", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("get_count")
            .returns(TypeName::primitive("int"))
            .suffix("const")
            .body(get_body)
            .build()
            .unwrap(),
    ));

    let ts = tb
        .extra_member(pub_section.build().unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("counter.hpp", Cpp::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/class_with_methods.cpp", &output);
}

#[test]
fn test_struct_with_fields() {
    // Struct — data-only, uses emit_split (no methods_inside_type_body).
    let ts = TypeSpec::builder("Point", TypeKind::Struct)
        .add_field(
            FieldSpec::builder("x", TypeName::primitive("double"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("y", TypeName::primitive("double"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("z", TypeName::primitive("double"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("point.hpp", Cpp::header())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/struct_with_fields.cpp", &output);
}

#[test]
fn test_enum_class() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .doc("Available colors.")
        .add_variant(EnumVariantSpec::new("Red").unwrap())
        .add_variant(EnumVariantSpec::new("Green").unwrap())
        .add_variant(EnumVariantSpec::new("Blue").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("color.hpp", Cpp::header())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/enum_class.cpp", &output);
}

#[test]
fn test_virtual_method() {
    // Abstract class with pure virtual methods.
    let tb = TypeSpec::builder("Shape", TypeKind::Class).doc("Abstract shape base class.");

    let mut pub_section = CodeBlock::builder();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Pure virtual: virtual double area() const = 0;
    pub_section.add_code(emit_fun(
        &FunSpec::builder("area")
            .is_abstract()
            .returns(TypeName::primitive("double"))
            .suffix("const")
            .suffix("= 0")
            .build()
            .unwrap(),
    ));

    // Virtual destructor: virtual ~Shape() = default;
    pub_section.add_line();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("~Shape")
            .is_abstract()
            .suffix("= default")
            .build()
            .unwrap(),
    ));

    let ts = tb
        .extra_member(pub_section.build().unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("shape.hpp", Cpp::header())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/virtual_method.cpp", &output);
}

#[test]
fn test_template_class() {
    let tb = TypeSpec::builder("Stack", TypeKind::Class)
        .annotation(CodeBlock::of("template<typename T>", ()).unwrap())
        .doc("A simple stack container.");

    // private: section
    let mut priv_section = CodeBlock::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("data_", TypeName::primitive("std::vector<T>"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    let tb = tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    let push_body = CodeBlock::of("data_.push_back(value);", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("push")
            .add_param(ParameterSpec::new("value", TypeName::primitive("const T&")).unwrap())
            .returns(TypeName::primitive("void"))
            .body(push_body)
            .build()
            .unwrap(),
    ));

    pub_section.add_line();
    let empty_body = CodeBlock::of("return data_.empty();", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("empty")
            .returns(TypeName::primitive("bool"))
            .suffix("const")
            .body(empty_body)
            .build()
            .unwrap(),
    ));

    let ts = tb
        .extra_member(pub_section.build().unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("stack.hpp", Cpp::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/template_class.cpp", &output);
}

#[test]
fn test_inheritance() {
    // Base class.
    let mut pub_section = CodeBlock::builder();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());
    pub_section.add_code(emit_fun(
        &FunSpec::builder("speak")
            .is_abstract()
            .returns(TypeName::primitive("void"))
            .suffix("const")
            .suffix("= 0")
            .build()
            .unwrap(),
    ));
    let base = TypeSpec::builder("Animal", TypeKind::Class)
        .extra_member(pub_section.build().unwrap())
        .build()
        .unwrap();

    // Derived class with multiple inheritance.
    let mut pub_section2 = CodeBlock::builder();
    pub_section2.add("%<", ());
    pub_section2.add("public:", ());
    pub_section2.add_line();
    pub_section2.add("%>", ());
    let body = CodeBlock::of("// bark", ()).unwrap();
    pub_section2.add_code(emit_fun(
        &FunSpec::builder("speak")
            .returns(TypeName::primitive("void"))
            .suffix("const")
            .suffix("override")
            .body(body)
            .build()
            .unwrap(),
    ));
    let derived = TypeSpec::builder("Dog", TypeKind::Class)
        .extends(TypeName::primitive("Animal"))
        .extends(TypeName::primitive("Serializable"))
        .extra_member(pub_section2.build().unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("animals.hpp", Cpp::header())
        .add_type(base)
        .add_type(derived)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/inheritance.cpp", &output);
}
