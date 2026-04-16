//! Example: Generate a C++ header file with sigil-stitch.
//!
//! Demonstrates:
//! - `#pragma once` file header
//! - `#include` directives (system and local)
//! - Namespace wrapping via raw content
//! - Abstract base class with pure virtual methods
//! - Derived class with `override` suffix
//! - Template function via annotation
//! - `enum class` (scoped enum)
//! - Method suffixes (`const`, `override`, `= 0`)
//! - Access specifiers (`public:`, `private:`) via extra_member
//!
//! Run: `cargo run --example cpp_codegen`

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp_lang::CppLang;
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

fn main() {
    // --- Imports ---
    let iostream = TypeName::<CppLang>::importable("iostream", "std::cout");
    let string_h = TypeName::<CppLang>::importable("string", "std::string");
    let vector_h = TypeName::<CppLang>::importable("vector", "std::vector");

    // --- Enum class: LogLevel ---
    let mut enum_b = TypeSpec::<CppLang>::builder("LogLevel", TypeKind::Enum);
    enum_b.doc("Severity levels for the logging system.");
    let mut members = CodeBlock::<CppLang>::builder();
    members.add("Debug,", ());
    members.add_line();
    members.add("Info,", ());
    members.add_line();
    members.add("Warning,", ());
    members.add_line();
    members.add("Error", ());
    members.add_line();
    enum_b.extra_member(members.build().unwrap());
    let log_level = enum_b.build().unwrap();

    // --- Abstract base class: Logger ---
    let mut logger_b = TypeSpec::<CppLang>::builder("Logger", TypeKind::Class);
    logger_b.doc("Abstract base class for loggers.");

    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Pure virtual: virtual void log(const char* msg) = 0;
    let mut log_fn = FunSpec::<CppLang>::builder("log");
    log_fn.is_abstract();
    log_fn.add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap());
    log_fn.returns(TypeName::primitive("void"));
    log_fn.suffix("= 0");
    pub_section.add_code(emit_fun(&log_fn.build().unwrap()));

    // Pure virtual: virtual LogLevel level() const = 0;
    pub_section.add_line();
    let mut level_fn = FunSpec::<CppLang>::builder("level");
    level_fn.is_abstract();
    level_fn.returns(TypeName::primitive("LogLevel"));
    level_fn.suffix("const");
    level_fn.suffix("= 0");
    pub_section.add_code(emit_fun(&level_fn.build().unwrap()));

    // Virtual destructor
    pub_section.add_line();
    let mut dtor = FunSpec::<CppLang>::builder("~Logger");
    dtor.is_abstract();
    dtor.suffix("= default");
    pub_section.add_code(emit_fun(&dtor.build().unwrap()));

    logger_b.extra_member(pub_section.build().unwrap());
    let logger = logger_b.build().unwrap();

    // --- Derived class: ConsoleLogger ---
    let mut console_b = TypeSpec::<CppLang>::builder("ConsoleLogger", TypeKind::Class);
    console_b.extends(TypeName::primitive("Logger"));
    console_b.doc("Logger that writes to stdout.");

    // private: section
    let mut priv_section = CodeBlock::<CppLang>::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let name_field = FieldSpec::builder("name_", TypeName::primitive("std::string"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&name_field));
    let level_field = FieldSpec::builder("level_", TypeName::primitive("LogLevel"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&level_field));
    console_b.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section2 = CodeBlock::<CppLang>::builder();
    pub_section2.add_line();
    pub_section2.add("%<", ());
    pub_section2.add("public:", ());
    pub_section2.add_line();
    pub_section2.add("%>", ());

    // Constructor
    let ctor_body =
        CodeBlock::<CppLang>::of("name_ = name;\nlevel_ = LogLevel::Info;", ()).unwrap();
    let mut ctor = FunSpec::<CppLang>::builder("ConsoleLogger");
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("const std::string&")).unwrap());
    ctor.body(ctor_body);
    pub_section2.add_code(emit_fun(&ctor.build().unwrap()));

    // log override
    pub_section2.add_line();
    let log_body = CodeBlock::<CppLang>::of(
        "%T << \"[\" << name_ << \"] \" << %T(msg) << std::endl;",
        (iostream, string_h),
    )
    .unwrap();
    let mut log_impl = FunSpec::<CppLang>::builder("log");
    log_impl.add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap());
    log_impl.returns(TypeName::primitive("void"));
    log_impl.suffix("override");
    log_impl.body(log_body);
    pub_section2.add_code(emit_fun(&log_impl.build().unwrap()));

    // level override
    pub_section2.add_line();
    let level_body = CodeBlock::<CppLang>::of("return level_;", ()).unwrap();
    let mut level_impl = FunSpec::<CppLang>::builder("level");
    level_impl.returns(TypeName::primitive("LogLevel"));
    level_impl.suffix("const");
    level_impl.suffix("override");
    level_impl.body(level_body);
    pub_section2.add_code(emit_fun(&level_impl.build().unwrap()));

    console_b.extra_member(pub_section2.build().unwrap());
    let console_logger = console_b.build().unwrap();

    // --- Template function: make_vector ---
    let mut make_vec_fn = FunSpec::<CppLang>::builder("make_vector");
    make_vec_fn.annotation(CodeBlock::<CppLang>::of("template<typename T>", ()).unwrap());
    make_vec_fn.add_param(ParameterSpec::new("first", TypeName::primitive("T")).unwrap());
    make_vec_fn.add_param(ParameterSpec::new("second", TypeName::primitive("T")).unwrap());
    make_vec_fn.returns(TypeName::primitive("std::vector<T>"));
    let vec_body = CodeBlock::<CppLang>::of(
        "%T<T> result;\nresult.push_back(first);\nresult.push_back(second);\nreturn result;",
        (vector_h,),
    )
    .unwrap();
    make_vec_fn.body(vec_body);
    let make_vector = make_vec_fn.build().unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("logging.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_raw("\nnamespace app {\n\n");
    fb.add_type(log_level);
    fb.add_type(logger);
    fb.add_type(console_logger);
    fb.add_function(make_vector);
    fb.add_raw("\n} // namespace app\n");

    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();
    print!("{output}");
}
