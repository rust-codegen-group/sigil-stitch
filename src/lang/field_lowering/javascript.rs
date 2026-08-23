//! JavaScript-owned class-field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::javascript::JavaScript;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

use super::{
    collect_escaped_name_collisions, collect_invalid_identifiers, emit_annotations, emit_doc,
};

const CAPABILITIES: &[FieldCapability] = &[
    FieldCapability::Initializer,
    FieldCapability::Attributes,
    FieldCapability::StaticField,
];

pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::InterfaceMember),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Interface), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Trait), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Enum), CAPABILITIES),
];

fn is_identifier_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || unicode_id_start::is_id_start(ch))
        && chars.all(|ch| {
            ch == '$'
                || ch == '\u{200c}'
                || ch == '\u{200d}'
                || unicode_id_start::is_id_continue(ch)
        })
}

fn decode_string_literal(name: &str) -> Option<String> {
    let chars: Vec<_> = name.chars().collect();
    let quote = *chars.first()?;
    if chars.len() < 2 || !matches!(quote, '\'' | '"') || chars.last() != Some(&quote) {
        return None;
    }

    let mut decoded = String::new();
    let mut index = 1;
    while index + 1 < chars.len() {
        let ch = chars[index];
        index += 1;
        if ch == quote || matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            return None;
        }
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }

        if index + 1 >= chars.len() {
            return None;
        }
        let escaped = *chars.get(index)?;
        index += 1;
        match escaped {
            '\n' | '\u{2028}' | '\u{2029}' => {}
            '\r' => {
                if chars.get(index) == Some(&'\n') {
                    index += 1;
                }
            }
            '\'' | '"' | '\\' => decoded.push(escaped),
            'b' => decoded.push('\u{0008}'),
            'f' => decoded.push('\u{000c}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'v' => decoded.push('\u{000b}'),
            '0' if !chars.get(index).is_some_and(char::is_ascii_digit) => decoded.push('\0'),
            'x' => {
                let digits: String = chars.get(index..index + 2)?.iter().collect();
                let value = u32::from_str_radix(&digits, 16).ok()?;
                decoded.push(char::from_u32(value)?);
                index += 2;
            }
            'u' => {
                let value = if chars.get(index) == Some(&'{') {
                    index += 1;
                    let start = index;
                    while chars.get(index).is_some_and(|ch| ch.is_ascii_hexdigit()) {
                        index += 1;
                    }
                    if index == start || index - start > 6 || chars.get(index) != Some(&'}') {
                        return None;
                    }
                    let digits: String = chars[start..index].iter().collect();
                    index += 1;
                    u32::from_str_radix(&digits, 16).ok()?
                } else {
                    let digits: String = chars.get(index..index + 4)?.iter().collect();
                    index += 4;
                    u32::from_str_radix(&digits, 16).ok()?
                };
                decoded.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            ch if ch.is_ascii_digit() => return None,
            ch => decoded.push(ch),
        }
    }
    Some(decoded)
}

fn digits_with_separators(value: &str, is_digit: fn(char) -> bool) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|ch| ch == '_' || is_digit(ch))
}

fn is_numeric_literal(name: &str) -> bool {
    let (value, bigint) = name
        .strip_suffix('n')
        .map_or((name, false), |value| (value, true));
    if let Some(digits) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return digits_with_separators(digits, |ch| ch.is_ascii_hexdigit());
    }
    if let Some(digits) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        return digits_with_separators(digits, |ch| matches!(ch, '0'..='7'));
    }
    if let Some(digits) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        return digits_with_separators(digits, |ch| matches!(ch, '0' | '1'));
    }
    if bigint {
        let valid = digits_with_separators(value, |ch| ch.is_ascii_digit());
        let digits = value.replace('_', "");
        return valid && (digits == "0" || !digits.starts_with('0'));
    }
    if value.starts_with(['+', '-'])
        || !value.chars().any(|ch| ch.is_ascii_digit())
        || value
            .chars()
            .any(|ch| !ch.is_ascii_digit() && !matches!(ch, '_' | '.' | 'e' | 'E' | '+' | '-'))
        || value
            .char_indices()
            .filter(|(_, ch)| *ch == '_')
            .any(|(index, _)| {
                !value[..index]
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ch.is_ascii_digit())
                    || !value[index + 1..]
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_digit())
            })
    {
        return false;
    }
    let normalized = value.replace('_', "");
    if normalized.len() > 1 && normalized.starts_with('0') && !normalized.contains(['.', 'e', 'E'])
    {
        return false;
    }
    normalized.parse::<f64>().is_ok()
}

fn property_key(name: &str) -> Option<String> {
    if let Some(private) = name.strip_prefix('#') {
        return is_identifier_name(private).then(|| private.to_string());
    }
    if is_identifier_name(name) {
        Some(name.to_string())
    } else if let Some(value) = decode_string_literal(name) {
        Some(value)
    } else {
        is_numeric_literal(name).then(|| name.to_string())
    }
}

fn is_valid_property_name(name: &str) -> bool {
    property_key(name).is_some()
}

pub(crate) fn validate(
    lang: &JavaScript,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &JavaScript,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_property_name, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        let private_name = field.name().starts_with('#');
        let visibility_is_valid = match field.modifiers().visibility {
            Visibility::Inherited => true,
            Visibility::Public => !private_name,
            Visibility::Private => private_name,
            Visibility::Protected | Visibility::PublicCrate | Visibility::PublicSuper => false,
        };
        if !visibility_is_valid {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "JavaScript class-field visibility is expressed in the field name, such as #private"
                    .to_string(),
            });
        }
        let key = property_key(field.name());
        if key.as_deref() == Some("constructor") {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "JavaScript class fields cannot be named constructor".to_string(),
            });
        }
        if field.modifiers().is_static && !private_name && key.as_deref() == Some("prototype") {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "JavaScript static class fields cannot be named prototype".to_string(),
            });
        }
        if field.tag().is_some() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "the legacy tag escape hatch is only valid for Go struct fields"
                    .to_string(),
            });
        }
    }
}

pub(crate) fn lower(
    lang: &JavaScript,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "@", "")?;
        if field.modifiers().is_static {
            block.add("static ", ());
        }
        block.add("%L", lang.escape_field_name(field.name()));
        if let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add(";", ());
        block.add_line();
    }
    block.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_property_names_decode_every_supported_escape_form() {
        for (name, expected) in [
            (r#""""#, ""),
            ("'plain'", "plain"),
            (r#""\"\\""#, "\"\\"),
            (r#""\b\f\n\r\t\v\0""#, "\u{0008}\u{000c}\n\r\t\u{000b}\0"),
            (r#""\x41\u0042\u{43}""#, "ABC"),
            ("\"a\\\nb\"", "ab"),
            ("\"a\\\r\nb\"", "ab"),
            ("\"a\\\u{2028}b\"", "ab"),
            ("\"a\\\u{2029}b\"", "ab"),
            (r#""\u{110000}\uD800""#, "\u{fffd}\u{fffd}"),
        ] {
            assert_eq!(
                decode_string_literal(name).as_deref(),
                Some(expected),
                "{name:?}"
            );
        }
    }

    #[test]
    fn malformed_string_property_names_fail_closed() {
        for name in [
            "",
            "\"",
            "\"unterminated",
            "'mismatch\"",
            "\"line\nfeed\"",
            "\"line\rfeed\"",
            "\"line\u{2028}feed\"",
            "\"line\u{2029}feed\"",
            r#""\""#,
            r#""\1""#,
            r#""\01""#,
            r#""\x""#,
            r#""\xGG""#,
            r#""\u123""#,
            r#""\u{}""#,
            r#""\u{1234567}""#,
            r#""\u{1234""#,
        ] {
            assert_eq!(decode_string_literal(name), None, "{name:?}");
        }
    }

    #[test]
    fn numeric_property_names_cover_radices_separators_bigints_and_exponents() {
        for name in [
            "0", "123", "1_000", "0xFF", "0Xf_f", "0o70", "0O7_0", "0b10", "0B1_0", "0n", "12n",
            "1_000n", ".5", "1.", "1.5", "1e2", "1E+2", "1e-2",
        ] {
            assert!(is_numeric_literal(name), "{name:?}");
            assert_eq!(property_key(name).as_deref(), Some(name), "{name:?}");
        }

        for name in [
            "0x", "0x_FF", "0xFF_", "0xF__F", "0xGG", "0o8", "0b2", "01n", "01", "+1", "-1", ".",
            "1a", "1_", "1__0", "1_e2", "1e_2", "1..0", "1e",
        ] {
            assert!(!is_numeric_literal(name), "{name:?}");
            assert_eq!(property_key(name), None, "{name:?}");
        }
        assert!(!is_numeric_literal("_1"));
        assert_eq!(property_key("_1").as_deref(), Some("_1"));
    }

    #[test]
    fn property_keys_distinguish_identifiers_private_names_and_literals() {
        for (name, expected) in [
            ("field", "field"),
            ("$field", "$field"),
            ("a\u{200d}b", "a\u{200d}b"),
            ("#private", "private"),
            (r#""field-name""#, "field-name"),
            (r#""\x66ield""#, "field"),
        ] {
            assert_eq!(property_key(name).as_deref(), Some(expected), "{name:?}");
        }
        for name in ["#", "#bad name", "bad name"] {
            assert_eq!(property_key(name), None, "{name:?}");
        }
    }
}
