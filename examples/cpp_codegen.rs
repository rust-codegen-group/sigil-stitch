//! Generate a C++ header file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: abstract class, inheritance, templates, scoped enum,
//! reference and pointer types, struct with fields, and static methods.
//!
//! Run: `cargo run --example cpp_codegen`

use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn emit_fun(fun: &FunSpec) -> CodeBlock {
    fun.emit(&Cpp::new(), DeclarationContext::Member).unwrap()
}

fn builder_approach() -> String {
    let iostream = TypeName::importable("iostream", "std::cout");
    let vector_h = TypeName::importable("vector", "std::vector");
    let comment_label = "INFO";
    let comment_reason = "Initialize logging components";
    let comment_note = "log to stdout";
    let v_interp = "stdout";

    let logger = {
        let mut pub_section = CodeBlock::builder();
        pub_section.add_comment(&format!("{}: {}", comment_label, comment_reason));
        pub_section.add("%<", ());
        pub_section.add("public: %R", (CommentArg(comment_note.to_string()),));
        pub_section.add_line();
        pub_section.add("%>", ());
        pub_section.add_code(emit_fun(
            &FunSpec::builder("log")
                .is_abstract()
                .add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap())
                .returns(TypeName::primitive("void"))
                .suffix("= 0")
                .build()
                .unwrap(),
        ));
        pub_section.add_line();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("~Logger")
                .is_abstract()
                .suffix("= default")
                .build()
                .unwrap(),
        ));

        TypeSpec::builder("Logger", TypeKind::Class)
            .doc("Abstract base class for loggers.")
            .extra_member(pub_section.build().unwrap())
            .build()
            .unwrap()
    };

    let console = {
        let mut priv_section = CodeBlock::builder();
        priv_section.add("%<", ());
        priv_section.add("private:", ());
        priv_section.add_line();
        priv_section.add("%>", ());
        priv_section.add("std::string name_;", ());
        priv_section.add_line();

        let mut pub_section = CodeBlock::builder();
        pub_section.add_line();
        pub_section.add("%<", ());
        pub_section.add("public:", ());
        pub_section.add_line();
        pub_section.add("%>", ());

        let mut ctor_build = CodeBlock::builder();
        ctor_build.add(
            "name_ = name; // %V",
            (VerbatimStrArg(v_interp.to_string()),),
        );
        let ctor_body = ctor_build.build().unwrap();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("ConsoleLogger")
                .add_param(
                    ParameterSpec::new("name", TypeName::primitive("const std::string&")).unwrap(),
                )
                .body(ctor_body)
                .build()
                .unwrap(),
        ));
        pub_section.add_line();

        let log_body = CodeBlock::of(
            "%T << \"[\" << name_ << \"] \" << msg << std::endl; %R",
            (iostream.clone(), CommentArg(comment_note.to_string())),
        )
        .unwrap();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("log")
                .add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap())
                .returns(TypeName::primitive("void"))
                .suffix("override")
                .body(log_body)
                .build()
                .unwrap(),
        ));
        pub_section.add_line();

        // Static method: defaultLevel
        let mut default_level_build = CodeBlock::builder();
        default_level_build.add_attribute("nodiscard");
        default_level_build.add("return LogLevel::Info;", ());
        let default_level_body = default_level_build.build().unwrap();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("defaultLevel")
                .is_static()
                .returns(TypeName::primitive("LogLevel"))
                .body(default_level_body)
                .build()
                .unwrap(),
        ));

        TypeSpec::builder("ConsoleLogger", TypeKind::Class)
            .extends(TypeName::primitive("Logger"))
            .doc("Logger that writes to stdout.")
            .extra_member(priv_section.build().unwrap())
            .extra_member(pub_section.build().unwrap())
            .build()
            .unwrap()
    };

    // --- Scoped enum: LogLevel ---
    let log_level = TypeSpec::builder("LogLevel", TypeKind::Enum)
        .doc("Severity levels for log messages.")
        .add_variant(EnumVariantSpec::new("Debug").unwrap())
        .add_variant(EnumVariantSpec::new("Info").unwrap())
        .add_variant(EnumVariantSpec::new("Warning").unwrap())
        .add_variant(EnumVariantSpec::new("Error").unwrap())
        .build()
        .unwrap();

    // --- Struct: Config ---
    let config = TypeSpec::builder("Config", TypeKind::Struct)
        .doc("Runtime configuration for the logging system.")
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("std::string"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("level", TypeName::primitive("LogLevel"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Free function with reference and pointer params: logAll ---
    let log_all_body = CodeBlock::of(
        "for (const auto& msg : messages) {\n    logger->log(msg.c_str());\n}",
        (),
    )
    .unwrap();
    let log_all = FunSpec::builder("logAll")
        .add_param(
            ParameterSpec::new(
                "messages",
                TypeName::reference(TypeName::primitive("std::vector<std::string>")),
            )
            .unwrap(),
        )
        .add_param(
            ParameterSpec::new("logger", TypeName::pointer(TypeName::primitive("Logger"))).unwrap(),
        )
        .returns(TypeName::primitive("void"))
        .body(log_all_body)
        .build()
        .unwrap();

    // --- Template function: make_vector ---
    let vec_body = CodeBlock::of(
        "%T<T> result;\nresult.push_back(first);\nresult.push_back(second);\nreturn result;",
        (vector_h,),
    )
    .unwrap();
    let make_vector = FunSpec::builder("make_vector")
        .annotation(CodeBlock::of("template<typename T>", ()).unwrap())
        .add_param(ParameterSpec::new("first", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("second", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("std::vector<T>"))
        .body(vec_body)
        .build()
        .unwrap();

    FileSpec::builder_with("logging.hpp", Cpp::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(log_level)
        .add_type(config)
        .add_type(logger)
        .add_type(console)
        .add_function(log_all)
        .add_function(make_vector)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let iostream = TypeName::importable("iostream", "std::cout");
    let vector_h = TypeName::importable("vector", "std::vector");
    let comment_label = "INFO";
    let comment_reason = "Initialize logging components";
    let comment_note = "log to stdout";
    let v_interp = "stdout";

    let logger = {
        let mut pub_section = CodeBlock::builder();
        pub_section.add("%<", ());
        pub_section.add("public:", ());
        pub_section.add_line();
        pub_section.add("%>", ());
        pub_section.add_code(emit_fun(
            &FunSpec::builder("log")
                .is_abstract()
                .add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap())
                .returns(TypeName::primitive("void"))
                .suffix("= 0")
                .build()
                .unwrap(),
        ));
        pub_section.add_line();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("~Logger")
                .is_abstract()
                .suffix("= default")
                .build()
                .unwrap(),
        ));

        TypeSpec::builder("Logger", TypeKind::Class)
            .doc("Abstract base class for loggers.")
            .extra_member(pub_section.build().unwrap())
            .build()
            .unwrap()
    };

    let console = {
        let mut priv_section = CodeBlock::builder();
        priv_section.add("%<", ());
        priv_section.add("private:", ());
        priv_section.add_line();
        priv_section.add("%>", ());
        priv_section.add("std::string name_;", ());
        priv_section.add_line();

        let mut pub_section = CodeBlock::builder();
        pub_section.add_line();
        pub_section.add("%<", ());
        pub_section.add("public:", ());
        pub_section.add_line();
        pub_section.add("%>", ());

        let ctor_body = sigil_quote!(Cpp {
            $comment("@{comment_label}: @{comment_reason}");
            name_ = name; $comment(comment_note)
        })
        .unwrap();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("ConsoleLogger")
                .add_param(
                    ParameterSpec::new("name", TypeName::primitive("const std::string&")).unwrap(),
                )
                .body(ctor_body)
                .build()
                .unwrap(),
        ));
        pub_section.add_line();

        let log_body = sigil_quote!(Cpp {
            $T(iostream) << $V("@{v_interp}: ") << "[" << name_ << "] " << msg << std::endl;
        })
        .unwrap();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("log")
                .add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap())
                .returns(TypeName::primitive("void"))
                .suffix("override")
                .body(log_body)
                .build()
                .unwrap(),
        ));
        pub_section.add_line();

        // Static method: defaultLevel
        let default_level_body = sigil_quote!(Cpp {
            $attr("nodiscard");
            return LogLevel::Info;
        })
        .unwrap();
        pub_section.add_code(emit_fun(
            &FunSpec::builder("defaultLevel")
                .is_static()
                .returns(TypeName::primitive("LogLevel"))
                .body(default_level_body)
                .build()
                .unwrap(),
        ));

        TypeSpec::builder("ConsoleLogger", TypeKind::Class)
            .extends(TypeName::primitive("Logger"))
            .doc("Logger that writes to stdout.")
            .extra_member(priv_section.build().unwrap())
            .extra_member(pub_section.build().unwrap())
            .build()
            .unwrap()
    };

    // --- Scoped enum: LogLevel ---
    let log_level = TypeSpec::builder("LogLevel", TypeKind::Enum)
        .doc("Severity levels for log messages.")
        .add_variant(EnumVariantSpec::new("Debug").unwrap())
        .add_variant(EnumVariantSpec::new("Info").unwrap())
        .add_variant(EnumVariantSpec::new("Warning").unwrap())
        .add_variant(EnumVariantSpec::new("Error").unwrap())
        .build()
        .unwrap();

    // --- Struct: Config ---
    let config = TypeSpec::builder("Config", TypeKind::Struct)
        .doc("Runtime configuration for the logging system.")
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("std::string"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("level", TypeName::primitive("LogLevel"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Free function with reference and pointer params: logAll ---
    let log_all_body = sigil_quote!(Cpp {
        for (const auto& msg : messages) {
            logger->log(msg.c_str());
        }
    })
    .unwrap();
    let log_all = FunSpec::builder("logAll")
        .add_param(
            ParameterSpec::new(
                "messages",
                TypeName::reference(TypeName::primitive("std::vector<std::string>")),
            )
            .unwrap(),
        )
        .add_param(
            ParameterSpec::new("logger", TypeName::pointer(TypeName::primitive("Logger"))).unwrap(),
        )
        .returns(TypeName::primitive("void"))
        .body(log_all_body)
        .build()
        .unwrap();

    // --- Template function: make_vector ---
    let vec_body = sigil_quote!(Cpp {
        $T(vector_h)<T> result;
        result.push_back(first);
        result.push_back(second);
        return result;
    })
    .unwrap();
    let make_vector = FunSpec::builder("make_vector")
        .annotation(CodeBlock::of("template<typename T>", ()).unwrap())
        .add_param(ParameterSpec::new("first", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("second", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("std::vector<T>"))
        .body(vec_body)
        .build()
        .unwrap();

    FileSpec::builder_with("logging.hpp", Cpp::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(log_level)
        .add_type(config)
        .add_type(logger)
        .add_type(console)
        .add_function(log_all)
        .add_function(make_vector)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
