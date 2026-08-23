//! Owner-wide validation across semantic member families.

pub(crate) mod php;

use crate::error::SigilStitchError;

pub(crate) fn validation_result(
    collect: impl FnOnce(&mut Vec<SigilStitchError>),
) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect(&mut errors);
    match errors.into_iter().next() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
