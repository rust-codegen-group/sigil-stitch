//! Generate a Zsh script — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: Zsh-specific features (arrays, parameter expansion,
//! `[[ ... ]]`, `autoload -Uz`), shell spacing (tight `NAME=value`,
//! `--flag`), and `$V` verbatim strings.
//!
//! Run: `cargo run --example zsh_codegen`

use sigil_stitch::lang::zsh::Zsh;
use sigil_stitch::prelude::*;

fn main() {
    println!("=== Builder API ===\n");
    let builder_output = builder_approach();
    println!("{builder_output}");

    println!("=== sigil_quote! Macro ===\n");
    let macro_output = macro_approach();
    println!("{macro_output}");
}

fn builder_approach() -> String {
    let mut b = CodeBlock::builder();

    // Shebang
    b.add("#!/usr/bin/env zsh", ());
    b.add_line();
    b.add_line();

    // Emulate standard Zsh behavior
    b.add("emulate -L zsh", ());
    b.add_line();
    b.add("set -euo pipefail", ());
    b.add_line();
    b.add_line();

    // Autoload — Zsh function loading system
    b.add("autoload -Uz add-zsh-hook compinit zmv", ());
    b.add_line();
    b.add_line();

    // Array with parameter expansion
    b.add("typeset -a TARGETS=(api worker scheduler)", ());
    b.add_line();
    b.add("typeset -A CONFIG", ());
    b.add_line();
    b.add("CONFIG=(", ());
    b.add_line();
    b.add("%>", ());
    b.add("NAME=myapp", ());
    b.add_line();
    b.add("VERSION=1.0.0", ());
    b.add_line();
    b.add("ENV=production", ());
    b.add_line();
    b.add("%<", ());
    b.add(")", ());
    b.add_line();
    b.add_line();

    // Check with [[ ... ]] conditional
    b.add("%<", ());
    b.add("if [[ -z \"$ENV\" ]]; then", ());
    b.add_line();
    b.add("%>", ());
    b.add("print -u2 \"Error: ENV not set\"", ());
    b.add_line();
    b.add("return 1", ());
    b.add_line();
    b.add("%<", ());
    b.add("fi", ());
    b.add_line();
    b.add("%>", ());
    b.add_line();

    // Loop over array
    b.add("for target in $TARGETS; do", ());
    b.add_line();
    b.add("%>", ());
    b.add("print \"Deploying $target to $ENV...\"", ());
    b.add_line();
    b.add("%<", ());
    b.add("done", ());
    b.add_line();
    b.add("%>", ());
    b.add_line();

    // Parameter expansion with defaults
    b.add("local name=${1:?Usage: $0 <name>}", ());
    b.add_line();
    b.add("local count=${2:-1}", ());
    b.add_line();
    b.add("print \"Running $name x${count}\"", ());

    FileSpec::builder_with("deploy.zsh", Zsh::new())
        .add_code(b.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let block = sigil_quote!(Zsh {
        #!/usr/bin/env zsh

        emulate -L zsh
        set -euo pipefail

        autoload -Uz add-zsh-hook compinit zmv

        typeset -a TARGETS=(api worker scheduler)
        typeset -A CONFIG
        CONFIG=(
            NAME=myapp
            VERSION=1.0.0
            ENV=production
        )

        if [[ -z $$ENV ]]; {
            print -u2 $V("\"Error: ENV not set\"")
            return 1
        }

        for target in $$TARGETS; {
            print $V("\"Deploying $target to ${ENV}...\"")
        }

        local name=$${1:?Usage: $$0 <name>}
        local count=$${2:-1}
        print $V("\"Running ${name} x${count}\"")
    })
    .unwrap();

    FileSpec::builder_with("deploy.zsh", Zsh::new())
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
