use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel, Weak};

use promptdeck_core::error::Result as CoreResult;
use promptdeck_core::model::Tag;
use promptdeck_core::storage::library::{Library, SETTING_RAIL_EXPANDED, SETTING_THEME};
use promptdeck_core::variables;

use crate::markdown;
use crate::selection::{BlockInfo, Document, Position, Selection};
use crate::strings;
use crate::theme::{self, ThemeMode};
use crate::{AppWindow, MarkdownBlock, MarkdownSelection, PromptRow};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);
/// 等布局稳定后再向 UI 取一次完整 atom 几何（首次渲染时 atom 可能尚未实例化）。
const GEOMETRY_SETTLE_DELAY: Duration = Duration::from_millis(80);
/// 复制成功提示的停留时长；到点自动淡出，无需用户操作。
const COPY_FEEDBACK_DURATION: Duration = Duration::from_millis(1600);
/// 列表行最多直接展示的标签 chip 数；其余以「+N」表示，避免行高与宽度被撑破。
const LIST_TAG_LIMIT: usize = 3;

/// Builds the Slint model of Tag labels shared by the canvas and list rows.
fn tags_model(tags: &[Tag]) -> ModelRc<slint::SharedString> {
    let labels = tags
        .iter()
        .map(|tag| slint::SharedString::from(tag.as_str()))
        .collect::<Vec<_>>();
    ModelRc::from(Rc::new(VecModel::from(labels)))
}

/// 画布模式。Markdown 模式只读渲染；Source 模式是唯一可编辑面（ADR-0003）。
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ViewMode {
    #[default]
    Source,
    Markdown,
}

impl ViewMode {
    fn from_ui(mode: i32) -> Self {
        if mode == 0 {
            Self::Source
        } else {
            Self::Markdown
        }
    }
}

/// Writes plain text to the OS clipboard. Returns whether it succeeded, so
/// callers can decide on feedback without leaking `arboard` types outward.
fn copy_to_clipboard(text: &str) -> bool {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .is_ok()
}

pub struct Controller {
    library: Library,
    ui: Weak<AppWindow>,
    items: Rc<VecModel<PromptRow>>,
    save_timer: Timer,
    geometry_timer: Timer,
    feedback_timer: Timer,
    dirty: Cell<bool>,
    active_id: RefCell<Option<String>>,
    modes: RefCell<HashMap<String, ViewMode>>,
    active_tag: RefCell<Option<String>>,
    document: RefCell<Document>,
    selection: RefCell<Option<Selection>>,
    selecting: Cell<bool>,
    theme_mode: Cell<ThemeMode>,
    rail_expanded: Cell<bool>,
}

impl Controller {
    pub fn attach(ui: &AppWindow, library: Library) -> CoreResult<Rc<Self>> {
        let items = Rc::new(VecModel::<PromptRow>::default());
        ui.set_prompts(ModelRc::from(items.clone()));

        let theme_mode = ThemeMode::from_setting(library.setting(SETTING_THEME)?.as_deref());
        theme::apply(ui, theme_mode);

        let rail_expanded = library.setting(SETTING_RAIL_EXPANDED)?.as_deref() == Some("true");
        ui.set_rail_expanded(rail_expanded);

        let controller = Rc::new(Self {
            library,
            ui: ui.as_weak(),
            items,
            save_timer: Timer::default(),
            geometry_timer: Timer::default(),
            feedback_timer: Timer::default(),
            dirty: Cell::new(false),
            active_id: RefCell::new(None),
            modes: RefCell::new(HashMap::new()),
            active_tag: RefCell::new(None),
            document: RefCell::new(Document::default()),
            selection: RefCell::new(None),
            selecting: Cell::new(false),
            theme_mode: Cell::new(theme_mode),
            rail_expanded: Cell::new(rail_expanded),
        });

        {
            let weak = Rc::downgrade(&controller);
            controller
                .save_timer
                .start(TimerMode::SingleShot, AUTOSAVE_DELAY, move || {
                    if let Some(controller) = weak.upgrade() {
                        controller.flush();
                    }
                });
            controller.save_timer.stop();
        }

        {
            let weak = Rc::downgrade(&controller);
            controller.geometry_timer.start(
                TimerMode::SingleShot,
                GEOMETRY_SETTLE_DELAY,
                move || {
                    if let Some(controller) = weak.upgrade() {
                        controller.bump_geometry();
                    }
                },
            );
            controller.geometry_timer.stop();
        }

        {
            let weak = Rc::downgrade(&controller);
            controller.feedback_timer.start(
                TimerMode::SingleShot,
                COPY_FEEDBACK_DURATION,
                move || {
                    if let Some(controller) = weak.upgrade()
                        && let Some(ui) = controller.ui.upgrade()
                    {
                        ui.set_copy_feedback(false);
                    }
                },
            );
            controller.feedback_timer.stop();
        }

        {
            let controller = controller.clone();
            ui.on_new_prompt(move || controller.new_prompt());
        }
        {
            let controller = controller.clone();
            ui.on_select_prompt(move |id| controller.select_prompt(Some(id.to_string())));
        }
        {
            let controller = controller.clone();
            ui.on_text_edited(move || controller.mark_dirty());
        }
        {
            let controller = controller.clone();
            ui.on_cycle_theme(move || controller.cycle_theme());
        }
        {
            let controller = controller.clone();
            ui.on_toggle_rail(move || controller.toggle_rail());
        }
        {
            let controller = controller.clone();
            ui.on_search_edited(move || controller.search_edited());
        }
        {
            let controller = controller.clone();
            ui.on_mode_selected(move |mode| controller.select_mode(mode));
        }
        {
            let controller = controller.clone();
            ui.on_toggle_mode(move || controller.toggle_mode());
        }
        {
            let controller = controller.clone();
            ui.on_toggle_pin(move || controller.toggle_pin());
        }
        {
            let controller = controller.clone();
            ui.on_add_tag(move |name| controller.add_tag(name.as_str()));
        }
        {
            let controller = controller.clone();
            ui.on_tag_edited(move || controller.tag_input_edited());
        }
        {
            let controller = controller.clone();
            ui.on_remove_tag(move |tag| controller.remove_tag(tag.as_str()));
        }
        {
            let controller = controller.clone();
            ui.on_toggle_tag_filter(move |tag| controller.toggle_tag_filter(tag.as_str()));
        }
        {
            let controller = controller.clone();
            ui.on_clear_tag_filter(move || controller.clear_tag_filter());
        }
        {
            let controller = controller.clone();
            ui.on_markdown_atom_geometry(move |block, atom, x, y, w, h| {
                controller.document.borrow_mut().set_atom_rect(
                    block.max(0) as usize,
                    atom.max(0) as usize,
                    x,
                    y,
                    w,
                    h,
                );
            });
        }
        {
            let controller = controller.clone();
            ui.on_markdown_pressed(move |x, y| controller.begin_selection(x, y));
        }
        {
            let controller = controller.clone();
            ui.on_markdown_moved(move |x, y| controller.extend_selection(x, y));
        }
        {
            let controller = controller.clone();
            ui.on_markdown_released(move || controller.selecting.set(false));
        }
        {
            let controller = controller.clone();
            ui.on_markdown_double_clicked(move |x, y| controller.select_atom_at(x, y));
        }
        {
            let controller = controller.clone();
            ui.on_copy_selection(move || controller.copy_selection());
        }
        {
            let controller = controller.clone();
            ui.on_copy_resolved(move || controller.copy_resolved());
        }
        {
            let controller = controller.clone();
            ui.on_select_all(move || controller.select_all());
        }

        controller.refresh_items();
        controller.restore_selection();

        Ok(controller)
    }

    pub fn new_prompt(&self) {
        self.flush();
        let Ok(item) = self.library.create_prompt() else {
            return;
        };
        self.refresh_items();
        self.select_prompt(Some(item.id));
    }

    pub fn select_prompt(&self, id: Option<String>) {
        self.flush();
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.clear_copy_feedback(&ui);
        ui.set_tag_error("".into());

        match id {
            Some(id) => {
                let Ok(Some(item)) = self.library.load(&id) else {
                    return;
                };
                *self.active_id.borrow_mut() = Some(item.id.clone());
                let _ = self.library.set_selected_prompt(Some(&item.id));
                ui.set_selected_id(item.id.clone().into());
                ui.set_title_text(item.title.into());
                ui.set_body_text(item.body_md.into());
                ui.set_pinned(item.pinned);
                self.sync_tags(&ui, &item.id);
                self.apply_mode(self.mode_for(&item.id), &ui);
                ui.set_editor_focus_request(ui.get_editor_focus_request() + 1);
            }
            None => {
                *self.active_id.borrow_mut() = None;
                ui.set_selected_id(String::new().into());
                ui.set_title_text(String::new().into());
                ui.set_body_text(String::new().into());
                ui.set_pinned(false);
                ui.set_tags(tags_model(&[]));
                ui.set_markdown_mode(false);
                self.clear_markdown_blocks(&ui);
            }
        }
    }

    /// 切换当前条目的画布模式；模式按条目在同一会话内保持（不落库）。
    pub fn select_mode(&self, mode: i32) {
        self.store_mode(ViewMode::from_ui(mode));
    }

    pub fn toggle_mode(&self) {
        let Some(id) = self.active_id.borrow().clone() else {
            return;
        };
        let next = match self.mode_for(&id) {
            ViewMode::Source => ViewMode::Markdown,
            ViewMode::Markdown => ViewMode::Source,
        };
        self.store_mode(next);
    }

    /// 切换当前 Prompt 的置顶状态并即时重排列表；置顶不动 `updated_at`，
    /// 取消置顶后 Prompt 回到原有的“最近更新”位置。
    pub fn toggle_pin(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(id) = self.active_id.borrow().clone() else {
            return;
        };
        // 以库中状态为准切换（SQLite 是唯一真源），不读可能滞后的 UI 镜像。
        let Ok(Some(current)) = self.library.load(&id) else {
            return;
        };
        let Ok(item) = self.library.set_pinned(&id, !current.pinned) else {
            return;
        };
        ui.set_pinned(item.pinned);
        self.refresh_items();
    }

    /// 给当前 Prompt 挂一个 Tag 并即时刷新画布与列表。空白、重复（大小写不敏感）
    /// 的输入会被拒绝并在输入框内给出明确提示；挂标签不改变 `updated_at`，因此
    /// 不会打乱“置顶优先、其余按最近更新”的顺序。
    pub fn add_tag(&self, name: &str) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(id) = self.active_id.borrow().clone() else {
            return;
        };

        let name = name.trim();
        if name.is_empty() {
            ui.set_tag_error(strings::TAG_EMPTY_ERROR.into());
            return;
        }
        let existing = self.library.tags(&id).unwrap_or_default();
        if existing
            .iter()
            .any(|tag| tag.as_str().eq_ignore_ascii_case(name))
        {
            ui.set_tag_error(strings::TAG_DUPLICATE_ERROR.into());
            return;
        }

        if self.library.add_tag(&id, name).is_ok() {
            ui.set_tag_error("".into());
            self.sync_tags(&ui, &id);
            self.refresh_items();
        }
    }

    /// 从当前 Prompt 摘掉一个 Tag；该 Tag 若不再被任何 Prompt 使用则从词表消失。
    pub fn remove_tag(&self, name: &str) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(id) = self.active_id.borrow().clone() else {
            return;
        };
        if self.library.remove_tag(&id, name).is_err() {
            return;
        }
        ui.set_tag_error("".into());
        self.sync_tags(&ui, &id);
        self.refresh_items();
    }

    /// 用户开始编辑标签输入：清掉上一次的拒绝提示。
    pub fn tag_input_edited(&self) {
        if let Some(ui) = self.ui.upgrade()
            && !ui.get_tag_error().is_empty()
        {
            ui.set_tag_error("".into());
        }
    }

    /// 点列表标签 chip 切换过滤：再次点击同一 Tag 或显式清除即恢复全量。
    pub fn toggle_tag_filter(&self, tag: &str) {
        let next = match self.active_tag.borrow().as_deref() {
            Some(active) if active.eq_ignore_ascii_case(tag) => None,
            _ => Some(tag.to_string()),
        };
        self.set_tag_filter(next);
    }

    pub fn clear_tag_filter(&self) {
        self.set_tag_filter(None);
    }

    fn set_tag_filter(&self, tag: Option<String>) {
        *self.active_tag.borrow_mut() = tag;
        if let Some(ui) = self.ui.upgrade() {
            ui.set_tag_filter(self.active_tag.borrow().clone().unwrap_or_default().into());
        }
        self.refresh_items();
    }

    fn sync_tags(&self, ui: &AppWindow, id: &str) {
        let tags = self.library.tags(id).unwrap_or_default();
        ui.set_tags(tags_model(&tags));
    }

    fn store_mode(&self, mode: ViewMode) {
        let Some(id) = self.active_id.borrow().clone() else {
            return;
        };
        self.modes.borrow_mut().insert(id, mode);
        if let Some(ui) = self.ui.upgrade() {
            self.apply_mode(mode, &ui);
        }
    }

    fn mode_for(&self, id: &str) -> ViewMode {
        self.modes.borrow().get(id).copied().unwrap_or_default()
    }

    fn apply_mode(&self, mode: ViewMode, ui: &AppWindow) {
        ui.set_markdown_mode(mode == ViewMode::Markdown);
        if mode == ViewMode::Markdown {
            self.render_markdown(ui);
        } else {
            self.selecting.set(false);
            *self.selection.borrow_mut() = None;
            self.push_selection(ui);
        }
    }

    fn render_markdown(&self, ui: &AppWindow) {
        let doc = markdown::render(&ui.get_body_text());
        let infos = doc
            .texts
            .iter()
            .zip(doc.atom_ranges.iter())
            .map(|(text, atoms)| BlockInfo {
                text: text.clone(),
                atoms: atoms.clone(),
            })
            .collect();
        self.document.borrow_mut().set_blocks(infos);
        self.selecting.set(false);
        *self.selection.borrow_mut() = None;

        ui.set_markdown_blocks(ModelRc::from(Rc::new(VecModel::from(doc.blocks))));
        self.push_selection(ui);
        ui.set_markdown_geometry_tick(ui.get_markdown_geometry_tick() + 1);
        self.geometry_timer.restart();
    }

    fn bump_geometry(&self) {
        if let Some(ui) = self.ui.upgrade() {
            ui.set_markdown_geometry_tick(ui.get_markdown_geometry_tick() + 1);
        }
    }

    fn begin_selection(&self, x: f32, y: f32) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let hit = self.document.borrow().hit_test(x, y);
        self.selecting.set(true);
        *self.selection.borrow_mut() = hit.map(|position| Selection {
            anchor: position,
            focus: position,
        });
        self.push_selection(&ui);
    }

    fn extend_selection(&self, x: f32, y: f32) {
        if !self.selecting.get() {
            return;
        }
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(focus) = self.document.borrow().hit_test(x, y) else {
            return;
        };
        let Some(anchor) = self.selection.borrow().map(|selection| selection.anchor) else {
            return;
        };
        *self.selection.borrow_mut() = Some(Selection { anchor, focus });
        self.push_selection(&ui);
    }

    fn select_atom_at(&self, x: f32, y: f32) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some((block, start, end)) = self.document.borrow().hit_test_atom(x, y) else {
            return;
        };
        *self.selection.borrow_mut() = Some(Selection {
            anchor: Position {
                block,
                offset: start,
            },
            focus: Position { block, offset: end },
        });
        self.push_selection(&ui);
    }

    fn select_all(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        *self.selection.borrow_mut() = self.document.borrow().select_all();
        self.push_selection(&ui);
    }

    /// `Ctrl+C`：复制当前选择的渲染文本（无选择时不改剪贴板）。
    fn copy_selection(&self) {
        let Some(selection) = *self.selection.borrow() else {
            return;
        };
        let text = self.document.borrow().text(selection);
        if !text.is_empty() {
            let _ = copy_to_clipboard(&text);
        }
    }

    /// `Ctrl+Shift+C`：复制默认值已填充的最终文本；无默认值的变量保持
    /// `{{name}}` 原样，`\{{` 还原为字面 `{{`。成功后才给出轻量提示。
    fn copy_resolved(&self) {
        if self.active_id.borrow().is_none() {
            return;
        }
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let resolved = variables::resolve(&ui.get_body_text());
        if resolved.is_empty() {
            return;
        }
        if copy_to_clipboard(&resolved) {
            ui.set_copy_feedback(true);
            self.feedback_timer.restart();
        }
    }

    /// 复制提示是瞬时的：切换条目时立即清掉，避免提示落在新条目上。
    fn clear_copy_feedback(&self, ui: &AppWindow) {
        self.feedback_timer.stop();
        if ui.get_copy_feedback() {
            ui.set_copy_feedback(false);
        }
    }

    fn push_selection(&self, ui: &AppWindow) {
        let payload = match *self.selection.borrow() {
            Some(selection) => {
                let (start, end) = Document::normalized(selection);
                MarkdownSelection {
                    active: !Document::is_selection_empty(selection),
                    block_a: start.block as i32,
                    off_a: start.offset as i32,
                    block_b: end.block as i32,
                    off_b: end.offset as i32,
                }
            }
            None => MarkdownSelection::default(),
        };
        ui.set_markdown_selection(payload);
    }

    fn clear_markdown_blocks(&self, ui: &AppWindow) {
        self.document.borrow_mut().clear();
        self.selecting.set(false);
        *self.selection.borrow_mut() = None;
        ui.set_markdown_blocks(ModelRc::from(Rc::new(VecModel::<MarkdownBlock>::default())));
        self.push_selection(ui);
    }

    pub fn mark_dirty(&self) {
        self.dirty.set(true);
        self.save_timer.restart();
    }

    pub fn flush(&self) {
        if !self.dirty.get() {
            return;
        }
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(id) = self.active_id.borrow().clone() else {
            return;
        };

        let title = ui.get_title_text().to_string();
        let body_md = ui.get_body_text().to_string();

        let Ok(item) = self.library.save(&id, &title, &body_md) else {
            return;
        };

        self.dirty.set(false);
        if !ui.get_title_focused() && ui.get_title_text().as_str() != item.title {
            ui.set_title_text(item.title.clone().into());
        }
        self.refresh_items();
    }

    pub fn cycle_theme(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let next = self.theme_mode.get().next();
        self.theme_mode.set(next);
        theme::apply(&ui, next);
        let _ = self.library.set_setting(SETTING_THEME, next.as_setting());
    }

    pub fn toggle_rail(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let expanded = !self.rail_expanded.get();
        self.rail_expanded.set(expanded);
        ui.set_rail_expanded(expanded);
        let _ = self.library.set_setting(
            SETTING_RAIL_EXPANDED,
            if expanded { "true" } else { "false" },
        );
    }

    fn restore_selection(&self) {
        let stored = self.library.selected_prompt().ok().flatten();
        let id = stored.or_else(|| {
            self.library
                .list_prompts()
                .ok()
                .and_then(|items| items.first().map(|item| item.id.clone()))
        });
        self.select_prompt(id);
    }

    pub fn search_edited(&self) {
        self.refresh_items();
    }

    fn refresh_items(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let now = self.library.now_ms();
        let query = ui.get_search_query().to_string();
        let tag = self.active_tag.borrow().clone();
        let Ok(summaries) = self.library.search_prompts(&query, tag.as_deref()) else {
            return;
        };

        let rows = summaries
            .into_iter()
            .map(|summary| {
                let visible = summary.tags.len().min(LIST_TAG_LIMIT);
                PromptRow {
                    id: summary.id.into(),
                    title: if summary.title.is_empty() {
                        strings::UNTITLED.into()
                    } else {
                        summary.title.into()
                    },
                    updated_label: strings::updated_label(now, summary.updated_at).into(),
                    pinned: summary.pinned,
                    tags: tags_model(&summary.tags[..visible]),
                    tag_overflow: (summary.tags.len() - visible) as i32,
                }
            })
            .collect::<Vec<_>>();

        let library_empty = rows.is_empty()
            && self
                .library
                .list_prompts()
                .map(|items| items.is_empty())
                .unwrap_or(false);
        ui.set_library_empty(library_empty);
        ui.set_search_empty_title(if rows.is_empty() && !query.trim().is_empty() {
            strings::empty_search_title(&query).into()
        } else {
            String::new().into()
        });
        self.items.set_vec(rows);
    }
}
