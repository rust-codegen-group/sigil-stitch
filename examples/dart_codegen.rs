//! Generate a Dart file — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: abstract class with abstract method, class inheritance with
//! override, enum with simple variants, async function returning `Future<T>`,
//! generic method with multiple type parameters, interface with generics,
//! and `$T_join` for mixin composition.
//!
//! Run: `cargo run --example dart_codegen`

use sigil_stitch::lang::dart::Dart;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn build_shared_types() -> (TypeSpec, TypeSpec, TypeSpec) {
    // --- Enum: TaskStatus ---
    let status = TypeSpec::builder("TaskStatus", TypeKind::Enum)
        .add_variant(EnumVariantSpec::new("Pending").unwrap())
        .add_variant(EnumVariantSpec::new("InProgress").unwrap())
        .add_variant(EnumVariantSpec::new("Done").unwrap())
        .build()
        .unwrap();

    // --- Abstract class: BaseEntity ---
    let base_entity = TypeSpec::builder("BaseEntity", TypeKind::Class)
        .is_abstract()
        .doc("Base class for all domain entities.")
        .add_field(
            FieldSpec::builder("id", TypeName::primitive("String"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("BaseEntity")
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .body(CodeBlock::of("this.id = id", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .is_abstract()
                .returns(TypeName::primitive("bool"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- Interface: TaskRepository<T> ---
    let repo = TypeSpec::builder("TaskRepository", TypeKind::Interface)
        .add_type_param(TypeParamSpec::new("T"))
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::primitive("T?"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    (status, base_entity, repo)
}

fn builder_approach() -> String {
    let convert = TypeName::importable("dart:convert", "jsonDecode");
    let (status, base_entity, repo) = build_shared_types();
    let comment_label = "FIXME";
    let comment_reason = "Parse task data";
    let comment_note = "decode JSON";
    let v_interp = "Task";

    // --- Class: Task extends BaseEntity ---
    let task = TypeSpec::builder("Task", TypeKind::Class)
        .doc("A task entity.")
        .extends(TypeName::primitive("BaseEntity"))
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("status", TypeName::primitive("TaskStatus"))
                .initializer(CodeBlock::of("TaskStatus.Pending", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("done", TypeName::primitive("bool"))
                .initializer(CodeBlock::of("false", ()).unwrap())
                .build()
                .unwrap(),
        );

    let mut ctor_body = CodeBlock::builder();
    ctor_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    ctor_body.add_statement("super(id) %R", (CommentArg(comment_note.to_string()),));
    ctor_body.add_statement(
        "this.name = name; // %V",
        (VerbatimStrArg(v_interp.to_string()),),
    );

    let task = task
        .add_method(
            FunSpec::builder("Task")
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
                .body(ctor_body.build().unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .is_override()
                .returns(TypeName::primitive("bool"))
                .body({
                    let mut vb = CodeBlock::builder();
                    vb.add_attribute("override");
                    vb.add("return id.isNotEmpty && name.isNotEmpty", ());
                    vb.build().unwrap()
                })
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- parseTask function ---
    let parse_body = CodeBlock::of(
        "final data = %T(json);\nreturn Task(data['id'], data['name']);",
        (convert,),
    )
    .unwrap();
    let parse_task = FunSpec::builder("parseTask")
        .returns(TypeName::primitive("Task"))
        .add_param(ParameterSpec::new("json", TypeName::primitive("String")).unwrap())
        .body(parse_body)
        .build()
        .unwrap();

    // --- Async function: fetchTask ---
    let mut fetch_body = CodeBlock::builder();
    fetch_body.add(
        "final json = await http.get('https://api.example.com/task')",
        (),
    );
    fetch_body.add_line();
    fetch_body.add("return parseTask(json.body)", ());

    let fetch_task = FunSpec::builder("fetchTask")
        .is_async()
        .returns(TypeName::generic(
            TypeName::primitive("Future"),
            vec![TypeName::primitive("Task")],
        ))
        .body(fetch_body.build().unwrap())
        .build()
        .unwrap();

    // --- Generic function: transform<T, R> ---
    let mut transform_body = CodeBlock::builder();
    transform_body.add("return mapper(input)", ());

    let transform = FunSpec::builder("transform")
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::new("R"))
        .returns(TypeName::primitive("R"))
        .add_param(ParameterSpec::new("input", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("mapper", TypeName::primitive("R Function(T)")).unwrap())
        .body(transform_body.build().unwrap())
        .build()
        .unwrap();

    // --- $T_join comparison: manual mixin list ---
    let mixin1 = TypeName::importable("./mixins", "JsonSerializable");
    let mixin2 = TypeName::importable("./mixins", "EquatableMixin");
    let mut join_body = CodeBlock::builder();
    join_body.add("class User extends BaseModel with ", ());
    join_body.add("%T", (mixin1,));
    join_body.add(", ", ());
    join_body.add("%T", (mixin2,));
    join_body.add(" {\n  final String name;\n  User(this.name);\n}", ());

    FileSpec::builder_with("task.dart", Dart::new())
        .add_type(status)
        .add_type(base_entity)
        .add_type(task)
        .add_type(repo)
        .add_function(parse_task)
        .add_function(fetch_task)
        .add_function(transform)
        .add_code(join_body.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let convert = TypeName::importable("dart:convert", "jsonDecode");
    let (status, base_entity, repo) = build_shared_types();
    let comment_label = "FIXME";
    let comment_reason = "Parse task data";
    let comment_note = "decode JSON";
    let v_interp = "Task";

    // --- Class: Task extends BaseEntity ---
    let task = TypeSpec::builder("Task", TypeKind::Class)
        .doc("A task entity.")
        .extends(TypeName::primitive("BaseEntity"))
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("status", TypeName::primitive("TaskStatus"))
                .initializer(CodeBlock::of("TaskStatus.Pending", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("done", TypeName::primitive("bool"))
                .initializer(CodeBlock::of("false", ()).unwrap())
                .build()
                .unwrap(),
        );

    let ctor_body = sigil_quote!(Dart {
        $comment("@{comment_label}: @{comment_reason}");
        super(id); $comment(comment_note)
        this.name = name;
        $V("// @{v_interp} created");
    })
    .unwrap();

    let validate_body = sigil_quote!(Dart {
        $attr("override");
        return id.isNotEmpty && name.isNotEmpty
    })
    .unwrap();

    let task = task
        .add_method(
            FunSpec::builder("Task")
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("validate")
                .is_override()
                .returns(TypeName::primitive("bool"))
                .body(validate_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // --- parseTask function ---
    let parse_body = sigil_quote!(Dart {
        final data = $T(convert)(json);
        return Task(data[$S("id")], data[$S("name")]);
    })
    .unwrap();
    let parse_task = FunSpec::builder("parseTask")
        .returns(TypeName::primitive("Task"))
        .add_param(ParameterSpec::new("json", TypeName::primitive("String")).unwrap())
        .body(parse_body)
        .build()
        .unwrap();

    // --- Async function: fetchTask ---
    let fetch_body = sigil_quote!(Dart {
        final json = await http.get($S("https://api.example.com/task"))
        return parseTask(json.body)
    })
    .unwrap();

    let fetch_task = FunSpec::builder("fetchTask")
        .is_async()
        .returns(TypeName::generic(
            TypeName::primitive("Future"),
            vec![TypeName::primitive("Task")],
        ))
        .body(fetch_body)
        .build()
        .unwrap();

    // --- Generic function: transform<T, R> ---
    let transform_body = sigil_quote!(Dart {
        return mapper(input)
    })
    .unwrap();

    let transform = FunSpec::builder("transform")
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::new("R"))
        .returns(TypeName::primitive("R"))
        .add_param(ParameterSpec::new("input", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("mapper", TypeName::primitive("R Function(T)")).unwrap())
        .body(transform_body)
        .build()
        .unwrap();

    // --- $T_join: mixin composition with import tracking ---
    let mixins = vec![
        TypeName::importable("./mixins", "JsonSerializable"),
        TypeName::importable("./mixins", "EquatableMixin"),
    ];
    let join_body = sigil_quote!(Dart {
        class User extends BaseModel with $T_join(", ", &mixins) {
            final String name;
            User(this.name);
        }
    })
    .unwrap();

    FileSpec::builder_with("task.dart", Dart::new())
        .add_type(status)
        .add_type(base_entity)
        .add_type(task)
        .add_type(repo)
        .add_function(parse_task)
        .add_function(fetch_task)
        .add_function(transform)
        .add_code(join_body)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
