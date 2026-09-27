//! 把 core 的 Markdown 块映射为 Slint 可渲染的块模型。
//!
//! 行内格式（粗体、斜体、行内代码、链接、删除线、下划线）由 core 解析为
//! 带样式的 atom；UI 逐 atom 渲染，使只读渲染面可以做命中测试与文本选择。

use promptdeck_core::markdown::{self, Atom, AtomKind, Block, InlineSpan, ListKind};
use slint::ModelRc;

use crate::{MarkdownAtom, MarkdownAtomKind, MarkdownBlock, MarkdownBlockKind};

/// 渲染结果：UI 块模型 + 每块的渲染纯文本与 atom 字符区间（用于复制与选择偏移）。
pub struct UiDocument {
    pub blocks: Vec<MarkdownBlock>,
    pub texts: Vec<String>,
    pub atom_ranges: Vec<Vec<(usize, usize)>>,
}

/// 解析 Markdown 源码并映射为 UI 文档模型。
pub fn render(source: &str) -> UiDocument {
    let mut blocks = Vec::new();
    let mut texts = Vec::new();
    let mut atom_ranges = Vec::new();
    for block in markdown::parse(source) {
        let ui = to_ui_block(block);
        texts.push(ui.text.clone());
        atom_ranges.push(ui.ranges);
        blocks.push(ui.block);
    }
    UiDocument {
        blocks,
        texts,
        atom_ranges,
    }
}

struct UiBlock {
    block: MarkdownBlock,
    text: String,
    ranges: Vec<(usize, usize)>,
}

fn to_ui_block(block: Block) -> UiBlock {
    match block {
        Block::Heading { level, text } => {
            let (atoms, ranges, rendered) = inline_atoms(&text);
            UiBlock {
                block: MarkdownBlock {
                    kind: MarkdownBlockKind::Heading,
                    level: i32::from(level),
                    indent: 0,
                    marker: Default::default(),
                    atoms,
                    char_length: rendered.chars().count() as i32,
                },
                text: rendered,
                ranges,
            }
        }
        Block::Paragraph { text } => {
            let (atoms, ranges, rendered) = inline_atoms(&text);
            UiBlock {
                block: MarkdownBlock {
                    kind: MarkdownBlockKind::Paragraph,
                    level: 0,
                    indent: 0,
                    marker: Default::default(),
                    atoms,
                    char_length: rendered.chars().count() as i32,
                },
                text: rendered,
                ranges,
            }
        }
        Block::ListItem {
            kind,
            number,
            indent,
            text,
        } => {
            let (atoms, ranges, rendered) = inline_atoms(&text);
            UiBlock {
                block: MarkdownBlock {
                    kind: MarkdownBlockKind::ListItem,
                    level: 0,
                    indent: i32::from(indent),
                    marker: match kind {
                        ListKind::Ordered => format!("{number}.").into(),
                        ListKind::Unordered => "•".into(),
                    },
                    atoms,
                    char_length: rendered.chars().count() as i32,
                },
                text: rendered,
                ranges,
            }
        }
        Block::Quote { text } => {
            let (atoms, ranges, rendered) = inline_atoms(&text);
            UiBlock {
                block: MarkdownBlock {
                    kind: MarkdownBlockKind::Quote,
                    level: 0,
                    indent: 0,
                    marker: Default::default(),
                    atoms,
                    char_length: rendered.chars().count() as i32,
                },
                text: rendered,
                ranges,
            }
        }
        Block::Code { text } => {
            let atoms = markdown::atomize_code(&text);
            let ranges = atoms.iter().map(|atom| (atom.start, atom.end)).collect();
            UiBlock {
                block: MarkdownBlock {
                    kind: MarkdownBlockKind::Code,
                    level: 0,
                    indent: 0,
                    marker: Default::default(),
                    atoms: to_ui_atoms(&atoms),
                    char_length: text.chars().count() as i32,
                },
                text,
                ranges,
            }
        }
        Block::Rule => UiBlock {
            block: MarkdownBlock {
                kind: MarkdownBlockKind::Rule,
                level: 0,
                indent: 0,
                marker: Default::default(),
                atoms: Default::default(),
                char_length: 0,
            },
            text: String::new(),
            ranges: Vec::new(),
        },
    }
}

fn inline_atoms(text: &str) -> (ModelRc<MarkdownAtom>, Vec<(usize, usize)>, String) {
    let spans: Vec<InlineSpan> = markdown::parse_inlines(text);
    let rendered: String = spans.iter().map(|span| span.text.as_str()).collect();
    let atoms = markdown::atomize_spans(&spans);
    let ranges = atoms.iter().map(|atom| (atom.start, atom.end)).collect();
    (to_ui_atoms(&atoms), ranges, rendered)
}

fn to_ui_atoms(atoms: &[Atom]) -> ModelRc<MarkdownAtom> {
    let ui: Vec<MarkdownAtom> = atoms.iter().map(to_ui_atom).collect();
    ModelRc::from(ui.as_slice())
}

fn to_ui_atom(atom: &Atom) -> MarkdownAtom {
    MarkdownAtom {
        text: atom.text.as_str().into(),
        kind: match atom.kind {
            AtomKind::Text => MarkdownAtomKind::Text,
            AtomKind::LineBreak => MarkdownAtomKind::LineBreak,
            AtomKind::BlankLine => MarkdownAtomKind::BlankLine,
        },
        strong: atom.style.strong,
        emphasis: atom.style.emphasis,
        code: atom.style.code,
        strike: atom.style.strike,
        link: atom.style.link,
        underline: atom.style.underline,
        start: atom.start as i32,
        end: atom.end as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    fn texts(doc: &UiDocument) -> Vec<&str> {
        doc.texts.iter().map(String::as_str).collect()
    }

    #[test]
    fn empty_source_has_no_blocks() {
        assert!(render("").blocks.is_empty());
        assert!(render("\n\n   \n").blocks.is_empty());
    }

    #[test]
    fn headings_keep_level_and_plain_text() {
        let doc = render("# Title\n\n### Third");
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].kind, MarkdownBlockKind::Heading);
        assert_eq!(doc.blocks[0].level, 1);
        assert_eq!(texts(&doc), vec!["Title", "Third"]);
        assert_eq!(doc.blocks[0].char_length, 5);
        assert!(doc.blocks[0].atoms.iter().all(|a| !a.strong));
    }

    #[test]
    fn inline_styles_map_to_atom_flags() {
        let doc = render("**粗** 和 *斜* 和 `码` 和 [链](u) 和 ~~删~~");
        let atoms: Vec<_> = doc.blocks[0].atoms.iter().collect();
        assert!(atoms.iter().any(|a| a.text.trim_end() == "粗" && a.strong));
        assert!(
            atoms
                .iter()
                .any(|a| a.text.trim_end() == "斜" && a.emphasis)
        );
        assert!(atoms.iter().any(|a| a.text.trim_end() == "码" && a.code));
        assert!(atoms.iter().any(|a| a.text.trim_end() == "链" && a.link));
        assert!(atoms.iter().any(|a| a.text.trim_end() == "删" && a.strike));
        assert_eq!(doc.texts[0], "粗 和 斜 和 码 和 链 和 删");
    }

    #[test]
    fn unsupported_inline_syntax_stays_visible() {
        let doc = render("before <span>after</span>\n\n![img](a.png)");
        assert_eq!(
            texts(&doc),
            vec!["before <span>after</span>", "![img](a.png)"]
        );
        assert!(doc.blocks[0].atoms.iter().all(|a| !a.code && !a.link));
    }

    #[test]
    fn code_blocks_are_atomized_per_char_and_keep_blank_lines() {
        let doc = render("```\nlet x = 1;\n\n**not bold**\n```");
        assert_eq!(doc.blocks[0].kind, MarkdownBlockKind::Code);
        assert_eq!(doc.texts[0], "let x = 1;\n\n**not bold**");
        assert_eq!(
            doc.blocks[0].char_length,
            doc.texts[0].chars().count() as i32
        );
        assert!(doc.blocks[0].atoms.iter().all(|a| a.code));
        assert!(
            doc.blocks[0]
                .atoms
                .iter()
                .any(|a| a.kind == MarkdownAtomKind::BlankLine)
        );
        assert!(
            doc.blocks[0]
                .atoms
                .iter()
                .any(|a| a.kind == MarkdownAtomKind::LineBreak)
        );
    }

    #[test]
    fn list_items_carry_markers_and_indent() {
        let doc = render("- one\n  - nested\n\n1. first\n2. second");
        let kinds: Vec<_> = doc.blocks.iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![
                MarkdownBlockKind::ListItem,
                MarkdownBlockKind::ListItem,
                MarkdownBlockKind::ListItem,
                MarkdownBlockKind::ListItem,
            ]
        );
        assert_eq!(doc.blocks[0].marker.as_str(), "•");
        assert_eq!(doc.blocks[0].indent, 0);
        assert_eq!(doc.blocks[1].indent, 1);
        assert_eq!(doc.blocks[2].marker.as_str(), "1.");
        assert_eq!(doc.blocks[3].marker.as_str(), "2.");
    }

    #[test]
    fn atom_offsets_cover_each_block_rendered_text() {
        let doc = render("# 标题\n\n你好 world `code`\n\n```\nab\n```");
        for (block, text) in doc.blocks.iter().zip(&doc.texts) {
            let len = text.chars().count() as i32;
            assert_eq!(block.char_length, len);
            for atom in block.atoms.iter() {
                assert!(atom.start <= atom.end);
                assert!(atom.end <= len);
            }
        }
    }

    #[test]
    fn long_documents_map_without_panicking() {
        let source = "段落文本 with `code` and **bold**.\n\n".repeat(2_000);
        let doc = render(&source);
        assert_eq!(doc.blocks.len(), 2_000);
        assert_eq!(doc.texts.len(), 2_000);
    }
}
