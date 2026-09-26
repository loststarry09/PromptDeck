use slint::ComponentHandle;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

use crate::{AppWindow, Strings};

pub const APP_TITLE: &str = "PromptDeck";
pub const RAIL_LIBRARY: &str = "提示词";
pub const RAIL_INBOX: &str = "收件箱";
pub const RAIL_SETTINGS: &str = "设置";
pub const COMING_SOON: &str = "尚未开放";
pub const LIBRARY_HEADING: &str = "提示词库";
pub const ACTION_NEW: &str = "新建";
pub const EMPTY_LIBRARY_TITLE: &str = "还没有提示词";
pub const EMPTY_LIBRARY_HINT: &str = "按 Ctrl+N 新建第一条";
pub const EMPTY_CANVAS_TITLE: &str = "未选择提示词";
pub const EMPTY_CANVAS_HINT: &str = "从左侧选择，或按 Ctrl+N 新建";
pub const UNTITLED: &str = "无标题";
pub const TITLE_PLACEHOLDER: &str = "无标题";
pub const EDITOR_PLACEHOLDER: &str = "开始输入 Markdown…";
pub const MODE_SOURCE: &str = "源码";
pub const MODE_MARKDOWN: &str = "Markdown";
pub const THEME_SYSTEM: &str = "跟随系统";
pub const THEME_LIGHT: &str = "亮色";
pub const THEME_DARK: &str = "暗色";
pub const RAIL_EXPAND: &str = "展开侧栏";
pub const RAIL_COLLAPSE: &str = "收起侧栏";

pub fn install(ui: &AppWindow) {
    let strings = ui.global::<Strings>();
    strings.set_app_title(APP_TITLE.into());
    strings.set_rail_library(RAIL_LIBRARY.into());
    strings.set_rail_inbox(RAIL_INBOX.into());
    strings.set_rail_settings(RAIL_SETTINGS.into());
    strings.set_coming_soon(COMING_SOON.into());
    strings.set_library_heading(LIBRARY_HEADING.into());
    strings.set_action_new(ACTION_NEW.into());
    strings.set_empty_library_title(EMPTY_LIBRARY_TITLE.into());
    strings.set_empty_library_hint(EMPTY_LIBRARY_HINT.into());
    strings.set_empty_canvas_title(EMPTY_CANVAS_TITLE.into());
    strings.set_empty_canvas_hint(EMPTY_CANVAS_HINT.into());
    strings.set_untitled(UNTITLED.into());
    strings.set_title_placeholder(TITLE_PLACEHOLDER.into());
    strings.set_editor_placeholder(EDITOR_PLACEHOLDER.into());
    strings.set_mode_source(MODE_SOURCE.into());
    strings.set_mode_markdown(MODE_MARKDOWN.into());
    strings.set_theme_system(THEME_SYSTEM.into());
    strings.set_theme_light(THEME_LIGHT.into());
    strings.set_theme_dark(THEME_DARK.into());
    strings.set_rail_expand(RAIL_EXPAND.into());
    strings.set_rail_collapse(RAIL_COLLAPSE.into());
}

pub fn updated_label(now_ms: i64, updated_ms: i64) -> String {
    let elapsed = now_ms - updated_ms;
    if elapsed < 60_000 {
        return "刚刚".to_string();
    }
    if elapsed < 3_600_000 {
        return format!("{} 分钟前", elapsed / 60_000);
    }

    let (Some(now), Some(updated)) = (local_time(now_ms), local_time(updated_ms)) else {
        return "刚刚".to_string();
    };
    let now_date = now.date_naive();
    let updated_date = updated.date_naive();

    if now_date == updated_date {
        return format!("今天 {:02}:{:02}", updated.hour(), updated.minute());
    }
    if now_date.signed_duration_since(updated_date).num_days() == 1 {
        return format!("昨天 {:02}:{:02}", updated.hour(), updated.minute());
    }
    if now.year() == updated.year() {
        return format!("{}月{}日", updated.month(), updated.day());
    }
    format!(
        "{}年{}月{}日",
        updated.year(),
        updated.month(),
        updated.day()
    )
}

fn local_time(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).earliest()
}
