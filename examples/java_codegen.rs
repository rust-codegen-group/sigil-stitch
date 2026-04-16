//! Example: Generate a Java source file with sigil-stitch.
//!
//! Demonstrates:
//! - `import` statements with java/javax/third-party grouping
//! - Generic interface with bounded type parameter
//! - Abstract class with Javadoc
//! - Concrete class extending + implementing
//! - Enum with constants
//! - `@Override` and `@Nullable` annotations
//! - `static final` constants
//! - Constructors (no return type)
//!
//! Run: `cargo run --example java_codegen`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // --- Imports (triggered by usage in code) ---
    let list = TypeName::<JavaLang>::importable("java.util", "List");
    let array_list = TypeName::<JavaLang>::importable("java.util", "ArrayList");
    let optional = TypeName::<JavaLang>::importable("java.util", "Optional");
    let nullable = TypeName::<JavaLang>::importable("javax.annotation", "Nullable");
    let logger = TypeName::<JavaLang>::importable("org.slf4j", "Logger");
    let logger_factory = TypeName::<JavaLang>::importable("org.slf4j", "LoggerFactory");

    // --- Enum: Priority ---
    let mut priority = TypeSpec::<JavaLang>::builder("Priority", TypeKind::Enum);
    priority.visibility(Visibility::Public);
    priority.doc("Task priority levels.");

    let mut constants = CodeBlock::<JavaLang>::builder();
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

    // --- Interface: TaskRepository<T> ---
    let tp = TypeParamSpec::<JavaLang>::new("T").with_bound(TypeName::primitive("Serializable"));

    let mut repo_iface = TypeSpec::<JavaLang>::builder("TaskRepository", TypeKind::Interface);
    repo_iface.visibility(Visibility::Public);
    repo_iface.add_type_param(tp);
    repo_iface.doc("Repository for task persistence.");
    repo_iface.doc("");
    repo_iface.doc("@param <T> the task entity type");

    let mut find = FunSpec::<JavaLang>::builder("findById");
    find.returns(TypeName::primitive("T"));
    find.add_param(ParameterSpec::new("id", TypeName::primitive("long")).unwrap());
    find.annotation(CodeBlock::<JavaLang>::of("@%T", (nullable.clone(),)).unwrap());
    repo_iface.add_method(find.build().unwrap());

    let mut find_all = FunSpec::<JavaLang>::builder("findAll");
    find_all.returns(TypeName::primitive("List<T>"));
    repo_iface.add_method(find_all.build().unwrap());

    let mut save = FunSpec::<JavaLang>::builder("save");
    save.returns(TypeName::primitive("void"));
    save.add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap());
    repo_iface.add_method(save.build().unwrap());

    let repo_spec = repo_iface.build().unwrap();

    // --- Abstract class: BaseTask ---
    let mut base_task = TypeSpec::<JavaLang>::builder("BaseTask", TypeKind::Class);
    base_task.visibility(Visibility::Public);
    base_task.is_abstract();
    base_task.doc("Base class for all tasks.");

    let mut id_field = FieldSpec::builder("id", TypeName::primitive("long"));
    id_field.visibility(Visibility::Private);
    id_field.is_readonly();
    base_task.add_field(id_field.build().unwrap());

    let mut name_field = FieldSpec::builder("name", TypeName::primitive("String"));
    name_field.visibility(Visibility::Private);
    base_task.add_field(name_field.build().unwrap());

    let mut priority_field = FieldSpec::builder("priority", TypeName::primitive("Priority"));
    priority_field.visibility(Visibility::Private);
    base_task.add_field(priority_field.build().unwrap());

    // Constructor
    let ctor_body = CodeBlock::<JavaLang>::of(
        "this.id = id;\nthis.name = name;\nthis.priority = priority;",
        (),
    )
    .unwrap();
    let mut base_ctor = FunSpec::<JavaLang>::builder("BaseTask");
    base_ctor.visibility(Visibility::Protected);
    base_ctor.add_param(ParameterSpec::new("id", TypeName::primitive("long")).unwrap());
    base_ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    base_ctor.add_param(ParameterSpec::new("priority", TypeName::primitive("Priority")).unwrap());
    base_ctor.body(ctor_body);
    base_task.add_method(base_ctor.build().unwrap());

    // Concrete getter
    let get_id_body = CodeBlock::<JavaLang>::of("return this.id;", ()).unwrap();
    let mut get_id = FunSpec::<JavaLang>::builder("getId");
    get_id.visibility(Visibility::Public);
    get_id.returns(TypeName::primitive("long"));
    get_id.body(get_id_body);
    base_task.add_method(get_id.build().unwrap());

    // Abstract method
    let mut execute = FunSpec::<JavaLang>::builder("execute");
    execute.visibility(Visibility::Public);
    execute.is_abstract();
    execute.returns(TypeName::primitive("void"));
    base_task.add_method(execute.build().unwrap());

    let base_task_spec = base_task.build().unwrap();

    // --- Concrete class: SimpleTask extends BaseTask implements Serializable ---
    let mut simple_task = TypeSpec::<JavaLang>::builder("SimpleTask", TypeKind::Class);
    simple_task.visibility(Visibility::Public);
    simple_task.extends(TypeName::primitive("BaseTask"));
    simple_task.implements(TypeName::primitive("Serializable"));
    simple_task.doc("A simple executable task.");

    // Logger constant — uses imports
    let logger_init =
        CodeBlock::<JavaLang>::of("%T.getLogger(SimpleTask.class)", (logger_factory,)).unwrap();
    let mut log_field = FieldSpec::builder("LOG", TypeName::primitive("Logger"));
    log_field.visibility(Visibility::Private);
    log_field.is_static();
    log_field.is_readonly();
    log_field.initializer(logger_init);
    simple_task.add_field(log_field.build().unwrap());

    // Trigger Logger import
    let logger_trigger = CodeBlock::<JavaLang>::of("// Logger type: %T", (logger,)).unwrap();

    // Constructor
    let simple_ctor_body =
        CodeBlock::<JavaLang>::of("super(id, name, Priority.MEDIUM);", ()).unwrap();
    let mut simple_ctor = FunSpec::<JavaLang>::builder("SimpleTask");
    simple_ctor.visibility(Visibility::Public);
    simple_ctor.add_param(ParameterSpec::new("id", TypeName::primitive("long")).unwrap());
    simple_ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    simple_ctor.body(simple_ctor_body);
    simple_task.add_method(simple_ctor.build().unwrap());

    // execute() override
    let exec_body = CodeBlock::<JavaLang>::of(
        "LOG.info(%S + this.getId());",
        (StringLitArg("Executing task: ".to_string()),),
    )
    .unwrap();
    let mut exec = FunSpec::<JavaLang>::builder("execute");
    exec.visibility(Visibility::Public);
    exec.returns(TypeName::primitive("void"));
    exec.annotation(CodeBlock::<JavaLang>::of("@Override", ()).unwrap());
    exec.body(exec_body);
    simple_task.add_method(exec.build().unwrap());

    let simple_task_spec = simple_task.build().unwrap();

    // --- Standalone utility function (wrapped in class body by user) ---
    let create_body = CodeBlock::<JavaLang>::of(
        "%T<SimpleTask> tasks = new %T<>();\ntasks.add(new SimpleTask(1, %S));\nreturn tasks;",
        (list, array_list, StringLitArg("Default Task".to_string())),
    )
    .unwrap();
    let mut create_fn = FunSpec::<JavaLang>::builder("createDefaultTasks");
    create_fn.visibility(Visibility::Public);
    create_fn.is_static();
    create_fn.returns(TypeName::primitive("List<SimpleTask>"));
    create_fn.body(create_body);
    let create_tasks = create_fn.build().unwrap();

    // findById using Optional — trigger import
    let find_body = CodeBlock::<JavaLang>::of(
        "return tasks.stream()\n    .filter(t -> t.getId() == id)\n    .findFirst();",
        (),
    )
    .unwrap();
    let mut find_fn = FunSpec::<JavaLang>::builder("findTaskById");
    find_fn.visibility(Visibility::Public);
    find_fn.is_static();
    find_fn.returns(TypeName::primitive("Optional<SimpleTask>"));
    find_fn
        .add_param(ParameterSpec::new("tasks", TypeName::primitive("List<SimpleTask>")).unwrap());
    find_fn.add_param(ParameterSpec::new("id", TypeName::primitive("long")).unwrap());
    find_fn.body(find_body);
    let find_task = find_fn.build().unwrap();

    // Trigger Optional import
    let optional_trigger = CodeBlock::<JavaLang>::of("// Optional: %T", (optional,)).unwrap();

    // --- Assemble file ---
    let mut fb = FileSpec::builder_with("TaskApp.java", JavaLang::new());
    fb.add_code(logger_trigger);
    fb.add_code(optional_trigger);
    fb.add_type(priority_spec);
    fb.add_type(repo_spec);
    fb.add_type(base_task_spec);
    fb.add_type(simple_task_spec);
    fb.add_function(create_tasks);
    fb.add_function(find_task);

    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();
    print!("{output}");
}
