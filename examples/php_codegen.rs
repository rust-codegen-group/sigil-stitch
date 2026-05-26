//! Generate a PHP file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: class with properties and constructor, interface with
//! nullable types (`?Type`), trait, enum (PHP 8.1+), PHPDoc comments,
//! `$`-prefixed variables (`$$` escape in macro), match expression,
//! and `use` imports via `TypeName::importable`.
//!
//! Run: `cargo run --example php_codegen`

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn build_shared_types() -> TypeSpec {
    // --- Interface with nullable return type ---
    TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("findById")
                .visibility(Visibility::Public)
                .returns(TypeName::optional(TypeName::primitive("User")))
                .add_param(ParameterSpec::new("id", TypeName::primitive("int")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("User")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn builder_approach() -> String {
    let logger_interface = TypeName::importable("Psr\\Log", "LoggerInterface");

    // --- Trait ---
    let mut log_body = CodeBlock::builder();
    log_body.add_statement("echo \"[INFO] \" . $message . PHP_EOL", ());

    let logger_trait = TypeSpec::builder("LoggerTrait", TypeKind::Trait)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("log")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("message", TypeName::primitive("string")).unwrap())
                .body(log_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Entity class ---
    let entity = TypeSpec::builder("Entity", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("status", TypeName::optional(TypeName::primitive("string")))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("null", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("__construct")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .add_param(
                    ParameterSpec::builder(
                        "status",
                        TypeName::optional(TypeName::primitive("string")),
                    )
                    .default_value(CodeBlock::of("null", ()).unwrap())
                    .build()
                    .unwrap(),
                )
                .body({
                    let mut b = CodeBlock::builder();
                    b.add_statement("$this->name = $name", ());
                    b.add_statement("$this->status = $status", ());
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("describe")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("string"))
                .body({
                    let mut b = CodeBlock::builder();
                    b.add("return match ($this->status) {", ());
                    b.add_line();
                    b.add("%>", ());
                    b.add("null => \"unknown\",", ());
                    b.add_line();
                    b.add("default => $this->status,", ());
                    b.add_line();
                    b.add("%<", ());
                    b.add("};", ());
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- User class (extends Entity) ---
    let user = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Entity"))
        .add_field(
            FieldSpec::builder("email", TypeName::optional(TypeName::primitive("string")))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("null", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("__construct")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .add_param(
                    ParameterSpec::new("email", TypeName::optional(TypeName::primitive("string")))
                        .unwrap(),
                )
                .body({
                    let mut b = CodeBlock::builder();
                    b.add_statement("parent::__construct($name)", ());
                    b.add_statement("$this->email = $email", ());
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getEmail")
                .visibility(Visibility::Public)
                .returns(TypeName::optional(TypeName::primitive("string")))
                .body(CodeBlock::of("return $this->email;", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Standalone function ---
    let create_logger = FunSpec::builder("createLogger")
        .returns(TypeName::primitive("LoggerInterface"))
        .body({
            let mut b = CodeBlock::builder();
            b.add_statement("return new %T()", (logger_interface,));
            b.build().unwrap()
        })
        .build()
        .unwrap();

    let repository = build_shared_types();

    FileSpec::builder_with("user.php", Php::new())
        .add_type(repository)
        .add_type(logger_trait)
        .add_type(entity)
        .add_type(user)
        .add_function(create_logger)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let logger_interface = TypeName::importable("Psr\\Log", "LoggerInterface");

    // --- Trait ---
    let log_body = sigil_quote!(Php {
        echo $S("[INFO] ") . $$message . PHP_EOL;
    })
    .unwrap();

    let logger_trait = TypeSpec::builder("LoggerTrait", TypeKind::Trait)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("log")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("message", TypeName::primitive("string")).unwrap())
                .body(log_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Entity class ---
    let entity = TypeSpec::builder("Entity", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("status", TypeName::optional(TypeName::primitive("string")))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("null", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("__construct")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .add_param(
                    ParameterSpec::builder(
                        "status",
                        TypeName::optional(TypeName::primitive("string")),
                    )
                    .default_value(CodeBlock::of("null", ()).unwrap())
                    .build()
                    .unwrap(),
                )
                .body(
                    sigil_quote!(Php {
                        $$this->name = $$name;
                        $$this->status = $$status;
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("describe")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("string"))
                .body(
                    sigil_quote!(Php {
                        return match ($$this->status) {
                            null => $S("unknown"),
                            default => $$this->status,
                        };
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- User class (extends Entity) ---
    let user = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Entity"))
        .add_field(
            FieldSpec::builder("email", TypeName::optional(TypeName::primitive("string")))
                .visibility(Visibility::Public)
                .initializer(CodeBlock::of("null", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("__construct")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
                .add_param(
                    ParameterSpec::new("email", TypeName::optional(TypeName::primitive("string")))
                        .unwrap(),
                )
                .body(
                    sigil_quote!(Php {
                        parent::__construct($$name);
                        $$this->email = $$email;
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getEmail")
                .visibility(Visibility::Public)
                .returns(TypeName::optional(TypeName::primitive("string")))
                .body(
                    sigil_quote!(Php {
                        return $$this->email;
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Standalone function ---
    let create_logger = FunSpec::builder("createLogger")
        .returns(TypeName::primitive("LoggerInterface"))
        .body(
            sigil_quote!(Php {
                return new $T(logger_interface)();
            })
            .unwrap(),
        )
        .build()
        .unwrap();

    let repository = build_shared_types();

    FileSpec::builder_with("user.php", Php::new())
        .add_type(repository)
        .add_type(logger_trait)
        .add_type(entity)
        .add_type(user)
        .add_function(create_logger)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
