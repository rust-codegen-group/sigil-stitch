use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.bash")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_control_flow() {
    let block = sigil_quote!(Bash {
        if [ $L("$x") -gt 0 ] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("bash/macro_control_flow.bash", &render(&block));
}

// ── $V in control flow conditions ────────────────────────

#[test]
fn while_double_bracket_verbatim() {
    let block = sigil_quote!(Bash {
        while [[ $V("\"${1:-}\"") == --* ]] {
            FLAGS=$V("\"$FLAGS $1\"");
            shift;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("while [[ \"${1:-}\" == --* ]]; do"),
        "got:\n{output}"
    );
    assert!(
        output.contains("FLAGS=\"$FLAGS $1\""),
        "Expected no space around = in assignment, got:\n{output}"
    );
    assert!(
        output.contains("done"),
        "Expected 'done' block close, got:\n{output}"
    );
}

#[test]
fn if_multiple_verbatim_in_condition() {
    let block = sigil_quote!(Bash {
        if [[ $V("$1") == $V("--help") ]] {
            usage;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("if [[ $1 == --help ]]; then"),
        "got:\n{output}"
    );
}

#[test]
fn while_single_bracket_verbatim() {
    let block = sigil_quote!(Bash {
        while [ $V("-f \"$file\"") ] {
            sleep 1;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("while [ -f \"$file\" ]; do"),
        "got:\n{output}"
    );
}

#[test]
fn for_verbatim_iterable() {
    let block = sigil_quote!(Bash {
        for arg in $V("\"$@\"") {
            echo $L("$arg");
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("for arg in \"$@\"; do"), "got:\n{output}");
}

// ── $S in control flow conditions ────────────────────────

#[test]
fn if_string_literal_in_condition() {
    let block = sigil_quote!(Bash {
        if [ $L("$x") == $S("hello") ] {
            echo $S("matched");
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("if [ $x == \"hello\" ]; then"),
        "got:\n{output}"
    );
}

#[test]
fn while_string_literal_in_condition() {
    let block = sigil_quote!(Bash {
        while [ $L("$input") != $S("quit") ] {
            read input;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("while [ $input != \"quit\" ]; do"),
        "got:\n{output}"
    );
}

// ── $L in control flow conditions ────────────────────────

#[test]
fn if_multiple_literals_in_condition() {
    let block = sigil_quote!(Bash {
        if [[ $L("$count") -lt $L("10") ]] {
            echo $S("under limit");
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("if [[ $count -lt 10 ]]; then"),
        "got:\n{output}"
    );
}

#[test]
fn for_literal_iterable() {
    let block = sigil_quote!(Bash {
        for i in $L("$(seq 1 10)") {
            echo $L("$i");
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("for i in $(seq 1 10); do"),
        "got:\n{output}"
    );
}

#[test]
fn while_literal_arithmetic_condition() {
    let block = sigil_quote!(Bash {
        while $L("(( i < n ))") {
            echo $L("$i");
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("while (( i < n )); do"), "got:\n{output}");
}

// ── Mixed markers in control flow conditions ─────────────

#[test]
fn if_verbatim_and_string_mixed() {
    let block = sigil_quote!(Bash {
        if [[ $V("\"${1:-}\"") == $S("--verbose") ]] {
            set -x;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("if [[ \"${1:-}\" == \"--verbose\" ]]; then"),
        "got:\n{output}"
    );
}

#[test]
fn while_compound_condition_mixed() {
    let block = sigil_quote!(Bash {
        while [[ $L("$#") -gt $L("0") ]] {
            case $V("$1") in {
                $L("--verbose") {
                    VERBOSE = 1;
                }
                * {
                    break;
                }
            }
            shift;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("while [[ $# -gt 0 ]]; do"),
        "got:\n{output}"
    );
    assert!(output.contains("case $1 in"), "got:\n{output}");
}
