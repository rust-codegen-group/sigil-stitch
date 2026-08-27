use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::CodeLang;

pub(crate) fn reject_aliases(
    lang: &dyn CodeLang,
    imports: &ImportGroup,
) -> Result<(), SigilStitchError> {
    if imports.entries().iter().any(|entry| entry.alias.is_some()) {
        return Err(SigilStitchError::InvalidResolvedImports {
            language: lang.file_extension().to_string(),
            reason: "the current target import form cannot express a local alias".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn validate_identifier_aliases(
    lang: &dyn CodeLang,
    imports: &ImportGroup,
    is_valid: impl Fn(&str) -> bool,
) -> Result<(), SigilStitchError> {
    for alias in imports
        .entries()
        .iter()
        .filter_map(|entry| entry.alias.as_deref())
    {
        if !is_valid(alias) {
            return Err(SigilStitchError::InvalidResolvedImports {
                language: lang.file_extension().to_string(),
                reason: format!("import alias {alias:?} is not a valid target identifier"),
            });
        }
    }
    Ok(())
}
