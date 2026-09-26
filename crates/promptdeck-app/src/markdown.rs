//! 把 core 的 Markdown 块映射为 Slint 可渲染的块模型。
//!
//! 行内格式（粗体、斜体、行内代码、链接）交给 Slint `StyledText` 解析；Slint 不
//! 支持的行内语法（图片、原始 HTML 等）整体降级为纯文本，不崩溃、不丢内容。

use promptdeck_core::markdown::{self, Block, ListKind};
use slint::StyledText;

use crate::{MarkdownBlock, MarkdownBlockKind};

/// 解析 Markdown 源码并映射为 UI 块模型。
pub fn to_ui_blocks(source: &str) -> Vec<MarkdownBlock> {
    markdown::parse(source)
        .into_iter()
        .map(to_ui_block)
        .collect()
}

fn to_ui_block(block: Block) -> MarkdownBlock {
    match block {
        Block::Heading { level, text } => MarkdownBlock {
            kind: MarkdownBlockKind::Heading,
            level: i32::from(level),
            indent: 0,
            marker: Default::default(),
            text: styled_heading(&text),
        },
        Block::Paragraph { text } => MarkdownBlock {
            kind: MarkdownBlockKind::Paragraph,
            level: 0,
            indent: 0,
            marker: Default::default(),
            text: styled_inline(&text),
        },
        Block::ListItem {
            kind,
            number,
            indent,
            text,
        } => MarkdownBlock {
            kind: MarkdownBlockKind::ListItem,
            level: 0,
            indent: i32::from(indent),
            marker: match kind {
                ListKind::Ordered => format!("{number}.").into(),
                ListKind::Unordered => "•".into(),
            },
            text: styled_inline(&text),
        },
        Block::Quote { text } => MarkdownBlock {
            kind: MarkdownBlockKind::Quote,
            level: 0,
            indent: 0,
            marker: Default::default(),
            text: styled_inline(&text),
        },
        Block::Code { text } => MarkdownBlock {
            kind: MarkdownBlockKind::Code,
            level: 0,
            indent: 0,
            marker: Default::default(),
            text: StyledText::from_plain_text(&text),
        },
        Block::Rule => MarkdownBlock {
            kind: MarkdownBlockKind::Rule,
            level: 0,
            indent: 0,
            marker: Default::default(),
            text: StyledText::default(),
        },
    }
}

fn styled_inline(text: &str) -> StyledText {
    StyledText::from_markdown(text).unwrap_or_else(|_| StyledText::from_plain_text(text))
}

/// 标题没有独立字重属性：用 strong 包裹渲染为粗体；包裹失败则退回原文，最后退回纯文本。
fn styled_heading(text: &str) -> StyledText {
    StyledText::from_markdown(&format!("**{text}**"))
        .or_else(|_| StyledText::from_markdown(text))
        .unwrap_or_else(|_| StyledText::from_plain_text(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_has_no_blocks() {
        assert!(to_ui_blocks("").is_empty());
        assert!(to_ui_blocks("\n\n   \n").is_empty());
    }

    #[test]
    fn headings_are_bold_and_keep_their_level() {
        let blocks = to_ui_blocks("# Title\n\n### Third");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].kind, MarkdownBlockKind::Heading);
        assert_eq!(blocks[0].level, 1);
        assert_eq!(
            blocks[0].text,
            StyledText::from_markdown("**Title**").expect("bold heading parses")
        );
        assert_eq!(blocks[1].level, 3);
    }

    #[test]
    fn headings_with_unbalanced_markup_still_render() {
        let blocks = to_ui_blocks("# 1 * 2");
        assert_eq!(blocks.len(), 1);
        assert_ne!(blocks[0].text, StyledText::default());
    }

    #[test]
    fn unsupported_inline_syntax_falls_back_to_plain_text() {
        let blocks = to_ui_blocks("before <span>after</span>\n\n![img](a.png)");
        assert_eq!(
            blocks[0].text,
            StyledText::from_plain_text("before <span>after</span>")
        );
        assert_eq!(blocks[1].text, StyledText::from_plain_text("![img](a.png)"));
    }

    #[test]
    fn code_blocks_render_as_plain_text() {
        let blocks = to_ui_blocks("```\n**not bold**\n```");
        assert_eq!(blocks[0].kind, MarkdownBlockKind::Code);
        assert_eq!(blocks[0].text, StyledText::from_plain_text("**not bold**"));
    }

    #[test]
    fn list_items_carry_markers_and_indent() {
        let blocks = to_ui_blocks("- one\n  - nested\n\n1. first\n2. second");
        let kinds: Vec<_> = blocks.iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![
                MarkdownBlockKind::ListItem,
                MarkdownBlockKind::ListItem,
                MarkdownBlockKind::ListItem,
                MarkdownBlockKind::ListItem,
            ]
        );
        assert_eq!(blocks[0].marker.as_str(), "•");
        assert_eq!(blocks[0].indent, 0);
        assert_eq!(blocks[1].indent, 1);
        assert_eq!(blocks[2].marker.as_str(), "1.");
        assert_eq!(blocks[3].marker.as_str(), "2.");
    }

    #[test]
    fn long_documents_map_without_panicking() {
        let source = "段落文本 with `code` and **bold**.\n\n".repeat(2_000);
        let blocks = to_ui_blocks(&source);
        assert_eq!(blocks.len(), 2_000);
    }
}
