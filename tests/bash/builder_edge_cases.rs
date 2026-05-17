use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_variable_assignment() {
    let mut b = CodeBlock::builder();
    b.add("NAME=%S\n", (StringLitArg("world".into()),));
    b.add("COUNT=42\n", ());
    b.add("READONLY_VAR=%S\n", (StringLitArg("constant".into()),));
    let block = b.build().unwrap();

    let file = FileSpec::builder("vars.bash")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/variable_assignment.bash", &output);
}

#[test]
fn test_shebang() {
    let mut header_b = CodeBlock::builder();
    header_b.add("#!/usr/bin/env bash\n", ());
    header_b.add("set -euo pipefail", ());
    let header = header_b.build().unwrap();

    let mut body = CodeBlock::builder();
    body.add_statement("echo \"hello\"", ());
    let block = body.build().unwrap();

    let file = FileSpec::builder("script.bash")
        .header(header)
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/shebang.bash", &output);
}

#[test]
fn test_string_escaping() {
    let mut b = CodeBlock::builder();
    b.add("MSG=%S\n", (StringLitArg("hello \"world\"".into()),));
    b.add("PATH_VAR=%S\n", (StringLitArg("$HOME/bin".into()),));
    b.add("CMD=%S\n", (StringLitArg("`whoami`".into()),));
    b.add("BANG=%S\n", (StringLitArg("wow!".into()),));
    let block = b.build().unwrap();

    let file = FileSpec::builder("escape.bash")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/string_escaping.bash", &output);
}

#[test]
fn test_dollar_escape_no_space() {
    let mut b = CodeBlock::builder();
    b.add_statement("local level=$1", ());
    b.add_statement("echo $level", ());
    let block = b.build().unwrap();

    let file = FileSpec::builder("test.bash")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("bash/builder_dollar_escape.bash", &output);
}
