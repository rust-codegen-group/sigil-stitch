use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.zsh")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_percent_escape_in_quote() {
    let block = sigil_quote!(Zsh {
        PROMPT=$S("%F{green}%n@%m%f");
        echo $L("$PROMPT");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_percent_escape.zsh", &render(&block));
}

#[test]
fn test_variable_expansion() {
    let block = sigil_quote!(Zsh {
        DEFAULT=$L("${NAME:-guest}");
        LENGTH=$L("${#ITEMS[@]}");
        echo $L("$DEFAULT") $L("$LENGTH");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_variable_expansion.zsh", &render(&block));
}

#[test]
fn test_array_operations() {
    let block = sigil_quote!(Zsh {
        typeset -a ITEMS;
        ITEMS=(one two three);
        echo $L("${ITEMS[@]}");
        echo $L("${#ITEMS[@]}");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_array_operations.zsh", &render(&block));
}

#[test]
fn test_function_definition() {
    let block = sigil_quote!(Zsh {
        function greet() {
            local name=$L("$1");
            echo $S("Hello, $name!");
        }
    })
    .unwrap();
    golden::assert_golden("zsh/quote_function.zsh", &render(&block));
}

#[test]
fn test_name_keyword_escape_in_macro() {
    let name = "autoload";
    let block = sigil_quote!(Zsh {
        $$name=$N(name)
        echo $$name
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("autoload_"), "got: {output}");
}

// ── Single-dash flags ────────────────────────────────────

#[test]
fn test_single_dash_flags() {
    let block = sigil_quote!(Zsh { cmd -q -f -avz }).unwrap();
    let output = render(&block);
    assert!(output.contains("-q"), "got: {output}");
    assert!(output.contains("-f"), "got: {output}");
    assert!(output.contains("-avz"), "got: {output}");
}

#[test]
fn test_typeset_dash_a_not_glued() {
    let block = sigil_quote!(Zsh {
        typeset -a ITEMS;
        typeset -A MAP;
        typeset -i COUNT;
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("typeset -a"), "got: {output}");
    assert!(output.contains("typeset -A"), "got: {output}");
    assert!(!output.contains("typeset-"), "got: {output}");
}

// ── Double-dash / hyphenated flags ───────────────────────

#[test]
fn test_double_dash_flags() {
    let block = sigil_quote!(Zsh { git commit --amend --no-edit }).unwrap();
    let output = render(&block);
    assert!(output.contains("--amend"), "got: {output}");
    assert!(output.contains("--no-edit"), "got: {output}");
}

#[test]
fn test_hyphenated_flag_long() {
    let block = sigil_quote!(Zsh { oras copy --from-oci-layout }).unwrap();
    let output = render(&block);
    assert!(output.contains("--from-oci-layout"), "got: {output}");
}

#[test]
fn test_hyphenated_flag_with_interpolation() {
    let label = "server-bff";
    let image_path = "server/bff";
    let block = sigil_quote!(Zsh {
        oras copy --from-oci-layout $S(format!("images/{label}")) $S(format!("${{TARGET}}/studio/ams/{image_path}:${{TAG}}"))
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("--from-oci-layout"), "got: {output}");
}

// ── Path separators ──────────────────────────────────────

#[test]
fn test_relative_path_tight() {
    let block = sigil_quote!(Zsh { cat src/main.rs }).unwrap();
    let output = render(&block);
    assert!(output.contains("src/main.rs"), "got: {output}");
}

#[test]
fn test_platform_path() {
    let block = sigil_quote!(Zsh {
        docker pull --platform linux/amd64 nginx
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("linux/amd64"), "got: {output}");
}

#[test]
fn test_flags_and_shell_vars() {
    let image = "myapp";
    let block = sigil_quote!(Zsh {
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
    let block = sigil_quote!(Zsh {
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
    let block = sigil_quote!(Zsh {
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
    let block = sigil_quote!(Zsh {
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
fn test_assignment() {
    let block = sigil_quote!(Zsh { NAME=$S("Alice"); }).unwrap();
    let output = render(&block);
    assert!(output.contains("NAME="), "got: {output}");
    assert!(!output.contains("NAME ="), "got: {output}");
}

// ── Language-aware shell fixes ──────────────────────────

#[test]
fn test_double_dash_separator() {
    let block = sigil_quote!(Zsh { git checkout -- file.txt }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("-- file"),
        "separator should have space after --, got: {output}"
    );
}

#[test]
fn test_double_dash_flag_still_tight() {
    let block = sigil_quote!(Zsh { git commit --amend --no-edit }).unwrap();
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
    let block = sigil_quote!(Zsh { ls /usr/local/bin }).unwrap();
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
fn test_dot_as_argument() {
    let block = sigil_quote!(Zsh { find . -type f }).unwrap();
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
    let block = sigil_quote!(Zsh { cd .. }).unwrap();
    let output = render(&block);
    assert!(output.contains("cd .."), "got: {output}");
    assert!(!output.contains("cd.."), "got: {output}");
}

// ── Dotfiles: dot adjacent to next must NOT split ────────

#[test]
fn test_dotfile_gitignore() {
    let block = sigil_quote!(Zsh { ls .gitignore }).unwrap();
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
fn test_dotfile_zshrc() {
    let block = sigil_quote!(Zsh { cat .zshrc }).unwrap();
    let output = render(&block);
    assert!(
        output.contains(".zshrc"),
        "dotfile must stay tight, got: {output}"
    );
}

#[test]
fn test_dotdir_config() {
    let block = sigil_quote!(Zsh { ls .config/nvim }).unwrap();
    let output = render(&block);
    assert!(
        output.contains(".config"),
        "dotdir must stay tight, got: {output}"
    );
}

#[test]
fn test_dot_standalone_vs_dotfile() {
    let block = sigil_quote!(Zsh { find . -name .gitignore }).unwrap();
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
