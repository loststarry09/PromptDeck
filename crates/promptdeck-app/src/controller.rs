use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel, Weak};

use promptdeck_core::error::Result as CoreResult;
use promptdeck_core::storage::library::{Library, SETTING_RAIL_EXPANDED, SETTING_THEME};

use crate::markdown;
use crate::strings;
use crate::theme::{self, ThemeMode};
use crate::{AppWindow, MarkdownBlock, PromptRow};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);

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

pub struct Controller {
    library: Library,
    ui: Weak<AppWindow>,
    items: Rc<VecModel<PromptRow>>,
    save_timer: Timer,
    dirty: Cell<bool>,
    active_id: RefCell<Option<String>>,
    modes: RefCell<HashMap<String, ViewMode>>,
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
            dirty: Cell::new(false),
            active_id: RefCell::new(None),
            modes: RefCell::new(HashMap::new()),
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
                self.apply_mode(self.mode_for(&item.id), &ui);
                ui.set_editor_focus_request(ui.get_editor_focus_request() + 1);
            }
            None => {
                *self.active_id.borrow_mut() = None;
                ui.set_selected_id(String::new().into());
                ui.set_title_text(String::new().into());
                ui.set_body_text(String::new().into());
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
        }
    }

    fn render_markdown(&self, ui: &AppWindow) {
        let blocks = markdown::to_ui_blocks(&ui.get_body_text());
        ui.set_markdown_blocks(ModelRc::from(Rc::new(VecModel::from(blocks))));
    }

    fn clear_markdown_blocks(&self, ui: &AppWindow) {
        ui.set_markdown_blocks(ModelRc::from(Rc::new(VecModel::<MarkdownBlock>::default())));
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
        let Ok(summaries) = self.library.search_prompts(&query) else {
            return;
        };

        let rows = summaries
            .into_iter()
            .map(|summary| PromptRow {
                id: summary.id.into(),
                title: if summary.title.is_empty() {
                    strings::UNTITLED.into()
                } else {
                    summary.title.into()
                },
                updated_label: strings::updated_label(now, summary.updated_at).into(),
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
