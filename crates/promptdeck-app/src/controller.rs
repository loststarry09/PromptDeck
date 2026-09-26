use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel, Weak};

use promptdeck_core::error::Result as CoreResult;
use promptdeck_core::storage::library::{Library, SETTING_RAIL_EXPANDED, SETTING_THEME};

use crate::strings;
use crate::theme::{self, ThemeMode};
use crate::{AppWindow, PromptRow};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);

pub struct Controller {
    library: Library,
    ui: Weak<AppWindow>,
    items: Rc<VecModel<PromptRow>>,
    save_timer: Timer,
    dirty: Cell<bool>,
    active_id: RefCell<Option<String>>,
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
                ui.set_selected_id(item.id.into());
                ui.set_title_text(item.title.into());
                ui.set_body_text(item.body_md.into());
                ui.set_editor_focus_request(ui.get_editor_focus_request() + 1);
            }
            None => {
                *self.active_id.borrow_mut() = None;
                ui.set_selected_id(String::new().into());
                ui.set_title_text(String::new().into());
                ui.set_body_text(String::new().into());
            }
        }
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

    fn refresh_items(&self) {
        let now = self.library.now_ms();
        let Ok(summaries) = self.library.list_prompts() else {
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

        self.items.set_vec(rows);
    }
}
