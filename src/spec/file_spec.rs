use crate::code_block::CodeBlock;
use crate::code_renderer::CodeRenderer;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::import_collector;
use crate::lang::CodeLang;
use crate::spec::emittable::Emittable;
use crate::spec::fun_spec::FunSpec;
use crate::spec::import_spec::ImportSpec;
use crate::spec::modifiers::DeclarationContext;
use crate::spec::type_spec::TypeSpec;
use crate::type_name::TypeName;

/// A member of a file.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum FileMember {
    /// A CodeBlock (e.g., module-level statements, class declarations).
    Code(CodeBlock),
    /// Raw content string (escape hatch, no import tracking).
    RawContent(String),
    /// Raw content string with associated types for import tracking.
    ///
    /// Content is emitted verbatim; types are walked for import collection only.
    /// The caller is responsible for ensuring type names in the raw content match
    /// what the import resolver will emit.
    RawContentWithImports {
        /// The raw content to emit verbatim.
        content: String,
        /// Types to register for import collection.
        types: Vec<TypeName>,
    },
    /// A type declaration (struct, class, interface, trait, enum).
    Type(TypeSpec),
    /// A top-level function.
    Fun(FunSpec),
    /// A type-erased spec for custom or third-party spec types.
    ///
    /// Use [`FileSpecBuilder::add_spec`] to add these. Not serializable —
    /// this variant is skipped during serde round-trips.
    #[serde(skip)]
    Spec(Box<dyn Emittable>),
}

/// A complete source file with automatic import management.
///
/// `FileSpec` is the top-level orchestrator that combines code blocks, type
/// declarations, and functions into a rendered source file. It drives the
/// three-pass rendering pipeline:
///
/// 1. **Materialize** -- Specs (`TypeSpec`, `FunSpec`) emit `CodeBlock`s
/// 2. **Collect imports** -- Walk all blocks, extract `ImportRef` from `%T` types
/// 3. **Render** -- Emit import header + body with resolved names and pretty printing
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let user = TypeName::importable_type("./models", "User");
///
/// let mut cb = CodeBlock::builder();
/// cb.add_statement("const u: %T = getUser()", (user,));
/// let body = cb.build().unwrap();
///
/// let file = FileSpec::builder("user.ts")
///     .add_code(body)
///     .build().unwrap();
///
/// let output = file.render(80).unwrap();
/// // output contains: import type { User } from './models'
/// // output contains: const u: User = getUser();
/// ```
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileSpec {
    filename: String,
    header: Option<CodeBlock>,
    members: Vec<FileMember>,
    explicit_imports: Vec<ImportSpec>,
    #[serde(skip)]
    lang: Option<Box<dyn CodeLang>>,
}

impl FileSpec {
    /// Create a builder that auto-detects the language from the filename extension.
    ///
    /// If the extension is not recognized, [`build()`](FileSpecBuilder::build) will
    /// return an error. Use [`builder_with`](FileSpec::builder_with) for explicit
    /// language control or unsupported extensions.
    pub fn builder(filename: &str) -> FileSpecBuilder {
        let ext = filename.rsplit('.').next().unwrap_or("");
        let lang = crate::lang::lang_from_extension(ext);
        FileSpecBuilder {
            filename: filename.to_string(),
            header: None,
            members: Vec::new(),
            explicit_imports: Vec::new(),
            lang,
        }
    }

    /// Create a builder with a specific language configuration.
    pub fn builder_with(filename: &str, lang: impl CodeLang) -> FileSpecBuilder {
        FileSpecBuilder {
            filename: filename.to_string(),
            header: None,
            members: Vec::new(),
            explicit_imports: Vec::new(),
            lang: Some(Box::new(lang)),
        }
    }

    /// Get the filename.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Set the language for this FileSpec.
    ///
    /// Required after deserialization, since the `lang` field is not serialized.
    /// Returns `self` for chaining.
    pub fn with_lang(mut self, lang: impl CodeLang) -> Self {
        self.lang = Some(Box::new(lang));
        self
    }

    /// Render the file to a string using the three-pass algorithm.
    ///
    /// `width` controls the target line width for pretty-printing.
    pub fn render(&self, width: usize) -> Result<String, SigilStitchError> {
        let lang: &dyn CodeLang =
            self.lang
                .as_deref()
                .ok_or_else(|| SigilStitchError::MissingLang {
                    filename: self.filename.clone(),
                })?;

        // Phase 0: Materialize specs into CodeBlocks.
        enum Materialized {
            Blocks(Vec<CodeBlock>),
            Raw(String),
            RawWithImports {
                content: String,
                types: Vec<TypeName>,
            },
        }

        let mut materialized: Vec<Materialized> = Vec::with_capacity(self.members.len());
        for m in &self.members {
            materialized.push(match m {
                FileMember::Code(b) => Materialized::Blocks(vec![b.clone()]),
                FileMember::RawContent(s) => Materialized::Raw(s.clone()),
                FileMember::RawContentWithImports { content, types } => {
                    Materialized::RawWithImports {
                        content: content.clone(),
                        types: types.clone(),
                    }
                }
                FileMember::Type(spec) => Materialized::Blocks(spec.emit(lang)?),
                FileMember::Fun(spec) => {
                    Materialized::Blocks(vec![spec.emit(lang, DeclarationContext::TopLevel)?])
                }
                FileMember::Spec(spec) => Materialized::Blocks(spec.emit_members(lang)?),
            });
        }

        // Pass 1: Collect imports from all CodeBlock members.
        let mut import_refs = Vec::new();

        if let Some(header) = &self.header {
            import_refs.extend(import_collector::collect_imports(header));
        }

        for mat in &materialized {
            match mat {
                Materialized::Blocks(blocks) => {
                    for block in blocks {
                        import_refs.extend(import_collector::collect_imports(block));
                    }
                }
                Materialized::RawWithImports { types, .. } => {
                    for ty in types {
                        ty.collect_imports(&mut import_refs);
                    }
                }
                Materialized::Raw(_) => {}
            }
        }

        // Import Resolution: Dedup, conflict detection, alias assignment.
        // Convert explicit ImportSpec entries to ImportEntry and merge.
        let explicit_entries: Vec<_> = self
            .explicit_imports
            .iter()
            .cloned()
            .map(|spec| spec.into_entry())
            .collect();
        let imports = ImportGroup::resolve_with_explicit(&import_refs, explicit_entries);

        // Pass 2: Render with resolved names.
        let mut output = String::new();

        // Render header block if present (e.g., license comment, Go package declaration).
        if let Some(header) = &self.header {
            let mut renderer = CodeRenderer::new(lang, &imports, width);
            let header_output = renderer.render(header)?;
            if !header_output.is_empty() {
                output.push_str(&header_output);
                if !header_output.ends_with('\n') {
                    output.push('\n');
                }
                output.push('\n');
            }
        }

        // Render import header.
        let import_header = lang.render_imports(&imports);
        if !import_header.is_empty() {
            output.push_str(&import_header);
            output.push_str("\n\n");
        }

        // Render materialized members.
        for (i, mat) in materialized.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            match mat {
                Materialized::Blocks(blocks) => {
                    for (j, block) in blocks.iter().enumerate() {
                        if j > 0 {
                            output.push('\n');
                        }
                        let mut renderer = CodeRenderer::new(lang, &imports, width);
                        let member_output = renderer.render(block)?;
                        output.push_str(&member_output);
                        if !member_output.ends_with('\n') {
                            output.push('\n');
                        }
                    }
                }
                Materialized::Raw(content) => {
                    output.push_str(content);
                    if !content.ends_with('\n') {
                        output.push('\n');
                    }
                }
                Materialized::RawWithImports { content, .. } => {
                    output.push_str(content);
                    if !content.ends_with('\n') {
                        output.push('\n');
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Builder for [`FileSpec`].
///
/// Use [`FileSpec::builder()`] to create. Add members with `add_code()`,
/// `add_type()`, `add_function()`, or `add_raw()`, then call `build()`.
#[derive(Debug)]
pub struct FileSpecBuilder {
    filename: String,
    header: Option<CodeBlock>,
    members: Vec<FileMember>,
    explicit_imports: Vec<ImportSpec>,
    lang: Option<Box<dyn CodeLang>>,
}

impl FileSpecBuilder {
    /// Set a file header (e.g., license comment, package declaration).
    pub fn header(mut self, block: CodeBlock) -> Self {
        self.header = Some(block);
        self
    }

    /// Add a CodeBlock member.
    pub fn add_code(mut self, block: CodeBlock) -> Self {
        self.members.push(FileMember::Code(block));
        self
    }

    /// Add raw content (no import tracking).
    pub fn add_raw(mut self, content: &str) -> Self {
        self.members
            .push(FileMember::RawContent(content.to_string()));
        self
    }

    /// Add raw content with associated types for import tracking.
    ///
    /// The content is emitted verbatim (no substitution). The types are walked
    /// during import collection so the correct import statements are generated.
    pub fn add_raw_with_imports(mut self, content: &str, types: Vec<TypeName>) -> Self {
        self.members.push(FileMember::RawContentWithImports {
            content: content.to_string(),
            types,
        });
        self
    }

    /// Add a generic member.
    pub fn add_member(mut self, member: FileMember) -> Self {
        self.members.push(member);
        self
    }

    /// Add a type declaration (struct, class, interface, trait, enum).
    pub fn add_type(mut self, spec: TypeSpec) -> Self {
        self.members.push(FileMember::Type(spec));
        self
    }

    /// Add a top-level function.
    pub fn add_function(mut self, spec: FunSpec) -> Self {
        self.members.push(FileMember::Fun(spec));
        self
    }

    /// Add a custom spec that implements [`Emittable`].
    pub fn add_spec(mut self, spec: impl Emittable + 'static) -> Self {
        self.members.push(FileMember::Spec(Box::new(spec)));
        self
    }

    /// Set the language configuration.
    pub fn lang(mut self, lang: impl CodeLang) -> Self {
        self.lang = Some(Box::new(lang));
        self
    }

    /// Add an explicit import (forced, aliased, side-effect, or wildcard).
    pub fn add_import(mut self, spec: ImportSpec) -> Self {
        self.explicit_imports.push(spec);
        self
    }

    /// Build the FileSpec.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `filename` is empty.
    /// Returns an error if no language was detected or configured.
    pub fn build(self) -> Result<FileSpec, SigilStitchError> {
        snafu::ensure!(
            !self.filename.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "FileSpecBuilder",
            }
        );
        let lang = self.lang.ok_or_else(|| {
            let ext = self.filename.rsplit('.').next().unwrap_or("");
            SigilStitchError::Render {
                context: "FileSpecBuilder::build()".to_string(),
                message: format!(
                    "unrecognized file extension '.{ext}' in filename '{}'; \
                     use FileSpec::builder_with() to specify the language explicitly",
                    self.filename
                ),
            }
        })?;
        Ok(FileSpec {
            filename: self.filename,
            header: self.header,
            members: self.members,
            explicit_imports: self.explicit_imports,
            lang: Some(lang),
        })
    }
}
