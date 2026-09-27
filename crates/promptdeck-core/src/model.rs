use std::str::FromStr;

use crate::error::Error;

pub const TITLE_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Prompt,
    Block,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemKind::Prompt => "prompt",
            ItemKind::Block => "block",
        }
    }
}

impl FromStr for ItemKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "prompt" => Ok(ItemKind::Prompt),
            "block" => Ok(ItemKind::Block),
            other => Err(Error::UnknownItemKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    pub title: String,
    pub body_md: String,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSummary {
    pub id: String,
    pub kind: ItemKind,
    pub title: String,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub title: String,
    pub body_md: String,
    pub created_at: i64,
}

/// A derived Variable definition parsed from a Prompt body: the name plus the
/// default from its first occurrence (`None` when declared without a default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDef {
    pub name: String,
    pub default_value: Option<String>,
}

pub fn derive_title(body_md: &str) -> String {
    for line in body_md.lines() {
        let candidate = strip_heading(line.trim()).trim();
        if candidate.is_empty() {
            continue;
        }
        return truncate_chars(candidate, TITLE_MAX_CHARS);
    }
    String::new()
}

fn strip_heading(line: &str) -> &str {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    let rest = line.get(hashes..).unwrap_or("");
    let trimmed = rest.trim_start();
    if hashes == 0 {
        return line;
    }
    strip_closing_hashes(trimmed)
}

fn strip_closing_hashes(text: &str) -> &str {
    let without = text.trim_end_matches('#');
    if without.len() < text.len() && without.ends_with(char::is_whitespace) {
        without.trim_end()
    } else {
        text
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn content_hash(title: &str, body_md: &str) -> String {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in title
        .as_bytes()
        .iter()
        .chain(&[0u8])
        .chain(body_md.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_kind_roundtrips_through_storage_strings() {
        assert_eq!(ItemKind::Prompt.as_str(), "prompt");
        assert_eq!(ItemKind::Block.as_str(), "block");
        assert_eq!(
            "prompt".parse::<ItemKind>().expect("prompt"),
            ItemKind::Prompt
        );
        assert_eq!("block".parse::<ItemKind>().expect("block"), ItemKind::Block);
        assert!("mystery".parse::<ItemKind>().is_err());
    }

    #[test]
    fn derive_title_prefers_the_first_heading() {
        assert_eq!(derive_title("# 你好世界"), "你好世界");
        assert_eq!(derive_title("\n\n## 二级标题\n正文"), "二级标题");
        assert_eq!(derive_title("#标题"), "标题");
        assert_eq!(derive_title("## 标题 ##\n正文"), "标题");
    }

    #[test]
    fn derive_title_falls_back_to_first_non_empty_line() {
        assert_eq!(
            derive_title("\n\n请把这段话翻译成英文\n更多"),
            "请把这段话翻译成英文"
        );
        assert_eq!(derive_title("- 列表项"), "- 列表项");
        assert_eq!(derive_title("> 引用"), "> 引用");
    }

    #[test]
    fn derive_title_ignores_empty_input_and_blank_heading_lines() {
        assert_eq!(derive_title(""), "");
        assert_eq!(derive_title("\n \n\t"), "");
        assert_eq!(derive_title("###\n\n正文"), "正文");
    }

    #[test]
    fn derive_title_truncates_long_lines_with_an_ellipsis() {
        let line = "字".repeat(TITLE_MAX_CHARS);
        assert_eq!(derive_title(&line), line);

        let longer = "字".repeat(TITLE_MAX_CHARS + 1);
        let title = derive_title(&longer);
        assert_eq!(title.chars().count(), TITLE_MAX_CHARS + 1);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn content_hash_matches_independent_fnv1a_vectors() {
        assert_eq!(content_hash("", ""), "af63bd4c8601b7df");
        assert_eq!(content_hash("a", ""), "089be207b544f1e4");
        assert_eq!(content_hash("标题", "正文"), "0ffae9951c38ead4");
    }

    #[test]
    fn content_hash_separates_title_from_body() {
        assert_ne!(content_hash("ab", ""), content_hash("a", "b"));
        assert_ne!(content_hash("", "ab"), content_hash("a", "b"));
    }
}
