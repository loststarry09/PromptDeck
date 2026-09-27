use std::collections::{HashMap, HashSet};

use crate::model::VariableDef;

enum Segment<'a> {
    Text(&'a str),
    LiteralBrace,
    Variable {
        name: &'a str,
        default: Option<&'a str>,
    },
}

/// Parses variable definitions from a Prompt body, in first-appearance order.
///
/// `{{name}}` and `{{name:默认值}}` both define a variable; a repeated name
/// merges into one definition and keeps the first default seen. `\{{` is an
/// escaped literal brace and never defines a variable.
pub fn parse(body_md: &str) -> Vec<VariableDef> {
    let segments = scan(body_md);
    definitions(&segments)
        .into_iter()
        .map(|(name, default)| VariableDef {
            name: name.to_string(),
            default_value: default.map(str::to_string),
        })
        .collect()
}

/// Resolves a Prompt body for copying: variables with a default are replaced by
/// that default (first occurrence wins), variables without a default keep their
/// literal `{{name}}`, and `\{{` becomes a literal `{{`.
pub fn resolve(body_md: &str) -> String {
    let segments = scan(body_md);
    let defaults: HashMap<&str, Option<&str>> = definitions(&segments).into_iter().collect();

    let mut resolved = String::with_capacity(body_md.len());
    for segment in &segments {
        match segment {
            Segment::Text(text) => resolved.push_str(text),
            Segment::LiteralBrace => resolved.push_str("{{"),
            Segment::Variable { name, .. } => match defaults.get(*name).copied().flatten() {
                Some(value) => resolved.push_str(value),
                None => {
                    resolved.push_str("{{");
                    resolved.push_str(name);
                    resolved.push_str("}}");
                }
            },
        }
    }
    resolved
}

/// The merged definitions for a scanned body, in first-appearance order: a
/// repeated name keeps the default from its first occurrence. `parse` and
/// `resolve` share this so the ADR-0008 merge rule lives in one place.
fn definitions<'a>(segments: &[Segment<'a>]) -> Vec<(&'a str, Option<&'a str>)> {
    let mut definitions = Vec::new();
    let mut seen = HashSet::new();
    for segment in segments {
        if let Segment::Variable { name, default } = segment
            && seen.insert(*name)
        {
            definitions.push((*name, *default));
        }
    }
    definitions
}

fn scan(body_md: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut cursor = 0;
    let mut text_start = 0;
    let len = body_md.len();

    while cursor < len {
        let rest = &body_md[cursor..];

        if rest.starts_with("\\{{") {
            push_text(&mut segments, body_md, text_start, cursor);
            segments.push(Segment::LiteralBrace);
            cursor += 3;
            text_start = cursor;
            continue;
        }

        if rest.starts_with("{{")
            && let Some((name, default, length)) = parse_variable(rest)
        {
            push_text(&mut segments, body_md, text_start, cursor);
            segments.push(Segment::Variable { name, default });
            cursor += length;
            text_start = cursor;
            continue;
        }

        let ch = rest.chars().next().expect("cursor is within the string");
        cursor += ch.len_utf8();
    }

    push_text(&mut segments, body_md, text_start, len);
    segments
}

fn push_text<'a>(segments: &mut Vec<Segment<'a>>, body_md: &'a str, start: usize, end: usize) {
    if start < end {
        segments.push(Segment::Text(&body_md[start..end]));
    }
}

/// Parses a variable starting at the `{{` prefix of `rest`, returning the name,
/// optional default, and total length consumed (including both braces).
fn parse_variable(rest: &str) -> Option<(&str, Option<&str>, usize)> {
    let inner_start = 2;
    let closing = rest[inner_start..].find("}}")?;
    let inner = &rest[inner_start..inner_start + closing];

    let (name, default) = match inner.find(':') {
        Some(colon) => (&inner[..colon], Some(&inner[colon + 1..])),
        None => (inner, None),
    };
    if !valid_name(name) {
        return None;
    }

    Some((name, default, inner_start + closing + 2))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '{' | '}' | ':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, default_value: Option<&str>) -> VariableDef {
        VariableDef {
            name: name.to_string(),
            default_value: default_value.map(str::to_string),
        }
    }

    #[test]
    fn parse_returns_nothing_without_a_valid_variable() {
        for body in [
            "",
            "普通文本，没有变量",
            "孤立的 { 与 } 花括号",
            "{{ name }}",
            "{{}}",
            "{{name",
            "name}}",
        ] {
            assert!(parse(body).is_empty(), "body {body:?}");
        }
    }

    #[test]
    fn parse_reads_single_variable_with_and_without_default() {
        assert_eq!(parse("你好 {{name}}"), vec![var("name", None)]);
        assert_eq!(parse("{{name:世界}}"), vec![var("name", Some("世界"))]);
        assert_eq!(parse("{{name:}}"), vec![var("name", Some(""))]);
    }

    #[test]
    fn parse_merges_same_name_keeping_first_default() {
        assert_eq!(parse("{{x:1}} {{x:2}}"), vec![var("x", Some("1"))]);
        assert_eq!(parse("{{x}} {{x:2}}"), vec![var("x", None)]);
        assert_eq!(parse("{{x:1}} {{x}}"), vec![var("x", Some("1"))]);
        assert_eq!(
            parse("{{a:1}} {{b:2}} {{a:3}}"),
            vec![var("a", Some("1")), var("b", Some("2"))]
        );
    }

    #[test]
    fn parse_ignores_escaped_braces() {
        assert!(parse(r"\{{name}}").is_empty());
        assert!(parse(r"\{{name:默认}}").is_empty());
        assert_eq!(parse(r"\{{a}} {{b}}"), vec![var("b", None)]);
    }

    #[test]
    fn parse_supports_multiline_defaults_and_unicode_names() {
        assert_eq!(
            parse("{{变量:第一行\n第二行}}"),
            vec![var("变量", Some("第一行\n第二行"))]
        );
    }

    #[test]
    fn resolve_leaves_text_without_variables_unchanged() {
        for body in ["", "普通文本", "a { b } c"] {
            assert_eq!(resolve(body), body);
        }
    }

    #[test]
    fn resolve_fills_defaults_and_keeps_defaultless_variables() {
        assert_eq!(resolve("你好 {{name}}"), "你好 {{name}}");
        assert_eq!(resolve("你好 {{name:世界}}"), "你好 世界");
        assert_eq!(resolve("{{a:1}} 与 {{b}}"), "1 与 {{b}}");
        assert_eq!(resolve("{{a:}}"), "");
    }

    #[test]
    fn resolve_uses_first_default_for_repeated_names() {
        assert_eq!(resolve("{{x:1}} {{x:2}}"), "1 1");
        assert_eq!(resolve("{{x}} {{x:2}}"), "{{x}} {{x}}");
    }

    #[test]
    fn resolve_unescapes_literal_braces() {
        assert_eq!(resolve(r"\{{name}}"), "{{name}}");
        assert_eq!(resolve(r"\{{a:1}} tail"), "{{a:1}} tail");
        assert_eq!(resolve(r"keep \ backslash"), r"keep \ backslash");
    }

    #[test]
    fn resolve_handles_multiline_defaults() {
        assert_eq!(resolve("{{v:第一行\n第二行}}!"), "第一行\n第二行!");
    }
}
