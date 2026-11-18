use proc_macro2::{Delimiter, Spacing, TokenTree};

use super::MacroLang;

/// Statement-level target role retained until body separators are decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StatementRole {
    Ordinary,
    Import,
}

/// Target-owned policy for statement preparation and statement boundaries.
///
/// The generic statement parser owns traversal and recovery. This policy owns
/// the target grammar facts that must be known before token formatting, such as
/// safe control-header parentheses and Python import separators.
#[derive(Clone, Copy)]
pub(super) struct StatementPolicy {
    lang: MacroLang,
}

impl StatementPolicy {
    pub(super) fn for_language(lang: MacroLang) -> Self {
        Self { lang }
    }

    pub(super) fn classify(&self, tokens: &[TokenTree]) -> StatementRole {
        if self.lang.is_python()
            && tokens
                .first()
                .and_then(|token| match token {
                    TokenTree::Ident(keyword) => Some(keyword.to_string()),
                    _ => None,
                })
                .is_some_and(|keyword| matches!(keyword.as_str(), "from" | "import"))
        {
            StatementRole::Import
        } else {
            StatementRole::Ordinary
        }
    }

    pub(super) fn needs_blank_line(
        &self,
        previous: Option<StatementRole>,
        current: StatementRole,
    ) -> bool {
        self.lang.is_python()
            && previous == Some(StatementRole::Import)
            && current != StatementRole::Import
    }

    pub(super) fn normalize_control_header(&self, tokens: &[TokenTree]) -> Vec<TokenTree> {
        if !self.lang.is_python() || tokens.len() != 2 {
            return tokens.to_vec();
        }
        let TokenTree::Ident(keyword) = &tokens[0] else {
            return tokens.to_vec();
        };
        if !matches!(keyword.to_string().as_str(), "if" | "while") {
            return tokens.to_vec();
        }
        let Some(inner) = strip_parenthesized_condition(&tokens[1..], self.lang) else {
            return tokens.to_vec();
        };
        let mut normalized = Vec::with_capacity(inner.len() + 1);
        normalized.push(tokens[0].clone());
        normalized.extend(inner);
        normalized
    }

    pub(super) fn normalize_branch_condition(
        &self,
        tokens: &[TokenTree],
        is_elif: bool,
    ) -> Vec<TokenTree> {
        if !is_elif {
            return tokens.to_vec();
        }
        strip_parenthesized_condition(tokens, self.lang).unwrap_or_else(|| tokens.to_vec())
    }
}

fn strip_parenthesized_condition(tokens: &[TokenTree], lang: MacroLang) -> Option<Vec<TokenTree>> {
    if !lang.is_python() || tokens.len() != 1 {
        return None;
    }
    let TokenTree::Group(group) = &tokens[0] else {
        return None;
    };
    if group.delimiter() != Delimiter::Parenthesis {
        return None;
    }
    let inner: Vec<TokenTree> = group.stream().into_iter().collect();
    if inner.is_empty()
        || inner.iter().any(|token| {
            matches!(token, TokenTree::Ident(value) if matches!(value.to_string().as_str(), "for" | "yield" | "lambda"))
                || matches!(token, TokenTree::Punct(p) if p.as_char() == ',')
                || matches!(token, TokenTree::Punct(p) if p.as_char() == ':' && p.spacing() == Spacing::Joint)
        })
    {
        return None;
    }
    Some(inner)
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    use super::*;

    fn tokens(source: &str) -> Vec<TokenTree> {
        TokenStream::from_str(source)
            .expect("test token stream parses")
            .into_iter()
            .collect()
    }

    fn text(tokens: &[TokenTree]) -> String {
        tokens.iter().cloned().collect::<TokenStream>().to_string()
    }

    #[test]
    fn generic_policy_does_not_normalize_control_headers_or_imports() {
        let policy = StatementPolicy::for_language(MacroLang::TypeScript);
        let input = tokens("if (value)");
        assert_eq!(text(&policy.normalize_control_header(&input)), text(&input));
        assert_eq!(
            policy.classify(&tokens("importer()")),
            StatementRole::Ordinary
        );
        assert!(!policy.needs_blank_line(None, StatementRole::Import));
    }

    #[test]
    fn python_policy_preserves_generator_parentheses() {
        let policy = StatementPolicy::for_language(MacroLang::Python);
        let input = tokens("if (item for item in values)");
        assert_eq!(text(&policy.normalize_control_header(&input)), text(&input));
    }

    #[test]
    fn python_policy_normalizes_safe_control_and_elif_parentheses() {
        let policy = StatementPolicy::for_language(MacroLang::Python);
        let input = tokens("if (enabled)");
        assert_eq!(
            text(&policy.normalize_control_header(&input)),
            text(&tokens("if enabled"))
        );
        assert_eq!(
            text(&policy.normalize_branch_condition(&tokens("(enabled)"), true)),
            text(&tokens("enabled"))
        );
    }

    #[test]
    fn python_policy_separates_imports_from_following_statements() {
        let policy = StatementPolicy::for_language(MacroLang::Python);
        assert_eq!(
            policy.classify(&tokens("from . import sibling")),
            StatementRole::Import
        );
        assert_eq!(
            policy.classify(&tokens("return sibling")),
            StatementRole::Ordinary
        );
        assert!(policy.needs_blank_line(Some(StatementRole::Import), StatementRole::Ordinary));
        assert!(!policy.needs_blank_line(Some(StatementRole::Import), StatementRole::Import));
    }
}
