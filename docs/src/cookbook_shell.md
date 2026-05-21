# Shell (Bash/Zsh) Cookbook

Practical, copy-paste-ready recipes for Bash and Zsh script generation. Covers `sigil_quote!` with shell-aware control flow, `$V` verbatim strings for preserving shell interpolation, and the builder API.

## Basic function with `sigil_quote!`

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let body = sigil_quote!(Bash {
    local name=$$1
    echo $V("\"Hello, ${name}!\"")
}).unwrap();

let fun = FunSpec::builder("greet")
    .body(body)
    .build()
    .unwrap();

let output = FileSpec::builder("greet.bash")
    .add_function(fun)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

```bash
function greet() {
    local name=$1
    echo "Hello, ${name}!"
}
```

## Control flow (`if/then/fi`, `for/do/done`)

Use `{ }` blocks in `sigil_quote!` — the backend maps them to the correct shell delimiters.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let body = sigil_quote!(Bash {
    if [ -z $$1 ]; {
        echo $S("Error: no argument")
        return 1
    }

    for file in $$@; {
        echo $V("\"Processing: ${file}\"")
    }
}).unwrap();

let fun = FunSpec::builder("process_files")
    .body(body)
    .build()
    .unwrap();

let output = FileSpec::builder("process.bash")
    .add_function(fun)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

```bash
function process_files() {
    if [ -z $1 ]; then
        echo "Error: no argument"
        return 1
    fi

    for file in $@; do
        echo "Processing: ${file}"
    done
}
```

## `$V` vs `$S` — when to use which

`$S` escapes everything and wraps in quotes (safe for static strings). `$V` is pure passthrough — no quoting, no escaping. Use `$V` when you want shell to expand variables, command substitutions, or arithmetic at runtime. Include your own quotes in the `$V` content when shell quoting is needed.

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let block = sigil_quote!(Bash {
    echo $S("$HOME")
    echo $V("$HOME")
    echo $V("\"$HOME\"")
}).unwrap();

let output = FileSpec::builder("test.bash")
    .add_code(block)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
// Line 1: echo "\$HOME"       ← $S escapes the dollar sign, wraps in quotes
// Line 2: echo $HOME           ← $V passthrough, no quotes (word-splitting possible)
// Line 3: echo "$HOME"         ← $V passthrough with user-provided quotes (safe)
# }
```

## Complex shell interpolation with `$V`

`$V` handles all shell expansion patterns — braced defaults, command substitution, arithmetic, arrays, special variables. Since `$V` is passthrough, include quotes in the content when the generated shell code should have them:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let body = sigil_quote!(Bash {
    local config_dir=$V("\"${XDG_CONFIG_HOME:-$HOME/.config}\"")
    local version=$V("\"$(git describe --tags 2>/dev/null || echo dev)\"")
    local port=$V("\"$((BASE_PORT + WORKER_ID))\"")

    echo $V("\"Deploying ${APP_NAME} v${version}\"")
    echo $V("\"Config: ${config_dir}/${APP_NAME}.conf\"")
    echo $V("\"Status: exit=$? pid=$$\"")
    echo $V("\"Args: count=$# all=$@\"")
    echo $V("\"Array: ${services[@]}\"")
}).unwrap();

let fun = FunSpec::builder("setup")
    .body(body)
    .build()
    .unwrap();

let output = FileSpec::builder("setup.bash")
    .add_function(fun)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

```bash
function setup() {
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}"
    local version="$(git describe --tags 2>/dev/null || echo dev)"
    local port="$((BASE_PORT + WORKER_ID))"

    echo "Deploying ${APP_NAME} v${version}"
    echo "Config: ${config_dir}/${APP_NAME}.conf"
    echo "Status: exit=$? pid=$$"
    echo "Args: count=$# all=$@"
    echo "Array: ${services[@]}"
}
```

## `@{expr}` interpolation in `$V`

When you need to mix Rust compile-time values with shell runtime variables, use `@{expr}` inside `$V` strings. The `@{...}` parts are evaluated at compile time; everything else passes through verbatim for shell to interpret at runtime:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let registry = "ghcr.io/myorg";
let app = "api-server";
let services = ["web", "worker", "scheduler"];

let block = sigil_quote!(Bash {
    docker push $V("@{registry}/@{app}:${TAG}")
    echo $V("Deploying @{services.len()} services to ${ENVIRONMENT}")
    echo $V("Contact: admin@@@{app}.internal")
}).unwrap();
# }
```

```bash
docker push ghcr.io/myorg/api-server:${TAG}
echo Deploying 3 services to ${ENVIRONMENT}
echo Contact: admin@api-server.internal
```

Use `@@` to emit a literal `@` in the output. Bare `@` not followed by `{` passes through unchanged.

## Shebang and header

Use `FileSpec::header()` for the shebang and preamble:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let header = CodeBlock::of("#!/usr/bin/env bash\nset -euo pipefail", ()).unwrap();

let body = sigil_quote!(Bash {
    echo $S("Starting...")
}).unwrap();

let main_fn = FunSpec::builder("main")
    .body(body)
    .build()
    .unwrap();

let output = FileSpec::builder_with("script.bash", Bash::new())
    .header(header)
    .add_function(main_fn)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

```bash
#!/usr/bin/env bash
set -euo pipefail

function main() {
    echo "Starting..."
}
```

## Imports (`source`)

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let utils = TypeName::importable("./lib/utils.sh", "");
let config = TypeName::importable("./lib/config.sh", "");

let body = CodeBlock::of("# uses %T and %T", (utils, config)).unwrap();

let output = FileSpec::builder_with("app.bash", Bash::new())
    .add_code(body)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
// Generates:
//   source "./lib/config.sh"
//   source "./lib/utils.sh"
# }
```

## Zsh-specific features

Zsh works identically to Bash for control flow. Use `$V` for Zsh-specific parameter expansion:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::zsh::Zsh;
# fn main() {
let body = sigil_quote!(Zsh {
    local lower=$V("\"${(L)USERNAME}\"")
    local joined=$V("\"${(j:,:)array}\"")
    local sliced=$V("\"${array[2,-1]}\"")
    local replaced=$V("\"${input//old/new}\"")
}).unwrap();

let fun = FunSpec::builder("zsh_features")
    .body(body)
    .build()
    .unwrap();

let output = FileSpec::builder_with("demo.zsh", Zsh::new())
    .add_function(fun)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

```zsh
function zsh_features() {
    local lower="${(L)USERNAME}"
    local joined="${(j:,:)array}"
    local sliced="${array[2,-1]}"
    local replaced="${input//old/new}"
}
```

## Double-bracket tests with `[[ ]]`

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let body = sigil_quote!(Bash {
    if [[ $$1 == $$2 ]]; {
        echo $S("match")
    }
}).unwrap();

let fun = FunSpec::builder("check_equal")
    .body(body)
    .build()
    .unwrap();

let output = FileSpec::builder("check.bash")
    .add_function(fun)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```

```bash
function check_equal() {
    if [[ $1 == $2 ]]; then
        echo "match"
    fi
}
```

## Combining `$V` with runtime Rust values

Mix `$V` (shell-expanded at runtime) with `$L`/`$S` (Rust values baked in at generation time):

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let app_name = "myapp";
let log_dir = "/var/log";

// Build the whole string in Rust and pass via $V (most ergonomic):
let log_pattern = format!("\"${{LOG_DIR:-{log_dir}}}/{app_name}.log\"");
let body = sigil_quote!(Bash {
    local log_file=$V(log_pattern)
    echo $V("\"Writing to ${log_file}\"")
}).unwrap();
# }
```

## File extension

Use `.with_extension("sh")` for POSIX-compatible scripts:

```rust
# extern crate sigil_stitch;
# use sigil_stitch::prelude::*;
# use sigil_stitch::lang::bash::Bash;
# fn main() {
let bash = Bash::new().with_extension("sh");
let output = FileSpec::builder_with("script.sh", bash)
    .build()
    .unwrap()
    .render(80)
    .unwrap();
# }
```
