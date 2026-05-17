use sigil_stitch::code_block::{CodeBlock, NameArg, StringLitArg};
use sigil_stitch::import::validate_module_path;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_long_params() {
    let config_type = TypeName::importable_type("./config", "Configuration");
    let request_type = TypeName::importable_type("./http", "RequestInit");
    let override_type = TypeName::importable_type("./runtime", "InitOverrideFunction");

    let mut b = CodeBlock::builder();
    b.add("export async function createUser(%Wname: string,%Wage: number,%Wconfig: %T,%Wrequest: %T,%Woverride: %T%W): Promise<void> {", (config_type, request_type, override_type));
    b.add_line();
    b.add("%>", ());
    b.add_statement("return undefined", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder("api.ts").add_code(block).build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/long_params.ts", &output);
}

#[test]
fn test_nested_codeblock() {
    let user_type = TypeName::importable_type("./models", "User");

    let mut inner_b = CodeBlock::builder();
    inner_b.add_statement("const user = new %T()", (user_type,));
    inner_b.add_statement("return user", ());
    let inner = inner_b.build().unwrap();

    let mut b = CodeBlock::builder();
    b.add("export function getUser(): User {", ());
    b.add_line();
    b.add("%>", ());
    b.add_code(inner);
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder("getUser.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/nested_codeblock.ts", &output);
}

#[test]
fn test_empty_file() {
    let file = FileSpec::builder("empty.ts").build().unwrap();
    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/empty.ts", &output);
}

#[test]
fn test_raw_content() {
    let file = FileSpec::builder("version.ts")
        .add_raw("// Auto-generated, do not edit.\n\nexport const VERSION = '1.0.0';\n")
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/raw_content.ts", &output);
}

#[test]
fn test_string_and_name() {
    let mut b = CodeBlock::builder();
    b.add_statement("const url = %S", (StringLitArg("/api/users".to_string()),));
    b.add_statement("this.%N(url)", (NameArg("fetchData".to_string()),));
    let block = b.build().unwrap();

    let file = FileSpec::builder("fetch.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/string_and_name.ts", &output);
}

#[test]
fn test_column_width_40() {
    let file = build_width_test_file();
    let output = file.render(40).unwrap();
    golden::assert_golden("typescript/width_40.ts", &output);
}

#[test]
fn test_column_width_80() {
    let file = build_width_test_file();
    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/width_80.ts", &output);
}

#[test]
fn test_column_width_120() {
    let file = build_width_test_file();
    let output = file.render(120).unwrap();
    golden::assert_golden("typescript/width_120.ts", &output);
}

fn build_width_test_file() -> FileSpec {
    let config = TypeName::importable_type("./config", "Configuration");
    let request = TypeName::importable_type("./http", "RequestInit");
    let response = TypeName::importable_type("./http", "ResponseBody");
    let logger = TypeName::importable_type("./logging", "Logger");

    let mut b = CodeBlock::builder();
    b.add(
        "export async function handleRequest(%Wconfig: %T,%Wrequest: %T,%Wlogger: %T%W): Promise<%T> {",
        (config, request, logger, response),
    );
    b.add_line();
    b.add("%>", ());
    b.add_statement("return undefined", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    FileSpec::builder("handler.ts")
        .add_code(block)
        .build()
        .unwrap()
}

#[test]
fn test_divergence_regression() {
    let config1 = TypeName::importable_type("./app", "Config");
    let config2 = TypeName::importable_type("./server", "Config");
    let config3 = TypeName::importable_type("./database", "Config");

    let mut b = CodeBlock::builder();
    b.add(
        "export function mergeConfigs(%Wapp: %T,%Wserver: %T,%Wdb: %T%W): %T {",
        (config1.clone(), config2.clone(), config3.clone(), config1),
    );
    b.add_line();
    b.add("%>", ());
    b.add_statement("const merged: %T = { ...app, ...server }", (config2,));
    b.add_statement("return { ...merged, ...db } as %T", (config3,));
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder("merge.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(60).unwrap();
    golden::assert_golden("typescript/divergence_regression.ts", &output);
}

#[test]
fn test_multiline_type_semicolons() {
    let user = TypeName::importable_type("./models", "UserAccount");
    let admin = TypeName::importable_type("./models", "AdminAccount");
    let service = TypeName::importable_type("./models", "ServiceAccount");
    let guest = TypeName::importable_type("./models", "GuestAccount");

    let union = TypeName::union(vec![user, admin, service, guest]);

    let mut b = CodeBlock::builder();
    b.add_statement("type Account = %T", (union,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("account.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(40).unwrap();
    golden::assert_golden("typescript/multiline_type_semicolons.ts", &output);
}

#[test]
fn test_deep_nesting() {
    let mut current = {
        let mut b = CodeBlock::builder();
        b.add_statement("console.log('leaf')", ());
        b.build().unwrap()
    };

    for i in (0..12).rev() {
        let mut b = CodeBlock::builder();
        b.add(&format!("// level {i}"), ());
        b.add_line();
        b.add_code(current);
        current = b.build().unwrap();
    }

    let file = FileSpec::builder("deep.ts")
        .add_code(current)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/deep_nesting.ts", &output);
}

#[test]
fn test_module_path_validation() {
    assert!(validate_module_path("./models").is_ok());
    assert!(validate_module_path("std::collections").is_ok());
    assert!(validate_module_path("@scope/package").is_ok());
    assert!(validate_module_path("crate::models::user").is_ok());

    assert!(validate_module_path("").is_err());

    let forbidden = ['\n', '\r', '\'', '"', '`', ';', '{', '}', '(', ')'];
    for ch in forbidden {
        let path = format!("./models{ch}inject");
        assert!(
            validate_module_path(&path).is_err(),
            "Should reject character {:?} in path {:?}",
            ch,
            path,
        );
    }
}

#[test]
fn test_optional_field() {
    let output = FileSpec::builder("Config.ts")
        .add_type(
            TypeSpec::builder("Config", TypeKind::Interface)
                .visibility(Visibility::Public)
                .add_field(
                    FieldSpec::builder("name", TypeName::primitive("string"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("description", TypeName::primitive("string"))
                        .is_optional()
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("typescript/optional_field.ts", &output);
}

// ── %N keyword escaping ─────────────────────────────────

#[test]
fn test_name_does_not_escape_ts_reserved_as_identifiers() {
    // TypeScript reserves keywords but they're valid in many positions.
    // escape_reserved uses the default (append _) for TS.
    let keywords = ["class", "function", "import", "export", "return"];
    for kw in keywords {
        let block = CodeBlock::of("const %N = 1", NameArg(kw.into())).unwrap();
        let file = FileSpec::builder("test.ts")
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("{kw}_")),
            "Expected '{kw}_' for reserved word '{kw}', got: {output}"
        );
    }
}

#[test]
fn test_name_no_escape_ts_non_keywords() {
    let names = ["user", "className", "functionName", "exportData"];
    for name in names {
        let block = CodeBlock::of("const %N = 1", NameArg(name.into())).unwrap();
        let file = FileSpec::builder("test.ts")
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("const {name} = 1")),
            "Expected 'const {name} = 1', got: {output}"
        );
    }
}

#[test]
fn test_name_escape_multiple_in_one_line() {
    let block = CodeBlock::of(
        "const %N: %N = new %N()",
        (
            NameArg("class".into()),
            NameArg("MyType".into()),
            NameArg("MyType".into()),
        ),
    )
    .unwrap();
    let file = FileSpec::builder("test.ts")
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("const class_: MyType = new MyType()"));
}

// ── Embedded types in TypeScript interface ───────────────

#[test]
fn test_embedded_in_ts_interface() {
    let file = FileSpec::builder("models.ts")
        .add_type(
            TypeSpec::builder("AdminUser", TypeKind::Interface)
                .visibility(Visibility::Public)
                .add_embedded(TypeName::importable_type("./base", "BaseUser"))
                .add_embedded(TypeName::importable_type("./roles", "AdminRole"))
                .add_field(
                    FieldSpec::builder("permissions", TypeName::primitive("string[]"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/embedded_interface.ts", &output);
}

// ── %W wrap point produces single space, not double ───────────────

#[test]
fn test_wrap_point_single_space() {
    let mut b = CodeBlock::builder();
    b.add("createUser(%WfirstName: string,%WlastName: string)", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder("test.ts")
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("typescript/builder_wrap_point.ts", &output);
}

// ── Consecutive %L specifiers join without spaces ───────────────

#[test]
fn test_consecutive_specifiers_no_space() {
    let mut b = CodeBlock::builder();
    b.add("const x = %L%L%L;", ("pre", "mid", "post"));
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder("test.ts")
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("typescript/builder_consecutive_specifiers.ts", &output);
}
