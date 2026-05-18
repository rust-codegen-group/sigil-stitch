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
fn test_array_syntax() {
    let block = sigil_quote!(Bash {
        declare -a ITEMS;
        ITEMS=(one two three);
        echo $L("${ITEMS[@]}");
        echo $L("${#ITEMS[@]}");
    })
    .unwrap();
    golden::assert_golden("bash/quote_array.bash", &render(&block));
}

#[test]
fn test_parameter_expansion() {
    let block = sigil_quote!(Bash {
        DEFAULT=$L("${NAME:-guest}");
        UPPER=$L("${NAME^^}");
        SUBSTR=$L("${NAME:0:3}");
    })
    .unwrap();
    golden::assert_golden("bash/quote_parameter_expansion.bash", &render(&block));
}

#[test]
fn test_here_string() {
    let block = sigil_quote!(Bash {
        read -r line <<< $S("hello world");
        echo $L("$line");
    })
    .unwrap();
    golden::assert_golden("bash/quote_here_string.bash", &render(&block));
}

#[test]
fn test_function_with_local() {
    let block = sigil_quote!(Bash {
        function greet() {
            local name=$L("$1");
            echo $S("Hello, ${name}!");
        }
    })
    .unwrap();
    golden::assert_golden("bash/quote_function_local.bash", &render(&block));
}

#[test]
fn test_name_keyword_escape_in_macro() {
    let name = "declare";
    let block = sigil_quote!(Bash {
        $$name=$N(name)
        echo $$name
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("declare_"), "got: {output}");
}

// ── Single-dash flags ────────────────────────────────────

#[test]
fn test_single_dash_flags() {
    let block = sigil_quote!(Bash { cmd -q -f -avz }).unwrap();
    let output = render(&block);
    assert!(output.contains("-q"), "got: {output}");
    assert!(output.contains("-f"), "got: {output}");
    assert!(output.contains("-avz"), "got: {output}");
}

#[test]
fn test_declare_dash_a_not_glued() {
    let block = sigil_quote!(Bash {
        declare -a ITEMS;
        declare -A MAP;
        declare -i COUNT;
        declare -r CONST;
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("declare -a"), "got: {output}");
    assert!(output.contains("declare -A"), "got: {output}");
    assert!(output.contains("declare -i"), "got: {output}");
    assert!(output.contains("declare -r"), "got: {output}");
    assert!(!output.contains("declare-"), "got: {output}");
}

#[test]
fn test_read_dash_r_not_glued() {
    let block = sigil_quote!(Bash { read -r line; }).unwrap();
    let output = render(&block);
    assert!(output.contains("read -r"), "got: {output}");
    assert!(!output.contains("read-r"), "got: {output}");
}

#[test]
fn test_test_operators() {
    let block = sigil_quote!(Bash { [ $L("$x") -gt 0 ] }).unwrap();
    let output = render(&block);
    assert!(output.contains("-gt"), "got: {output}");
    assert!(!output.contains("- gt"), "got: {output}");
}

// ── Double-dash / hyphenated flags ───────────────────────

#[test]
fn test_double_dash_flags() {
    let block = sigil_quote!(Bash { git commit --amend --no-edit }).unwrap();
    let output = render(&block);
    assert!(output.contains("--amend"), "got: {output}");
    assert!(output.contains("--no-edit"), "got: {output}");
}

#[test]
fn test_hyphenated_flag_long() {
    let block = sigil_quote!(Bash { oras copy --from-oci-layout }).unwrap();
    let output = render(&block);
    assert!(output.contains("--from-oci-layout"), "got: {output}");
}

#[test]
fn test_hyphenated_flag_with_interpolation() {
    let label = "server-bff";
    let image_path = "server/bff";
    let block = sigil_quote!(Bash {
        oras copy --from-oci-layout $S(format!("images/{label}")) $S(format!("${{TARGET}}/studio/ams/{image_path}:${{TAG}}"))
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("--from-oci-layout"), "got: {output}");
}

// ── Path separators ──────────────────────────────────────

#[test]
fn test_relative_path_tight() {
    let block = sigil_quote!(Bash { cat src/main.rs }).unwrap();
    let output = render(&block);
    assert!(output.contains("src/main.rs"), "got: {output}");
}

#[test]
fn test_platform_path() {
    let block = sigil_quote!(Bash {
        docker pull --platform linux/amd64 nginx
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("linux/amd64"), "got: {output}");
}

#[test]
fn test_flags_and_shell_vars() {
    let image = "myapp";
    let block = sigil_quote!(Bash {
        docker pull -q --platform linux/amd64 $L("${REGISTRY}")/$N(image):$L("${TAG}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("-q"), "got: {output}");
    assert!(output.contains("linux/amd64"), "got: {output}");
    assert!(output.contains("myapp"), "got: {output}");
}

// ── Real-world commands ──────────────────────────────────

#[test]
fn test_docker_build() {
    let block = sigil_quote!(Bash {
        docker build -t myapp:latest -f docker/Dockerfile --build-arg VERSION=$S("1.0")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("-t"), "got: {output}");
    assert!(output.contains("-f"), "got: {output}");
    assert!(output.contains("--build-arg"), "got: {output}");
    assert!(output.contains("docker/Dockerfile"), "got: {output}");
}

#[test]
fn test_curl_with_flags() {
    let block = sigil_quote!(Bash {
        curl -sSL --retry 3 --retry-delay 5 example.com/download
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("-sSL"), "got: {output}");
    assert!(output.contains("--retry 3"), "got: {output}");
    assert!(output.contains("--retry-delay"), "got: {output}");
    assert!(output.contains("example.com/download"), "got: {output}");
}

#[test]
fn test_kubectl() {
    let ns = "prod";
    let block = sigil_quote!(Bash {
        kubectl get pods -n $N(ns) --sort-by=.metadata.creationTimestamp -o json
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("-n"), "got: {output}");
    assert!(output.contains("--sort-by="), "got: {output}");
    assert!(output.contains("-o"), "got: {output}");
    assert!(output.contains("prod"), "got: {output}");
}

#[test]
fn test_ssh_with_flags() {
    let block = sigil_quote!(Bash {
        ssh -p 2222 -i key host
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("-p"), "got: {output}");
    assert!(output.contains("-i"), "got: {output}");
}

#[test]
fn test_assignment() {
    let block = sigil_quote!(Bash { NAME=$S("Alice"); }).unwrap();
    let output = render(&block);
    assert!(output.contains("NAME="), "got: {output}");
    assert!(!output.contains("NAME ="), "got: {output}");
}

// ── Language-aware shell fixes ──────────────────────────

#[test]
fn test_double_dash_separator() {
    let block = sigil_quote!(Bash { git checkout -- file.txt }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("-- file"),
        "separator should have space after --, got: {output}"
    );
}

#[test]
fn test_double_dash_flag_still_tight() {
    let block = sigil_quote!(Bash { git commit --amend --no-edit }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("--amend"),
        "flag should be tight, got: {output}"
    );
    assert!(
        output.contains("--no-edit"),
        "flag should be tight, got: {output}"
    );
}

#[test]
fn test_leading_slash_path() {
    let block = sigil_quote!(Bash { ls /usr/local/bin }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("/usr/local/bin"),
        "leading path should be tight, got: {output}"
    );
    assert!(
        !output.contains("/ usr"),
        "no space after leading /, got: {output}"
    );
}

#[test]
fn test_leading_slash_etc() {
    let block = sigil_quote!(Bash { cat /etc/hosts }).unwrap();
    let output = render(&block);
    assert!(output.contains("/etc/hosts"), "got: {output}");
}

#[test]
fn test_dot_as_argument() {
    let block = sigil_quote!(Bash { find . -type f }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("find ."),
        "dot should be standalone, got: {output}"
    );
    assert!(
        !output.contains("find."),
        "dot must not glue to prev, got: {output}"
    );
}

#[test]
fn test_dotdot_as_argument() {
    let block = sigil_quote!(Bash { cd .. }).unwrap();
    let output = render(&block);
    assert!(output.contains("cd .."), "got: {output}");
    assert!(!output.contains("cd.."), "got: {output}");
}

#[test]
fn test_dot_in_find_complex() {
    let block = sigil_quote!(Bash { find . -name $S("*.rs") -type f }).unwrap();
    let output = render(&block);
    assert!(output.contains("find ."), "got: {output}");
    assert!(output.contains(". -name"), "space after dot, got: {output}");
}

// ── Dotfiles: dot adjacent to next must NOT split ────────

#[test]
fn test_dotfile_gitignore() {
    let block = sigil_quote!(Bash { ls .gitignore }).unwrap();
    let output = render(&block);
    assert!(
        output.contains(".gitignore"),
        "dotfile must stay tight, got: {output}"
    );
    assert!(
        !output.contains(". gitignore"),
        "dot must not split from name, got: {output}"
    );
}

#[test]
fn test_dotfile_bashrc() {
    let block = sigil_quote!(Bash { cat .bashrc }).unwrap();
    let output = render(&block);
    assert!(
        output.contains(".bashrc"),
        "dotfile must stay tight, got: {output}"
    );
}

#[test]
fn test_dotdir_config() {
    let block = sigil_quote!(Bash { ls .config/nvim }).unwrap();
    let output = render(&block);
    assert!(
        output.contains(".config"),
        "dotdir must stay tight, got: {output}"
    );
}

#[test]
fn test_dot_standalone_vs_dotfile() {
    // Both patterns in one block: standalone dot AND dotfile
    let block = sigil_quote!(Bash { find . -name .gitignore }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("find ."),
        "standalone dot needs space, got: {output}"
    );
    assert!(
        !output.contains("find."),
        "standalone dot must not glue, got: {output}"
    );
    assert!(
        output.contains(".gitignore"),
        "dotfile must stay tight, got: {output}"
    );
}
