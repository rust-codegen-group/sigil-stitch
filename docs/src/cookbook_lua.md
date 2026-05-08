# Lua Cookbook

Practical, copy-paste-ready recipes for Lua code generation. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

Lua is an end-delimited language (`end` instead of `}`). sigil-stitch handles this via `close_on_transition: false` in its block syntax config -- you get correct `if/elseif/else ... end` without spurious `end` before `else`. Since Lua has no type system, you'll mostly use `CodeBlock` directly and `sigil_quote!` rather than `TypeSpec`.

## Function

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::lua::Lua;

let body = sigil_quote!(Lua {
    function greet(name) {
        return "Hello, "..name
    }
}).unwrap();

let file = FileSpec::builder_with("greeter.lua", Lua::new())
    .add_code(body)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```lua
function greet(name)
  return "Hello, "..name
end
```

## Module with require

Use `TypeName::importable` to track Lua `require()` imports. The module path is converted to a slash-separated path in the `require()` call.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::lua::Lua;

let json = TypeName::importable("dkjson", "json");
let inspect = TypeName::importable("inspect", "inspect");

let mut cb = CodeBlock::builder();
cb.add_statement("-- %T %T", (json, inspect));
let block = cb.build().unwrap();

let file = FileSpec::builder_with("app.lua", Lua::new())
    .add_code(block)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```lua
local json = require("dkjson");
local inspect = require("inspect");

-- json inspect
```

## Control flow with sigil_quote!

`sigil_quote!` supports `if/elseif/else`, `for/do`, and `while/do` blocks. Use `{` and `}` in the macro to delimit bodies -- they render as indented blocks closed by `end`.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::lua::Lua;

let block = sigil_quote!(Lua {
    if x > 0 then {
        return $S("positive")
    } elseif x < 0 then {
        return $S("negative")
    } else {
        return $S("zero")
    }
}).unwrap();

let file = FileSpec::builder_with("classify.lua", Lua::new())
    .add_code(block)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```lua
if x > 0 then
  return "positive"
elseif x < 0 then
  return "negative"
else
  return "zero"
end
```

## Table constructor with sigil_quote!

Braces after `=` or in assignments are recognized as table constructors (not control flow). No `end` is emitted -- the braces render as literal `{...}`.

```rust,ignore
use sigil_stitch::prelude::*;
use sigil_stitch::lang::lua::Lua;

let block = sigil_quote!(Lua {
    local user = {
        name = $S("Bob"),
        age = 42,
    }
    print(user.name)
}).unwrap();

let file = FileSpec::builder_with("user.lua", Lua::new())
    .add_code(block)
    .build()
    .unwrap();
let output = file.render(80).unwrap();
```

```lua
local user = {name = "Bob", age = 42,}
print(user.name)
```
