#![allow(deprecated)]

use pretty::BoxDoc;
use serde_json::Value;
use sigil_stitch::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::{ImportEntry, ImportGroup, ImportRef};
use sigil_stitch::lang::capability::{LanguageCapabilities, TypeCapabilityProfile};
use sigil_stitch::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    OptionalFieldStyle, QuoteStyle, TypeDeclSyntaxConfig, TypePresentationConfig,
};
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::modifiers::{DeclarationContext, PropertyStyle, TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec, render_type_params};
use sigil_stitch::type_name::TypeName;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
enum MarkerMode {
    Standard,
    Reorder,
    Duplicate,
    Drop,
    Changed,
    Malformed,
    Unknown,
}

#[derive(Debug)]
struct Legacy068Adapter {
    marker_mode: MarkerMode,
}

impl Legacy068Adapter {
    fn new(marker_mode: MarkerMode) -> Self {
        Self { marker_mode }
    }
}

impl RendererLang for Legacy068Adapter {
    fn file_extension(&self) -> &str {
        "legacy"
    }

    fn render_string_literal(&self, value: &str) -> String {
        format!("\"{value}\"")
    }

    fn render_verbatim_string(&self, value: &str) -> String {
        format!("r\"{value}\"")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn line_comment_suffix(&self) -> &str {
        ""
    }

    fn render_attribute(&self, text: &str) -> String {
        format!("@{text}")
    }

    fn reserved_words(&self) -> &[&str] {
        &["reserved"]
    }

    fn escape_reserved(&self, name: &str) -> String {
        if name == "reserved" {
            "reserved_".to_string()
        } else {
            name.to_string()
        }
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig::default()
    }

    fn block_open_for(&self, condition: &str) -> Option<&str> {
        match condition {
            "if legacy" => Some(" <legacy-if-open>"),
            "else" => Some(" <legacy-else-open>"),
            _ => None,
        }
    }

    fn block_close_for(&self, _condition: &str) -> Option<&str> {
        Some("<legacy-close>")
    }

    fn rewrite_nodes(&self, _nodes: &mut Vec<sigil_stitch::code_node::CodeNode>) {}

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig::default()
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig::default()
    }

    fn qualify_import_name(&self, module: &str, resolved_name: &str) -> String {
        format!("{module}::{resolved_name}")
    }

    fn module_separator(&self) -> Option<&str> {
        Some("::")
    }
}

impl CodeLang for Legacy068Adapter {
    fn render_imports(&self, _imports: &ImportGroup) -> String {
        String::new()
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        lines
            .iter()
            .map(|line| format!("/// {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_visibility(&self, visibility: Visibility, _context: DeclarationContext) -> &str {
        match visibility {
            Visibility::Public => "pub ",
            _ => "",
        }
    }

    fn function_keyword(&self, _context: DeclarationContext) -> &str {
        "fn"
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        "type"
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn escape_field_name(&self, name: &str) -> String {
        self.escape_reserved(name)
    }

    fn variable_prefix(&self) -> &str {
        ""
    }

    fn type_kind_suffix(&self, _kind: TypeKind) -> &str {
        ""
    }

    fn render_newtype_line(&self, visibility: &str, name: &str, inner: &str) -> String {
        match self.marker_mode {
            MarkerMode::Standard => format!("{visibility}legacy {name} = {inner};"),
            MarkerMode::Reorder => format!("{inner} {name}"),
            MarkerMode::Duplicate => format!("{name} {inner} {inner}"),
            MarkerMode::Drop => name.to_string(),
            MarkerMode::Changed => inner.replacen("00000000", "00000001", 1),
            MarkerMode::Malformed => "__SIGIL_STITCH_LEGACY_TYPE_".to_string(),
            MarkerMode::Unknown => {
                "__SIGIL_STITCH_LEGACY_TYPE_ffffffffffffffff_00000000__".to_string()
            }
        }
    }

    fn fun_block_open(&self) -> &str {
        " {"
    }

    fn type_header_block_open(&self, _kind: TypeKind) -> &str {
        " {"
    }

    fn doc_comment_inside_body(&self) -> bool {
        false
    }

    fn doc_before_annotations(&self) -> bool {
        true
    }

    fn optional_field_style(&self) -> OptionalFieldStyle {
        OptionalFieldStyle::Ignored
    }

    fn property_style(&self) -> PropertyStyle {
        PropertyStyle::Accessor
    }

    fn property_getter_keyword(&self) -> &str {
        "get"
    }

    fn render_type_context(&self, type_params: &[TypeParamSpec]) -> String {
        let mut arguments = Vec::new();
        let parameters = render_type_params(type_params, self, &mut arguments);
        if parameters.is_empty() {
            String::new()
        } else {
            let block = CodeBlock::of(&parameters, arguments).unwrap();
            format!("where {} => ", render(&block, self))
        }
    }

    fn render_type_close_suffix(&self, _kind: TypeKind, impl_types: &[String]) -> String {
        if impl_types.is_empty() {
            String::new()
        } else {
            format!(" derives {}", impl_types.join(" + "))
        }
    }

    fn type_body_prefix(&self, _name: &str, _kind: TypeKind) -> String {
        String::new()
    }

    fn type_body_suffix(&self, _name: &str, _kind: TypeKind) -> String {
        String::new()
    }

    fn render_type_param_kind(&self, _kind: &TypeParamKind) -> String {
        String::new()
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig::default()
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig::default()
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig::default()
    }
}

fn render(block: &CodeBlock, lang: &dyn RendererLang) -> String {
    let imports = ImportGroup::new();
    CodeRenderer::new(lang, &imports, 80).render(block).unwrap()
}

fn bounded_type_params() -> Vec<TypeParamSpec> {
    vec![TypeParamSpec::new("T").with_bound(TypeName::primitive("Bound"))]
}

#[test]
fn exact_0_6_8_signatures_remain_callable() {
    type Resolve = fn(&str, &str) -> String;

    let _: fn(QuoteStyle) -> char = QuoteStyle::char;
    let _: fn(&TypeName, &Resolve) -> BoxDoc<'static, ()> = TypeName::to_doc::<Resolve>;
    let _: fn(&TypeName, usize, &Resolve) -> Result<String, SigilStitchError> =
        TypeName::render::<Resolve>;
    let _: fn(&TypeName, &Resolve, &dyn RendererLang) -> BoxDoc<'static, ()> =
        TypeName::to_doc_with_lang::<Resolve>;
    let _: fn(&[TypeParamSpec], &dyn CodeLang, &mut Vec<Arg>) -> String = render_type_params;
    let _: fn(&ParameterSpec, &mut CodeBlockBuilder, &dyn CodeLang) = ParameterSpec::emit_into;
    let _: fn() -> ImportGroup = ImportGroup::new;
    let _: fn(&[ImportRef]) -> ImportGroup = ImportGroup::resolve;
    let _: fn(&[ImportRef], Vec<ImportEntry>) -> ImportGroup = ImportGroup::resolve_with_explicit;
    let _: fn(
        sigil_stitch::lang::typescript::TypeScript,
        QuoteStyle,
    ) -> sigil_stitch::lang::typescript::TypeScript =
        sigil_stitch::lang::typescript::TypeScript::with_quote_style;
    let _: fn(
        sigil_stitch::lang::javascript::JavaScript,
        QuoteStyle,
    ) -> sigil_stitch::lang::javascript::JavaScript =
        sigil_stitch::lang::javascript::JavaScript::with_quote_style;
    let _: fn(
        sigil_stitch::lang::python::Python,
        QuoteStyle,
    ) -> sigil_stitch::lang::python::Python = sigil_stitch::lang::python::Python::with_quote_style;
    let _: fn(&Legacy068Adapter, &str, &str) -> String =
        <Legacy068Adapter as RendererLang>::qualify_import_name;
    let _: fn(&Legacy068Adapter, &str, &str, &str) -> String =
        <Legacy068Adapter as CodeLang>::render_newtype_line;
    let _: fn(&Legacy068Adapter, &[TypeParamSpec]) -> String =
        <Legacy068Adapter as CodeLang>::render_type_context;
    let _: fn(&Legacy068Adapter, TypeKind, &[String]) -> String =
        <Legacy068Adapter as CodeLang>::render_type_close_suffix;

    let resolve = |_module: &str, name: &str| name.to_string();
    let primitive = TypeName::primitive("Value");
    let mut rendered_doc = Vec::new();
    primitive
        .to_doc(&resolve)
        .render(80, &mut rendered_doc)
        .unwrap();
    assert_eq!(String::from_utf8(rendered_doc).unwrap(), "Value");
    assert_eq!(primitive.render(80, &resolve).unwrap(), "Value");

    let _: QuoteStyle = sigil_stitch::lang::typescript::TypeScript::new().quote_style;
    let _: QuoteStyle = sigil_stitch::lang::javascript::JavaScript::new().quote_style;
    let _: QuoteStyle = sigil_stitch::lang::python::Python::new().quote_style;
}

#[test]
fn external_0_6_8_adapter_defaults_compile_and_render_structurally() {
    let lang = Legacy068Adapter::new(MarkerMode::Standard);
    let params = bounded_type_params();
    let newtype = lang
        .emit_newtype_decl("pub ", "Meters", &params, &TypeName::primitive("Inner"))
        .unwrap();
    assert_eq!(
        render(&newtype, &lang),
        "pub legacy Meters<T: Bound> = Inner;"
    );

    let context = lang.emit_type_context(&params).unwrap().unwrap();
    assert_eq!(render(&context, &lang), "where <T: Bound> => ");

    let suffix = lang
        .emit_type_close_suffix(TypeKind::Struct, &[TypeName::primitive("Eq")])
        .unwrap()
        .unwrap();
    assert_eq!(render(&suffix, &lang), " derives Eq");

    let imported = TypeName::importable("legacy.module", "Value").with_alias("Alias");
    let imports = ImportGroup::resolve(&[ImportRef {
        module: "legacy.module".to_string(),
        name: "Value".to_string(),
        is_type_only: false,
        alias: Some("Alias".to_string()),
    }]);
    let block = CodeBlock::of("%T", imported).unwrap();
    assert_eq!(
        CodeRenderer::new(&lang, &imports, 80)
            .render(&block)
            .unwrap(),
        "legacy.module::Alias"
    );
}

#[test]
fn external_0_6_8_block_hooks_render_through_intent_defaults() {
    let lang = Legacy068Adapter::new(MarkerMode::Standard);
    let mut block = CodeBlock::builder();
    block.begin_control_flow("if legacy", ());
    block.add_statement("first", ());
    block.next_control_flow("else", ());
    block.add_statement("second", ());
    block.end_control_flow();

    assert_eq!(
        render(&block.build().unwrap(), &lang),
        "if legacy <legacy-if-open>\n  first;\n<legacy-close> else <legacy-else-open>\n  second;\n<legacy-close>\n"
    );
}

#[test]
fn marker_recovery_preserves_reordering_and_duplication() {
    let params = bounded_type_params();

    let reordered = Legacy068Adapter::new(MarkerMode::Reorder)
        .emit_newtype_decl("", "Value", &params, &TypeName::primitive("Inner"))
        .unwrap();
    assert_eq!(
        render(&reordered, &Legacy068Adapter::new(MarkerMode::Standard)),
        "Inner Value<T: Bound>"
    );

    let duplicated = Legacy068Adapter::new(MarkerMode::Duplicate)
        .emit_newtype_decl("", "Value", &params, &TypeName::primitive("Inner"))
        .unwrap();
    assert_eq!(
        render(&duplicated, &Legacy068Adapter::new(MarkerMode::Standard)),
        "Value<T: Bound> Inner Inner"
    );
}

#[test]
fn marker_recovery_fails_closed_for_lossy_or_invalid_hooks() {
    for mode in [
        MarkerMode::Drop,
        MarkerMode::Changed,
        MarkerMode::Malformed,
        MarkerMode::Unknown,
    ] {
        let error = Legacy068Adapter::new(mode)
            .emit_newtype_decl("", "Value", &[], &TypeName::primitive("Inner"))
            .unwrap_err();
        assert!(
            matches!(error, SigilStitchError::Render { .. }),
            "unexpected error for {mode:?}: {error}"
        );
        assert!(error.to_string().contains("0.6.8 compatibility hook"));
    }
}

#[derive(Debug)]
struct StrictIncompleteAdapter;

const STRICT_TYPES: &[TypeCapabilityProfile<'_>] =
    &[TypeCapabilityProfile::new(TypeKind::Struct, &[])];

impl RendererLang for StrictIncompleteAdapter {
    fn file_extension(&self) -> &str {
        "strict"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for StrictIncompleteAdapter {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict().with_types(STRICT_TYPES)
    }
}

#[test]
fn strict_family_without_complete_lowerer_fails_closed() {
    let spec = TypeSpec::builder("Record", TypeKind::Struct)
        .build()
        .unwrap();
    let error = spec.emit(&StrictIncompleteAdapter).unwrap_err();
    assert!(matches!(error, SigilStitchError::MissingTypeLowerer { .. }));
}

#[test]
fn default_empty_legacy_bridges_remain_empty() {
    assert!(
        StrictIncompleteAdapter
            .emit_type_context(&[])
            .unwrap()
            .is_none()
    );
    assert!(
        StrictIncompleteAdapter
            .emit_type_close_suffix(TypeKind::Struct, &[])
            .unwrap()
            .is_none()
    );
}

fn import_ref(module: &str, name: &str) -> ImportRef {
    ImportRef {
        module: module.to_string(),
        name: name.to_string(),
        is_type_only: false,
        alias: None,
    }
}

#[test]
fn legacy_import_resolvers_keep_duplicate_binding_behavior() {
    let refs = [
        import_ref("./models", "User"),
        import_ref("./other", "User"),
    ];
    let resolved = ImportGroup::resolve(&refs);
    assert_eq!(resolved.entries()[0].alias, None);
    assert_eq!(resolved.entries()[1].alias.as_deref(), Some("OtherUser"));

    let preferred_duplicates = [
        ImportRef {
            alias: Some("Shared".to_string()),
            ..import_ref("./models", "User")
        },
        ImportRef {
            alias: Some("Shared".to_string()),
            ..import_ref("./other", "Account")
        },
    ];
    let resolved = ImportGroup::resolve(&preferred_duplicates);
    assert_eq!(resolved.entries()[0].resolved_name(), "Shared");
    assert_eq!(resolved.entries()[1].resolved_name(), "Shared");

    let explicit = ImportEntry {
        module: "./explicit".to_string(),
        name: "User".to_string(),
        alias: None,
        is_type_only: false,
        is_side_effect: false,
        is_wildcard: false,
    };
    let resolved =
        ImportGroup::resolve_with_explicit(&[import_ref("./other", "User")], vec![explicit]);
    assert_eq!(resolved.entries()[0].alias, None);
    assert_eq!(resolved.entries()[1].alias.as_deref(), Some("OtherUser"));

    let duplicate_explicit = vec![
        ImportEntry {
            module: "./first".to_string(),
            name: "User".to_string(),
            alias: Some("Shared".to_string()),
            is_type_only: false,
            is_side_effect: false,
            is_wildcard: false,
        },
        ImportEntry {
            module: "./second".to_string(),
            name: "Account".to_string(),
            alias: Some("Shared".to_string()),
            is_type_only: false,
            is_side_effect: false,
            is_wildcard: false,
        },
    ];
    let resolved = ImportGroup::resolve_with_explicit(&[], duplicate_explicit);
    assert_eq!(resolved.entries()[0].resolved_name(), "Shared");
    assert_eq!(resolved.entries()[1].resolved_name(), "Shared");
}

#[test]
fn documented_0_6_8_type_name_json_values_are_unchanged() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("compatibility/type-name-0.6.8.json")).unwrap();
    let cases = [
        TypeName::importable("./models", "User"),
        TypeName::generic(
            TypeName::primitive("Result"),
            vec![
                TypeName::primitive("Value"),
                TypeName::optional(TypeName::primitive("Error")),
            ],
        ),
        TypeName::Reference {
            inner: Box::new(TypeName::primitive("str")),
            mutable: false,
            lifetime: Some("'a".to_string()),
        },
        TypeName::Wildcard {
            upper_bound: Some(Box::new(TypeName::primitive("Number"))),
            lower_bound: None,
        },
        TypeName::Function {
            params: vec![TypeName::primitive("Input")],
            return_type: Box::new(TypeName::primitive("Output")),
        },
    ];

    assert_eq!(fixtures.len(), cases.len());
    for (fixture, case) in fixtures.into_iter().zip(cases) {
        assert_eq!(serde_json::to_value(&case).unwrap(), fixture);
        assert_eq!(serde_json::from_value::<TypeName>(fixture).unwrap(), case);
    }
}

#[test]
fn restored_built_in_hooks_match_0_6_8() {
    use sigil_stitch::lang::c::C;
    use sigil_stitch::lang::go::Go;
    use sigil_stitch::lang::haskell::Haskell;
    use sigil_stitch::lang::kotlin::Kotlin;
    use sigil_stitch::lang::php::Php;
    use sigil_stitch::lang::python::Python;
    use sigil_stitch::lang::scala::Scala;

    assert_eq!(
        C::new().render_newtype_line("", "Id", "u64"),
        "typedef u64 Id;"
    );
    assert_eq!(
        Go::new().render_newtype_line("", "Id", "uint64"),
        "type Id uint64"
    );
    assert_eq!(
        Haskell::new().render_newtype_line("", "Id", "Word64"),
        "newtype Id = Id Word64"
    );
    assert_eq!(
        Kotlin::new().render_newtype_line("public ", "Id", "ULong"),
        "public value class Id(val value: ULong)"
    );
    assert_eq!(
        Php::new().render_newtype_line("public ", "Id", "int"),
        "public class Id { public function __construct(private int $value) {} }"
    );
    assert_eq!(
        Python::new().render_newtype_line("", "Id", "int"),
        "Id = NewType(\"Id\", int)"
    );
    assert_eq!(
        Scala::new().render_newtype_line("final ", "Id", "Long"),
        "final class Id(val value: Long)"
    );

    let go = Go::new();
    let go_newtype = go
        .emit_newtype_decl(
            "",
            "Id",
            &bounded_type_params(),
            &TypeName::primitive("uint64"),
        )
        .unwrap();
    assert_eq!(render(&go_newtype, &go), "type Id[T Bound] uint64");

    let kotlin = Kotlin::new();
    let kotlin_newtype = kotlin
        .emit_newtype_decl(
            "public ",
            "Id",
            &bounded_type_params(),
            &TypeName::primitive("ULong"),
        )
        .unwrap();
    assert_eq!(
        render(&kotlin_newtype, &kotlin),
        "public value class Id<T : Bound>(val value: ULong)"
    );

    let haskell = Haskell::new();
    assert_eq!(haskell.render_type_context(&[]), "");
    let context = haskell.render_type_context(&bounded_type_params());
    assert_eq!(context, "Bound T => ");
    let multiple_bounds = vec![
        TypeParamSpec::new("T")
            .with_bound(TypeName::primitive("Bound"))
            .with_bound(TypeName::primitive("Other")),
    ];
    assert_eq!(
        haskell.render_type_context(&multiple_bounds),
        "(Bound T, Other T) => "
    );
    assert_eq!(haskell.render_type_close_suffix(TypeKind::Struct, &[]), "");
    assert_eq!(
        haskell.render_type_close_suffix(TypeKind::Struct, &["Eq".to_string()]),
        "  deriving (Eq)"
    );
}

#[test]
fn public_compatibility_manifest_is_bounded_and_well_formed() {
    let manifest = include_str!("compatibility/public-api-0.6.8.txt");
    let mut records = BTreeSet::new();
    let mut signatures = std::collections::BTreeMap::new();
    for (index, line) in manifest.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        assert_eq!(fields.len(), 4, "malformed manifest line {}", index + 1);
        assert!(
            matches!(fields[0], "type" | "field" | "method" | "function"),
            "unknown manifest kind on line {}",
            index + 1
        );
        assert!(
            matches!(
                fields[3],
                "compat"
                    | "type-name-lowering"
                    | "generic-declaration-lowering"
                    | "renderer-events"
                    | "quote-handling"
            ),
            "unknown retirement owner on line {}",
            index + 1
        );
        assert!(
            records.insert(fields[1]),
            "duplicate manifest surface {}",
            fields[1]
        );
        match fields[0] {
            "type" => assert!(
                fields[2].starts_with("pub enum ") || fields[2].starts_with("pub struct "),
                "invalid type signature on line {}",
                index + 1
            ),
            "field" => assert!(
                fields[2].starts_with("pub ") && fields[2].contains(": "),
                "invalid field signature on line {}",
                index + 1
            ),
            "method" | "function" => assert!(
                fields[2].starts_with("fn ") || fields[2].starts_with("pub fn "),
                "invalid function signature on line {}",
                index + 1
            ),
            _ => unreachable!(),
        }
        signatures.insert(fields[1], fields[2]);
    }

    assert_eq!(records.len(), 144, "unexpected compatibility surface count");

    for required in [
        "lang::config::TypePresentationConfig",
        "lang::config::GenericSyntaxConfig",
        "lang::config::BlockSyntaxConfig",
        "lang::config::QuoteStyle",
        "lang::RendererLang::qualify_import_name",
        "lang::CodeLang::function_keyword",
        "lang::CodeLang::type_keyword",
        "lang::CodeLang::methods_inside_type_body",
        "lang::CodeLang::variable_prefix",
        "lang::CodeLang::type_kind_suffix",
        "lang::CodeLang::render_newtype_line",
        "lang::CodeLang::fun_block_open",
        "lang::CodeLang::type_header_block_open",
        "lang::CodeLang::doc_comment_inside_body",
        "lang::CodeLang::doc_before_annotations",
        "lang::CodeLang::render_type_context",
        "lang::CodeLang::type_body_prefix",
        "lang::CodeLang::type_body_suffix",
        "lang::CodeLang::render_type_close_suffix",
        "lang::CodeLang::render_type_param_kind",
        "type_name::TypeName::to_doc",
        "type_name::TypeName::render",
        "spec::where_spec::render_type_params",
        "spec::parameter_spec::ParameterSpec::emit_into",
        "import::ImportGroup::new",
        "import::ImportGroup::resolve",
        "import::ImportGroup::resolve_with_explicit",
        "lang::config::QuoteStyle::char",
        "lang::typescript::TypeScript::quote_style",
        "lang::javascript::JavaScript::quote_style",
        "lang::python::Python::quote_style",
    ] {
        assert!(
            records.contains(required),
            "missing manifest surface {required}"
        );
    }

    for (surface, signature) in [
        (
            "lang::config::QuoteStyle::char",
            "pub fn char(self) -> char",
        ),
        (
            "type_name::TypeName::to_doc",
            "pub fn to_doc<F>(&self, resolve: &F) -> BoxDoc<'static, ()> where F: Fn(&str, &str) -> String",
        ),
        (
            "type_name::TypeName::render",
            "pub fn render<F>(&self, width: usize, resolve: &F) -> Result<String, crate::error::SigilStitchError> where F: Fn(&str, &str) -> String",
        ),
        (
            "type_name::TypeName::to_doc_with_lang",
            "pub fn to_doc_with_lang<F>(&self, resolve: &F, lang: &dyn RendererLang) -> BoxDoc<'static, ()> where F: Fn(&str, &str) -> String",
        ),
        (
            "spec::where_spec::render_type_params",
            "pub fn render_type_params(params: &[TypeParamSpec], lang: &dyn CodeLang, args: &mut Vec<Arg>) -> String",
        ),
        (
            "spec::parameter_spec::ParameterSpec::emit_into",
            "pub fn emit_into(&self, cb: &mut CodeBlockBuilder, lang: &dyn CodeLang)",
        ),
        ("import::ImportGroup::new", "pub fn new() -> Self"),
        (
            "import::ImportGroup::resolve",
            "pub fn resolve(refs: &[ImportRef]) -> Self",
        ),
        (
            "import::ImportGroup::resolve_with_explicit",
            "pub fn resolve_with_explicit(refs: &[ImportRef], explicit: Vec<ImportEntry>) -> Self",
        ),
        (
            "lang::typescript::TypeScript::quote_style",
            "pub quote_style: QuoteStyle",
        ),
        (
            "lang::typescript::TypeScript::with_quote_style",
            "pub fn with_quote_style(mut self, qs: QuoteStyle) -> Self",
        ),
        (
            "lang::javascript::JavaScript::quote_style",
            "pub quote_style: QuoteStyle",
        ),
        (
            "lang::javascript::JavaScript::with_quote_style",
            "pub fn with_quote_style(mut self, qs: QuoteStyle) -> Self",
        ),
        (
            "lang::python::Python::quote_style",
            "pub quote_style: QuoteStyle",
        ),
        (
            "lang::python::Python::with_quote_style",
            "pub fn with_quote_style(mut self, qs: QuoteStyle) -> Self",
        ),
    ] {
        assert_eq!(
            signatures.get(surface),
            Some(&signature),
            "wrong 0.6.8 signature for {surface}"
        );
    }
}
