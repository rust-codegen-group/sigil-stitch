//! Multi-file project specification.
//!
//! `ProjectSpec` orchestrates rendering multiple `FileSpec`s as a unit,
//! returning an in-memory collection of rendered files or writing them
//! to the filesystem.

use crate::error::SigilStitchError;
use crate::lang::CodeLang;
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
/// Each file resolves imports independently.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let mut models_b = FileSpec::<TypeScript>::builder("src/models.ts");
/// models_b.add_type(TypeSpec::builder("User", TypeKind::Interface).build().unwrap());
/// let models = models_b.build().unwrap();
///
/// let mut index_b = FileSpec::<TypeScript>::builder("src/index.ts");
/// index_b.add_code(CodeBlock::of("export {}", ()).unwrap());
/// let index = index_b.build().unwrap();
///
/// let mut pb = ProjectSpec::<TypeScript>::builder();
/// pb.add_file(models);
/// pb.add_file(index);
/// let project = pb.build();
///
/// let rendered = project.render(80).unwrap();
/// assert_eq!(rendered.len(), 2);
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(bound(serialize = "", deserialize = "L: Default"))]
pub struct ProjectSpec<L: CodeLang> {
    pub(crate) files: Vec<FileSpec<L>>,
}

impl<L: CodeLang> ProjectSpec<L> {
    /// Create a new builder for a project specification.
    pub fn builder() -> ProjectSpecBuilder<L> {
        ProjectSpecBuilder { files: Vec::new() }
    }

    /// Render all files and return their paths and contents.
    ///
    /// Each file resolves imports independently. File ordering is preserved.
    /// Fails on the first render error, including the filename in the message.
    pub fn render(&self, width: usize) -> Result<Vec<RenderedFile>, SigilStitchError> {
        let mut rendered = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let content = file.render(width)?;
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

        let mut written = Vec::with_capacity(rendered.len());
        for file in &rendered {
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
}

/// Builder for [`ProjectSpec`].
#[derive(Debug)]
pub struct ProjectSpecBuilder<L: CodeLang> {
    files: Vec<FileSpec<L>>,
}

impl<L: CodeLang> ProjectSpecBuilder<L> {
    /// Add a file to the project.
    pub fn add_file(&mut self, file: FileSpec<L>) -> &mut Self {
        self.files.push(file);
        self
    }

    /// Build the [`ProjectSpec`].
    pub fn build(self) -> ProjectSpec<L> {
        ProjectSpec { files: self.files }
    }
}
