# Language-Aware Tokenizer (MacroLang)

`sigil_quote!` uses Rust's proc-macro tokenizer to parse target-language code. Since the
tokenizer sees Rust tokens, not the target language's tokens, certain patterns are ambiguous:
shell flags (`-q`) look like negation, paths (`/usr`) look like division, and standalone dots
(`.`) look like member access. The `MacroLang` system resolves these ambiguities by making the
tokenizer annotation pass language-aware.

## How It Works

The `sigil_quote!` macro pipeline has three stages:

```text
sigil_quote!(Go { val := <-ch; })
        │
        ▼
┌─ parse_input ─────────────────────────────────────┐
│  1. Extract language: MacroLang::Go           │
│  2. Parse body tokens                             │
│  3. annotate_tokens(tokens, lang)                 │
│     └─ Pre-scan: classify each token              │
│  4. tokens_to_format(tokens, annotations, lang)   │
│     └─ Build format string + args                 │
└───────────────────────────────────────────────────┘
        │
        ▼
  CodeBlockBuilder method calls
```

The `MacroLang` enum is extracted from the first identifier in the macro invocation
(`Bash`, `Zsh`, `Go`, `Haskell`, etc.) and threaded through the entire parse pipeline.
Languages not in the enum get `MacroLang::Unaware`, which applies only universal heuristics.

## MacroLang Variants

| Variant | Recognized from | Tokenizer behavior |
|---------|-----------------|-------------------|
| `Unaware` | All other languages | Universal heuristics only |
| `Bash` | `sigil_quote!(Bash { ... })` | Shell-specific (see below) |
| `Zsh` | `sigil_quote!(Zsh { ... })` | Shell-specific (same as Bash) |
| `Go` | `sigil_quote!(Go { ... })` | `<-` prefix receive |
| `Haskell` | `sigil_quote!(Haskell { ... })` | `$$` dollar operator spacing |

### Shell Languages (Bash, Zsh)

These share a common `is_shell()` check and enable:

- **DashFlag**: `-q`, `-avz` — standalone `-` span-adjacent to the next identifier suppresses
  space after it, so `declare -a` renders correctly.
- **DashSep downgrade**: `-- file.txt` — the second `-` of `--` is downgraded from `PrefixOp`
  to `Normal` when NOT span-adjacent to the next token, preserving the separator space.
  `--amend` (flag, adjacent) stays tight.
- **SlashSep leading path**: `/usr/local/bin` — allows `SlashSep` annotation with no left
  neighbor (relaxes the `i > 0` requirement for shell mode).
- **DotArg**: `find .`, `cd ..` — standalone `.` or `..` not span-adjacent to the previous
  token is marked as a shell argument, not member access. Space is preserved on both sides.
  Guard: if the dot is adjacent to the *next* token (`.gitignore`), it stays as `Normal`.

### Go

- **`<-` prefix receive**: When `-` follows a Joint `<` (not GenericOpen) and is span-adjacent
  to the next token, it gets `PrefixOp` annotation — suppressing the space to produce `<-ch`.
  When NOT adjacent (`ch <- 42`), the `-` stays `Normal` and the space is preserved.
- **Paren-delimited blocks**: `const (`, `var (`, `import (`, and `type (` are detected as
  structural blocks. The parser recursively processes the body so `$for`, `$if`, and other
  directives expand inside. The codegen emits `%>` after the header and `%<` before the
  closing `)` for proper indentation.

### Haskell

- **`$$` dollar operator**: The `$$` escape normally sets `PrevTokenKind::DollarLiteral`, which
  suppresses space after `$` (designed for shell `$VAR`). For Haskell, it sets
  `PrevTokenKind::Punct('$', Alone)` instead, allowing the normal spacing rule to insert a
  space — producing `putStrLn $ show 42`.

## Universal Heuristics (all languages)

These annotations fire regardless of `MacroLang`:

| Annotation | Pattern | Effect |
|---|---|---|
| `PathSepComplete` | `::` span-adjacent to left | Suppress space after (path: `std::fmt`) |
| `DoubleColonOp` | `::` NOT adjacent to left | Space before (Haskell: `fmap :: Type`) |
| `MethodCallColon` | `:` adjacent to both sides | Suppress space (Lua: `obj:method()`) |
| `GenericOpen/Close` | `<`/`>` with type context | Suppress space (generics: `Vec<T>`) |
| `ArrowOp` | `->` adjacent to left | Suppress space (member: `ptr->field`) |
| `PrefixOp` | `&`, `*`, `-` as prefix | Suppress space after (`&self`, `*ptr`) |
| `PostfixStar` | `*`/`&` adjacent to ident | Suppress space before (`Config*`) |
| `PostfixIncDec` | `++`/`--` after ident | Suppress space before (`i++`) |
| `PostfixQuestion` | `?` adjacent to ident | Suppress space before (`Int?`) |
| `SafeCallQ` | `?.` | Suppress space before (`x?.y`) |
| `MacroBang` | `!` after ident | Suppress space before (`println!()`) |
| `CallOpen` | `(`/`[` adjacent to ident | Suppress space (call: `f(x)`) |
| `AssignAdjacent` | `=` adjacent to ident | Suppress space (shell: `NAME=val`) |
| `DashSep` | `-` adjacent to both sides | Hyphenated word (`from-oci-layout`) |
| `SlashSep` | `/` adjacent to both sides | Path separator (`linux/amd64`) |

## Runtime Rewrite Passes

Some language-specific fixups operate on the rendered `CodeNode` tree rather than the
source token stream. These handle cases that the tokenizer can't reach — either because
the pattern is structural (node-level, not token-level) or because it applies to the
builder API (manually-constructed format strings, not `sigil_quote!`).

| Language | Pass | Purpose | Applies to |
|---|---|---|---|
| Go | `rewrite_iife` | Fuse `}()` for immediately-invoked functions | Builder API |
| Go | `rewrite_receive_op` | `<- ch` → `<-ch` | Builder API only (tokenizer handles `sigil_quote!`) |
| C++ | `rewrite_lambda_semicolon` | `}` → `};` for lambda block close | Builder API |
| Lua | `rewrite_method_colon` | `obj: m()` → `obj:m()` | Builder API only (tokenizer handles `sigil_quote!`) |
| Haskell | `rewrite_dollar_spacing` | `$word` → `$ word` | Builder API only (tokenizer handles `sigil_quote!`) |

For `sigil_quote!` users, the tokenizer-level fixes mean correct output without runtime
patching. The runtime passes remain as safety nets for the builder API path.

## Adding MacroLang Support for a New Language

If your language has tokenizer conflicts that universal heuristics can't handle:

1. Add a variant to `MacroLang` in `macros/src/parse/types.rs`
2. Map the language identifier in `parse_macro_lang()` in `macros/src/parse/mod.rs`
3. Add language-guarded annotation logic in `annotate_tokens()` in `macros/src/parse/format.rs`
4. If the fix is in spacing after a token, you may also need to adjust `state.prev` assignment
   in `tokens_to_format_inner()`
5. Add tests in `tests/<lang>/quote_edge_cases.rs`

Only add a `MacroLang` variant when the universal heuristics produce wrong output for your
language. Most languages work correctly with `Unaware`.
