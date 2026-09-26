//! Search query planning for the Library.
//!
//! `items_fts` is tokenized with `trigram`, which matches case-insensitive
//! substrings of three or more characters. Shorter queries produce no
//! trigrams at all, so they fall back to a `LIKE` scan with escaped
//! wildcards. The planner turns a user query into the expression each path
//! binds, keeping query syntax out of the repository.

const MIN_TRIGRAM_CHARS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryPlan {
    All,
    Trigram(String),
    Like(String),
}

pub fn plan(query: &str) -> QueryPlan {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return QueryPlan::All;
    }
    if trimmed.chars().count() < MIN_TRIGRAM_CHARS {
        QueryPlan::Like(like_pattern(trimmed))
    } else {
        QueryPlan::Trigram(fts_phrase(trimmed))
    }
}

/// FTS5 treats a double-quoted string as a literal phrase; embedded quotes
/// are escaped by doubling them.
fn fts_phrase(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

/// `LIKE` pattern matching the query anywhere in a column, with `\`, `%` and
/// `_` escaped for use with `ESCAPE '\'`.
fn like_pattern(query: &str) -> String {
    let mut escaped = String::with_capacity(query.len());
    for character in query.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_queries_list_everything() {
        assert_eq!(plan(""), QueryPlan::All);
        assert_eq!(plan(" \t\n "), QueryPlan::All);
    }

    #[test]
    fn three_or_more_characters_use_a_trigram_phrase() {
        assert_eq!(plan("翻译成"), QueryPlan::Trigram("\"翻译成\"".into()));
        assert_eq!(plan("  hello  "), QueryPlan::Trigram("\"hello\"".into()));
        assert_eq!(
            plan("SUBSTRING"),
            QueryPlan::Trigram("\"SUBSTRING\"".into())
        );
    }

    #[test]
    fn short_queries_fall_back_to_like() {
        assert_eq!(plan("翻译"), QueryPlan::Like("%翻译%".into()));
        assert_eq!(plan("AI"), QueryPlan::Like("%AI%".into()));
        assert_eq!(plan(" a "), QueryPlan::Like("%a%".into()));
    }

    #[test]
    fn like_patterns_escape_wildcards_and_backslashes() {
        assert_eq!(plan("%"), QueryPlan::Like("%\\%%".into()));
        assert_eq!(plan("_"), QueryPlan::Like("%\\_%".into()));
        assert_eq!(plan("\\"), QueryPlan::Like("%\\\\%".into()));
        assert_eq!(plan("a%"), QueryPlan::Like("%a\\%%".into()));
    }

    #[test]
    fn fts_phrases_double_embedded_quotes() {
        assert_eq!(fts_phrase("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(
            plan("say \"hi\""),
            QueryPlan::Trigram("\"say \"\"hi\"\"\"".into())
        );
    }
}
