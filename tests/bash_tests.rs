use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::bash::Bash;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Basic variable assignment ===

#[test]
fn test_bash_variable_assignment() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("NAME=%S\n", (StringLitArg("world".into()),));
    b.add("COUNT=42\n", ());
    b.add("READONLY_VAR=%S\n", (StringLitArg("constant".into()),));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("vars.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/variable_assignment.bash", &output);
}

// === Shebang header ===

#[test]
fn test_bash_shebang() {
    let mut header_b = CodeBlock::<Bash>::builder();
    header_b.add("#!/usr/bin/env bash\n", ());
    header_b.add("set -euo pipefail", ());
    let header = header_b.build().unwrap();

    let mut body = CodeBlock::<Bash>::builder();
    body.add_statement("echo \"hello\"", ());
    let block = body.build().unwrap();

    let mut fb = FileSpec::builder_with("script.bash", Bash::new());
    fb.header(header);
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/shebang.bash", &output);
}

// === Function declaration via FunSpec ===

#[test]
fn test_bash_function_spec() {
    let mut body_b = CodeBlock::<Bash>::builder();
    body_b.add("local name=$1\n", ());
    body_b.add_statement("echo \"Hello, $name\"", ());
    let body = body_b.build().unwrap();

    let mut fb = FunSpec::<Bash>::builder("greet");
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("funcs.bash", Bash::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/function_spec.bash", &output);
}

// === If/then/fi control flow (manual) ===

#[test]
fn test_bash_if_then_fi() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("if [ -f \"$1\" ]; then\n", ());
    b.add("%>", ());
    b.add_statement("echo \"file exists\"", ());
    b.add("%<", ());
    b.add("else\n", ());
    b.add("%>", ());
    b.add_statement("echo \"file not found\"", ());
    b.add("%<", ());
    b.add("fi\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("check.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/if_then_fi.bash", &output);
}

// === For loop ===

#[test]
fn test_bash_for_loop() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("for f in *.txt; do\n", ());
    b.add("%>", ());
    b.add_statement("echo \"Processing $f\"", ());
    b.add("%<", ());
    b.add("done\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("loop.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/for_loop.bash", &output);
}

// === While loop ===

#[test]
fn test_bash_while_loop() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("while read -r line; do\n", ());
    b.add("%>", ());
    b.add_statement("echo \"$line\"", ());
    b.add("%<", ());
    b.add("done\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("reader.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/while_loop.bash", &output);
}

// === Case/esac ===

#[test]
fn test_bash_case_esac() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("case \"$1\" in\n", ());
    b.add("%>", ());
    b.add("start)\n", ());
    b.add("%>", ());
    b.add_statement("start_service", ());
    b.add("%<", ());
    b.add(";;\n", ());
    b.add("stop)\n", ());
    b.add("%>", ());
    b.add_statement("stop_service", ());
    b.add("%<", ());
    b.add(";;\n", ());
    b.add("*)\n", ());
    b.add("%>", ());
    b.add_statement("echo \"Usage: $0 {start|stop}\"", ());
    b.add("%<", ());
    b.add(";;\n", ());
    b.add("%<", ());
    b.add("esac\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("service.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/case_esac.bash", &output);
}

// === Source imports ===

#[test]
fn test_bash_source_imports() {
    let utils_fn = TypeName::<Bash>::importable("./lib/utils.sh", "log_info");
    let config_fn = TypeName::<Bash>::importable("./lib/config.sh", "load_config");

    let mut b = CodeBlock::<Bash>::builder();
    b.add_statement("%T", (config_fn,));
    b.add_statement("%T \"starting up\"", (utils_fn,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("main.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/source_imports.bash", &output);
}

// === Nested control flow ===

#[test]
fn test_bash_nested_control_flow() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("if [ -d \"$1\" ]; then\n", ());
    b.add("%>", ());
    b.add("for f in \"$1\"/*; do\n", ());
    b.add("%>", ());
    b.add("if [ -f \"$f\" ]; then\n", ());
    b.add("%>", ());
    b.add_statement("process \"$f\"", ());
    b.add("%<", ());
    b.add("fi\n", ());
    b.add("%<", ());
    b.add("done\n", ());
    b.add("%<", ());
    b.add("fi\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("nested.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/nested_control_flow.bash", &output);
}

// === String escaping in context ===

#[test]
fn test_bash_string_escaping() {
    let mut b = CodeBlock::<Bash>::builder();
    b.add("MSG=%S\n", (StringLitArg("hello \"world\"".into()),));
    b.add("PATH_VAR=%S\n", (StringLitArg("$HOME/bin".into()),));
    b.add("CMD=%S\n", (StringLitArg("`whoami`".into()),));
    b.add("BANG=%S\n", (StringLitArg("wow!".into()),));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("escape.bash", Bash::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/string_escaping.bash", &output);
}

// === Complete script with shebang, imports, function, and control flow ===

#[test]
fn test_bash_complete_script() {
    let utils_fn = TypeName::<Bash>::importable("./lib/utils.sh", "log_info");

    let mut header_b = CodeBlock::<Bash>::builder();
    header_b.add("#!/usr/bin/env bash\n", ());
    header_b.add("set -euo pipefail", ());
    let header = header_b.build().unwrap();

    // Function
    let mut fun_body = CodeBlock::<Bash>::builder();
    fun_body.add("local target=$1\n", ());
    fun_body.add("if [ -z \"$target\" ]; then\n", ());
    fun_body.add("%>", ());
    fun_body.add_statement("echo \"error: no target\"", ());
    fun_body.add_statement("return 1", ());
    fun_body.add("%<", ());
    fun_body.add("fi\n", ());
    fun_body.add_statement("%T \"deploying to $target\"", (utils_fn,));
    let fun_body = fun_body.build().unwrap();

    let mut fun = FunSpec::<Bash>::builder("deploy");
    fun.body(fun_body);
    let fun = fun.build().unwrap();

    // Main body
    let mut main = CodeBlock::<Bash>::builder();
    main.add_statement("deploy \"$@\"", ());
    let main = main.build().unwrap();

    let mut fb = FileSpec::builder_with("deploy.bash", Bash::new());
    fb.header(header);
    fb.add_function(fun);
    fb.add_code(main);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/complete_script.bash", &output);
}
