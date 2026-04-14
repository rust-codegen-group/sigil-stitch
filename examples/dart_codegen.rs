//! Example: Generate a Dart source file with sigil-stitch.
//!
//! Demonstrates:
//! - `import 'dart:...'` / `import 'package:...'` with grouped imports
//! - `abstract class` as interface with method signatures
//! - Concrete `class` with `extends` and `implements`
//! - `final` (readonly) and mutable fields with initializers
//! - `enum` with cases
//! - Generic class with `<T extends Bound>`
//! - `@override` annotations via `annotation()`
//! - `static final` constants with `%S` string literals
//! - `///` dartdoc comments
//! - Type-before-name declarations (`String name`, `User? findById(...)`)
//!
//! Run: `cargo run --example dart_codegen`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // --- Imports (triggered by usage in code) ---
    let future = TypeName::<DartLang>::importable("dart:async", "Future");
    let convert = TypeName::<DartLang>::importable("dart:convert", "jsonDecode");
    let http_client = TypeName::<DartLang>::importable("package:http/http.dart", "Client");

    // --- Enum: Priority ---
    let mut priority = TypeSpec::<DartLang>::builder("Priority", TypeKind::Enum);
    priority.doc("Task priority levels.");

    let mut cases = CodeBlock::<DartLang>::builder();
    cases.add("low,", ());
    cases.add_line();
    cases.add("medium,", ());
    cases.add_line();
    cases.add("high,", ());
    cases.add_line();
    cases.add("critical", ());
    cases.add_line();
    priority.extra_member(cases.build().unwrap());

    let priority_spec = priority.build();

    // --- Abstract class (interface): TaskRepository ---
    let tp = TypeParamSpec::<DartLang>::new("T");

    let mut repo = TypeSpec::<DartLang>::builder("TaskRepository", TypeKind::Interface);
    repo.add_type_param(tp);
    repo.doc("Repository for task persistence.");

    let mut find = FunSpec::<DartLang>::builder("findById");
    find.returns(TypeName::primitive("T?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")));
    repo.add_method(find.build());

    let mut find_all = FunSpec::<DartLang>::builder("findAll");
    find_all.returns(TypeName::primitive("List<T>"));
    repo.add_method(find_all.build());

    let mut save = FunSpec::<DartLang>::builder("save");
    save.add_param(ParameterSpec::new("entity", TypeName::primitive("T")));
    repo.add_method(save.build());

    let repo_spec = repo.build();

    // --- Class: Task ---
    let mut task_cls = TypeSpec::<DartLang>::builder("Task", TypeKind::Class);
    task_cls.doc("A task entity.");

    let id_field = FieldSpec::builder("id", TypeName::primitive("String"));
    task_cls.add_field(id_field.build());

    let name_field = FieldSpec::builder("name", TypeName::primitive("String"));
    task_cls.add_field(name_field.build());

    let mut priority_field = FieldSpec::builder("priority", TypeName::primitive("Priority"));
    priority_field.is_readonly();
    task_cls.add_field(priority_field.build());

    let mut completed_field = FieldSpec::builder("completed", TypeName::primitive("bool"));
    completed_field.initializer(CodeBlock::<DartLang>::of("false", ()).unwrap());
    task_cls.add_field(completed_field.build());

    // Constructor.
    let ctor_body = CodeBlock::<DartLang>::of(
        "this.id = id;\nthis.name = name;\nthis.priority = priority;",
        (),
    )
    .unwrap();
    let mut ctor = FunSpec::<DartLang>::builder("Task");
    ctor.add_param(ParameterSpec::new("id", TypeName::primitive("String")));
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")));
    ctor.add_param(ParameterSpec::new(
        "priority",
        TypeName::primitive("Priority"),
    ));
    ctor.body(ctor_body);
    task_cls.add_method(ctor.build());

    let task_spec = task_cls.build();

    // --- Class: Constants with static final ---
    let mut constants = TypeSpec::<DartLang>::builder("Constants", TypeKind::Class);
    constants.doc("Application constants.");

    let mut max_field = FieldSpec::builder("maxRetries", TypeName::primitive("int"));
    max_field.is_static();
    max_field.is_readonly();
    max_field.initializer(CodeBlock::<DartLang>::of("3", ()).unwrap());
    constants.add_field(max_field.build());

    let mut api_field = FieldSpec::builder("apiUrl", TypeName::primitive("String"));
    api_field.is_static();
    api_field.is_readonly();
    api_field.initializer(
        CodeBlock::<DartLang>::of(
            "%S",
            (StringLitArg("https://api.example.com".to_string()),),
        )
        .unwrap(),
    );
    constants.add_field(api_field.build());

    let constants_spec = constants.build();

    // --- Class: InMemoryTaskRepository extends nothing, implements TaskRepository ---
    let mut impl_cls = TypeSpec::<DartLang>::builder("InMemoryTaskRepository", TypeKind::Class);
    impl_cls.implements(TypeName::primitive("TaskRepository<Task>"));
    impl_cls.doc("In-memory implementation of TaskRepository.");

    let mut tasks_field = FieldSpec::builder("_tasks", TypeName::primitive("List<Task>"));
    tasks_field.is_readonly();
    tasks_field.initializer(CodeBlock::<DartLang>::of("[]", ()).unwrap());
    impl_cls.add_field(tasks_field.build());

    // findById with @override.
    let find_body = CodeBlock::<DartLang>::of(
        "return _tasks.cast<Task?>().firstWhere(\n  (t) => t?.id == id,\n  orElse: () => null,\n);",
        (),
    )
    .unwrap();
    let mut find_impl = FunSpec::<DartLang>::builder("findById");
    find_impl.returns(TypeName::primitive("Task?"));
    find_impl.add_param(ParameterSpec::new("id", TypeName::primitive("String")));
    find_impl.annotation(CodeBlock::<DartLang>::of("@override", ()).unwrap());
    find_impl.body(find_body);
    impl_cls.add_method(find_impl.build());

    // findAll with @override.
    let find_all_body =
        CodeBlock::<DartLang>::of("return List.unmodifiable(_tasks);", ()).unwrap();
    let mut find_all_impl = FunSpec::<DartLang>::builder("findAll");
    find_all_impl.returns(TypeName::primitive("List<Task>"));
    find_all_impl.annotation(CodeBlock::<DartLang>::of("@override", ()).unwrap());
    find_all_impl.body(find_all_body);
    impl_cls.add_method(find_all_impl.build());

    // save with @override.
    let save_body = CodeBlock::<DartLang>::of("_tasks.add(entity);", ()).unwrap();
    let mut save_impl = FunSpec::<DartLang>::builder("save");
    save_impl.add_param(ParameterSpec::new("entity", TypeName::primitive("Task")));
    save_impl.annotation(CodeBlock::<DartLang>::of("@override", ()).unwrap());
    save_impl.body(save_body);
    impl_cls.add_method(save_impl.build());

    let impl_spec = impl_cls.build();

    // --- Generic class: SortedList<T extends Comparable> ---
    let sorted_tp = TypeParamSpec::<DartLang>::new("T")
        .with_bound(TypeName::primitive("Comparable"));

    let mut sorted = TypeSpec::<DartLang>::builder("SortedList", TypeKind::Class);
    sorted.add_type_param(sorted_tp);
    sorted.doc("A sorted list backed by a type-bounded generic.");

    let mut items_field = FieldSpec::builder("_items", TypeName::primitive("List<T>"));
    items_field.is_readonly();
    items_field.initializer(CodeBlock::<DartLang>::of("[]", ()).unwrap());
    sorted.add_field(items_field.build());

    let add_body =
        CodeBlock::<DartLang>::of("_items.add(item);\n_items.sort();", ()).unwrap();
    let mut add_fn = FunSpec::<DartLang>::builder("add");
    add_fn.returns(TypeName::primitive("void"));
    add_fn.add_param(ParameterSpec::new("item", TypeName::primitive("T")));
    add_fn.body(add_body);
    sorted.add_method(add_fn.build());

    let get_body = CodeBlock::<DartLang>::of("return List.unmodifiable(_items);", ()).unwrap();
    let mut get_fn = FunSpec::<DartLang>::builder("items");
    get_fn.returns(TypeName::primitive("List<T>"));
    get_fn.body(get_body);
    sorted.add_method(get_fn.build());

    let sorted_spec = sorted.build();

    // --- Standalone function using imports ---
    let parse_body = CodeBlock::<DartLang>::of(
        "final data = %T(json);\nreturn Task.fromMap(data);",
        (convert,),
    )
    .unwrap();
    let mut parse_fn = FunSpec::<DartLang>::builder("parseTask");
    parse_fn.returns(TypeName::primitive("Task"));
    parse_fn.add_param(ParameterSpec::new("json", TypeName::primitive("String")));
    parse_fn.body(parse_body);
    let parse_task = parse_fn.build();

    // Trigger Future + http imports.
    let future_trigger = CodeBlock::<DartLang>::of("// %T", (future,)).unwrap();
    let http_trigger = CodeBlock::<DartLang>::of("// %T", (http_client,)).unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("task_app.dart", DartLang::new());
    fb.add_code(future_trigger);
    fb.add_code(http_trigger);
    fb.add_type(priority_spec);
    fb.add_type(repo_spec);
    fb.add_type(task_spec);
    fb.add_type(constants_spec);
    fb.add_type(impl_spec);
    fb.add_type(sorted_spec);
    fb.add_function(parse_task);

    let file = fb.build();
    let output = file.render(80).unwrap();
    print!("{output}");
}
