use sigil_stitch::code_block::{CodeBlock, NameArg, StringLitArg};
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Hello World: simple class with one import ===

#[test]
fn test_hello_world() {
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add("export class UserService {", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("private user: %T", (user_type,));
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("UserService.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/hello_world.ts", &output);
}

// === Conflicting imports: two "User" from different modules ===

#[test]
fn test_conflicting_imports() {
    let user1 = TypeName::<TypeScript>::importable_type("./models", "User");
    let user2 = TypeName::<TypeScript>::importable_type("./other", "User");

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add_statement("const u1: %T = getUser1()", (user1,));
    b.add_statement("const u2: %T = getUser2()", (user2,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("users.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/conflicting_imports.ts", &output);
}

// === Long params: parameter list exceeding 80 cols ===

#[test]
fn test_long_params() {
    let config_type = TypeName::<TypeScript>::importable_type("./config", "Configuration");
    let request_type = TypeName::<TypeScript>::importable_type("./http", "RequestInit");
    let override_type =
        TypeName::<TypeScript>::importable_type("./runtime", "InitOverrideFunction");

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add("export async function createUser(%Wname: string,%Wage: number,%Wconfig: %T,%Wrequest: %T,%Woverride: %T%W): Promise<void> {", (config_type, request_type, override_type));
    b.add_line();
    b.add("%>", ());
    b.add_statement("return undefined", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("api.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/long_params.ts", &output);
}

// === Nested CodeBlock via %L ===

#[test]
fn test_nested_codeblock() {
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");

    let mut inner_b = CodeBlock::<TypeScript>::builder();
    inner_b.add_statement("const user = new %T()", (user_type,));
    inner_b.add_statement("return user", ());
    let inner = inner_b.build().unwrap();

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add("export function getUser(): User {", ());
    b.add_line();
    b.add("%>", ());
    b.add_code(inner);
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("getUser.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/nested_codeblock.ts", &output);
}

// === Control flow: if/else ===

#[test]
fn test_control_flow() {
    let error_type = TypeName::<TypeScript>::importable_type("./errors", "NotFoundError");

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add("export function validate(input: string): boolean {", ());
    b.add_line();
    b.add("%>", ());
    b.begin_control_flow("if (input.length === 0)", ());
    b.add_statement("throw new %T('empty input')", (error_type,));
    b.next_control_flow("else if (input.length > 100)", ());
    b.add_statement("return false", ());
    b.next_control_flow("else", ());
    b.add_statement("return true", ());
    b.end_control_flow();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("validate.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/control_flow.ts", &output);
}

// === Empty file ===

#[test]
fn test_empty_file() {
    let file = FileSpec::<TypeScript>::builder("empty.ts").build().unwrap();
    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/empty.ts", &output);
}

// === RawContent member ===

#[test]
fn test_raw_content() {
    let mut fb = FileSpec::<TypeScript>::builder("version.ts");
    fb.add_raw("// Auto-generated, do not edit.\n\nexport const VERSION = '1.0.0';\n");
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/raw_content.ts", &output);
}

// === String literal and name rendering ===

#[test]
fn test_string_and_name() {
    let mut b = CodeBlock::<TypeScript>::builder();
    b.add_statement("const url = %S", (StringLitArg("/api/users".to_string()),));
    b.add_statement("this.%N(url)", (NameArg("fetchData".to_string()),));
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("fetch.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/string_and_name.ts", &output);
}

// === Multiple types from same module (dedup) ===

#[test]
fn test_same_module_multiple_types() {
    let user = TypeName::<TypeScript>::importable_type("./models", "User");
    let tag = TypeName::<TypeScript>::importable_type("./models", "Tag");
    let category = TypeName::<TypeScript>::importable_type("./models", "Category");

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add_statement("const u: %T = null", (user,));
    b.add_statement("const t: %T = null", (tag,));
    b.add_statement("const c: %T = null", (category,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("types.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/same_module_types.ts", &output);
}
