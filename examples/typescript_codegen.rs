//! Generate a TypeScript file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: class, interface with generics, enum, PropertySpec, union types,
//! function types, optional fields, aliased imports, side-effect imports, default
//! parameter values, `next_control_flow` (else-if chaining), and `$for` in object literals.
//!
//! Run: `cargo run --example typescript_codegen`

use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn builder_approach() -> String {
    let user_type = TypeName::importable_type("./models", "User");
    let comment_reason = "Initialize service state";
    let comment_label = "TODO";
    let v_interp = "User";
    let comment_note = "validate user input";
    let not_found = TypeName::importable_type("./errors", "NotFoundError");
    let event_emitter = TypeName::importable("events", "EventEmitter");

    // --- Enum: string-valued variants ---
    let status_enum = TypeSpec::builder("Status", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(
            EnumVariantSpec::builder("Active")
                .discriminant(CodeBlock::of("%S", (StringLitArg("active".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Inactive")
                .discriminant(CodeBlock::of("%S", (StringLitArg("inactive".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Suspended")
                .discriminant(CodeBlock::of("%S", (StringLitArg("suspended".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Interface with generic type param ---
    let repo_iface = TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T"))
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::union(vec![
                        TypeName::primitive("T"),
                        TypeName::primitive("null"),
                    ])],
                ))
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("void")],
                ))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findAll")
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::array(TypeName::primitive("T"))],
                ))
                .add_param(
                    ParameterSpec::builder("limit", TypeName::primitive("number"))
                        .default_value(CodeBlock::of("10", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Class with service methods ---
    let tb = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("userRepo", TypeName::primitive("UserRepository"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("nickname", TypeName::primitive("string"))
                .is_optional()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "onChange",
                TypeName::function(
                    vec![TypeName::primitive("User"), TypeName::primitive("string")],
                    TypeName::primitive("void"),
                ),
            )
            .is_optional()
            .build()
            .unwrap(),
        );

    // PropertySpec: getter/setter
    let get_body = CodeBlock::of("return this.userRepo.count;", ()).unwrap();
    let prop = PropertySpec::builder("totalUsers", TypeName::primitive("number"))
        .getter(get_body)
        .build()
        .unwrap();

    let mut body = CodeBlock::builder();
    body.add_attribute("override");
    body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    body.add("const _user = %V", (VerbatimStrArg(v_interp.to_string()),));
    body.add_line();
    body.add_statement(
        "const user = await this.userRepo.findById(%S) %R",
        (
            StringLitArg("id".into()),
            CommentArg(comment_note.to_string()),
        ),
    );
    body.begin_control_flow("if (!user)", ());
    body.add_statement("throw new %T('User not found')", (not_found,));
    body.end_control_flow();
    body.add_statement("return user", ());

    let get_user = FunSpec::builder("getUser")
        .is_async()
        .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
        .returns(TypeName::generic(
            TypeName::primitive("Promise"),
            vec![user_type],
        ))
        .body(body.build().unwrap())
        .build()
        .unwrap();

    let mut log_body = CodeBlock::builder();
    log_body.add("const emitter = new %T();", (event_emitter,));
    log_body.add_line();
    log_body.add_statement("emitter.emit('log', message)", ());

    let log_fn = FunSpec::builder("log")
        .is_static()
        .add_param(ParameterSpec::new("message", TypeName::primitive("string")).unwrap())
        .body(log_body.build().unwrap())
        .build()
        .unwrap();

    // --- toJson(): $C_each inside object literal (builder API) ---
    let fields = ["id", "name", "email"];
    let mut to_json_body = CodeBlock::builder();
    to_json_body.add("return {", ());
    to_json_body.add_line();
    for f in &fields {
        to_json_body.add(&format!("{f}: this.{f},"), ());
        to_json_body.add_line();
    }
    to_json_body.add("};", ());

    let to_json = FunSpec::builder("toJson")
        .returns(TypeName::primitive("Record<string, unknown>"))
        .body(to_json_body.build().unwrap())
        .build()
        .unwrap();

    let tb = tb
        .add_property(prop)
        .add_method(get_user)
        .add_method(log_fn)
        .add_method(to_json);

    FileSpec::builder("UserService.ts")
        .add_import(ImportSpec::side_effect("reflect-metadata"))
        .add_import(ImportSpec::named_as("lodash", "merge", "deepMerge"))
        .add_type(status_enum)
        .add_type(repo_iface)
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let user_type = TypeName::importable_type("./models", "User");
    let comment_reason = "Initialize service state";
    let comment_label = "TODO";
    let v_interp = "User";
    let comment_note = "validate user input";
    let not_found = TypeName::importable_type("./errors", "NotFoundError");
    let event_emitter = TypeName::importable("events", "EventEmitter");

    // --- Enum: string-valued variants ---
    let status_enum = TypeSpec::builder("Status", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(
            EnumVariantSpec::builder("Active")
                .discriminant(CodeBlock::of("%S", (StringLitArg("active".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Inactive")
                .discriminant(CodeBlock::of("%S", (StringLitArg("inactive".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Suspended")
                .discriminant(CodeBlock::of("%S", (StringLitArg("suspended".into()),)).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Interface with generic type param ---
    let repo_iface = TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T"))
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::union(vec![
                        TypeName::primitive("T"),
                        TypeName::primitive("null"),
                    ])],
                ))
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("void")],
                ))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findAll")
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::array(TypeName::primitive("T"))],
                ))
                .add_param(
                    ParameterSpec::builder("limit", TypeName::primitive("number"))
                        .default_value(CodeBlock::of("10", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Class with service methods ---
    let body = sigil_quote!(TypeScript {
        $attr("override")
        $comment("@{comment_label}: @{comment_reason}");
        const _user = $V("@{v_interp}");
        const user = await this.userRepo.findById($S("id")) $comment(comment_note);
        if(!user) {
            throw new $T(not_found)($S("User not found"));
        }
        return user;
    })
    .unwrap();

    let get_user = FunSpec::builder("getUser")
        .is_async()
        .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
        .returns(TypeName::generic(
            TypeName::primitive("Promise"),
            vec![user_type],
        ))
        .body(body)
        .build()
        .unwrap();

    let log_body = sigil_quote!(TypeScript {
        const emitter = new $T(event_emitter)();
        emitter.emit($S("log"), message);
    })
    .unwrap();

    let log_fn = FunSpec::builder("log")
        .is_static()
        .add_param(ParameterSpec::new("message", TypeName::primitive("string")).unwrap())
        .body(log_body)
        .build()
        .unwrap();

    // --- toJson(): $for inside object literal (sigil_quote! macro) ---
    let fields = ["id", "name", "email"];

    let to_json_body = sigil_quote!(TypeScript {
        return {
            $for(f in &fields) {
                $N(*f): this.$N(*f),
            }
        };
    })
    .unwrap();

    let to_json = FunSpec::builder("toJson")
        .returns(TypeName::primitive("Record<string, unknown>"))
        .body(to_json_body)
        .build()
        .unwrap();

    let get_body = sigil_quote!(TypeScript {
        return this.userRepo.count;
    })
    .unwrap();

    let prop = PropertySpec::builder("totalUsers", TypeName::primitive("number"))
        .getter(get_body)
        .build()
        .unwrap();

    let tb = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("userRepo", TypeName::primitive("UserRepository"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("nickname", TypeName::primitive("string"))
                .is_optional()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "onChange",
                TypeName::function(
                    vec![TypeName::primitive("User"), TypeName::primitive("string")],
                    TypeName::primitive("void"),
                ),
            )
            .is_optional()
            .build()
            .unwrap(),
        )
        .add_property(prop)
        .add_method(get_user)
        .add_method(log_fn)
        .add_method(to_json);

    FileSpec::builder("UserService.ts")
        .add_import(ImportSpec::side_effect("reflect-metadata"))
        .add_import(ImportSpec::named_as("lodash", "merge", "deepMerge"))
        .add_type(status_enum)
        .add_type(repo_iface)
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
