//! Generate a Bash script — builder API vs `sigil_quote!` comparison.
//!
//! Demonstrates: functions with control flow (`if/then/fi`, `for/do/done`),
//! shebang header, `set -euo pipefail`, `source` imports, string escaping,
//! and `$V` verbatim strings (preserving shell interpolation).
//! The macro approach uses `{ }` blocks that the Bash backend automatically
//! converts to the correct delimiters (`then`/`fi`, `do`/`done`).
//!
//! Run: `cargo run --example bash_codegen`

use sigil_stitch::lang::bash::Bash;
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
    let comment_label = "INFO";
    let comment_reason = "Deploy to target environment";
    let comment_note = "check env var";

    // --- log function ---
    let mut log_body = CodeBlock::builder();
    log_body.add("local level=$1", ());
    log_body.add_line();
    log_body.add("local message=$2", ());
    log_body.add_line();
    log_body.add("echo \"[$level] $(date): $message\"", ());

    let log_fn = FunSpec::builder("log_message")
        .body(log_body.build().unwrap())
        .build()
        .unwrap();

    // --- deploy function with if/then/fi ---
    let mut deploy_body = CodeBlock::builder();
    deploy_body.add_comment(&format!("{}: {}", comment_label, comment_reason));
    deploy_body.add("local env=$1", ());
    deploy_body.add_line();
    deploy_body.add_line();
    deploy_body.add("%<", ());
    deploy_body.add("if [ -z \"$env\" ]; then", ());
    deploy_body.add_line();
    deploy_body.add("%>", ());
    deploy_body.add("log_message \"ERROR\" \"No environment specified\"", ());
    deploy_body.add_line();
    deploy_body.add("return 1", ());
    deploy_body.add_line();
    deploy_body.add("%<", ());
    deploy_body.add("fi", ());
    deploy_body.add_line();
    deploy_body.add("%>", ());
    deploy_body.add_line();
    deploy_body.add(
        "log_message \"INFO\" \"Deploying to $env\" %R",
        (CommentArg(comment_note.to_string()),),
    );
    deploy_body.add_line();
    deploy_body.add_line();
    deploy_body.add("%<", ());
    deploy_body.add("for service in api worker scheduler; do", ());
    deploy_body.add_line();
    deploy_body.add("%>", ());
    deploy_body.add("log_message \"INFO\" \"Starting $service\"", ());
    deploy_body.add_line();
    deploy_body.add("%<", ());
    deploy_body.add("done", ());
    deploy_body.add_line();
    deploy_body.add("%>", ());

    let deploy_fn = FunSpec::builder("deploy")
        .doc("Deploy services to the given environment.")
        .body(deploy_body.build().unwrap())
        .build()
        .unwrap();

    FileSpec::builder_with("deploy.bash", Bash::new())
        .header(CodeBlock::of("#!/usr/bin/env bash\nset -euo pipefail", ()).unwrap())
        .add_import(ImportSpec::side_effect("./utils.sh"))
        .add_function(log_fn)
        .add_function(deploy_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn macro_approach() -> String {
    let comment_label = "INFO";
    let comment_reason = "Deploy to target environment";
    let comment_note = "check env var";

    // $V is passthrough — include your own quotes where shell quoting is needed.
    // $S would escape shell sigils; $V passes them through for shell expansion.
    let log_body = sigil_quote!(Bash {
        local level=$$1
        local message=$$2
        echo $V("\"[$level] $(date +%Y-%m-%dT%H:%M:%S): $message\"")
    })
    .unwrap();

    let log_fn = FunSpec::builder("log_message")
        .body(log_body)
        .build()
        .unwrap();

    // sigil_quote! with { } — Bash backend emits then/fi, do/done.
    // $V is used for strings that need shell expansion at runtime.
    let deploy_body = sigil_quote!(Bash {
        $comment("@{comment_label}: @{comment_reason}");
        local env=$$1
        local app_name=$V("\"${APP_NAME:-myapp}\"")
        local version=$V("\"$(cat VERSION 2>/dev/null || echo unknown)\"")

        if [ -z $$env ]; {
            log_message $S("ERROR") $V("\"No environment specified for ${app_name}\"")
            return 1
        }

        log_message $S("INFO") $V("\"Deploying ${app_name} v${version} to ${env}\"") $comment(comment_note)

        for service in api worker scheduler; {
            log_message $S("INFO") $V("\"Starting $service on $env\"")
        }

        log_message $S("INFO") $V("\"Deploy complete. PID=$$, exit=$?\"")
    })
    .unwrap();

    let deploy_fn = FunSpec::builder("deploy")
        .doc("Deploy services to the given environment.")
        .body(deploy_body)
        .build()
        .unwrap();

    FileSpec::builder_with("deploy.bash", Bash::new())
        .header(CodeBlock::of("#!/usr/bin/env bash\nset -euo pipefail", ()).unwrap())
        .add_import(ImportSpec::side_effect("./utils.sh"))
        .add_function(log_fn)
        .add_function(deploy_fn)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}
