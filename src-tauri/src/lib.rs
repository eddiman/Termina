mod commands;
mod config;
mod process_manager;
mod tray;
mod types;

use crate::process_manager::{check_process_status, kill_process_group, spawn_command, spawn_log_reader, ProcessCheckResult};
use crate::types::{AppState, CommandType, HealthStatus, LogBuffer, ProcessExitInfo, RunningProcess};
use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

fn cleanup_orphaned_processes() {
    let old_pgids = config::load_running_pids();
    if old_pgids.is_empty() {
        return;
    }
    for pgid in &old_pgids {
        let _ = kill_process_group(*pgid);
    }
    config::clear_running_pids();
}

fn auto_start_enabled_commands(state: &AppState) {
    let commands_to_start: Vec<(String, String, String, std::collections::HashMap<String, String>)> = {
        if let Ok(commands) = state.commands.lock() {
            commands
                .iter()
                .filter(|c| c.enabled && c.command_type == CommandType::Process)
                .map(|c| (c.id.clone(), c.cwd.clone(), c.command.clone(), c.env.clone()))
                .collect()
        } else {
            return;
        }
    };

    let (shell_path, init_script) = if let Ok(settings) = state.shell_settings.lock() {
        (settings.effective_shell(), settings.effective_init_script().to_string())
    } else {
        (std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()), String::new())
    };

    for (id, cwd, cmd, env) in commands_to_start {
        // Init log buffer
        if let Ok(mut logs) = state.logs.lock() {
            logs.insert(id.clone(), LogBuffer::new(500));
        }

        if let Ok(mut spawned) = spawn_command(&cwd, &cmd, &env, &shell_path, &init_script) {
            // Start log readers
            spawn_log_reader(id.clone(), &mut spawned.child, state.logs.clone());

            if let Ok(mut processes) = state.processes.lock() {
                processes.insert(
                    id,
                    RunningProcess {
                        pgid: spawned.pgid,
                        child: spawned.child,
                    },
                );
            }
        }
    }
}

fn start_process_monitor(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));

            let state: tauri::State<AppState> = app_handle.state();

            let mut exited_commands: Vec<(String, Option<i32>)> = Vec::new();

            if let Ok(mut processes) = state.processes.lock() {
                let ids: Vec<String> = processes.keys().cloned().collect();
                for id in ids {
                    if let Some(proc) = processes.get_mut(&id) {
                        if let ProcessCheckResult::Exited(code) = check_process_status(&mut proc.child) {
                            exited_commands.push((id.clone(), code));
                        }
                    }
                }

                // Remove exited processes
                for (id, _) in &exited_commands {
                    processes.remove(id);
                }
            }

            // Sync PID file if any processes exited
            if !exited_commands.is_empty() {
                if let Ok(processes) = state.processes.lock() {
                    let pgids: Vec<i32> = processes.values().map(|p| p.pgid).collect();
                    config::save_running_pids(&pgids);
                }
            }

            // Record exit info and emit events
            for (id, code) in &exited_commands {
                let command_name = if let Ok(commands) = state.commands.lock() {
                    commands.iter().find(|c| &c.id == id).map(|c| c.name.clone()).unwrap_or_default()
                } else {
                    String::new()
                };

                if let Ok(mut exit_info) = state.exit_info.lock() {
                    exit_info.insert(
                        id.clone(),
                        ProcessExitInfo {
                            code: *code,
                            command_name: command_name.clone(),
                        },
                    );
                }

                #[derive(Clone, serde::Serialize)]
                struct ProcessExitedPayload {
                    id: String,
                    code: Option<i32>,
                    name: String,
                }

                let _ = app_handle.emit(
                    "process-exited",
                    ProcessExitedPayload {
                        id: id.clone(),
                        code: *code,
                        name: command_name.clone(),
                    },
                );

                tray::update_tray_menu(&app_handle);

                // Send notification for non-zero exit codes
                if *code != Some(0) {
                    let msg = match code {
                        Some(c) => format!("\"{}\" exited with code {}", command_name, c),
                        None => format!("\"{}\" was terminated", command_name),
                    };
                    let _ = app_handle.emit("send-notification", msg);
                }
            }
        }
    });
}

fn start_health_checker(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(15));

            let state: tauri::State<AppState> = app_handle.state();

            // Collect commands with health check URLs
            let checks: Vec<(String, String)> = if let Ok(commands) = state.commands.lock() {
                commands
                    .iter()
                    .filter_map(|c| {
                        c.health_check_url.as_ref().map(|url| (c.id.clone(), url.clone()))
                    })
                    .collect()
            } else {
                continue;
            };

            if checks.is_empty() {
                continue;
            }

            let mut updates: Vec<(String, HealthStatus)> = Vec::new();

            for (id, url) in &checks {
                // Only check health for running processes
                let is_running = if let Ok(processes) = state.processes.lock() {
                    processes.contains_key(id)
                } else {
                    false
                };

                if !is_running {
                    updates.push((id.clone(), HealthStatus::Unknown));
                    continue;
                }

                let status = match ureq::get(url).call() {
                    Ok(response) => {
                        if response.status() >= 200 && response.status() < 400 {
                            HealthStatus::Healthy
                        } else {
                            HealthStatus::Unhealthy
                        }
                    }
                    Err(_) => HealthStatus::Unhealthy,
                };

                updates.push((id.clone(), status));
            }

            // Update health state and emit event
            if let Ok(mut health) = state.health.lock() {
                for (id, status) in &updates {
                    health.insert(id.clone(), status.clone());
                }
            }

            #[derive(Clone, serde::Serialize)]
            struct HealthUpdatePayload {
                statuses: std::collections::HashMap<String, HealthStatus>,
            }

            let map: std::collections::HashMap<String, HealthStatus> =
                updates.into_iter().collect();
            let _ = app_handle.emit("health-update", HealthUpdatePayload { statuses: map });
        }
    });
}

fn build_app_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let quit_item = MenuItem::with_id(app, "app-quit", "Quit Termina", true, Some("CommandOrControl+Q"))?;
    let hide_item = MenuItem::with_id(app, "app-hide", "Hide Termina", true, Some("CommandOrControl+H"))?;
    let app_submenu = Submenu::with_id(app, "app-menu", "Termina", true)?;
    app_submenu.append(&hide_item)?;
    app_submenu.append(&quit_item)?;

    let edit_submenu = Submenu::with_id(app, "edit-menu", "Edit", true)?;
    edit_submenu.append(&PredefinedMenuItem::undo(app, None)?)?;
    edit_submenu.append(&PredefinedMenuItem::redo(app, None)?)?;
    edit_submenu.append(&PredefinedMenuItem::separator(app)?)?;
    edit_submenu.append(&PredefinedMenuItem::cut(app, None)?)?;
    edit_submenu.append(&PredefinedMenuItem::copy(app, None)?)?;
    edit_submenu.append(&PredefinedMenuItem::paste(app, None)?)?;
    edit_submenu.append(&PredefinedMenuItem::select_all(app, None)?)?;

    Menu::with_items(app, &[&app_submenu, &edit_submenu])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Kill any orphaned processes from a previous crash
    cleanup_orphaned_processes();

    let state = AppState::new(config::load_commands(), config::load_shell_settings());

    // Auto-start enabled commands
    auto_start_enabled_commands(&state);

    // Persist PGIDs of auto-started processes
    if let Ok(processes) = state.processes.lock() {
        let pgids: Vec<i32> = processes.values().map(|p| p.pgid).collect();
        config::save_running_pids(&pgids);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::start_command,
            commands::stop_command,
            commands::restart_command,
            commands::get_commands,
            commands::get_status,
            commands::add_command,
            commands::update_command,
            commands::delete_command,
            commands::get_logs,
            commands::get_health,
            commands::get_running_commands,
            commands::kill_orphaned_processes,
            commands::kill_by_ports,
            commands::quit_app,
            commands::get_shell_settings,
            commands::update_shell_settings,
        ])
        .setup(|app| {
            // Set up custom app menu (replaces default macOS menu)
            let menu = build_app_menu(app.handle())?;
            app.set_menu(menu)?;

            tray::create_tray(app.handle())?;

            // Start background process monitor
            start_process_monitor(app.handle().clone());

            // Start health checker
            start_health_checker(app.handle().clone());

            // Register global shortcut (Cmd+Shift+T) to toggle window
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            let handle = app.handle().clone();
            app.global_shortcut().on_shortcut("CommandOrControl+Shift+T", move |_app, _shortcut, event| {
                if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    if let Some(window) = handle.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            })?;

            Ok(())
        })
        .on_menu_event(|app, event| {
            let id_str = event.id.as_ref();
            if id_str == "app-quit" {
                let _ = app.emit("confirm-quit", ());
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            } else if id_str == "app-hide" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                // Let programmatic exits (app.exit(0)) through
                if code.is_some() {
                    return;
                }
                // OS-initiated exit — show confirmation dialog
                api.prevent_exit();
                let _ = app_handle.emit("confirm-quit", ());
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}
