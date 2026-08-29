use crate::code_block::CodeBlock;
use crate::code_renderer::CodeRenderer;
use crate::error::SigilStitchError;
use crate::import::{ImportAliasConflictResolver, ImportGroup};
use crate::import_collector;
use crate::lang::CodeLang;
use crate::spec::emittable::Emittable;
use crate::spec::fun_spec::FunSpec;
use crate::spec::import_spec::ImportSpec;
use crate::spec::modifiers::DeclarationContext;
use crate::spec::type_spec::TypeSpec;
use crate::type_name::TypeName;
use crate::type_name_lowering::{DiagnosticPath, TypeNameMaterializer};

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
/// complete rendering pipeline: declarations are materialized, source trees are
/// rewritten and validated, type references are lowered, imports are collected
/// and resolved, and the prepared source is rendered.
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

    /// Validate every spec member against the active language capabilities.
    ///
    /// Unknown external adapters inherit the permissive legacy capability
    /// profile, so this is strict only when the adapter declares a matrix.
    ///
    /// Validation is collected rather than fail-fast: every invalid type
    /// member is checked and all resulting errors are returned in one
    /// [`SigilStitchError::FileSpecValidation`]. A missing language is
    /// returned immediately as [`SigilStitchError::MissingLang`].
    pub fn validate(&self) -> Result<(), SigilStitchError> {
        let lang: &dyn CodeLang =
            self.lang
                .as_deref()
                .ok_or_else(|| SigilStitchError::MissingLang {
                    filename: self.filename.clone(),
                })?;

        let mut errors = Vec::new();
        for member in &self.members {
            match member {
                FileMember::Type(spec) => spec.collect_validation_errors(lang, &mut errors),
                FileMember::Fun(spec) => {
                    if let Err(error) = spec.validate(lang, DeclarationContext::TopLevel) {
                        errors.push(error);
                    }
                }
                FileMember::Spec(spec) => spec.collect_validation_errors(lang, &mut errors),
                FileMember::Code(_)
                | FileMember::RawContent(_)
                | FileMember::RawContentWithImports { .. } => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            let error_count = errors.len();
            Err(SigilStitchError::FileSpecValidation {
                filename: self.filename.clone(),
                error_count,
                errors,
            })
        }
    }

    /// Render the file with the built-in fallible import-alias policy.
    pub fn render(&self, width: usize) -> Result<String, SigilStitchError> {
        self.render_with_resolver(width, None)
    }

    /// Render the file with a borrowed import-alias conflict resolver.
    ///
    /// The resolver is an execution dependency and is never stored or
    /// serialized with this `FileSpec`.
    pub fn render_with_import_alias_resolver(
        &self,
        width: usize,
        resolver: &dyn ImportAliasConflictResolver,
    ) -> Result<String, SigilStitchError> {
        self.render_with_resolver(width, Some(resolver))
    }

    fn render_with_resolver(
        &self,
        width: usize,
        resolver: Option<&dyn ImportAliasConflictResolver>,
    ) -> Result<String, SigilStitchError> {
        self.validate()?;

        self.render_validated_with_resolver(width, resolver)
    }

    pub(crate) fn render_validated_with_resolver(
        &self,
        width: usize,
        resolver: Option<&dyn ImportAliasConflictResolver>,
    ) -> Result<String, SigilStitchError> {
        let lang: &dyn CodeLang =
            self.lang
                .as_deref()
                .ok_or_else(|| SigilStitchError::MissingLang {
                    filename: self.filename.clone(),
                })?;

        enum Emitted {
            Blocks(Vec<CodeBlock>),
            Raw(String),
            RawWithImports {
                content: String,
                types: Vec<TypeName>,
            },
        }

        enum Prepared {
            Blocks(Vec<CodeBlock>),
            Raw(String),
            RawWithImports {
                content: String,
                metadata: Vec<CodeBlock>,
            },
        }

        let mut emitted = Vec::with_capacity(self.members.len());
        for member in &self.members {
            emitted.push(match member {
                FileMember::Code(block) => Emitted::Blocks(vec![block.clone()]),
                FileMember::RawContent(s) => Emitted::Raw(s.clone()),
                FileMember::RawContentWithImports { content, types } => Emitted::RawWithImports {
                    content: content.clone(),
                    types: types.clone(),
                },
                FileMember::Type(spec) => Emitted::Blocks(spec.emit(lang)?),
                FileMember::Fun(spec) => {
                    Emitted::Blocks(vec![spec.emit(lang, DeclarationContext::TopLevel)?])
                }
                FileMember::Spec(spec) => Emitted::Blocks(spec.emit_members(lang)?),
            });
        }

        let mut materializer = TypeNameMaterializer::new(lang);
        let prepared_header = self
            .header
            .as_ref()
            .map(|header| materializer.prepare_source_block(header, DiagnosticPath::root("header")))
            .transpose()?;
        let mut prepared = Vec::with_capacity(emitted.len());
        for (member_index, member) in emitted.into_iter().enumerate() {
            prepared.push(match member {
                Emitted::Blocks(blocks) => Prepared::Blocks(
                    blocks
                        .iter()
                        .enumerate()
                        .map(|(block_index, block)| {
                            materializer.prepare_source_block(
                                block,
                                DiagnosticPath::member_block(member_index, block_index),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                Emitted::Raw(content) => Prepared::Raw(content),
                Emitted::RawWithImports { content, types } => {
                    let metadata = types
                        .iter()
                        .enumerate()
                        .map(|(type_index, type_name)| {
                            materializer.prepare_metadata_type(
                                type_name,
                                DiagnosticPath::raw_metadata(member_index, type_index),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Prepared::RawWithImports { content, metadata }
                }
            });
        }

        let mut import_refs = Vec::new();

        if let Some(header) = &prepared_header {
            import_refs.extend(import_collector::collect_imports(header));
        }

        for member in &prepared {
            match member {
                Prepared::Blocks(blocks) => {
                    for block in blocks {
                        import_refs.extend(import_collector::collect_imports(block));
                    }
                }
                Prepared::RawWithImports { metadata, .. } => {
                    for block in metadata {
                        import_refs.extend(import_collector::collect_imports(block));
                    }
                }
                Prepared::Raw(_) => {}
            }
        }

        let explicit_entries: Vec<_> = self
            .explicit_imports
            .iter()
            .cloned()
            .map(|spec| spec.into_entry())
            .collect();
        let imports = match resolver {
            Some(resolver) => {
                ImportGroup::try_resolve_with(&import_refs, explicit_entries, resolver)?
            }
            None => ImportGroup::try_resolve(&import_refs, explicit_entries)?,
        };
        lang.validate_resolved_imports(&imports)?;

        let mut output = String::new();

        if let Some(header) = &prepared_header {
            let renderer = CodeRenderer::new(lang, &imports, width);
            let header_output = renderer.render_prepared(header)?;
            if !header_output.is_empty() {
                output.push_str(&header_output);
                if !header_output.ends_with('\n') {
                    output.push('\n');
                }
                output.push('\n');
            }
        }

        let import_header = lang.render_imports(&imports);
        if !import_header.is_empty() {
            output.push_str(&import_header);
            output.push_str("\n\n");
        }

        for (i, member) in prepared.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            match member {
                Prepared::Blocks(blocks) => {
                    for (j, block) in blocks.iter().enumerate() {
                        if j > 0 {
                            output.push('\n');
                        }
                        let renderer = CodeRenderer::new(lang, &imports, width);
                        let member_output = renderer.render_prepared(block)?;
                        output.push_str(&member_output);
                        if !member_output.ends_with('\n') {
                            output.push('\n');
                        }
                    }
                }
                Prepared::Raw(content) => {
                    output.push_str(content);
                    if !content.ends_with('\n') {
                        output.push('\n');
                    }
                }
                Prepared::RawWithImports { content, .. } => {
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
