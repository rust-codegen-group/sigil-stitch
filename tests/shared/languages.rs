use sigil_stitch::lang::CodeLang;

/// One built-in language participating in cross-language test matrices.
#[derive(Clone, Copy)]
pub struct BuiltInLanguage {
    /// Stable human-readable identifier used in assertion messages.
    pub id: &'static str,
    /// Primary file extension recognized by `FileSpec`.
    pub extension: &'static str,
    factory: fn() -> Box<dyn CodeLang>,
    indented_factory: fn(&str) -> Box<dyn CodeLang>,
}

impl BuiltInLanguage {
    /// Construct a fresh adapter for this language.
    pub fn adapter(self) -> Box<dyn CodeLang> {
        (self.factory)()
    }

    /// Construct a fresh adapter with exact non-default indentation bytes.
    pub fn adapter_with_indent(self, indent: &str) -> Box<dyn CodeLang> {
        (self.indented_factory)(indent)
    }
}

macro_rules! language {
    ($id:literal, $extension:literal, $path:path) => {
        BuiltInLanguage {
            id: $id,
            extension: $extension,
            factory: || Box::new(<$path>::new()),
            indented_factory: |indent| {
                let mut language = <$path>::new();
                language.indent = indent.to_string();
                Box::new(language)
            },
        }
    };
}

/// Canonical inventory consumed by every cross-language test matrix.
pub const BUILT_IN_LANGUAGES: [BuiltInLanguage; 20] = [
    language!("bash", "bash", sigil_stitch::lang::bash::Bash),
    language!("c", "c", sigil_stitch::lang::c::C),
    language!("cpp", "cpp", sigil_stitch::lang::cpp::Cpp),
    language!("csharp", "cs", sigil_stitch::lang::csharp::CSharp),
    language!("dart", "dart", sigil_stitch::lang::dart::Dart),
    language!("go", "go", sigil_stitch::lang::go::Go),
    language!("haskell", "hs", sigil_stitch::lang::haskell::Haskell),
    language!("java", "java", sigil_stitch::lang::java::Java),
    language!(
        "javascript",
        "js",
        sigil_stitch::lang::javascript::JavaScript
    ),
    language!("kotlin", "kt", sigil_stitch::lang::kotlin::Kotlin),
    language!("lua", "lua", sigil_stitch::lang::lua::Lua),
    language!("ocaml", "ml", sigil_stitch::lang::ocaml::OCaml),
    language!("php", "php", sigil_stitch::lang::php::Php),
    language!("python", "py", sigil_stitch::lang::python::Python),
    language!("ruby", "rb", sigil_stitch::lang::ruby::Ruby),
    language!("rust", "rs", sigil_stitch::lang::rust::Rust),
    language!("scala", "scala", sigil_stitch::lang::scala::Scala),
    language!("swift", "swift", sigil_stitch::lang::swift::Swift),
    language!(
        "typescript",
        "ts",
        sigil_stitch::lang::typescript::TypeScript
    ),
    language!("zsh", "zsh", sigil_stitch::lang::zsh::Zsh),
];

/// Construct the adapter registered under one stable language ID.
pub fn adapter_for(id: &str) -> Box<dyn CodeLang> {
    BUILT_IN_LANGUAGES
        .iter()
        .copied()
        .find(|language| language.id == id)
        .unwrap_or_else(|| panic!("unknown built-in language ID: {id}"))
        .adapter()
}

#[test]
fn registry_contains_every_builtin_once() {
    use std::collections::BTreeSet;

    assert_eq!(BUILT_IN_LANGUAGES.len(), 20);
    assert_eq!(
        BUILT_IN_LANGUAGES
            .iter()
            .map(|language| language.id)
            .collect::<BTreeSet<_>>()
            .len(),
        20
    );
    assert_eq!(
        BUILT_IN_LANGUAGES
            .iter()
            .map(|language| language.extension)
            .collect::<BTreeSet<_>>()
            .len(),
        20
    );
    for language in BUILT_IN_LANGUAGES {
        assert_eq!(language.adapter().file_extension(), language.extension);
        assert_eq!(
            adapter_for(language.id).file_extension(),
            language.extension
        );
        assert_eq!(language.adapter_with_indent("--->").indent_unit(), "--->");
    }
}
