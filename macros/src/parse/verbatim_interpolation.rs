/// Result of scanning a `$V` string literal for `@{expr}` interpolation patterns.
#[derive(Debug)]
pub(crate) enum VerbatimResult {
    /// No `@` characters found — use the original literal as-is.
    Unchanged,
    /// Only `@@` escapes found (no `@{}`). Contains the de-escaped string.
    Literal(String),
    /// One or more `@{expr}` interpolations found.
    Interpolated {
        format_string: String,
        expressions: Vec<String>,
    },
}

/// Scan a string for `@{expr}` patterns and `@@` escapes.
///
/// Returns `Unchanged` if no `@` present, `Literal` if only `@@` escapes,
/// or `Interpolated` with a format string and expression list.
pub(crate) fn parse_verbatim_interpolation(input: &str) -> Result<VerbatimResult, String> {
    let mut chars = input.chars().peekable();
    let mut format_string = String::with_capacity(input.len());
    let mut expressions: Vec<String> = Vec::new();
    let mut has_at = false;
    let mut has_interpolation = false;

    while let Some(ch) = chars.next() {
        if ch == '@' {
            match chars.peek() {
                Some(&'@') => {
                    has_at = true;
                    chars.next();
                    format_string.push('@');
                }
                Some(&'{') => {
                    has_at = true;
                    chars.next();
                    has_interpolation = true;

                    let mut expr = String::new();
                    let mut depth: u32 = 1;

                    loop {
                        match chars.next() {
                            None => {
                                return Err("unclosed `@{` — missing `}`".to_string());
                            }
                            Some('{') => {
                                depth += 1;
                                expr.push('{');
                            }
                            Some('}') => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                expr.push('}');
                            }
                            Some(c) => expr.push(c),
                        }
                    }

                    let trimmed = expr.trim();
                    if trimmed.is_empty() {
                        return Err("empty `@{}` — expression required".to_string());
                    }

                    format_string.push_str("{}");
                    expressions.push(trimmed.to_string());
                }
                _ => {
                    format_string.push('@');
                }
            }
        } else if ch == '{' {
            format_string.push_str("{{");
        } else if ch == '}' {
            format_string.push_str("}}");
        } else {
            format_string.push(ch);
        }
    }

    if !has_at {
        return Ok(VerbatimResult::Unchanged);
    }

    if !has_interpolation {
        let literal = format_string.replace("{{", "{").replace("}}", "}");
        return Ok(VerbatimResult::Literal(literal));
    }

    Ok(VerbatimResult::Interpolated {
        format_string,
        expressions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_at_returns_unchanged() {
        assert!(matches!(
            parse_verbatim_interpolation("hello world").unwrap(),
            VerbatimResult::Unchanged
        ));
    }

    #[test]
    fn bare_at_passes_through() {
        let result = parse_verbatim_interpolation("user@host").unwrap();
        assert!(matches!(result, VerbatimResult::Unchanged));
    }

    #[test]
    fn double_at_returns_literal() {
        let result = parse_verbatim_interpolation("user@@host").unwrap();
        match result {
            VerbatimResult::Literal(s) => assert_eq!(s, "user@host"),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn simple_interpolation() {
        let result = parse_verbatim_interpolation("Hello @{name}!").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "Hello {}!");
                assert_eq!(expressions, vec!["name"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }

    #[test]
    fn multiple_interpolations() {
        let result = parse_verbatim_interpolation("@{registry}/app:@{tag}").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "{}/app:{}");
                assert_eq!(expressions, vec!["registry", "tag"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }

    #[test]
    fn nested_braces_in_expr() {
        let result = parse_verbatim_interpolation("count=@{map[\"key\"]}").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "count={}");
                assert_eq!(expressions, vec!["map[\"key\"]"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }

    #[test]
    fn braces_in_content_escaped() {
        let result = parse_verbatim_interpolation("json: {\"key\": @{val}}").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "json: {{\"key\": {}}}");
                assert_eq!(expressions, vec!["val"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }

    #[test]
    fn mixed_escape_and_interpolation() {
        let result = parse_verbatim_interpolation("@@admin: @{msg}").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "@admin: {}");
                assert_eq!(expressions, vec!["msg"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }

    #[test]
    fn expr_with_method_call() {
        let result = parse_verbatim_interpolation("len=@{items.len()}").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "len={}");
                assert_eq!(expressions, vec!["items.len()"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }

    #[test]
    fn unclosed_at_brace_errors() {
        let result = parse_verbatim_interpolation("hello @{name");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unclosed"));
    }

    #[test]
    fn empty_at_brace_errors() {
        let result = parse_verbatim_interpolation("hello @{}");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn whitespace_only_at_brace_errors() {
        let result = parse_verbatim_interpolation("hello @{  }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn shell_vars_pass_through() {
        let result = parse_verbatim_interpolation("@{prefix}/$HOME/bin").unwrap();
        match result {
            VerbatimResult::Interpolated {
                format_string,
                expressions,
            } => {
                assert_eq!(format_string, "{}/$HOME/bin");
                assert_eq!(expressions, vec!["prefix"]);
            }
            _ => panic!("expected Interpolated"),
        }
    }
}
