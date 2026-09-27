#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

mod controller;
mod listnav;
mod markdown;
mod selection;
mod strings;
mod theme;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let data_dir = promptdeck_core::paths::resolve_data_dir()?;
    std::fs::create_dir_all(&data_dir.path)?;

    let library =
        promptdeck_core::storage::library::Library::open(data_dir.path.join("promptdeck.sqlite3"))?;

    let ui = AppWindow::new()?;
    strings::install(&ui);
    let controller = controller::Controller::attach(&ui, library)?;

    ui.run()?;

    controller.flush();
    Ok(())
}
