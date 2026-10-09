//! Showcase: `sigil_quote!` meta-statements for runtime code generation control.
//!
//! Meta-statements (`$if`, `$for`, `$let`, `$C_each`) control which builder calls
//! are emitted at Rust runtime. They do NOT produce target-language syntax — they
//! control what code gets generated.
//!
//! Run: `cargo run --example meta_programming`

use sigil_stitch::prelude::*;

fn render_ts(block: &CodeBlock) -> String {
    FileSpec::builder("demo.ts")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn main() {
    meta_if_else();
    meta_for_loop();
    meta_let_binding();
    splice_each();
    nested_meta();
    schema_driven_codegen();
}

/// `$if` / `$else_if` / `$else` — conditionally emit code based on Rust values.
fn meta_if_else() {
    println!("--- $if / $else_if / $else ---\n");

    let auth_mode = "bearer";
    let block = sigil_quote!(TypeScript {
        $if(auth_mode == "bearer") {
            const token = getAuthToken();
            headers.set($S("Authorization"), $S("Bearer ") + token);
        } $else_if(auth_mode == "basic") {
            const creds = btoa(username + $S(":") + password);
            headers.set($S("Authorization"), $S("Basic ") + creds);
        } $else_if(auth_mode == "api_key") {
            headers.set($S("X-API-Key"), apiKey);
        } $else {
            headers.set($S("X-Anonymous"), $S("true"));
        }
    })
    .unwrap();

    println!("With auth_mode = \"bearer\":");
    println!("{}", render_ts(&block));

    let auth_mode = "api_key";
    let block2 = sigil_quote!(TypeScript {
        $if(auth_mode == "bearer") {
            const token = getAuthToken();
            headers.set($S("Authorization"), $S("Bearer ") + token);
        } $else_if(auth_mode == "basic") {
            const creds = btoa(username + $S(":") + password);
            headers.set($S("Authorization"), $S("Basic ") + creds);
        } $else_if(auth_mode == "api_key") {
            headers.set($S("X-API-Key"), apiKey);
        } $else {
            headers.set($S("X-Anonymous"), $S("true"));
        }
    })
    .unwrap();

    println!("With auth_mode = \"api_key\":");
    println!("{}", render_ts(&block2));
}

/// `$for` — iterate at Rust runtime, emitting code per element.
fn meta_for_loop() {
    println!("--- $for ---\n");

    let fields = vec!["name", "email", "age"];
    let block = sigil_quote!(TypeScript {
        $for(field in &fields) {
            this.$N(*field) = data.$N(*field);
        }
    })
    .unwrap();

    println!("Generated from fields = {:?}:", fields);
    println!("{}", render_ts(&block));
}

/// `$let` — Rust-level variable binding inside macro bodies.
fn meta_let_binding() {
    println!("--- $let ---\n");

    let fields = vec![("user_name", "String"), ("is_active", "boolean")];
    let block = sigil_quote!(TypeScript {
        $for((name, ty) in &fields) {
            $let(pascal = name
                .split('_')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                })
                .collect::<String>());
            $let(getter = format!("get{}", pascal));
            $N(getter)(): $L(*ty);
        }
    })
    .unwrap();

    println!("Generated getters:");
    println!("{}", render_ts(&block));
}

/// `$C_each` — splice pre-built CodeBlocks sequentially.
fn splice_each() {
    println!("--- $C_each ---\n");

    let field_names = ["host", "port", "debug"];
    let assignments: Vec<CodeBlock> = field_names
        .iter()
        .map(|f| {
            sigil_quote!(TypeScript {
                this.$N(*f) = config.$N(*f);
            })
            .unwrap()
        })
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(assignments);
    })
    .unwrap();

    println!("Spliced {} assignments:", field_names.len());
    println!("{}", render_ts(&block));
}

/// Nested meta-statements — `$if` inside `$for`, `$for` inside `$if`.
fn nested_meta() {
    println!("--- Nested Meta-Statements ---\n");

    let fields: Vec<(&str, &str, bool)> = vec![
        ("id", "number", true),
        ("name", "string", false),
        ("email", "string", true),
    ];

    let block = sigil_quote!(TypeScript {
        $for((name, ty, required) in &fields) {
            $if(*required) {
                $N(*name): $L(*ty);
            } $else {
                $L(format!("{}?: {}", name, ty));
            }
        }
    })
    .unwrap();

    println!("Conditional field generation:");
    println!("{}", render_ts(&block));
}

/// Real-world pattern: generate a TypeScript interface from a schema definition.
fn schema_driven_codegen() {
    println!("--- Schema-Driven Codegen ---\n");

    struct Field {
        name: &'static str,
        ty: &'static str,
        optional: bool,
    }

    let schema = vec![
        Field {
            name: "id",
            ty: "number",
            optional: false,
        },
        Field {
            name: "name",
            ty: "string",
            optional: false,
        },
        Field {
            name: "email",
            ty: "string",
            optional: true,
        },
        Field {
            name: "avatar",
            ty: "string",
            optional: true,
        },
    ];

    let body = sigil_quote!(TypeScript {
        $for(field in &schema) {
            $let(line = if field.optional {
                format!("{}?: {};", field.name, field.ty)
            } else {
                format!("{}: {};", field.name, field.ty)
            });
            $L(line)
        }
    })
    .unwrap();

    let iface = TypeSpec::builder("User", TypeKind::Interface)
        .visibility(Visibility::Public)
        .doc("Auto-generated from schema.")
        .extra_member(body)
        .build()
        .unwrap();

    let mut getters_body = CodeBlock::builder();
    for field in &schema {
        if !field.optional {
            let body = sigil_quote!(TypeScript {
                return this.$N(field.name);
            })
            .unwrap();
            let getter = FunSpec::builder(&format!("get{}", capitalize(field.name)))
                .returns(vec![TypeName::primitive(field.ty)])
                .body(body)
                .build()
                .unwrap();
            getters_body.add_code(
                getter
                    .emit(
                        &sigil_stitch::lang::typescript::TypeScript::new(),
                        DeclarationContext::Member,
                    )
                    .unwrap(),
            );
            getters_body.add_line();
        }
    }

    let cls = TypeSpec::builder("UserImpl", TypeKind::Class)
        .visibility(Visibility::Public)
        .implements(TypeName::primitive("User"))
        .extra_member(getters_body.build().unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder("user.ts")
        .add_type(iface)
        .add_type(cls)
        .build()
        .unwrap();
    println!("{}", file.render(80).unwrap());
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + chars.as_str(),
    }
}
