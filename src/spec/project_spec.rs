//! Multi-file project specification.
//!
//! `ProjectSpec` orchestrates rendering multiple `FileSpec`s as a unit,
//! returning an in-memory collection of rendered files or writing them
//! to the filesystem.

use crate::lang::CodeLang;
use crate::spec::file_spec::FileSpec;

/// A rendered file: path and content.
#[derive(Debug, Clone)]
pub struct RenderedFile {
    /// The file path (as provided to `FileSpec::builder`).
    pub path: String,
    /// The rendered file content.
    pub content: String,
}

/// A multi-file project that renders all files as a unit.
#[derive(Debug, Clone)]
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
    pub fn render(&self, width: usize) -> Result<Vec<RenderedFile>, String> {
        let mut rendered = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let content = file
                .render(width)
                .map_err(|e| format!("{}: {e}", file.filename()))?;
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
    ) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
        let rendered = self
            .render(width)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut written = Vec::with_capacity(rendered.len());
        for file in &rendered {
            let full_path = base_dir.join(&file.path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, &file.content)?;
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
