//! Generate a Java file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: interface with generics, abstract class, enum with constructor-arg
//! values, static/override methods, constructor delegation (`super()`), wildcard
//! type bounds (`? extends T`, `? super T`), protected visibility,
//! import-tracked annotations, and `$T_join` for intersection bounds.
//!
//! Run: `cargo run --example java_codegen`

use sigil_stitch::lang::java::Java;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn build_enum() -> TypeSpec {
    TypeSpec::builder("Priority", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(
            EnumVariantSpec::builder("LOW")
                .value(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("MEDIUM")
                .value(CodeBlock::of("2", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("HIGH")
                .value(CodeBlock::of("3", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("int"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn build_interface() -> TypeSpec {
    let optional = TypeName::importable("java.util", "Optional");
    let list = TypeName::importable("java.util", "List");

    TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T"))
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::generic(optional, vec![TypeName::primitive("T")]))
                .add_param(ParameterSpec::new("id", TypeName::primitive("long")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("findAll")
                .returns(TypeName::generic(list, vec![TypeName::primitive("T")]))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("addAll")
                .returns(TypeName::primitive("void"))
                .add_param(
                    ParameterSpec::new(
                        "items",
                        TypeName::generic(
                            TypeName::importable("java.util", "Collection"),
                            vec![TypeName::wildcard_extends(TypeName::primitive("T"))],
                        ),
                    )
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn builder_approach() -> String {
    let priority_enum = build_enum();
    let repo_iface = build_interface();
    let comment_label = "TODO";
    let comment_reason = "Return entity name";
    let comment_note = "access field";
    let v_interp = "entity";

    // --- Abstract base class ---
    let mut get_name_body = CodeBlock::builder();
    get_name_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    get_name_body.add_statement(
        "return this.name; // %V %R",
        (
            VerbatimStrArg(v_interp.to_string()),
            CommentArg(comment_note.to_string()),
        ),
    );

    let base_entity = TypeSpec::builder("BaseEntity", TypeKind::Class)
        .visibility(Visibility::Public)
        .is_abstract()
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Protected)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("BaseEntity")
                .visibility(Visibility::Protected)
                .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
                .body({
                    let mut b = CodeBlock::builder();
                    b.add_statement("this.name = name", ());
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getName")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .body(get_name_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .visibility(Visibility::Public)
                .is_abstract()
                .returns(TypeName::primitive("boolean"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Concrete class with override and delegation ---
    let mut validate_body = CodeBlock::builder();
    validate_body.add_attribute("Override");
    validate_body.add_statement("return this.name != null && !this.name.isEmpty()", ());

    let mut to_string_body = CodeBlock::builder();
    to_string_body.add_statement(
        "return \"User{name=\" + this.name + \", email=\" + this.email + \"}\"",
        (),
    );

    let user_cls = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("BaseEntity"))
        .add_field(
            FieldSpec::builder("email", TypeName::primitive("String"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("User")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
                .add_param(ParameterSpec::new("email", TypeName::primitive("String")).unwrap())
                .delegation(CodeBlock::of("super(name)", ()).unwrap())
                .body({
                    let mut b = CodeBlock::builder();
                    b.add_statement("this.email = email", ());
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("boolean"))
                .body(validate_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getEmail")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .body({
                    let mut b = CodeBlock::builder();
                    b.add_statement("return this.email", ());
                    b.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("toString")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("String"))
                .body(to_string_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Static utility method with generic where-bound ---
    let collections = TypeName::importable("java.util", "Collections");
    let comparable = TypeName::primitive("Comparable");
    let mut sort_body = CodeBlock::builder();
    sort_body.add_statement("%T.sort(list)", (collections,));
    sort_body.add_statement("return list", ());

    let sort_fn = FunSpec::builder("sortList")
        .visibility(Visibility::Public)
        .is_static()
        .add_type_param(TypeParamSpec::new("T").with_bound(comparable))
        .add_param(
            ParameterSpec::new(
                "list",
                TypeName::generic(
                    TypeName::importable("java.util", "List"),
                    vec![TypeName::primitive("T")],
                ),
            )
            .unwrap(),
        )
        .returns(TypeName::generic(
            TypeName::importable("java.util", "List"),
            vec![TypeName::primitive("T")],
        ))
        .body(sort_body.build().unwrap())
        .build()
        .unwrap();

    // --- $T_join comparison: manual intersection bounds ---
    let serializable = TypeName::importable("java.io", "Serializable");
    let comparable_bound = TypeName::importable("java.lang", "Comparable");
    let mut join_body = CodeBlock::builder();
    join_body.add("class Box<T extends ", ());
    join_body.add("%T", (serializable,));
    join_body.add(" & ", ());
    join_body.add("%T", (comparable_bound,));
    join_body.add(
        "> {\n    private T value;\n    Box(T value) { this.value = value; }\n}",
        (),
    );

    FileSpec::builder_with("User.java", Java::new())
        .add_type(priority_enum)
        .add_type(repo_iface)
        .add_type(base_entity)
        .add_type(user_cls)
        .add_function(sort_fn)
        .add_code(join_body.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let priority_enum = build_enum();
    let repo_iface = build_interface();
    let comment_label = "TODO";
    let comment_reason = "Return entity name";
    let comment_note = "access field";
    let v_interp = "entity";

    // --- Abstract base class ---
    let base_entity = TypeSpec::builder("BaseEntity", TypeKind::Class)
        .visibility(Visibility::Public)
        .is_abstract()
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Protected)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("BaseEntity")
                .visibility(Visibility::Protected)
                .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
                .body(
                    sigil_quote!(Java {
                        $comment("@{comment_label}: @{comment_reason}");
                        this.name = name;
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getName")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .body(sigil_quote!(Java { $V("// @{v_interp} name"); return this.name; $comment(comment_note) }).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .visibility(Visibility::Public)
                .is_abstract()
                .returns(TypeName::primitive("boolean"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Concrete class ---
    let user_cls = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("BaseEntity"))
        .add_field(
            FieldSpec::builder("email", TypeName::primitive("String"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("User")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
                .add_param(ParameterSpec::new("email", TypeName::primitive("String")).unwrap())
                .delegation(CodeBlock::of("super(name)", ()).unwrap())
                .body(sigil_quote!(Java { this.email = email; }).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("boolean"))
                .body(
                    sigil_quote!(Java {
                        $attr("Override");
                        return this.name != null && !this.name.isEmpty();
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getEmail")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("String"))
                .body(sigil_quote!(Java { return this.email; }).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("toString")
                .visibility(Visibility::Public)
                .is_override()
                .returns(TypeName::primitive("String"))
                .body(
                    sigil_quote!(Java {
                        return "User{name=" + this.name + ", email=" + this.email + "}";
                    })
                    .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Static generic method ---
    let collections = TypeName::importable("java.util", "Collections");
    let sort_body = sigil_quote!(Java {
        $T(collections).sort(list);
        return list;
    })
    .unwrap();

    let sort_fn = FunSpec::builder("sortList")
        .visibility(Visibility::Public)
        .is_static()
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("Comparable")))
        .add_param(
            ParameterSpec::new(
                "list",
                TypeName::generic(
                    TypeName::importable("java.util", "List"),
                    vec![TypeName::primitive("T")],
                ),
            )
            .unwrap(),
        )
        .returns(TypeName::generic(
            TypeName::importable("java.util", "List"),
            vec![TypeName::primitive("T")],
        ))
        .body(sort_body)
        .build()
        .unwrap();

    // --- $T_join: intersection bounds with import tracking ---
    let bounds = vec![
        TypeName::importable("java.io", "Serializable"),
        TypeName::importable("java.lang", "Comparable"),
    ];
    let join_body = sigil_quote!(Java {
        class Box<T extends $T_join(" & ", &bounds)> {
            private T value;
            Box(T value) { this.value = value; }
        }
    })
    .unwrap();

    FileSpec::builder_with("User.java", Java::new())
        .add_type(priority_enum)
        .add_type(repo_iface)
        .add_type(base_entity)
        .add_type(user_cls)
        .add_function(sort_fn)
        .add_code(join_body)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
