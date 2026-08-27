#[cfg(test)]
macro_rules! assert_string_literal_rejection {
    ($language:literal, $reason:literal) => {
        #[test]
        fn rejects_string_singleton_types() {
            assert!(matches!(
                lower(&TypeName::string_literal("active")),
                Err(SigilStitchError::UnsupportedTypeName {
                    language,
                    context,
                    reason,
                }) if language == $language && context == "root" && reason == $reason
            ));
        }
    };
}

mod bash;
mod c;
mod cpp;
mod csharp;
mod dart;
mod go;
mod haskell;
mod java;
mod javascript;
mod kotlin;
mod lua;
mod ocaml;
mod php;
mod python;
mod ruby;
mod rust;
mod scala;
mod swift;
mod typescript;
mod zsh;

pub(crate) use bash::lower as bash;
pub(crate) use c::lower as c;
pub(crate) use cpp::lower as cpp;
pub(crate) use csharp::lower as csharp;
pub(crate) use dart::lower as dart;
pub(crate) use go::lower as go;
pub(crate) use haskell::lower as haskell;
pub(crate) use java::lower as java;
pub(crate) use javascript::lower as javascript;
pub(crate) use kotlin::lower as kotlin;
pub(crate) use lua::lower as lua;
pub(crate) use ocaml::lower as ocaml;
pub(crate) use php::lower as php;
pub(crate) use python::lower as python;
pub(crate) use ruby::lower as ruby;
pub(crate) use rust::lower as rust;
pub(crate) use scala::lower as scala;
pub(crate) use swift::lower as swift;
pub(crate) use typescript::lower as typescript;
pub(crate) use zsh::lower as zsh;
