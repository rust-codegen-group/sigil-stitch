//! Tests for `$V` verbatim string literal in Zsh.
//!
//! $V for Zsh is pure passthrough — no quoting, no escaping.
//! Users include their own quotes in the $V content when shell quoting is needed.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.zsh")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_preserves_zsh_parameter_flags() {
    let block = sigil_quote!(Zsh {
        local lower = $V("${(L)USERNAME}")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("= ${(L)USERNAME}"),
        "Expected passthrough zsh parameter flag, got:\n{output}"
    );
    assert!(
        !output.contains("\"${(L)USERNAME}\""),
        "Should NOT wrap in quotes, got:\n{output}"
    );
}

#[test]
fn verbatim_preserves_zsh_glob_qualifier() {
    let block = sigil_quote!(Zsh {
        local files = $V("${~pattern}")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("= ${~pattern}"),
        "Expected passthrough zsh glob, got:\n{output}"
    );
}

#[test]
fn verbatim_preserves_zsh_array_slicing() {
    let block = sigil_quote!(Zsh {
        echo $V("${array[2,-1]}")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("echo ${array[2,-1]}"),
        "Expected passthrough zsh array slice, got:\n{output}"
    );
}

#[test]
fn verbatim_preserves_zsh_substitution_flags() {
    let block = sigil_quote!(Zsh {
        local replaced = $V("${input//pattern/replacement}")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("= ${input//pattern/replacement}"),
        "Expected passthrough zsh substitution, got:\n{output}"
    );
}

#[test]
fn verbatim_preserves_process_substitution() {
    let block = sigil_quote!(Zsh {
        diff $V("$(cat file1.txt)") $V("$(cat file2.txt)")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("diff $(cat file1.txt) $(cat file2.txt)"),
        "Expected passthrough process subs, got:\n{output}"
    );
}

#[test]
fn verbatim_complex_zsh_script() {
    let block = sigil_quote!(Zsh {
        echo $V("Building ${(U)PROJECT_NAME} from ${GIT_BRANCH:-main}")
        echo $V("Artifacts: ${build_dir}/${(j:/:)path_components}")
        echo $V("Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("echo Building ${(U)PROJECT_NAME} from ${GIT_BRANCH:-main}"),
        "Expected passthrough uppercase flag + default, got:\n{output}"
    );
    assert!(
        output.contains("echo Artifacts: ${build_dir}/${(j:/:)path_components}"),
        "Expected passthrough join flag, got:\n{output}"
    );
    assert!(
        output.contains("echo Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"),
        "Expected passthrough command sub, got:\n{output}"
    );
}

#[test]
fn verbatim_with_explicit_quotes() {
    // When the user wants shell quoting, they include the quotes in the $V content
    let block = sigil_quote!(Zsh {
        echo $V("\"${(L)USERNAME}\"")
        local config=$V("\"${XDG_CONFIG_HOME:-$HOME/.config}\"")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("echo \"${(L)USERNAME}\""),
        "Expected user-provided quotes passed through, got:\n{output}"
    );
    assert!(
        output.contains("config=\"${XDG_CONFIG_HOME:-$HOME/.config}\""),
        "Expected user-provided quotes for assignment, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let name = "world";
    let block = sigil_quote!(Zsh {
        echo $V("Hello @{name}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("echo Hello world"), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let registry = "ghcr.io";
    let tag = "latest";
    let block = sigil_quote!(Zsh {
        docker push $V("@{registry}/myapp:@{tag}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("ghcr.io/myapp:latest"), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Zsh {
        echo $V("user@@host")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("echo user@host"), "got:\n{output}");
}

#[test]
fn verbatim_at_with_shell_vars() {
    let prefix = "/opt";
    let block = sigil_quote!(Zsh {
        echo $V("@{prefix}/$USER/bin")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("echo /opt/$USER/bin"), "got:\n{output}");
}

#[test]
fn verbatim_at_with_zsh_flags() {
    let var = "USERNAME";
    let block = sigil_quote!(Zsh {
        echo $V("${(L)@{var}}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("echo ${(L)USERNAME}"), "got:\n{output}");
}
