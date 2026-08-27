//! Multi-file project specification.
//!
//! `ProjectSpec` orchestrates rendering multiple `FileSpec`s as a unit,
//! returning an in-memory collection of rendered files or writing them
//! to the filesystem. It validates cross-file invariants (e.g. no
//! duplicate filenames) that individual `FileSpec`s cannot check alone.

use std::collections::HashMap;

use crate::error::SigilStitchError;
use crate::import::ImportAliasConflictResolver;
use crate::spec::file_spec::FileSpec;

/// A rendered file produced by [`ProjectSpec::render()`]: path and content pair.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderedFile {
    /// The file path (as provided to `FileSpec::builder`).
    pub path: String,
    /// The rendered file content.
    pub content: String,
}

/// A multi-file project that renders all files as a unit.
///
/// `ProjectSpec` orchestrates rendering multiple [`FileSpec`]s, returning an
/// in-memory collection of [`RenderedFile`]s or writing them to the filesystem.
/// Each file resolves imports independently. Cross-file invariants — such as
/// unique filenames — are validated at build time.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let models = FileSpec::builder("src/models.ts")
///     .add_type(TypeSpec::builder("User", TypeKind::Interface).build().unwrap())
///     .build().unwrap();
///
/// let index = FileSpec::builder("src/index.ts")
///     .add_code(CodeBlock::of("export {}", ()).unwrap())
///     .build().unwrap();
///
/// let project = ProjectSpec::builder()
///     .add_file(models)
///     .add_file(index)
///     .build().unwrap();
///
/// let rendered = project.render(80).unwrap();
/// assert_eq!(rendered.len(), 2);
/// ```
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectSpec {
    pub(crate) files: Vec<FileSpec>,
}

impl ProjectSpec {
    /// Create a new builder for a project specification.
    pub fn builder() -> ProjectSpecBuilder {
        ProjectSpecBuilder { files: Vec::new() }
    }

    /// Render all files and return their paths and contents.
    ///
    /// Each file resolves imports independently. File ordering is preserved.
    /// Fails on the first render error, including the filename in the message.
    pub fn render(&self, width: usize) -> Result<Vec<RenderedFile>, SigilStitchError> {
        self.render_with_resolver(width, None)
    }

    /// Render every file with one borrowed per-execution alias policy.
    pub fn render_with_import_alias_resolver(
        &self,
        width: usize,
        resolver: &dyn ImportAliasConflictResolver,
    ) -> Result<Vec<RenderedFile>, SigilStitchError> {
        self.render_with_resolver(width, Some(resolver))
    }

    fn render_with_resolver(
        &self,
        width: usize,
        resolver: Option<&dyn ImportAliasConflictResolver>,
    ) -> Result<Vec<RenderedFile>, SigilStitchError> {
        let mut rendered = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let content = match resolver {
                Some(resolver) => file.render_with_import_alias_resolver(width, resolver)?,
                None => file.render(width)?,
            };
            rendered.push(RenderedFile {
                path: file.filename().to_string(),
                content,
            });
        }
        Ok(rendered)
    }

    /// Render all files and write them to `base_dir`.
    ///
    /// Creates parent directories as needed. Returns the list of written paths.
    pub fn write_to(
        &self,
        base_dir: &std::path::Path,
        width: usize,
    ) -> Result<Vec<std::path::PathBuf>, SigilStitchError> {
        let rendered = self.render(width)?;
        write_rendered_files(base_dir, &rendered)
    }

    /// Render with a borrowed alias policy, then write only after every file succeeds.
    pub fn write_to_with_import_alias_resolver(
        &self,
        base_dir: &std::path::Path,
        width: usize,
        resolver: &dyn ImportAliasConflictResolver,
    ) -> Result<Vec<std::path::PathBuf>, SigilStitchError> {
        let rendered = self.render_with_import_alias_resolver(width, resolver)?;
        write_rendered_files(base_dir, &rendered)
    }
}

fn write_rendered_files(
    base_dir: &std::path::Path,
    rendered: &[RenderedFile],
) -> Result<Vec<std::path::PathBuf>, SigilStitchError> {
    let mut written = Vec::with_capacity(rendered.len());
    for file in rendered {
        let full_path = base_dir.join(&file.path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| SigilStitchError::Io {
                source,
                context: format!("creating directory for {}", file.path),
            })?;
        }
        std::fs::write(&full_path, &file.content).map_err(|source| SigilStitchError::Io {
            source,
            context: format!("writing file {}", file.path),
        })?;
        written.push(full_path);
    }
    Ok(written)
}

/// Builder for [`ProjectSpec`].
#[derive(Debug)]
pub struct ProjectSpecBuilder {
    files: Vec<FileSpec>,
}

impl ProjectSpecBuilder {
    /// Add a file to the project.
    pub fn add_file(mut self, file: FileSpec) -> Self {
        self.files.push(file);
        self
    }

    /// Build the [`ProjectSpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::DuplicateFileName`] if any two files
    /// share the same filename.
    pub fn build(self) -> Result<ProjectSpec, SigilStitchError> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for file in &self.files {
            *counts.entry(file.filename()).or_insert(0) += 1;
        }
        if let Some((&filename, count)) = counts.iter().find(|(_, c)| **c > 1) {
            return Err(SigilStitchError::DuplicateFileName {
                filename: filename.to_string(),
                count: *count,
            });
        }
        Ok(ProjectSpec { files: self.files })
    }
}
