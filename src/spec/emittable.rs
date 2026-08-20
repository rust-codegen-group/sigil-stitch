use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;

/// A spec that can emit itself as top-level file members.
///
/// Implement this trait to create custom spec types that can be added to a
/// [`FileSpec`](super::file_spec::FileSpec) via
/// [`add_spec`](super::file_spec::FileSpecBuilder::add_spec), without needing a
/// new [`FileMember`](super::file_spec::FileMember) variant.
///
/// Built-in implementations: [`TypeSpec`](super::type_spec::TypeSpec),
/// [`FunSpec`](super::fun_spec::FunSpec).
pub trait Emittable: std::fmt::Debug {
    /// Append any language-specific validation failures for this spec.
    ///
    /// The default preserves compatibility for custom specs that predate
    /// capability validation. Built-in specs override this so using
    /// [`add_spec`](super::file_spec::FileSpecBuilder::add_spec) cannot bypass
    /// aggregate file validation.
    fn collect_validation_errors(&self, _lang: &dyn CodeLang, _errors: &mut Vec<SigilStitchError>) {
    }

    /// Emit this spec as one or more code blocks for a given language.
    fn emit_members(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError>;
}
