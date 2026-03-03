use crate::config;
use crate::process_manager::{check_process_status, kill_process_group, spawn_command, spawn_log_reader, ProcessCheckResult};
use crate::tray;
use crate::types::{AppState, CommandEntry, CommandType, HealthStatus, LogBuffer, LogLine, ProcessStatus, RunningProcess, ShellSettings};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

/// Persist current PGIDs to disk so orphans can be cleaned up if the app crashes.
fn sync_pid_file(state: &AppState) {
    if let Ok(processes) = state.processes.lock() {
        let pgids: Vec<i32> = processes.values().map(|p| p.pgid).collect();
        config::save_running_pids(&pgids);
    }
}

// ── Core logic functions (used by both Tauri IPC and socket handler) ──

pub fn do_start_command(id: &str, state: &AppState, app_handle: Option<&tauri::AppHandle>) -> Result<(), String> {
    let (cmd_entry, env) = {
        let commands = state.commands.lock().map_err(|e| e.to_string())?;
        let cmd = commands.iter().find(|c| c.id == id).cloned();
        match cmd {
            Some(c) => (c.clone(), c.env.clone()),
            None => return Err("Command not found".to_string()),
        }
    };

    // Check if already running
    {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        if let Some(proc) = processes.get_mut(id) {
            if matches!(check_process_status(&mut proc.child), ProcessCheckResult::Running) {
                return Err("Command is already running".to_string());
            }
            processes.remove(id);
        }
    }

    // Clear exit info
    if let Ok(mut exit_info) = state.exit_info.lock() {
        exit_info.remove(id);
    }

    // Init log buffer
    if let Ok(mut logs) = state.logs.lock() {
        logs.insert(id.to_string(), LogBuffer::new(500));
    }

    // Read shell settings
    let (shell_path, init_script) = {
        let settings = state.shell_settings.lock().map_err(|e| e.to_string())?;
        (settings.effective_shell(), settings.effective_init_script().to_string())
    };

    // Spawn new process
    let mut spawned = spawn_command(&cmd_entry.cwd, &cmd_entry.command, &env, &shell_path, &init_script)?;

    // Start log readers
    spawn_log_reader(id.to_string(), &mut spawned.child, state.logs.clone());

    {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        processes.insert(
            id.to_string(),
            RunningProcess {
                pgid: spawned.pgid,
                child: spawned.child,
            },
        );
    }

    sync_pid_file(state);
    if let Some(app) = app_handle {
        tray::update_tray_menu(app);
    }

    Ok(())
}

pub fn do_stop_command(id: &str, state: &AppState, app_handle: Option<&tauri::AppHandle>) -> Result<(), String> {
    {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;

        if let Some(proc) = processes.get(id) {
            kill_process_group(proc.pgid)?;
        }
        processes.remove(id);
    }

    // Clear exit info on manual stop
    if let Ok(mut exit_info) = state.exit_info.lock() {
        exit_info.remove(id);
    }

    sync_pid_file(state);
    if let Some(app) = app_handle {
        tray::update_tray_menu(app);
    }

    Ok(())
}

pub fn do_restart_command(id: &str, state: &AppState, app_handle: Option<&tauri::AppHandle>) -> Result<(), String> {
    // Stop first
    {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        if let Some(proc) = processes.remove(id) {
            let _ = kill_process_group(proc.pgid);
        }
    }

    // Clear exit info
    if let Ok(mut exit_info) = state.exit_info.lock() {
        exit_info.remove(id);
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    // Start again
    let (cmd_entry, env) = {
        let commands = state.commands.lock().map_err(|e| e.to_string())?;
        let cmd = commands.iter().find(|c| c.id == id).cloned();
        match cmd {
            Some(c) => (c.clone(), c.env.clone()),
            None => return Err("Command not found".to_string()),
        }
    };

    // Init log buffer
    if let Ok(mut logs) = state.logs.lock() {
        logs.insert(id.to_string(), LogBuffer::new(500));
    }

    // Read shell settings
    let (shell_path, init_script) = {
        let settings = state.shell_settings.lock().map_err(|e| e.to_string())?;
        (settings.effective_shell(), settings.effective_init_script().to_string())
    };

    let mut spawned = spawn_command(&cmd_entry.cwd, &cmd_entry.command, &env, &shell_path, &init_script)?;

    // Start log readers
    spawn_log_reader(id.to_string(), &mut spawned.child, state.logs.clone());

    {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        processes.insert(
            id.to_string(),
            RunningProcess {
                pgid: spawned.pgid,
                child: spawned.child,
            },
        );
    }

    sync_pid_file(state);
    if let Some(app) = app_handle {
        tray::update_tray_menu(app);
    }

    Ok(())
}

pub fn do_get_commands(state: &AppState) -> Result<Vec<CommandEntry>, String> {
    let commands = state.commands.lock().map_err(|e| e.to_string())?;
    Ok(commands.clone())
}

pub fn do_get_status(id: &str, state: &AppState) -> Result<ProcessStatus, String> {
    let mut processes = state.processes.lock().map_err(|e| e.to_string())?;

    if let Some(proc) = processes.get_mut(id) {
        match check_process_status(&mut proc.child) {
            ProcessCheckResult::Running => return Ok(ProcessStatus::Running),
            ProcessCheckResult::Exited(code) => {
                if let Ok(commands) = state.commands.lock() {
                    let name = commands
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    if let Ok(mut exit_info) = state.exit_info.lock() {
                        exit_info.insert(
                            id.to_string(),
                            crate::types::ProcessExitInfo {
                                code,
                                command_name: name,
                            },
                        );
                    }
                }
                processes.remove(id);
                return Ok(ProcessStatus::Exited { code });
            }
        }
    }

    if let Ok(exit_info) = state.exit_info.lock() {
        if let Some(info) = exit_info.get(id) {
            return Ok(ProcessStatus::Exited { code: info.code });
        }
    }

    Ok(ProcessStatus::Stopped)
}

pub fn do_get_logs(id: &str, state: &AppState) -> Result<Vec<LogLine>, String> {
    let logs = state.logs.lock().map_err(|e| e.to_string())?;
    if let Some(buf) = logs.get(id) {
        Ok(buf.lines.iter().cloned().collect())
    } else {
        Ok(Vec::new())
    }
}

pub fn do_add_command(
    name: String,
    cwd: String,
    command: String,
    command_type: Option<CommandType>,
    tags: Option<Vec<String>>,
    state: &AppState,
) -> Result<CommandEntry, String> {
    let entry = CommandEntry {
        id: Uuid::new_v4().to_string(),
        name,
        cwd,
        command,
        enabled: false,
        env: HashMap::new(),
        health_check_url: None,
        command_type: command_type.unwrap_or_default(),
        tags: tags.unwrap_or_default(),
    };

    let mut commands = state.commands.lock().map_err(|e| e.to_string())?;
    commands.push(entry.clone());
    config::save_commands(&commands)?;

    Ok(entry)
}

pub fn do_get_shell_settings(state: &AppState) -> Result<ShellSettings, String> {
    let settings = state.shell_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

pub fn do_update_shell_settings(
    shell_path: Option<String>,
    init_script: Option<String>,
    state: &AppState,
) -> Result<(), String> {
    let new_settings = ShellSettings {
        shell_path,
        init_script,
    };
    config::save_shell_settings(&new_settings)?;
    let mut settings = state.shell_settings.lock().map_err(|e| e.to_string())?;
    *settings = new_settings;
    Ok(())
}

// ── Tauri command wrappers (thin wrappers around do_* functions) ──

#[tauri::command]
pub async fn start_command(id: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    do_start_command(&id, &state, Some(&app))
}

#[tauri::command]
pub async fn stop_command(id: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    do_stop_command(&id, &state, Some(&app))
}

#[tauri::command]
pub async fn restart_command(id: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    do_restart_command(&id, &state, Some(&app))
}

#[tauri::command]
pub fn get_shell_settings(state: State<'_, AppState>) -> Result<ShellSettings, String> {
    do_get_shell_settings(&state)
}

#[tauri::command]
pub fn update_shell_settings(
    shell_path: Option<String>,
    init_script: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    do_update_shell_settings(shell_path, init_script, &state)
}

#[tauri::command]
pub fn get_commands(state: State<'_, AppState>) -> Result<Vec<CommandEntry>, String> {
    do_get_commands(&state)
}

#[tauri::command]
pub fn get_status(id: String, state: State<'_, AppState>) -> Result<ProcessStatus, String> {
    do_get_status(&id, &state)
}

#[tauri::command]
pub fn get_logs(id: String, state: State<'_, AppState>) -> Result<Vec<LogLine>, String> {
    do_get_logs(&id, &state)
}

#[tauri::command]
pub fn get_health(id: String, state: State<'_, AppState>) -> Result<HealthStatus, String> {
    let health = state.health.lock().map_err(|e| e.to_string())?;
    Ok(health.get(&id).cloned().unwrap_or(HealthStatus::Unknown))
}

#[tauri::command]
pub fn add_command(
    name: String,
    cwd: String,
    command: String,
    env: Option<HashMap<String, String>>,
    health_check_url: Option<String>,
    command_type: Option<CommandType>,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<CommandEntry, String> {
    let entry = CommandEntry {
        id: Uuid::new_v4().to_string(),
        name,
        cwd,
        command,
        enabled: false,
        env: env.unwrap_or_default(),
        health_check_url,
        command_type: command_type.unwrap_or_default(),
        tags: tags.unwrap_or_default(),
    };

    let mut commands = state.commands.lock().map_err(|e| e.to_string())?;
    commands.push(entry.clone());

    config::save_commands(&commands)?;

    Ok(entry)
}

#[tauri::command]
pub fn update_command(
    id: String,
    name: String,
    cwd: String,
    command: String,
    enabled: bool,
    env: Option<HashMap<String, String>>,
    health_check_url: Option<String>,
    command_type: Option<CommandType>,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut commands = state.commands.lock().map_err(|e| e.to_string())?;

    if let Some(cmd) = commands.iter_mut().find(|c| c.id == id) {
        cmd.name = name;
        cmd.cwd = cwd;
        cmd.command = command;
        cmd.enabled = enabled;
        cmd.env = env.unwrap_or_default();
        cmd.health_check_url = health_check_url;
        cmd.command_type = command_type.unwrap_or_default();
        cmd.tags = tags.unwrap_or_default();

        config::save_commands(&commands)?;
        Ok(())
    } else {
        Err("Command not found".to_string())
    }
}

#[tauri::command]
pub fn delete_command(id: String, state: State<'_, AppState>) -> Result<(), String> {
    // Stop process if running
    {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        if let Some(proc) = processes.remove(&id) {
            let _ = kill_process_group(proc.pgid);
        }
    }

    sync_pid_file(&state);

    // Clean up exit info, logs, health
    if let Ok(mut exit_info) = state.exit_info.lock() {
        exit_info.remove(&id);
    }
    if let Ok(mut logs) = state.logs.lock() {
        logs.remove(&id);
    }
    if let Ok(mut health) = state.health.lock() {
        health.remove(&id);
    }

    let mut commands = state.commands.lock().map_err(|e| e.to_string())?;
    commands.retain(|c| c.id != id);

    config::save_commands(&commands)?;

    Ok(())
}

#[tauri::command]
pub fn get_running_commands(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let processes = state.processes.lock().map_err(|e| e.to_string())?;
    let commands = state.commands.lock().map_err(|e| e.to_string())?;

    let names: Vec<String> = processes
        .keys()
        .filter_map(|id| {
            commands.iter().find(|c| &c.id == id).map(|c| c.name.clone())
        })
        .collect();

    Ok(names)
}

#[tauri::command]
pub fn kill_orphaned_processes() -> Result<u32, String> {
    let pgids = config::load_running_pids();
    if pgids.is_empty() {
        return Ok(0);
    }
    let mut killed = 0u32;
    for pgid in &pgids {
        if nix::sys::signal::killpg(nix::unistd::Pid::from_raw(*pgid), None).is_ok() {
            let is_termina_process = std::process::Command::new("ps")
                .args(["-p", &pgid.to_string(), "-o", "command="])
                .output()
                .ok()
                .map(|out| {
                    let cmd = String::from_utf8_lossy(&out.stdout);
                    cmd.trim().starts_with("sh -c")
                })
                .unwrap_or(false);

            if is_termina_process {
                let _ = kill_process_group(*pgid);
                killed += 1;
            }
        }
    }
    config::clear_running_pids();
    Ok(killed)
}

/// Parse a port spec like "3000", "3000-3005", "3000,3001,8080", or "3000-3005,8080"
fn parse_ports(spec: &str) -> Result<Vec<u16>, String> {
    let mut ports = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let s: u16 = start.trim().parse().map_err(|_| format!("Invalid port: {}", start.trim()))?;
            let e: u16 = end.trim().parse().map_err(|_| format!("Invalid port: {}", end.trim()))?;
            if s > e {
                return Err(format!("Invalid range: {}-{}", s, e));
            }
            ports.extend(s..=e);
        } else {
            let p: u16 = part.parse().map_err(|_| format!("Invalid port: {}", part))?;
            ports.push(p);
        }
    }
    Ok(ports)
}

#[tauri::command]
pub fn kill_by_ports(ports: String, state: State<'_, AppState>) -> Result<u32, String> {
    let port_list = parse_ports(&ports)?;
    if port_list.is_empty() {
        return Ok(0);
    }

    let known_pgids: std::collections::HashSet<i32> = if let Ok(processes) = state.processes.lock() {
        processes.values().map(|p| p.pgid).collect()
    } else {
        std::collections::HashSet::new()
    };

    let mut killed_pids = std::collections::HashSet::new();

    for port in &port_list {
        let output = std::process::Command::new("lsof")
            .args(["-i", &format!(":{}", port), "-t"])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.trim().parse::<i32>() {
                    if pid <= 0 || !killed_pids.insert(pid) {
                        continue;
                    }
                    let pgid = nix::unistd::getpgid(Some(nix::unistd::Pid::from_raw(pid)));
                    let belongs_to_termina = match pgid {
                        Ok(pg) => known_pgids.contains(&pg.as_raw()),
                        Err(_) => false,
                    };
                    if belongs_to_termina {
                        let _ = nix::sys::signal::kill(
                            nix::unistd::Pid::from_raw(pid),
                            nix::sys::signal::Signal::SIGTERM,
                        );
                    } else {
                        killed_pids.remove(&pid);
                    }
                }
            }
        }
    }

    if !killed_pids.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(300));
        for pid in &killed_pids {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(*pid),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
    }

    Ok(killed_pids.len() as u32)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Only https:// URLs are allowed".to_string());
    }
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("Failed to open URL: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(mut processes) = state.processes.lock() {
        for (_, proc) in processes.drain() {
            let _ = kill_process_group(proc.pgid);
        }
    }
    config::clear_running_pids();
    let _ = std::fs::remove_file(crate::socket::socket_path());
    app.exit(0);
    Ok(())
}
