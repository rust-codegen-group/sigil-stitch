use proc_macro2::TokenStream;

/// Language identity for macro-level tokenizer decisions.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroLang {
    Unaware,
    Bash,
    C,
    Cpp,
    CSharp,
    Dart,
    Go,
    Haskell,
    Kotlin,
    OCaml,
    Php,
    Ruby,
    Swift,
    TypeScript,
    Zsh,
}

impl MacroLang {
    pub(crate) fn parse(tokens: TokenStream) -> syn::Result<Self> {
        let path = syn::parse2::<syn::Path>(tokens)?;
        let Some(segment) = path.segments.last() else {
            return Ok(Self::Unaware);
        };
        Ok(match segment.ident.to_string().as_str() {
            "Bash" => Self::Bash,
            "C" => Self::C,
            "CSharp" => Self::CSharp,
            "Cpp" => Self::Cpp,
            "Dart" => Self::Dart,
            "Go" => Self::Go,
            "Haskell" => Self::Haskell,
            "Kotlin" => Self::Kotlin,
            "OCaml" => Self::OCaml,
            "Php" => Self::Php,
            "Ruby" => Self::Ruby,
            "Swift" => Self::Swift,
            "TypeScript" => Self::TypeScript,
            "Zsh" => Self::Zsh,
            _ => Self::Unaware,
        })
    }

    pub(crate) fn is_shell(self) -> bool {
        matches!(self, Self::Bash | Self::Zsh)
    }

    pub(crate) fn default_colon_is_space_before(self) -> bool {
        matches!(self, Self::OCaml | Self::Bash | Self::Zsh)
    }

    pub(crate) fn has_angle_generics(self) -> bool {
        !matches!(
            self,
            Self::Ruby
                | Self::Bash
                | Self::C
                | Self::Zsh
                | Self::OCaml
                | Self::Php
                | Self::Go
                | Self::Haskell
        )
    }

    pub(crate) fn nullable_prefix_is_valid(self) -> bool {
        matches!(self, Self::Php | Self::OCaml)
    }

    pub(crate) fn has_postfix_star(self) -> bool {
        matches!(self, Self::C | Self::Cpp | Self::CSharp)
    }

    pub(crate) fn has_postfix_ampersand(self) -> bool {
        matches!(self, Self::Cpp)
    }

    pub(crate) fn has_postfix_question_type(self) -> bool {
        matches!(
            self,
            Self::CSharp | Self::Dart | Self::Kotlin | Self::Swift | Self::TypeScript
        )
    }
}
