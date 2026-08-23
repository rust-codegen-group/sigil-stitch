//! Complete computed-property lowering.
//!
//! The compatibility module is the sole interpreter of the pre-0.6.8 shared
//! property grammar. Built-in adapters use language-local modules.

mod compatibility;

pub(crate) mod javascript;
pub(crate) mod kotlin;
pub(crate) mod php;
pub(crate) mod scala;
pub(crate) mod swift;
pub(crate) mod typescript;

pub(crate) use compatibility::lower as lower_compatibility;

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
