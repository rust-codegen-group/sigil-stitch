//! Example: Generate a Swift source file with sigil-stitch.
//!
//! Demonstrates:
//! - `import` statements with Apple framework / third-party grouping
//! - Generic protocol with method requirements
//! - Struct (value type) with `let` properties
//! - Class with inheritance and protocol conformance (single `:`)
//! - Enum with cases
//! - `async func` for concurrency
//! - `override` via annotations
//! - `///` Swift Markup doc comments
//! - `let`/`var` properties
//! - `@objc` and `@discardableResult` attributes
//!
//! Run: `cargo run --example swift_codegen`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // --- Imports (triggered by usage in code) ---
    let url = TypeName::<Swift>::importable("Foundation", "URL");
    let data = TypeName::<Swift>::importable("Foundation", "Data");
    let json_decoder = TypeName::<Swift>::importable("Foundation", "JSONDecoder");
    let publisher = TypeName::<Swift>::importable("Combine", "AnyPublisher");
    let url_session = TypeName::<Swift>::importable("Foundation", "URLSession");

    // --- Enum: Priority ---
    let mut priority = TypeSpec::<Swift>::builder("Priority", TypeKind::Enum);
    priority.visibility(Visibility::Public);
    priority.doc("Task priority levels.");

    let mut cases = CodeBlock::<Swift>::builder();
    cases.add("case low", ());
    cases.add_line();
    cases.add("case medium", ());
    cases.add_line();
    cases.add("case high", ());
    cases.add_line();
    cases.add("case critical", ());
    cases.add_line();
    priority.extra_member(cases.build().unwrap());

    let priority_spec = priority.build().unwrap();

    // --- Protocol: TaskRepository ---
    let tp = TypeParamSpec::<Swift>::new("T");

    let mut repo_proto = TypeSpec::<Swift>::builder("TaskRepository", TypeKind::Interface);
    repo_proto.add_type_param(tp);
    repo_proto.doc("Repository for task persistence.");
    repo_proto.doc("");
    repo_proto.doc("- Parameter T: the task entity type");

    let mut find = FunSpec::<Swift>::builder("findById");
    find.returns(TypeName::primitive("T?"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    repo_proto.add_method(find.build().unwrap());

    let mut find_all = FunSpec::<Swift>::builder("findAll");
    find_all.returns(TypeName::primitive("[T]"));
    repo_proto.add_method(find_all.build().unwrap());

    let mut save = FunSpec::<Swift>::builder("save");
    save.add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap());
    repo_proto.add_method(save.build().unwrap());

    let repo_spec = repo_proto.build().unwrap();

    // --- Struct: Task ---
    let mut task_struct = TypeSpec::<Swift>::builder("Task", TypeKind::Struct);
    task_struct.visibility(Visibility::Public);
    task_struct.doc("A task entity.");

    let mut id_field = FieldSpec::builder("id", TypeName::primitive("String"));
    id_field.visibility(Visibility::Public);
    id_field.is_readonly();
    task_struct.add_field(id_field.build().unwrap());

    let mut name_field = FieldSpec::builder("name", TypeName::primitive("String"));
    name_field.visibility(Visibility::Public);
    name_field.is_readonly();
    task_struct.add_field(name_field.build().unwrap());

    let mut priority_field = FieldSpec::builder("priority", TypeName::primitive("Priority"));
    priority_field.visibility(Visibility::Public);
    priority_field.is_readonly();
    task_struct.add_field(priority_field.build().unwrap());

    let mut completed_field = FieldSpec::builder("completed", TypeName::primitive("Bool"));
    completed_field.visibility(Visibility::Public);
    completed_field.initializer(CodeBlock::<Swift>::of("false", ()).unwrap());
    task_struct.add_field(completed_field.build().unwrap());

    let task_spec = task_struct.build().unwrap();

    // --- Class: BaseService ---
    let mut base_svc = TypeSpec::<Swift>::builder("BaseService", TypeKind::Class);
    base_svc.visibility(Visibility::Public);
    base_svc.doc("Base class for services with logging.");

    let mut svc_name_field = FieldSpec::builder("serviceName", TypeName::primitive("String"));
    svc_name_field.visibility(Visibility::Public);
    svc_name_field.is_readonly();
    base_svc.add_field(svc_name_field.build().unwrap());

    let log_body = CodeBlock::<Swift>::of("print(\"[\\(serviceName)] \\(message)\")", ()).unwrap();
    let mut log_fn = FunSpec::<Swift>::builder("log");
    log_fn.add_param(ParameterSpec::new("message", TypeName::primitive("String")).unwrap());
    log_fn.body(log_body);
    base_svc.add_method(log_fn.build().unwrap());

    let base_svc_spec = base_svc.build().unwrap();

    // --- Class: TaskService extends BaseService, conforms to TaskRepository ---
    let mut task_svc = TypeSpec::<Swift>::builder("TaskService", TypeKind::Class);
    task_svc.visibility(Visibility::Public);
    task_svc.extends(TypeName::primitive("BaseService"));
    task_svc.extends(TypeName::primitive("TaskRepository"));
    task_svc.doc("Task management service.");

    let mut tasks_field = FieldSpec::builder("tasks", TypeName::primitive("[Task]"));
    tasks_field.visibility(Visibility::Private);
    tasks_field.initializer(CodeBlock::<Swift>::of("[]", ()).unwrap());
    task_svc.add_field(tasks_field.build().unwrap());

    // findById
    let find_body = CodeBlock::<Swift>::of("return tasks.first { $0.id == id }", ()).unwrap();
    let mut find_impl = FunSpec::<Swift>::builder("findById");
    find_impl.returns(TypeName::primitive("Task?"));
    find_impl.add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap());
    find_impl.body(find_body);
    task_svc.add_method(find_impl.build().unwrap());

    // findAll
    let find_all_body = CodeBlock::<Swift>::of("return tasks", ()).unwrap();
    let mut find_all_impl = FunSpec::<Swift>::builder("findAll");
    find_all_impl.returns(TypeName::primitive("[Task]"));
    find_all_impl.body(find_all_body);
    task_svc.add_method(find_all_impl.build().unwrap());

    // save
    let save_body = CodeBlock::<Swift>::of("tasks.append(entity)", ()).unwrap();
    let mut save_impl = FunSpec::<Swift>::builder("save");
    save_impl.add_param(ParameterSpec::new("entity", TypeName::primitive("Task")).unwrap());
    save_impl.body(save_body);
    task_svc.add_method(save_impl.build().unwrap());

    let task_svc_spec = task_svc.build().unwrap();

    // --- Async function: fetchTasks ---
    let fetch_body = CodeBlock::<Swift>::of(
        "let (responseData, _) = try await %T.shared.data(from: endpoint)\nlet decoder = %T()\nreturn try decoder.decode([Task].self, from: responseData)",
        (url_session, json_decoder),
    )
    .unwrap();
    let mut fetch_fn = FunSpec::<Swift>::builder("fetchTasks");
    fetch_fn.visibility(Visibility::Public);
    fetch_fn.is_async();
    fetch_fn.returns(TypeName::primitive("[Task]"));
    fetch_fn.add_param(ParameterSpec::new("endpoint", TypeName::primitive("URL")).unwrap());
    fetch_fn.body(fetch_body);
    let fetch_tasks = fetch_fn.build().unwrap();

    // --- Function using URL + Combine ---
    let create_body = CodeBlock::<Swift>::of(
        "guard let url = %T(string: urlString) else {\n    fatalError(%S)\n}\nreturn url",
        (url, StringLitArg("Invalid URL".to_string())),
    )
    .unwrap();
    let mut create_fn = FunSpec::<Swift>::builder("makeURL");
    create_fn.returns(TypeName::primitive("URL"));
    create_fn.add_param(ParameterSpec::new("urlString", TypeName::primitive("String")).unwrap());
    create_fn.body(create_body);
    let make_url = create_fn.build().unwrap();

    // Trigger Combine import
    let combine_trigger = CodeBlock::<Swift>::of("// Publisher: %T", (publisher,)).unwrap();

    // Trigger Data import
    let data_trigger = CodeBlock::<Swift>::of("// Data: %T", (data,)).unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("TaskApp.swift", Swift::new());
    fb.add_code(combine_trigger);
    fb.add_code(data_trigger);
    fb.add_type(priority_spec);
    fb.add_type(repo_spec);
    fb.add_type(task_spec);
    fb.add_type(base_svc_spec);
    fb.add_type(task_svc_spec);
    fb.add_function(fetch_tasks);
    fb.add_function(make_url);

    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();
    print!("{output}");
}
