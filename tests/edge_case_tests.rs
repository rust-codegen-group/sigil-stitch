//! Edge case tests for Phase 1 hardening.
//!
//! Covers: column width variation, divergence regression, multi-line %T + semicolons,
//! deep %L nesting, module path validation, single %T format string.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::import::validate_module_path;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Column width variation: same code at 40, 80, and 120 columns ===

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

fn build_width_test_file() -> FileSpec<TypeScript> {
    let config = TypeName::<TypeScript>::importable_type("./config", "Configuration");
    let request = TypeName::<TypeScript>::importable_type("./http", "RequestInit");
    let response = TypeName::<TypeScript>::importable_type("./http", "ResponseBody");
    let logger = TypeName::<TypeScript>::importable_type("./logging", "Logger");

    let mut b = CodeBlock::<TypeScript>::builder();
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

    let mut fb = FileSpec::<TypeScript>::builder("handler.ts");
    fb.add_code(block);
    fb.build().unwrap()
}

// === Divergence regression: conflicting imports near %W break points ===
//
// Three-pass model should prevent Pass 1/2 divergence. This test forces aliases
// (conflicting names) AND uses %W at a sensitive width to verify consistent output.

#[test]
fn test_divergence_regression() {
    // Three types named "Config" from different modules — forces aliasing.
    let config1 = TypeName::<TypeScript>::importable_type("./app", "Config");
    let config2 = TypeName::<TypeScript>::importable_type("./server", "Config");
    let config3 = TypeName::<TypeScript>::importable_type("./database", "Config");

    let mut b = CodeBlock::<TypeScript>::builder();
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

    let mut fb = FileSpec::<TypeScript>::builder("merge.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    // Render at a tight width to stress both aliasing and line breaking.
    let output = file.render(60).unwrap();
    golden::assert_golden("typescript/divergence_regression.ts", &output);
}

// === Multi-line %T + semicolons: union type that wraps ===
//
// A wide union type forced to wrap. The semicolon must appear on the final line.

#[test]
fn test_multiline_type_semicolons() {
    let user = TypeName::<TypeScript>::importable_type("./models", "UserAccount");
    let admin = TypeName::<TypeScript>::importable_type("./models", "AdminAccount");
    let service = TypeName::<TypeScript>::importable_type("./models", "ServiceAccount");
    let guest = TypeName::<TypeScript>::importable_type("./models", "GuestAccount");

    let union = TypeName::<TypeScript>::union(vec![user, admin, service, guest]);

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add_statement("type Account = %T", (union,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("account.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    // Width 40 forces the union to wrap across multiple lines.
    let output = file.render(40).unwrap();
    golden::assert_golden("typescript/multiline_type_semicolons.ts", &output);
}

// === Deep %L nesting: 10+ levels of nested CodeBlocks ===

#[test]
fn test_deep_nesting() {
    // Build from inside out: 12 levels of nesting.
    let mut current = {
        let mut b = CodeBlock::<TypeScript>::builder();
        b.add_statement("console.log('leaf')", ());
        b.build().unwrap()
    };

    for i in (0..12).rev() {
        let mut b = CodeBlock::<TypeScript>::builder();
        b.add(&format!("// level {i}"), ());
        b.add_line();
        b.add_code(current);
        current = b.build().unwrap();
    }

    let mut fb = FileSpec::<TypeScript>::builder("deep.ts");
    fb.add_code(current);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/deep_nesting.ts", &output);
}

// === Module path validation: reject injection-prone characters ===

#[test]
fn test_module_path_validation() {
    // Valid paths.
    assert!(validate_module_path("./models").is_ok());
    assert!(validate_module_path("std::collections").is_ok());
    assert!(validate_module_path("@scope/package").is_ok());
    assert!(validate_module_path("crate::models::user").is_ok());

    // Empty path.
    assert!(validate_module_path("").is_err());

    // Each forbidden character.
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

// === Single %T format string: bare type reference, no surrounding text ===

#[test]
fn test_single_type_reference() {
    let user = TypeName::<TypeScript>::importable_type("./models", "User");

    let mut b = CodeBlock::<TypeScript>::builder();
    b.add_statement("type Alias = %T", (user,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("alias.ts");
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("typescript/single_type_ref.ts", &output);
}
