mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp_lang::CppLang;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

/// Helper: emit a FunSpec as a CodeBlock for embedding in extra_member.
fn emit_fun(fun: &FunSpec<CppLang>) -> CodeBlock<CppLang> {
    let lang = CppLang::new();
    fun.emit(&lang, DeclarationContext::Member).unwrap()
}

/// Helper: emit a FieldSpec as a CodeBlock for embedding in extra_member.
fn emit_field(field: &FieldSpec<CppLang>) -> CodeBlock<CppLang> {
    let lang = CppLang::new();
    field.emit(&lang, DeclarationContext::Member).unwrap()
}

#[test]
fn test_cpp_class_with_methods() {
    let mut tb = TypeSpec::<CppLang>::builder("Counter", TypeKind::Class);
    tb.doc("A simple counter class.");

    // Build class body with access specifiers via extra_member.
    // private: section
    let mut priv_section = CodeBlock::<CppLang>::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("count_", TypeName::primitive("int"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Constructor
    let ctor_body = CodeBlock::<CppLang>::of("count_ = 0;", ()).unwrap();
    let mut ctor = FunSpec::<CppLang>::builder("Counter");
    ctor.body(ctor_body);
    pub_section.add_code(emit_fun(&ctor.build().unwrap()));

    // increment method
    pub_section.add_line();
    let inc_body = CodeBlock::<CppLang>::of("++count_;", ()).unwrap();
    let mut inc = FunSpec::<CppLang>::builder("increment");
    inc.returns(TypeName::primitive("void"));
    inc.body(inc_body);
    pub_section.add_code(emit_fun(&inc.build().unwrap()));

    // get_count — const method
    pub_section.add_line();
    let get_body = CodeBlock::<CppLang>::of("return count_;", ()).unwrap();
    let mut get = FunSpec::<CppLang>::builder("get_count");
    get.returns(TypeName::primitive("int"));
    get.suffix("const");
    get.body(get_body);
    pub_section.add_code(emit_fun(&get.build().unwrap()));

    tb.extra_member(pub_section.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("counter.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/class_with_methods.cpp", &output);
}

#[test]
fn test_cpp_struct_with_fields() {
    // Struct — data-only, uses emit_split (no methods_inside_type_body).
    let mut tb = TypeSpec::<CppLang>::builder("Point", TypeKind::Struct);
    tb.add_field(
        FieldSpec::builder("x", TypeName::primitive("double"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("y", TypeName::primitive("double"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("z", TypeName::primitive("double"))
            .build()
            .unwrap(),
    );
    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("point.hpp", CppLang::header());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/struct_with_fields.cpp", &output);
}

#[test]
fn test_cpp_enum_class() {
    let mut tb = TypeSpec::<CppLang>::builder("Color", TypeKind::Enum);
    tb.doc("Available colors.");
    tb.add_variant(EnumVariantSpec::new("Red").unwrap());
    tb.add_variant(EnumVariantSpec::new("Green").unwrap());
    tb.add_variant(EnumVariantSpec::new("Blue").unwrap());
    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("color.hpp", CppLang::header());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/enum_class.cpp", &output);
}

#[test]
fn test_cpp_virtual_method() {
    // Abstract class with pure virtual methods.
    let mut tb = TypeSpec::<CppLang>::builder("Shape", TypeKind::Class);
    tb.doc("Abstract shape base class.");

    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Pure virtual: virtual double area() const = 0;
    let mut area = FunSpec::<CppLang>::builder("area");
    area.is_abstract();
    area.returns(TypeName::primitive("double"));
    area.suffix("const");
    area.suffix("= 0");
    pub_section.add_code(emit_fun(&area.build().unwrap()));

    // Virtual destructor: virtual ~Shape() = default;
    pub_section.add_line();
    let mut dtor = FunSpec::<CppLang>::builder("~Shape");
    dtor.is_abstract();
    dtor.suffix("= default");
    pub_section.add_code(emit_fun(&dtor.build().unwrap()));

    tb.extra_member(pub_section.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("shape.hpp", CppLang::header());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/virtual_method.cpp", &output);
}

#[test]
fn test_cpp_const_method() {
    let mut fb = FunSpec::<CppLang>::builder("size");
    fb.returns(TypeName::primitive("int"));
    fb.suffix("const");
    fb.suffix("noexcept");
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("api.hpp", CppLang::header());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/const_method.cpp", &output);
}

#[test]
fn test_cpp_template_function() {
    let mut fb = FunSpec::<CppLang>::builder("max_of");
    fb.annotation(CodeBlock::<CppLang>::of("template<typename T>", ()).unwrap());
    fb.add_param(ParameterSpec::new("a", TypeName::primitive("const T&")).unwrap());
    fb.add_param(ParameterSpec::new("b", TypeName::primitive("const T&")).unwrap());
    fb.returns(TypeName::primitive("T"));
    let body = CodeBlock::<CppLang>::of("return (a > b) ? a : b;", ()).unwrap();
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("algo.hpp", CppLang::header());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/template_function.cpp", &output);
}

#[test]
fn test_cpp_template_class() {
    let mut tb = TypeSpec::<CppLang>::builder("Stack", TypeKind::Class);
    tb.annotation(CodeBlock::<CppLang>::of("template<typename T>", ()).unwrap());
    tb.doc("A simple stack container.");

    // private: section
    let mut priv_section = CodeBlock::<CppLang>::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("data_", TypeName::primitive("std::vector<T>"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    let push_body = CodeBlock::<CppLang>::of("data_.push_back(value);", ()).unwrap();
    let mut push = FunSpec::<CppLang>::builder("push");
    push.add_param(ParameterSpec::new("value", TypeName::primitive("const T&")).unwrap());
    push.returns(TypeName::primitive("void"));
    push.body(push_body);
    pub_section.add_code(emit_fun(&push.build().unwrap()));

    pub_section.add_line();
    let empty_body = CodeBlock::<CppLang>::of("return data_.empty();", ()).unwrap();
    let mut empty = FunSpec::<CppLang>::builder("empty");
    empty.returns(TypeName::primitive("bool"));
    empty.suffix("const");
    empty.body(empty_body);
    pub_section.add_code(emit_fun(&empty.build().unwrap()));

    tb.extra_member(pub_section.build().unwrap());

    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("stack.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/template_class.cpp", &output);
}

#[test]
fn test_cpp_inheritance() {
    // Base class.
    let mut base_tb = TypeSpec::<CppLang>::builder("Animal", TypeKind::Class);

    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());
    let mut speak = FunSpec::<CppLang>::builder("speak");
    speak.is_abstract();
    speak.returns(TypeName::primitive("void"));
    speak.suffix("const");
    speak.suffix("= 0");
    pub_section.add_code(emit_fun(&speak.build().unwrap()));
    base_tb.extra_member(pub_section.build().unwrap());
    let base = base_tb.build().unwrap();

    // Derived class with multiple inheritance.
    let mut derived_tb = TypeSpec::<CppLang>::builder("Dog", TypeKind::Class);
    derived_tb.extends(TypeName::primitive("Animal"));
    derived_tb.extends(TypeName::primitive("Serializable"));

    let mut pub_section2 = CodeBlock::<CppLang>::builder();
    pub_section2.add("%<", ());
    pub_section2.add("public:", ());
    pub_section2.add_line();
    pub_section2.add("%>", ());
    let body = CodeBlock::<CppLang>::of("// bark", ()).unwrap();
    let mut speak_impl = FunSpec::<CppLang>::builder("speak");
    speak_impl.returns(TypeName::primitive("void"));
    speak_impl.suffix("const");
    speak_impl.suffix("override");
    speak_impl.body(body);
    pub_section2.add_code(emit_fun(&speak_impl.build().unwrap()));
    derived_tb.extra_member(pub_section2.build().unwrap());
    let derived = derived_tb.build().unwrap();

    let mut fb = FileSpec::builder_with("animals.hpp", CppLang::header());
    fb.add_type(base);
    fb.add_type(derived);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/inheritance.cpp", &output);
}

#[test]
fn test_cpp_static_method() {
    let body = CodeBlock::<CppLang>::of("return instance_count_;", ()).unwrap();
    let mut fb = FunSpec::<CppLang>::builder("count");
    fb.is_static();
    fb.returns(TypeName::primitive("int"));
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("helpers.cpp", CppLang::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/static_method.cpp", &output);
}

#[test]
fn test_cpp_full_header() {
    let iostream = TypeName::<CppLang>::importable("iostream", "std::cout");
    let string_h = TypeName::<CppLang>::importable("string", "std::string");

    // Logger class inheriting from Base.
    let mut tb = TypeSpec::<CppLang>::builder("Logger", TypeKind::Class);
    tb.extends(TypeName::primitive("Base"));
    tb.doc("Application logger.");

    // private: section
    let mut priv_section = CodeBlock::<CppLang>::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("name_", TypeName::primitive("std::string"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Constructor
    let ctor_body = CodeBlock::<CppLang>::of("name_ = name;", ()).unwrap();
    let mut ctor = FunSpec::<CppLang>::builder("Logger");
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("const std::string&")).unwrap());
    ctor.body(ctor_body);
    pub_section.add_code(emit_fun(&ctor.build().unwrap()));

    // log method using imports
    pub_section.add_line();
    let log_body = CodeBlock::<CppLang>::of(
        "%T << name_ << \": \" << %T(msg) << std::endl;",
        (iostream, string_h),
    )
    .unwrap();
    let mut log = FunSpec::<CppLang>::builder("log");
    log.add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap());
    log.returns(TypeName::primitive("void"));
    log.body(log_body);
    pub_section.add_code(emit_fun(&log.build().unwrap()));

    // name getter — const
    pub_section.add_line();
    let name_body = CodeBlock::<CppLang>::of("return name_;", ()).unwrap();
    let mut name_fn = FunSpec::<CppLang>::builder("name");
    name_fn.returns(TypeName::primitive("const std::string&"));
    name_fn.suffix("const");
    name_fn.body(name_body);
    pub_section.add_code(emit_fun(&name_fn.build().unwrap()));

    tb.extra_member(pub_section.build().unwrap());

    let ts = tb.build().unwrap();

    // Trigger base.hpp import.
    let import_trigger = CodeBlock::<CppLang>::of(
        "// Uses %T",
        (TypeName::<CppLang>::importable("./base.hpp", "Base"),),
    )
    .unwrap();

    let mut fb = FileSpec::builder_with("logger.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_code(import_trigger);
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/full_header.cpp", &output);
}
