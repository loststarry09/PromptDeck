//! Markdown 只读模式的选择模型：atom 几何收集、命中测试、选择规范化与文本提取。
//!
//! UI 只负责渲染与上报 atom 的内容坐标矩形；所有选择语义（命中哪个字符、
//! 高亮哪一段、复制什么文本）都在这里以纯函数实现，便于单测。

/// 单个 atom 在 Markdown 内容坐标系中的矩形（由 UI 上报）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AtomRect {
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// 文档内的字符位置：块序号 + 块内字符偏移。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub block: usize,
    pub offset: usize,
}

/// 选择：anchor 是不动端，focus 随指针移动。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

/// 单块的渲染文本与每个 atom 的字符区间（与 UI atom 一一对应）。
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub text: String,
    pub atoms: Vec<(usize, usize)>,
}

/// 渲染文档的选择状态。
#[derive(Debug, Default)]
pub struct Document {
    blocks: Vec<BlockInfo>,
    geometry: Vec<Vec<AtomRect>>,
}

impl Document {
    /// 设置块内容（切换条目/重渲染时调用），同时清空旧几何。
    pub fn set_blocks(&mut self, blocks: Vec<BlockInfo>) {
        self.geometry = blocks
            .iter()
            .map(|block| vec![AtomRect::default(); block.atoms.len()])
            .collect();
        self.blocks = blocks;
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.geometry.clear();
    }

    /// 记录某个 atom 的几何（UI 上报）。
    pub fn set_atom_rect(&mut self, block: usize, atom: usize, x: f32, y: f32, w: f32, h: f32) {
        let Some(info) = self.blocks.get(block) else {
            return;
        };
        let Some((start, end)) = info.atoms.get(atom).copied() else {
            return;
        };
        let Some(rects) = self.geometry.get_mut(block) else {
            return;
        };
        let Some(rect) = rects.get_mut(atom) else {
            return;
        };
        *rect = AtomRect {
            start,
            end,
            x,
            y,
            w,
            h,
        };
    }

    /// 命中测试：返回点击位置对应的字符位置。
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Position> {
        self.best_atom(x, y).map(|(block, rect)| Position {
            block,
            offset: if x < rect.x + rect.w / 2.0 {
                rect.start
            } else {
                rect.end
            },
        })
    }

    /// 命中测试：返回点击位置所在 atom 的（块，起，止）。
    pub fn hit_test_atom(&self, x: f32, y: f32) -> Option<(usize, usize, usize)> {
        self.best_atom(x, y)
            .map(|(block, rect)| (block, rect.start, rect.end))
    }

    /// 全选当前文档。
    pub fn select_all(&self) -> Option<Selection> {
        let last = self.blocks.len().checked_sub(1)?;
        let end = self.blocks[last].text.chars().count();
        Some(Selection {
            anchor: Position {
                block: 0,
                offset: 0,
            },
            focus: Position {
                block: last,
                offset: end,
            },
        })
    }

    /// 规范化选择（anchor/focus 排序）。
    pub fn normalized(selection: Selection) -> (Position, Position) {
        if selection.anchor <= selection.focus {
            (selection.anchor, selection.focus)
        } else {
            (selection.focus, selection.anchor)
        }
    }

    pub fn is_selection_empty(selection: Selection) -> bool {
        selection.anchor == selection.focus
    }

    /// 提取所选渲染文本；跨块以换行连接。
    pub fn text(&self, selection: Selection) -> String {
        let (start, end) = Self::normalized(selection);
        if start == end {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        for (index, block) in self.blocks.iter().enumerate() {
            if index < start.block || index > end.block {
                continue;
            }
            let len = block.text.chars().count();
            let from = if index == start.block {
                start.offset.min(len)
            } else {
                0
            };
            let to = if index == end.block {
                end.offset.min(len)
            } else {
                len
            };
            parts.push(
                block
                    .text
                    .chars()
                    .skip(from)
                    .take(to.saturating_sub(from))
                    .collect(),
            );
        }
        parts.join("\n")
    }

    fn best_atom(&self, x: f32, y: f32) -> Option<(usize, &AtomRect)> {
        let mut best: Option<(f32, f32, usize, &AtomRect)> = None;
        for (block, rects) in self.geometry.iter().enumerate() {
            for rect in rects.iter() {
                if rect.w <= 0.0 || rect.h <= 0.0 {
                    continue;
                }
                let vertical = axis_distance(y, rect.y, rect.y + rect.h);
                let horizontal = axis_distance(x, rect.x, rect.x + rect.w);
                let better = match &best {
                    None => true,
                    Some((best_v, best_h, _, _)) => {
                        vertical < *best_v || (vertical == *best_v && horizontal < *best_h)
                    }
                };
                if better {
                    best = Some((vertical, horizontal, block, rect));
                }
            }
        }
        best.map(|(_, _, block, rect)| (block, rect))
    }
}

fn axis_distance(value: f32, lo: f32, hi: f32) -> f32 {
    if value < lo {
        lo - value
    } else if value > hi {
        value - hi
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> Document {
        let mut doc = Document::default();
        doc.set_blocks(vec![
            BlockInfo {
                text: "hello world".into(),
                atoms: vec![(0, 5), (6, 11)],
            },
            BlockInfo {
                text: "second block".into(),
                atoms: vec![(0, 6), (7, 12)],
            },
        ]);
        // block 0 line: "hello " at (0,0,50,20), "world" at (50,0,50,20)
        doc.set_atom_rect(0, 0, 0.0, 0.0, 50.0, 20.0);
        doc.set_atom_rect(0, 1, 50.0, 0.0, 50.0, 20.0);
        // block 1 line: "second " at (0,40,60,20), "block" at (60,40,60,20)
        doc.set_atom_rect(1, 0, 0.0, 40.0, 60.0, 20.0);
        doc.set_atom_rect(1, 1, 60.0, 40.0, 60.0, 20.0);
        doc
    }

    #[test]
    fn hit_test_uses_left_or_right_half_of_the_atom() {
        let doc = document();
        assert_eq!(
            doc.hit_test(10.0, 10.0),
            Some(Position {
                block: 0,
                offset: 0
            })
        );
        assert_eq!(
            doc.hit_test(40.0, 10.0),
            Some(Position {
                block: 0,
                offset: 5
            })
        );
        assert_eq!(
            doc.hit_test(60.0, 10.0),
            Some(Position {
                block: 0,
                offset: 6
            })
        );
        assert_eq!(
            doc.hit_test(90.0, 10.0),
            Some(Position {
                block: 0,
                offset: 11
            })
        );
    }

    #[test]
    fn hit_test_clamps_to_the_nearest_line_and_side() {
        let doc = document();
        // right of the first line -> end of its last atom
        assert_eq!(
            doc.hit_test(300.0, 10.0),
            Some(Position {
                block: 0,
                offset: 11
            })
        );
        // between the two blocks -> closer to block 0
        assert_eq!(
            doc.hit_test(10.0, 25.0),
            Some(Position {
                block: 0,
                offset: 0
            })
        );
        // below everything -> last line, offset from the click's x position
        assert_eq!(
            doc.hit_test(70.0, 400.0),
            Some(Position {
                block: 1,
                offset: 7
            })
        );
        assert_eq!(
            doc.hit_test(110.0, 400.0),
            Some(Position {
                block: 1,
                offset: 12
            })
        );
        // above everything -> first atom
        assert_eq!(
            doc.hit_test(5.0, -50.0),
            Some(Position {
                block: 0,
                offset: 0
            })
        );
    }

    #[test]
    fn hit_test_returns_none_without_geometry() {
        let doc = Document::default();
        assert_eq!(doc.hit_test(10.0, 10.0), None);
        let mut doc = document();
        doc.clear();
        assert_eq!(doc.hit_test(10.0, 10.0), None);
    }

    #[test]
    fn hit_test_atom_returns_full_atom_range() {
        let doc = document();
        assert_eq!(doc.hit_test_atom(10.0, 10.0), Some((0, 0, 5)));
        assert_eq!(doc.hit_test_atom(90.0, 50.0), Some((1, 7, 12)));
    }

    #[test]
    fn selection_normalizes_both_directions() {
        let forward = Selection {
            anchor: Position {
                block: 0,
                offset: 2,
            },
            focus: Position {
                block: 1,
                offset: 3,
            },
        };
        let backward = Selection {
            anchor: forward.focus,
            focus: forward.anchor,
        };
        assert_eq!(
            Document::normalized(forward),
            Document::normalized(backward)
        );
        assert!(!Document::is_selection_empty(forward));
        assert!(Document::is_selection_empty(Selection {
            anchor: forward.anchor,
            focus: forward.anchor
        }));
    }

    #[test]
    fn text_extracts_within_one_block() {
        let doc = document();
        let selection = Selection {
            anchor: Position {
                block: 0,
                offset: 0,
            },
            focus: Position {
                block: 0,
                offset: 5,
            },
        };
        assert_eq!(doc.text(selection), "hello");
    }

    #[test]
    fn text_extracts_across_blocks_with_newlines() {
        let doc = document();
        let selection = Selection {
            anchor: Position {
                block: 0,
                offset: 6,
            },
            focus: Position {
                block: 1,
                offset: 6,
            },
        };
        assert_eq!(doc.text(selection), "world\nsecond");
    }

    #[test]
    fn text_keeps_the_newline_when_selection_spans_a_boundary() {
        let doc = document();
        let selection = Selection {
            anchor: Position {
                block: 0,
                offset: 11,
            },
            focus: Position {
                block: 1,
                offset: 6,
            },
        };
        assert_eq!(doc.text(selection), "\nsecond");
    }

    #[test]
    fn empty_selection_copies_nothing() {
        let doc = document();
        let selection = Selection {
            anchor: Position {
                block: 0,
                offset: 3,
            },
            focus: Position {
                block: 0,
                offset: 3,
            },
        };
        assert_eq!(doc.text(selection), "");
    }

    #[test]
    fn select_all_covers_every_block() {
        let doc = document();
        let selection = doc.select_all().expect("document has blocks");
        assert_eq!(
            selection.anchor,
            Position {
                block: 0,
                offset: 0
            }
        );
        assert_eq!(
            selection.focus,
            Position {
                block: 1,
                offset: 12
            }
        );
        assert_eq!(doc.text(selection), "hello world\nsecond block");
        assert!(Document::default().select_all().is_none());
    }

    #[test]
    fn offsets_beyond_the_block_are_clamped() {
        let doc = document();
        let selection = Selection {
            anchor: Position {
                block: 0,
                offset: 999,
            },
            focus: Position {
                block: 1,
                offset: 999,
            },
        };
        assert_eq!(doc.text(selection), "\nsecond block");
    }

    #[test]
    fn stray_geometry_reports_are_ignored() {
        let mut doc = document();
        doc.set_atom_rect(9, 0, 0.0, 0.0, 10.0, 10.0);
        doc.set_atom_rect(0, 9, 0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            doc.hit_test(10.0, 10.0),
            Some(Position {
                block: 0,
                offset: 0
            })
        );
    }
}
