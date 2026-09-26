//! 轻量 Markdown 分块解析（无 Slint）。
//!
//! 只承担块级结构：标题、段落、列表项、引用、代码、分隔线。行内格式（粗体、
//! 斜体、行内代码、链接）留给渲染层解析——本模块的输出被逐块交给渲染器。
//! 目标是 CommonMark 的常用子集：无法识别的语法保持为普通段落文本，不丢弃内容。

/// 列表类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Unordered,
    Ordered,
}

/// 渲染无关的块级元素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    ListItem {
        kind: ListKind,
        number: u64,
        indent: u8,
        text: String,
    },
    Quote {
        text: String,
    },
    Code {
        text: String,
    },
    Rule,
}

/// 缩进层级上限，避免深层嵌套把正文挤出画布。
const MAX_INDENT_LEVEL: u8 = 3;

/// 把 Markdown 源码解析为块序列。
pub fn parse(source: &str) -> Vec<Block> {
    let lines: Vec<&str> = source.split('\n').map(strip_cr).collect();
    parse_lines(&lines)
}

fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

fn is_rule(line: &str) -> bool {
    let mut chars = line.chars().filter(|c| !c.is_whitespace());
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '*' | '-' | '_') {
        return false;
    }
    let mut count = 1;
    for c in chars {
        if c != first {
            return false;
        }
        count += 1;
    }
    count >= 3
}

fn setext_underline(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    let first = trimmed.chars().next()?;
    if first != '=' && first != '-' {
        return None;
    }
    if !trimmed.chars().all(|c| c == first) {
        return None;
    }
    Some(if first == '=' { 1 } else { 2 })
}

fn atx_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let mut text = rest.trim().to_string();
    strip_closing_hashes(&mut text);
    Some((hashes as u8, text.trim().to_string()))
}

fn strip_closing_hashes(text: &mut String) {
    let bytes = text.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1] == b'#' {
        end -= 1;
    }
    if end < bytes.len() && (end == 0 || matches!(bytes[end - 1], b' ' | b'\t')) {
        text.truncate(end);
    }
}

fn quote_marker(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

struct ListMarker<'a> {
    ordered: bool,
    lead: usize,
    number: u64,
    rest: &'a str,
}

fn list_marker(line: &str) -> Option<ListMarker<'_>> {
    let lead = leading_spaces(line);
    let trimmed = &line[lead..];
    for marker in ['-', '*', '+'] {
        if let Some(rest) = trimmed.strip_prefix(marker)
            && (rest.is_empty() || rest.starts_with([' ', '\t']))
        {
            return Some(ListMarker {
                ordered: false,
                lead,
                number: 0,
                rest: rest.trim_start(),
            });
        }
    }

    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 9 {
        return None;
    }
    let after_digits = &trimmed[digits.len()..];
    let rest = after_digits
        .strip_prefix('.')
        .or_else(|| after_digits.strip_prefix(')'))?;
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some(ListMarker {
        ordered: true,
        lead,
        number: digits.parse::<u64>().unwrap_or(1),
        rest: rest.trim_start(),
    })
}

struct Fence {
    ch: char,
    len: usize,
}

impl Fence {
    fn open(line: &str) -> Option<Fence> {
        let trimmed = line.trim_start();
        let ch = trimmed.chars().next()?;
        if ch != '`' && ch != '~' {
            return None;
        }
        let len = trimmed.chars().take_while(|c| *c == ch).count();
        if len < 3 {
            return None;
        }
        if ch == '`' && trimmed[len..].contains('`') {
            return None;
        }
        Some(Fence { ch, len })
    }

    fn closes(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        let run = trimmed.chars().take_while(|c| *c == self.ch).count();
        run >= self.len && trimmed[run..].trim().is_empty()
    }
}

fn is_paragraph_continuation(line: &str) -> bool {
    !(Fence::open(line).is_some()
        || atx_heading(line).is_some()
        || quote_marker(line).is_some()
        || list_marker(line).is_some()
        || is_rule(line.trim()))
}

fn starts_indented_code(lines: &[&str], i: usize) -> bool {
    leading_spaces(lines[i]) >= 4 && (i == 0 || lines[i - 1].trim().is_empty())
}

struct ListState {
    lead: usize,
    ordered: bool,
    next: u64,
}

fn parse_lines(lines: &[&str]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut list_stack: Vec<ListState> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            list_stack.clear();
            i += 1;
            continue;
        }

        if let Some(fence) = Fence::open(line) {
            list_stack.clear();
            let (text, next) = collect_fenced_code(lines, i + 1, &fence);
            blocks.push(Block::Code { text });
            i = next;
            continue;
        }

        if let Some((level, text)) = atx_heading(line) {
            list_stack.clear();
            blocks.push(Block::Heading { level, text });
            i += 1;
            continue;
        }

        if quote_marker(line).is_some() {
            list_stack.clear();
            let mut parts = Vec::new();
            while i < lines.len() {
                let Some(stripped) = quote_marker(lines[i]) else {
                    break;
                };
                let mut text = stripped;
                while let Some(inner) = quote_marker(text) {
                    text = inner;
                }
                parts.push(text.trim_end().to_string());
                i += 1;
            }
            blocks.push(Block::Quote {
                text: parts.join("\n"),
            });
            continue;
        }

        if is_rule(line.trim()) {
            list_stack.clear();
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }

        if starts_indented_code(lines, i) {
            list_stack.clear();
            let mut parts = Vec::new();
            while i < lines.len() {
                let current = lines[i];
                if current.trim().is_empty() {
                    parts.push(String::new());
                    i += 1;
                    continue;
                }
                if leading_spaces(current) < 4 {
                    break;
                }
                parts.push(current[4..].to_string());
                i += 1;
            }
            while parts.last().is_some_and(String::is_empty) {
                parts.pop();
            }
            blocks.push(Block::Code {
                text: parts.join("\n"),
            });
            continue;
        }

        if let Some(marker) = list_marker(line) {
            while let Some(top) = list_stack.last() {
                if marker.lead < top.lead {
                    list_stack.pop();
                } else {
                    break;
                }
            }
            if list_stack
                .last()
                .map(|top| marker.lead > top.lead)
                .unwrap_or(true)
            {
                list_stack.push(ListState {
                    lead: marker.lead,
                    ordered: marker.ordered,
                    next: marker.number,
                });
            }
            let state = list_stack.last_mut().expect("list state was just pushed");
            if state.ordered != marker.ordered {
                state.ordered = marker.ordered;
                state.next = marker.number;
            }
            let number = if marker.ordered {
                let number = state.next;
                state.next += 1;
                number
            } else {
                0
            };
            let indent = (list_stack.len() - 1).min(MAX_INDENT_LEVEL as usize) as u8;

            let mut text = marker.rest.trim().to_string();
            let mut j = i + 1;
            while j < lines.len() {
                let candidate = lines[j];
                if candidate.trim().is_empty()
                    || leading_spaces(candidate) <= marker.lead
                    || !is_paragraph_continuation(candidate)
                {
                    break;
                }
                text.push('\n');
                text.push_str(candidate.trim());
                j += 1;
            }

            blocks.push(Block::ListItem {
                kind: if marker.ordered {
                    ListKind::Ordered
                } else {
                    ListKind::Unordered
                },
                number,
                indent,
                text,
            });
            i = j;
            continue;
        }

        list_stack.clear();
        let mut parts = vec![line.trim().to_string()];
        i += 1;
        let mut setext = None;
        while i < lines.len() {
            let next = lines[i];
            if next.trim().is_empty() {
                break;
            }
            if let Some(level) = setext_underline(next) {
                setext = Some(level);
                i += 1;
                break;
            }
            if !is_paragraph_continuation(next) {
                break;
            }
            parts.push(next.trim().to_string());
            i += 1;
        }
        let text = parts.join("\n");
        match setext {
            Some(level) => blocks.push(Block::Heading { level, text }),
            None => blocks.push(Block::Paragraph { text }),
        }
    }

    blocks
}

fn collect_fenced_code(lines: &[&str], mut i: usize, fence: &Fence) -> (String, usize) {
    let start = i;
    while i < lines.len() {
        if fence.closes(lines[i]) {
            return (lines[start..i].join("\n"), i + 1);
        }
        i += 1;
    }
    (lines[start..].join("\n"), lines.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(text: &str) -> Block {
        Block::Paragraph { text: text.into() }
    }

    fn heading(level: u8, text: &str) -> Block {
        Block::Heading {
            level,
            text: text.into(),
        }
    }

    fn item(kind: ListKind, number: u64, indent: u8, text: &str) -> Block {
        Block::ListItem {
            kind,
            number,
            indent,
            text: text.into(),
        }
    }

    fn quote(text: &str) -> Block {
        Block::Quote { text: text.into() }
    }

    fn code(text: &str) -> Block {
        Block::Code { text: text.into() }
    }

    #[test]
    fn blank_source_has_no_blocks() {
        assert_eq!(parse(""), vec![]);
        assert_eq!(parse("   \n\n\t\n"), vec![]);
    }

    #[test]
    fn plain_lines_form_one_paragraph_preserving_line_breaks() {
        assert_eq!(parse("hello world"), vec![paragraph("hello world")]);
        assert_eq!(
            parse("line one\nline two"),
            vec![paragraph("line one\nline two")]
        );
    }

    #[test]
    fn blank_lines_separate_paragraphs() {
        assert_eq!(
            parse("first\n\nsecond"),
            vec![paragraph("first"), paragraph("second")]
        );
    }

    #[test]
    fn carriage_returns_are_stripped() {
        assert_eq!(
            parse("first\r\n\r\nsecond\r\n"),
            vec![paragraph("first"), paragraph("second")]
        );
    }

    #[test]
    fn atx_headings_cover_all_levels() {
        assert_eq!(
            parse("# a\n## b\n### c\n#### d\n##### e\n###### f"),
            vec![
                heading(1, "a"),
                heading(2, "b"),
                heading(3, "c"),
                heading(4, "d"),
                heading(5, "e"),
                heading(6, "f"),
            ]
        );
    }

    #[test]
    fn atx_heading_needs_space_or_end_after_hashes() {
        assert_eq!(parse("#hashtag"), vec![paragraph("#hashtag")]);
        assert_eq!(parse("####### seven"), vec![paragraph("####### seven")]);
        assert_eq!(parse("##"), vec![heading(2, "")]);
    }

    #[test]
    fn atx_heading_strips_closing_hashes_only_after_space() {
        assert_eq!(parse("## title ##"), vec![heading(2, "title")]);
        assert_eq!(parse("## title##"), vec![heading(2, "title##")]);
        assert_eq!(parse("### c# minor ###"), vec![heading(3, "c# minor")]);
    }

    #[test]
    fn atx_heading_keeps_inline_markup() {
        assert_eq!(
            parse("## **bold** and `code`"),
            vec![heading(2, "**bold** and `code`")]
        );
    }

    #[test]
    fn setext_headings_use_underline() {
        assert_eq!(parse("Title\n==="), vec![heading(1, "Title")]);
        assert_eq!(parse("Subtitle\n---"), vec![heading(2, "Subtitle")]);
    }

    #[test]
    fn rules_are_recognized_when_not_setext() {
        assert_eq!(parse("***"), vec![Block::Rule]);
        assert_eq!(parse("---"), vec![Block::Rule]);
        assert_eq!(parse("___"), vec![Block::Rule]);
        assert_eq!(parse("- - -"), vec![Block::Rule]);
        assert_eq!(parse("***\n\n---"), vec![Block::Rule, Block::Rule]);
    }

    #[test]
    fn fenced_code_keeps_raw_text_and_ignores_info_string() {
        assert_eq!(
            parse("```rust\nfn main() {}\n```"),
            vec![code("fn main() {}")]
        );
        assert_eq!(parse("~~~\na\nb\n~~~"), vec![code("a\nb")]);
    }

    #[test]
    fn unclosed_fence_consumes_the_rest() {
        assert_eq!(parse("```\nlet x = 1;"), vec![code("let x = 1;")]);
    }

    #[test]
    fn fence_with_longer_closing_sequence_closes() {
        assert_eq!(parse("```\nx\n`````"), vec![code("x")]);
        // 不同围栏字符不能闭合
        assert_eq!(parse("```\nx\n~~~"), vec![code("x\n~~~")]);
    }

    #[test]
    fn indented_code_block_is_kept() {
        assert_eq!(
            parse("    let x = 1;\n    let y = 2;"),
            vec![code("let x = 1;\nlet y = 2;")]
        );
        assert_eq!(
            parse("para\n\n    code"),
            vec![paragraph("para"), code("code")]
        );
    }

    #[test]
    fn blockquote_joins_lines_and_strips_markers() {
        assert_eq!(parse("> one\n> two"), vec![quote("one\ntwo")]);
        assert_eq!(parse(">no space"), vec![quote("no space")]);
        assert_eq!(
            parse("> quote\n\npara"),
            vec![quote("quote"), paragraph("para")]
        );
    }

    #[test]
    fn nested_quote_markers_flatten() {
        assert_eq!(parse("> > deep"), vec![quote("deep")]);
    }

    #[test]
    fn unordered_list_items_track_nesting() {
        assert_eq!(
            parse("- a\n  - b\n- c"),
            vec![
                item(ListKind::Unordered, 0, 0, "a"),
                item(ListKind::Unordered, 0, 1, "b"),
                item(ListKind::Unordered, 0, 0, "c"),
            ]
        );
        assert_eq!(
            parse("* a\n+ b"),
            vec![
                item(ListKind::Unordered, 0, 0, "a"),
                item(ListKind::Unordered, 0, 0, "b"),
            ]
        );
    }

    #[test]
    fn ordered_list_items_number_sequentially_from_the_first_number() {
        assert_eq!(
            parse("1. a\n2. b\n3. c"),
            vec![
                item(ListKind::Ordered, 1, 0, "a"),
                item(ListKind::Ordered, 2, 0, "b"),
                item(ListKind::Ordered, 3, 0, "c"),
            ]
        );
        assert_eq!(
            parse("3. a\n7. b"),
            vec![
                item(ListKind::Ordered, 3, 0, "a"),
                item(ListKind::Ordered, 4, 0, "b"),
            ]
        );
    }

    #[test]
    fn list_markers_allow_a_tab_after_the_marker() {
        assert_eq!(
            parse("-\titem\n1.\tfirst"),
            vec![
                item(ListKind::Unordered, 0, 0, "item"),
                item(ListKind::Ordered, 1, 0, "first"),
            ]
        );
    }

    #[test]
    fn deeply_indented_list_markers_at_top_level_are_code() {
        assert_eq!(parse("    1. not a list"), vec![code("1. not a list")]);
        assert_eq!(parse("    - not a list"), vec![code("- not a list")]);
    }

    #[test]
    fn list_item_continuation_lines_are_appended() {
        assert_eq!(
            parse("- first\n  continued\n- second"),
            vec![
                item(ListKind::Unordered, 0, 0, "first\ncontinued"),
                item(ListKind::Unordered, 0, 0, "second"),
            ]
        );
    }

    #[test]
    fn list_ends_at_a_paragraph() {
        assert_eq!(
            parse("- a\n\nplain"),
            vec![item(ListKind::Unordered, 0, 0, "a"), paragraph("plain")]
        );
    }

    #[test]
    fn heading_style_lines_inside_quotes_stay_literal_quote_text() {
        assert_eq!(parse("> # title"), vec![quote("# title")]);
    }

    #[test]
    fn unsupported_pipe_tables_degrade_to_paragraph_text() {
        let blocks = parse("| a | b |\n| --- | --- |\n| 1 | 2 |");
        assert_eq!(
            blocks,
            vec![paragraph("| a | b |\n| --- | --- |\n| 1 | 2 |")]
        );
    }

    #[test]
    fn raw_html_lines_stay_paragraph_text() {
        assert_eq!(
            parse("<div>hello</div>"),
            vec![paragraph("<div>hello</div>")]
        );
    }

    #[test]
    fn a_realistic_prompt_parses_into_expected_blocks() {
        let source = "# 角色\n你是一名翻译。\n\n## 要求\n- 保持语气\n- 输出 Markdown\n\n> 注意：不要解释。\n\n```md\nhello\n```\n\n---\n结束。\n";
        assert_eq!(
            parse(source),
            vec![
                heading(1, "角色"),
                paragraph("你是一名翻译。"),
                heading(2, "要求"),
                item(ListKind::Unordered, 0, 0, "保持语气"),
                item(ListKind::Unordered, 0, 0, "输出 Markdown"),
                quote("注意：不要解释。"),
                code("hello"),
                Block::Rule,
                paragraph("结束。"),
            ]
        );
    }
}
