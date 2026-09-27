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

/// 行内样式标志，可组合。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
    pub strike: bool,
    pub link: bool,
    pub underline: bool,
}

/// 行内片段：渲染文本（不含 Markdown 标记）与样式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    pub text: String,
    pub style: InlineStyle,
}

/// atom 类型：文本、换行（`\n`）、空行（代码块中连续的换行之间的空行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomKind {
    Text,
    LineBreak,
    BlankLine,
}

/// 可渲染 / 可选择的最小单元。
///
/// `start`/`end` 是块内渲染文本的字符偏移。文本 atom 的 `text` 可能包含不
/// 计入 `[start, end)` 的尾随空白（空白跟随前一个 atom 参与换行，避免行首出现
/// 隐形空格）；空行 atom 的 `start == end`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub text: String,
    pub style: InlineStyle,
    pub kind: AtomKind,
    pub start: usize,
    pub end: usize,
}

/// 单个 atom 的最大字符数（超长 token 允许在中间换行，避免溢出画布）。
pub const MAX_ATOM_CHARS: usize = 16;

/// 宽字符（CJK、假名、谚文、全角等）：逐字成 atom，可任意换行。
pub fn is_wide_char(c: char) -> bool {
    matches!(c,
        '\u{1100}'..='\u{11FF}'
            | '\u{2E80}'..='\u{2FFF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{3100}'..='\u{312F}'
            | '\u{3130}'..='\u{318F}'
            | '\u{31C0}'..='\u{31EF}'
            | '\u{3200}'..='\u{32FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FE30}'..='\u{FE4F}'
            | '\u{FF00}'..='\u{FFEF}'
            | '\u{20000}'..='\u{2FA1F}'
    )
}

/// 解析行内 Markdown 为渲染片段。
///
/// 支持的子集：`**strong**`/`__strong__`、`*em*`/`_em_`、`` `code` ``、
/// `~~strike~~`、`[text](url)`、`<u>underline</u>`、`\` 转义。未闭合或无法
/// 识别时保留字面文本；图片语法与未知 HTML 标签按字面保留。
pub fn parse_inlines(text: &str) -> Vec<InlineSpan> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    parse_span_range(&chars, 0, chars.len(), InlineStyle::default(), &mut spans);
    merge_spans(spans)
}

fn merge_spans(spans: Vec<InlineSpan>) -> Vec<InlineSpan> {
    let mut merged: Vec<InlineSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut()
            && last.style == span.style
        {
            last.text.push_str(&span.text);
            continue;
        }
        merged.push(span);
    }
    merged
}

fn push_span(out: &mut Vec<InlineSpan>, text: String, style: InlineStyle) {
    if !text.is_empty() {
        out.push(InlineSpan { text, style });
    }
}

fn parse_span_range(
    chars: &[char],
    start: usize,
    end: usize,
    style: InlineStyle,
    out: &mut Vec<InlineSpan>,
) {
    let mut literal = String::new();
    let mut i = start;
    while i < end {
        let c = chars[i];

        if c == '\\' && i + 1 < end && chars[i + 1].is_ascii_punctuation() {
            literal.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if c == '`' {
            let run = run_length(chars, i, end, '`');
            if let Some(close) = find_exact_run(chars, i + run, end, '`', run) {
                let content: String = chars[i + run..close].iter().collect();
                push_span(out, literal.clone(), style);
                literal.clear();
                push_span(
                    out,
                    trim_code_span(content),
                    InlineStyle {
                        code: true,
                        ..style
                    },
                );
                i = close + run;
                continue;
            }
            for _ in 0..run {
                literal.push('`');
            }
            i += run;
            continue;
        }

        if c == '<' {
            if let Some(inner_end) = tag_end(chars, i, end, "u")
                && let Some(close) = find_tag(chars, inner_end, end, "u")
            {
                push_span(out, literal.clone(), style);
                literal.clear();
                parse_span_range(
                    chars,
                    inner_end,
                    close,
                    InlineStyle {
                        underline: true,
                        ..style
                    },
                    out,
                );
                i = close + 4;
                continue;
            }
            if let Some(skip) = font_tag_length(chars, i, end) {
                i += skip;
                continue;
            }
            literal.push(c);
            i += 1;
            continue;
        }

        if c == '!' && i + 1 < end && chars[i + 1] == '[' {
            if let Some((_text_end, url_end)) = link_bounds(chars, i + 1, end) {
                for ch in &chars[i..url_end] {
                    literal.push(*ch);
                }
                i = url_end;
                continue;
            }
            literal.push('!');
            i += 1;
            continue;
        }

        if c == '[' {
            if let Some((text_end, url_end)) = link_bounds(chars, i, end) {
                push_span(out, literal.clone(), style);
                literal.clear();
                parse_span_range(
                    chars,
                    i + 1,
                    text_end,
                    InlineStyle {
                        link: true,
                        ..style
                    },
                    out,
                );
                i = url_end;
                continue;
            }
            literal.push('[');
            i += 1;
            continue;
        }

        if c == '*' || c == '_' || c == '~' {
            let run = run_length(chars, i, end, c);
            if let Some((open_len, mut inner_style, close_len)) = delimiter(c, run, chars, i, start)
                && let Some(close) = find_delimiter_close(chars, i + open_len, end, c, open_len)
            {
                inner_style.strong |= style.strong;
                inner_style.emphasis |= style.emphasis;
                inner_style.strike |= style.strike;
                inner_style.code |= style.code;
                inner_style.link |= style.link;
                inner_style.underline |= style.underline;
                push_span(out, literal.clone(), style);
                literal.clear();
                parse_span_range(chars, i + open_len, close, inner_style, out);
                i = close + close_len;
                continue;
            }
            for _ in 0..run {
                literal.push(c);
            }
            i += run;
            continue;
        }

        literal.push(c);
        i += 1;
    }
    push_span(out, literal, style);
}

fn trim_code_span(content: String) -> String {
    let all_spaces = !content.is_empty() && content.chars().all(|c| c == ' ');
    if content.len() >= 2 && content.starts_with(' ') && content.ends_with(' ') && !all_spaces {
        return content[1..content.len() - 1].to_string();
    }
    content
}

fn run_length(chars: &[char], from: usize, end: usize, ch: char) -> usize {
    chars[from..end].iter().take_while(|c| **c == ch).count()
}

fn find_exact_run(chars: &[char], from: usize, end: usize, ch: char, len: usize) -> Option<usize> {
    let mut i = from;
    while i < end {
        if chars[i] == ch {
            let run = run_length(chars, i, end, ch);
            if run == len {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

fn tag_end(chars: &[char], from: usize, end: usize, name: &str) -> Option<usize> {
    let expected: Vec<char> = format!("<{name}>").chars().collect();
    if from + expected.len() > end || chars[from..from + expected.len()] != expected[..] {
        return None;
    }
    Some(from + expected.len())
}

fn find_tag(chars: &[char], from: usize, end: usize, name: &str) -> Option<usize> {
    let expected: Vec<char> = format!("</{name}>").chars().collect();
    let mut i = from;
    while i + expected.len() <= end {
        if chars[i..i + expected.len()] == expected[..] {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn font_tag_length(chars: &[char], from: usize, end: usize) -> Option<usize> {
    let rest = &chars[from..end];
    let closing = rest.starts_with(&['<', '/', 'f', 'o', 'n', 't']);
    let opening = rest.starts_with(&['<', 'f', 'o', 'n', 't']);
    if !closing && !opening {
        return None;
    }
    if opening && !closing && rest.get(5).is_some_and(|c| !c.is_whitespace() && *c != '>') {
        return None;
    }
    rest.iter().position(|c| *c == '>').map(|p| p + 1)
}

fn link_bounds(chars: &[char], open: usize, end: usize) -> Option<(usize, usize)> {
    if chars.get(open) != Some(&'[') {
        return None;
    }
    let text_end = (open + 1..end).find(|i| chars[*i] == ']')?;
    if chars.get(text_end + 1) != Some(&'(') {
        return None;
    }
    let url_end = (text_end + 2..end).find(|i| chars[*i] == ')')?;
    Some((text_end, url_end + 1))
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || is_wide_char(c)
}

fn delimiter(
    c: char,
    run: usize,
    chars: &[char],
    i: usize,
    start: usize,
) -> Option<(usize, InlineStyle, usize)> {
    match c {
        '*' => match run {
            1 => Some((
                1,
                InlineStyle {
                    emphasis: true,
                    ..Default::default()
                },
                1,
            )),
            _ => {
                let len = run.min(3);
                let style = match len {
                    2 => InlineStyle {
                        strong: true,
                        ..Default::default()
                    },
                    _ => InlineStyle {
                        strong: true,
                        emphasis: true,
                        ..Default::default()
                    },
                };
                Some((len, style, len))
            }
        },
        '_' => {
            if i > start
                && chars
                    .get(i.wrapping_sub(1))
                    .is_some_and(|p| is_word_char(*p))
            {
                return None;
            }
            match run {
                1 => Some((
                    1,
                    InlineStyle {
                        emphasis: true,
                        ..Default::default()
                    },
                    1,
                )),
                _ => {
                    let len = run.min(3);
                    let style = match len {
                        2 => InlineStyle {
                            strong: true,
                            ..Default::default()
                        },
                        _ => InlineStyle {
                            strong: true,
                            emphasis: true,
                            ..Default::default()
                        },
                    };
                    Some((len, style, len))
                }
            }
        }
        '~' if run >= 2 => Some((
            2,
            InlineStyle {
                strike: true,
                ..Default::default()
            },
            2,
        )),
        _ => None,
    }
}

fn find_delimiter_close(
    chars: &[char],
    from: usize,
    end: usize,
    c: char,
    open_len: usize,
) -> Option<usize> {
    let mut i = from;
    while i < end {
        if chars[i] != c {
            i += 1;
            continue;
        }
        let run = run_length(chars, i, end, c);
        let matches = match c {
            '_' => {
                run >= open_len
                    && chars.get(i + run).is_none_or(|next| !is_word_char(*next))
                    && (open_len == 1 || run == open_len)
            }
            _ => run == open_len || (open_len >= 2 && run >= open_len),
        };
        if matches {
            return Some(i);
        }
        i += run;
    }
    None
}

/// 把行内片段拆成 atom（块内字符偏移连续）。
pub fn atomize_spans(spans: &[InlineSpan]) -> Vec<Atom> {
    let mut atoms = Vec::new();
    let mut pos = 0usize;
    for span in spans {
        atomize_span(span, &mut pos, &mut atoms);
    }
    atoms
}

fn atomize_span(span: &InlineSpan, pos: &mut usize, atoms: &mut Vec<Atom>) {
    let chars: Vec<char> = span.text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            atoms.push(Atom {
                text: "\n".to_string(),
                style: span.style,
                kind: AtomKind::LineBreak,
                start: *pos + i,
                end: *pos + i + 1,
            });
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            let ws_start = i;
            while i < chars.len() && chars[i].is_whitespace() && chars[i] != '\n' {
                i += 1;
            }
            let ws: String = chars[ws_start..i].iter().collect();
            match atoms.last_mut() {
                Some(last) if last.kind == AtomKind::Text => last.text.push_str(&ws),
                _ => atoms.push(Atom {
                    text: ws,
                    style: span.style,
                    kind: AtomKind::Text,
                    start: *pos + ws_start,
                    end: *pos + i,
                }),
            }
            continue;
        }
        if span.style.code || is_wide_char(c) {
            atoms.push(Atom {
                text: c.to_string(),
                style: span.style,
                kind: AtomKind::Text,
                start: *pos + i,
                end: *pos + i + 1,
            });
            i += 1;
            continue;
        }
        let run_start = i;
        while i < chars.len()
            && !chars[i].is_whitespace()
            && chars[i] != '\n'
            && !is_wide_char(chars[i])
            && !span.style.code
        {
            i += 1;
        }
        let run_len = i - run_start;
        let mut k = 0;
        while k < run_len {
            let take = (run_len - k).min(MAX_ATOM_CHARS);
            let text: String = chars[run_start + k..run_start + k + take].iter().collect();
            atoms.push(Atom {
                text,
                style: span.style,
                kind: AtomKind::Text,
                start: *pos + run_start + k,
                end: *pos + run_start + k + take,
            });
            k += take;
        }
    }
    *pos += chars.len();
}

/// 代码块 atom 化：逐字（等宽字体无需保字距），保留换行与空行。
pub fn atomize_code(text: &str) -> Vec<Atom> {
    let style = InlineStyle {
        code: true,
        ..Default::default()
    };
    let chars: Vec<char> = text.chars().collect();
    let mut atoms = Vec::new();
    let mut line_start = 0usize;
    for i in 0..=chars.len() {
        if i == chars.len() {
            if i > line_start {
                for (k, c) in chars.iter().enumerate().take(i).skip(line_start) {
                    atoms.push(code_char_atom(*c, style, k));
                }
            }
            break;
        }
        if chars[i] == '\n' {
            if i == line_start {
                atoms.push(Atom {
                    text: String::new(),
                    style,
                    kind: AtomKind::BlankLine,
                    start: i,
                    end: i,
                });
            } else {
                for (k, c) in chars.iter().enumerate().take(i).skip(line_start) {
                    atoms.push(code_char_atom(*c, style, k));
                }
            }
            atoms.push(Atom {
                text: "\n".to_string(),
                style,
                kind: AtomKind::LineBreak,
                start: i,
                end: i + 1,
            });
            line_start = i + 1;
        }
    }
    atoms
}

fn code_char_atom(c: char, style: InlineStyle, index: usize) -> Atom {
    Atom {
        text: c.to_string(),
        style,
        kind: AtomKind::Text,
        start: index,
        end: index + 1,
    }
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

#[cfg(test)]
mod inline_tests {
    use super::*;

    fn style_strong() -> InlineStyle {
        InlineStyle {
            strong: true,
            ..Default::default()
        }
    }

    fn style_emphasis() -> InlineStyle {
        InlineStyle {
            emphasis: true,
            ..Default::default()
        }
    }

    fn style_code() -> InlineStyle {
        InlineStyle {
            code: true,
            ..Default::default()
        }
    }

    fn span(text: &str, style: InlineStyle) -> InlineSpan {
        InlineSpan {
            text: text.into(),
            style,
        }
    }

    fn text_atom(text: &str, start: usize, end: usize) -> Atom {
        Atom {
            text: text.into(),
            style: Default::default(),
            kind: AtomKind::Text,
            start,
            end,
        }
    }

    #[test]
    fn plain_text_is_one_span() {
        assert_eq!(
            parse_inlines("hello world"),
            vec![span("hello world", Default::default())]
        );
    }

    #[test]
    fn empty_text_has_no_spans() {
        assert_eq!(parse_inlines(""), vec![]);
    }

    #[test]
    fn strong_and_emphasis_are_parsed() {
        assert_eq!(
            parse_inlines("**bold**"),
            vec![span("bold", style_strong())]
        );
        assert_eq!(
            parse_inlines("__bold__"),
            vec![span("bold", style_strong())]
        );
        assert_eq!(parse_inlines("*em*"), vec![span("em", style_emphasis())]);
        assert_eq!(parse_inlines("_em_"), vec![span("em", style_emphasis())]);
        assert_eq!(
            parse_inlines("***both***"),
            vec![span(
                "both",
                InlineStyle {
                    strong: true,
                    emphasis: true,
                    ..Default::default()
                }
            )]
        );
    }

    #[test]
    fn nested_emphasis_works() {
        assert_eq!(
            parse_inlines("*a **b** c*"),
            vec![
                span("a ", style_emphasis()),
                span(
                    "b",
                    InlineStyle {
                        emphasis: true,
                        strong: true,
                        ..Default::default()
                    }
                ),
                span(" c", style_emphasis()),
            ]
        );
        assert_eq!(
            parse_inlines("**a *b* c**"),
            vec![
                span("a ", style_strong()),
                span(
                    "b",
                    InlineStyle {
                        strong: true,
                        emphasis: true,
                        ..Default::default()
                    }
                ),
                span(" c", style_strong()),
            ]
        );
    }

    #[test]
    fn intraword_underscores_stay_literal() {
        assert_eq!(
            parse_inlines("foo_bar_baz"),
            vec![span("foo_bar_baz", Default::default())]
        );
    }

    #[test]
    fn unmatched_delimiters_stay_literal() {
        assert_eq!(
            parse_inlines("a * b"),
            vec![span("a * b", Default::default())]
        );
        assert_eq!(
            parse_inlines("**open"),
            vec![span("**open", Default::default())]
        );
        assert_eq!(
            parse_inlines("~single~"),
            vec![span("~single~", Default::default())]
        );
    }

    #[test]
    fn strikethrough_is_parsed() {
        assert_eq!(
            parse_inlines("~~gone~~"),
            vec![span(
                "gone",
                InlineStyle {
                    strike: true,
                    ..Default::default()
                }
            )]
        );
    }

    #[test]
    fn code_spans_are_literal() {
        assert_eq!(parse_inlines("`**x**`"), vec![span("**x**", style_code())]);
        assert_eq!(parse_inlines("``a`b``"), vec![span("a`b", style_code())]);
        assert_eq!(
            parse_inlines("`unclosed"),
            vec![span("`unclosed", Default::default())]
        );
    }

    #[test]
    fn links_are_parsed_and_images_stay_literal() {
        assert_eq!(
            parse_inlines("[docs](https://example.com)"),
            vec![span(
                "docs",
                InlineStyle {
                    link: true,
                    ..Default::default()
                }
            )]
        );
        assert_eq!(
            parse_inlines("![alt](a.png)"),
            vec![span("![alt](a.png)", Default::default())]
        );
        assert_eq!(
            parse_inlines("[not a link]"),
            vec![span("[not a link]", Default::default())]
        );
    }

    #[test]
    fn escapes_remove_the_backslash() {
        assert_eq!(
            parse_inlines("\\*not em\\*"),
            vec![span("*not em*", Default::default())]
        );
        assert_eq!(
            parse_inlines("a\\\\b"),
            vec![span("a\\b", Default::default())]
        );
    }

    #[test]
    fn underline_tags_are_parsed_and_font_tags_stripped() {
        assert_eq!(
            parse_inlines("<u>under</u>"),
            vec![span(
                "under",
                InlineStyle {
                    underline: true,
                    ..Default::default()
                }
            )]
        );
        assert_eq!(
            parse_inlines("<font color=\"red\">x</font>"),
            vec![span("x", Default::default())]
        );
        assert_eq!(
            parse_inlines("<span>x</span>"),
            vec![span("<span>x</span>", Default::default())]
        );
    }

    #[test]
    fn mixed_inline_markup_round_trips_offsets() {
        let spans = parse_inlines("前 **粗** 后");
        assert_eq!(
            spans,
            vec![
                span("前 ", Default::default()),
                span("粗", style_strong()),
                span(" 后", Default::default()),
            ]
        );
        let atoms = atomize_spans(&spans);
        assert_eq!(atoms[0], text_atom("前 ", 0, 1));
        assert_eq!(
            atoms[1],
            Atom {
                text: "粗 ".into(),
                style: style_strong(),
                kind: AtomKind::Text,
                start: 2,
                end: 3
            }
        );
        assert_eq!(atoms[2], text_atom("后", 4, 5));
    }

    #[test]
    fn latin_words_keep_kerning_friendly_atoms() {
        assert_eq!(
            atomize_spans(&parse_inlines("hello world")),
            vec![text_atom("hello ", 0, 5), text_atom("world", 6, 11)]
        );
    }

    #[test]
    fn wide_chars_are_one_atom_each() {
        assert_eq!(
            atomize_spans(&parse_inlines("你好世界")),
            vec![
                text_atom("你", 0, 1),
                text_atom("好", 1, 2),
                text_atom("世", 2, 3),
                text_atom("界", 3, 4),
            ]
        );
    }

    #[test]
    fn mixed_wide_and_latin_splits_at_boundaries() {
        let atoms = atomize_spans(&parse_inlines("使用 Ctrl+M 切换"));
        let texts: Vec<_> = atoms.iter().map(|a| a.text.as_str()).collect();
        assert_eq!(texts, vec!["使", "用 ", "Ctrl+M ", "切", "换"]);
        assert_eq!(atoms[2].start, 3);
        assert_eq!(atoms[2].end, 9);
        assert_eq!(atoms[3].start, 10);
        assert_eq!(atoms[3].end, 11);
        assert_eq!(atoms[4].start, 11);
        assert_eq!(atoms[4].end, 12);
    }

    #[test]
    fn long_latin_runs_are_chunked_for_wrapping() {
        let atoms = atomize_spans(&parse_inlines("abcdefghijklmnopqrs"));
        assert_eq!(atoms.len(), 2);
        assert_eq!(atoms[0].text, "abcdefghijklmnop");
        assert_eq!(atoms[0].start, 0);
        assert_eq!(atoms[0].end, 16);
        assert_eq!(atoms[1].text, "qrs");
        assert_eq!(atoms[1].start, 16);
        assert_eq!(atoms[1].end, 19);
    }

    #[test]
    fn newlines_become_break_atoms() {
        let atoms = atomize_spans(&parse_inlines("a\nb"));
        assert_eq!(
            atoms,
            vec![
                text_atom("a", 0, 1),
                Atom {
                    text: "\n".into(),
                    style: Default::default(),
                    kind: AtomKind::LineBreak,
                    start: 1,
                    end: 2
                },
                text_atom("b", 2, 3),
            ]
        );
    }

    #[test]
    fn inline_code_is_atomized_per_char() {
        let atoms = atomize_spans(&parse_inlines("a `bc` d"));
        let texts: Vec<_> = atoms.iter().map(|a| a.text.as_str()).collect();
        assert_eq!(texts, vec!["a ", "b", "c ", "d"]);
        assert_eq!(atoms[1].start, 2);
        assert_eq!(atoms[2].start, 3);
    }

    #[test]
    fn code_blocks_keep_blank_lines() {
        assert_eq!(
            atomize_code("a\n\nb"),
            vec![
                Atom {
                    text: "a".into(),
                    style: style_code(),
                    kind: AtomKind::Text,
                    start: 0,
                    end: 1
                },
                Atom {
                    text: "\n".into(),
                    style: style_code(),
                    kind: AtomKind::LineBreak,
                    start: 1,
                    end: 2
                },
                Atom {
                    text: String::new(),
                    style: style_code(),
                    kind: AtomKind::BlankLine,
                    start: 2,
                    end: 2
                },
                Atom {
                    text: "\n".into(),
                    style: style_code(),
                    kind: AtomKind::LineBreak,
                    start: 2,
                    end: 3
                },
                Atom {
                    text: "b".into(),
                    style: style_code(),
                    kind: AtomKind::Text,
                    start: 3,
                    end: 4
                },
            ]
        );
    }

    #[test]
    fn code_blocks_do_not_add_trailing_blank_lines() {
        let atoms = atomize_code("a\n");
        assert_eq!(atoms.len(), 2);
        assert_eq!(atoms[1].kind, AtomKind::LineBreak);
        assert!(atomize_code("").is_empty());
    }

    #[test]
    fn code_blocks_keep_indentation_as_separate_atoms() {
        let atoms = atomize_code("  x");
        assert_eq!(
            atoms,
            vec![
                Atom {
                    text: " ".into(),
                    style: style_code(),
                    kind: AtomKind::Text,
                    start: 0,
                    end: 1
                },
                Atom {
                    text: " ".into(),
                    style: style_code(),
                    kind: AtomKind::Text,
                    start: 1,
                    end: 2
                },
                Atom {
                    text: "x".into(),
                    style: style_code(),
                    kind: AtomKind::Text,
                    start: 2,
                    end: 3
                },
            ]
        );
    }
}
