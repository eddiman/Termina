use crate::types::AppState;
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::TrayIconBuilder,
    Manager, Emitter,
};

pub fn create_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id_str = event.id.as_ref();
            if id_str == "quit" {
                handle_quit(app);
            } else if id_str == "show" {
                show_window(app);
            } else if id_str.starts_with("cmd:") {
                let cmd_id = id_str.strip_prefix("cmd:").unwrap_or("");
                let _ = app.emit("open-command", cmd_id.to_string());
                show_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_quit(app: &tauri::AppHandle) {
    let _ = app.emit("confirm-quit", ());
    show_window(app);
}

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Open Termina", true, None::<&str>)?;

    // Build active commands submenu
    let state: tauri::State<AppState> = app.state();
    let running_names: Vec<(String, String)> = {
        let processes = state.processes.lock().unwrap_or_else(|e| e.into_inner());
        let commands = state.commands.lock().unwrap_or_else(|e| e.into_inner());
        processes
            .keys()
            .filter_map(|id| {
                commands.iter().find(|c| &c.id == id).map(|c| (c.id.clone(), c.name.clone()))
            })
            .collect()
    };

    let submenu = Submenu::with_id(app, "active", "Active Commands", true)?;
    if running_names.is_empty() {
        let empty = MenuItem::with_id(app, "no-active", "No active commands", false, None::<&str>)?;
        submenu.append(&empty)?;
    } else {
        for (id, name) in &running_names {
            let item = MenuItem::with_id(app, format!("cmd:{}", id), name, true, None::<&str>)?;
            submenu.append(&item)?;
        }
    }

    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &submenu, &quit])?;
    Ok(menu)
}

pub fn update_tray_menu(app: &tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}
