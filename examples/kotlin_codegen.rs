//! Example: Generate a Kotlin source file with sigil-stitch.
//!
//! Demonstrates:
//! - `import` statements with kotlin/kotlinx/java/javax/third-party grouping
//! - Generic interface with type parameter
//! - Abstract class with KDoc
//! - Data class
//! - Concrete class extending + implementing (all via `:`)
//! - Enum class with constants
//! - `override` annotations
//! - `suspend` functions (coroutines)
//! - `val`/`var` properties
//! - `@JvmStatic` and `@JvmField` annotations
//!
//! Run: `cargo run --example kotlin_codegen`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // --- Imports (triggered by usage in code) ---
    let list = TypeName::<Kotlin>::importable("kotlin.collections", "List");
    let mutable_list = TypeName::<Kotlin>::importable("kotlin.collections", "MutableList");
    let array_list = TypeName::<Kotlin>::importable("kotlin.collections", "ArrayList");
    let coroutine_scope = TypeName::<Kotlin>::importable("kotlinx.coroutines", "CoroutineScope");
    let uuid = TypeName::<Kotlin>::importable("java.util", "UUID");

    // --- Enum class: Priority ---
    let mut priority = TypeSpec::<Kotlin>::builder("Priority", TypeKind::Enum);
    priority.doc("Task priority levels.");

    let mut constants = CodeBlock::<Kotlin>::builder();
    constants.add("LOW,", ());
    constants.add_line();
    constants.add("MEDIUM,", ());
    constants.add_line();
    constants.add("HIGH,", ());
    constants.add_line();
    constants.add("CRITICAL", ());
    constants.add_line();
    priority.extra_member(constants.build().unwrap());

    let priority_spec = priority.build().unwrap();

    // --- Interface: Repository<T> ---
    let tp = TypeParamSpec::<Kotlin>::new("T");

    let mut repo_iface = TypeSpec::<Kotlin>::builder("Repository", TypeKind::Interface);
    repo_iface.add_type_param(tp);
    repo_iface.doc("Generic repository for data persistence.");
    repo_iface.doc("");
    repo_iface.doc("@param T the entity type");

    let mut find = FunSpec::<Kotlin>::builder("findById");
    find.returns(TypeName::primitive("T?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    repo_iface.add_method(find.build().unwrap());

    let mut find_all = FunSpec::<Kotlin>::builder("findAll");
    find_all.returns(list.clone());
    repo_iface.add_method(find_all.build().unwrap());

    let mut save = FunSpec::<Kotlin>::builder("save");
    save.add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap());
    repo_iface.add_method(save.build().unwrap());

    let repo_spec = repo_iface.build().unwrap();

    // --- Data class: Task ---
    let mut task_dc = TypeSpec::<Kotlin>::builder("Task", TypeKind::Struct);
    task_dc.doc("A task entity.");

    let mut id_field = FieldSpec::builder("id", TypeName::primitive("String"));
    id_field.is_readonly();
    task_dc.add_field(id_field.build().unwrap());

    let mut name_field = FieldSpec::builder("name", TypeName::primitive("String"));
    name_field.is_readonly();
    task_dc.add_field(name_field.build().unwrap());

    let mut priority_field = FieldSpec::builder("priority", TypeName::primitive("Priority"));
    priority_field.is_readonly();
    task_dc.add_field(priority_field.build().unwrap());

    let mut completed_field = FieldSpec::builder("completed", TypeName::primitive("Boolean"));
    completed_field.initializer(CodeBlock::<Kotlin>::of("false", ()).unwrap());
    task_dc.add_field(completed_field.build().unwrap());

    let task_spec = task_dc.build().unwrap();

    // --- Abstract class: BaseService ---
    let mut base_svc = TypeSpec::<Kotlin>::builder("BaseService", TypeKind::Class);
    base_svc.is_abstract();
    base_svc.doc("Base class for services with logging.");

    let mut name_f = FieldSpec::builder("serviceName", TypeName::primitive("String"));
    name_f.visibility(Visibility::Protected);
    name_f.is_readonly();
    base_svc.add_field(name_f.build().unwrap());

    // Concrete method
    let log_body = CodeBlock::<Kotlin>::of("println(\"[$serviceName] $message\")", ()).unwrap();
    let mut log_fn = FunSpec::<Kotlin>::builder("log");
    log_fn.visibility(Visibility::Protected);
    log_fn.add_param(ParameterSpec::new("message", TypeName::primitive("String")).unwrap());
    log_fn.body(log_body);
    base_svc.add_method(log_fn.build().unwrap());

    // Abstract method
    let mut init_fn = FunSpec::<Kotlin>::builder("initialize");
    init_fn.is_abstract();
    base_svc.add_method(init_fn.build().unwrap());

    let base_svc_spec = base_svc.build().unwrap();

    // --- Concrete class: TaskService extends BaseService, implements Repository<Task> ---
    let mut task_svc = TypeSpec::<Kotlin>::builder("TaskService", TypeKind::Class);
    // All supertypes go into extends() for Kotlin's single `:` syntax
    task_svc.extends(TypeName::primitive("BaseService"));
    task_svc.extends(TypeName::primitive("Repository<Task>"));
    task_svc.doc("Task management service.");

    let mut tasks_field = FieldSpec::builder("tasks", mutable_list);
    tasks_field.visibility(Visibility::Private);
    tasks_field.is_readonly();
    tasks_field.initializer(CodeBlock::<Kotlin>::of("%T()", (array_list,)).unwrap());
    task_svc.add_field(tasks_field.build().unwrap());

    // initialize override
    let init_body = CodeBlock::<Kotlin>::of(
        "log(%S)",
        (StringLitArg("TaskService initialized".to_string()),),
    )
    .unwrap();
    let mut init_impl = FunSpec::<Kotlin>::builder("initialize");
    init_impl.is_override();
    init_impl.body(init_body);
    task_svc.add_method(init_impl.build().unwrap());

    // findById override — trigger UUID import
    let find_body =
        CodeBlock::<Kotlin>::of("return tasks.firstOrNull { it.id == id }", ()).unwrap();
    let mut find_impl = FunSpec::<Kotlin>::builder("findById");
    find_impl.returns(TypeName::primitive("Task?"));
    find_impl.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    find_impl.is_override();
    find_impl.body(find_body);
    task_svc.add_method(find_impl.build().unwrap());

    // findAll override
    let find_all_body = CodeBlock::<Kotlin>::of("return %T(tasks)", (list.clone(),)).unwrap();
    let mut find_all_impl = FunSpec::<Kotlin>::builder("findAll");
    find_all_impl.returns(list);
    find_all_impl.is_override();
    find_all_impl.body(find_all_body);
    task_svc.add_method(find_all_impl.build().unwrap());

    // save override
    let save_body = CodeBlock::<Kotlin>::of("tasks.add(entity)", ()).unwrap();
    let mut save_impl = FunSpec::<Kotlin>::builder("save");
    save_impl.add_param(ParameterSpec::new("entity", TypeName::primitive("Task")).unwrap());
    save_impl.is_override();
    save_impl.body(save_body);
    task_svc.add_method(save_impl.build().unwrap());

    let task_svc_spec = task_svc.build().unwrap();

    // --- Suspend function: fetchTasks ---
    let fetch_body = CodeBlock::<Kotlin>::of(
        "val service = TaskService()\nservice.initialize()\nreturn service.findAll()",
        (),
    )
    .unwrap();
    let mut fetch_fn = FunSpec::<Kotlin>::builder("fetchTasks");
    fetch_fn.is_async();
    fetch_fn.returns(TypeName::primitive("List<Task>"));
    fetch_fn.body(fetch_body);
    let fetch_tasks = fetch_fn.build().unwrap();

    // --- Standalone function using UUID + CoroutineScope ---
    let create_body = CodeBlock::<Kotlin>::of(
        "return Task(\n    id = %T.randomUUID().toString(),\n    name = name,\n    priority = priority\n)",
        (uuid,),
    )
    .unwrap();
    let mut create_fn = FunSpec::<Kotlin>::builder("createTask");
    create_fn.returns(TypeName::primitive("Task"));
    create_fn.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    create_fn.add_param(ParameterSpec::new("priority", TypeName::primitive("Priority")).unwrap());
    create_fn.body(create_body);
    let create_task = create_fn.build().unwrap();

    // Trigger CoroutineScope import
    let scope_trigger =
        CodeBlock::<Kotlin>::of("// CoroutineScope: %T", (coroutine_scope,)).unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("TaskApp.kt", Kotlin::new());
    fb.add_code(scope_trigger);
    fb.add_type(priority_spec);
    fb.add_type(repo_spec);
    fb.add_type(task_spec);
    fb.add_type(base_svc_spec);
    fb.add_type(task_svc_spec);
    fb.add_function(fetch_tasks);
    fb.add_function(create_task);

    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();
    print!("{output}");
}
