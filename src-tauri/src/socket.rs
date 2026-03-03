use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use crate::commands;
use crate::types::{AppState, CommandType};

// ── Protocol types ──

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum CliRequest {
    Ping,
    List,
    Start { name_or_id: String },
    Stop { name_or_id: String },
    Restart { name_or_id: String },
    Add {
        name: String,
        dir: String,
        command: String,
        #[serde(default)]
        command_type: Option<CommandType>,
        #[serde(default)]
        tags: Option<Vec<String>>,
    },
    Logs {
        name_or_id: String,
        #[serde(default)]
        tail: Option<usize>,
    },
    GetSettings,
    SetSettings {
        shell_path: Option<String>,
        init_script: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum CliResponse {
    Ok { data: Value },
    Error { message: String },
}

// ── Socket path ──

pub fn socket_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not determine config directory")
        .join("termina")
        .join("termina.sock")
}

// ── Command resolution ──

fn resolve_command_id(state: &AppState, name_or_id: &str) -> Result<String, String> {
    let commands = state.commands.lock().map_err(|e| e.to_string())?;

    // Try exact ID match first
    if commands.iter().any(|c| c.id == name_or_id) {
        return Ok(name_or_id.to_string());
    }

    // Case-insensitive name match
    let matches: Vec<&crate::types::CommandEntry> = commands
        .iter()
        .filter(|c| c.name.to_lowercase() == name_or_id.to_lowercase())
        .collect();

    match matches.len() {
        0 => Err(format!("No command found matching '{}'", name_or_id)),
        1 => Ok(matches[0].id.clone()),
        _ => {
            let names: Vec<String> = matches.iter().map(|c| format!("  {} ({})", c.name, c.id)).collect();
            Err(format!(
                "Ambiguous name '{}'. Matches:\n{}",
                name_or_id,
                names.join("\n")
            ))
        }
    }
}

// ── Server ──

pub fn start_socket_server(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let path = socket_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Remove stale socket
        let _ = std::fs::remove_file(&path);

        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to start socket server: {}", e);
                return;
            }
        };

        // Set socket permissions to 0o600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handle = app_handle.clone();
                    std::thread::spawn(move || {
                        handle_connection(stream, &handle);
                    });
                }
                Err(e) => {
                    // Socket was removed (app shutting down)
                    if e.kind() == std::io::ErrorKind::InvalidInput {
                        break;
                    }
                }
            }
        }
    });
}

fn handle_connection(stream: UnixStream, app_handle: &tauri::AppHandle) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(5)));

    let reader = BufReader::new(&stream);
    let mut writer = &stream;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: CliRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = CliResponse::Error {
                    message: format!("Invalid request: {}", e),
                };
                let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap());
                break;
            }
        };

        let response = handle_request(request, app_handle);
        let resp_json = serde_json::to_string(&response).unwrap();
        if writeln!(writer, "{}", resp_json).is_err() {
            break;
        }
        // One request per connection
        break;
    }
}

fn handle_request(request: CliRequest, app_handle: &tauri::AppHandle) -> CliResponse {
    use tauri::Manager;
    let state: tauri::State<AppState> = app_handle.state();

    match request {
        CliRequest::Ping => CliResponse::Ok {
            data: serde_json::json!("pong"),
        },

        CliRequest::List => {
            match commands::do_get_commands(&state) {
                Ok(cmds) => {
                    // Enrich with status
                    let mut entries = Vec::new();
                    for cmd in &cmds {
                        let status = commands::do_get_status(&cmd.id, &state)
                            .unwrap_or(crate::types::ProcessStatus::Stopped);
                        entries.push(serde_json::json!({
                            "id": cmd.id,
                            "name": cmd.name,
                            "command": cmd.command,
                            "cwd": cmd.cwd,
                            "type": cmd.command_type,
                            "tags": cmd.tags,
                            "enabled": cmd.enabled,
                            "status": status,
                        }));
                    }
                    CliResponse::Ok {
                        data: serde_json::json!(entries),
                    }
                }
                Err(e) => CliResponse::Error { message: e },
            }
        }

        CliRequest::Start { name_or_id } => {
            match resolve_command_id(&state, &name_or_id) {
                Ok(id) => match commands::do_start_command(&id, &state, Some(app_handle)) {
                    Ok(()) => {
                        emit_commands_changed(app_handle);
                        CliResponse::Ok {
                            data: serde_json::json!("started"),
                        }
                    }
                    Err(e) => CliResponse::Error { message: e },
                },
                Err(e) => CliResponse::Error { message: e },
            }
        }

        CliRequest::Stop { name_or_id } => {
            match resolve_command_id(&state, &name_or_id) {
                Ok(id) => match commands::do_stop_command(&id, &state, Some(app_handle)) {
                    Ok(()) => {
                        emit_commands_changed(app_handle);
                        CliResponse::Ok {
                            data: serde_json::json!("stopped"),
                        }
                    }
                    Err(e) => CliResponse::Error { message: e },
                },
                Err(e) => CliResponse::Error { message: e },
            }
        }

        CliRequest::Restart { name_or_id } => {
            match resolve_command_id(&state, &name_or_id) {
                Ok(id) => match commands::do_restart_command(&id, &state, Some(app_handle)) {
                    Ok(()) => {
                        emit_commands_changed(app_handle);
                        CliResponse::Ok {
                            data: serde_json::json!("restarted"),
                        }
                    }
                    Err(e) => CliResponse::Error { message: e },
                },
                Err(e) => CliResponse::Error { message: e },
            }
        }

        CliRequest::Add {
            name,
            dir,
            command,
            command_type,
            tags,
        } => match commands::do_add_command(name, dir, command, command_type, tags, &state) {
            Ok(entry) => {
                emit_commands_changed(app_handle);
                CliResponse::Ok {
                    data: serde_json::to_value(entry).unwrap(),
                }
            }
            Err(e) => CliResponse::Error { message: e },
        },

        CliRequest::Logs { name_or_id, tail } => {
            match resolve_command_id(&state, &name_or_id) {
                Ok(id) => match commands::do_get_logs(&id, &state) {
                    Ok(logs) => {
                        let tail_n = tail.unwrap_or(50);
                        let logs: Vec<_> = if logs.len() > tail_n {
                            logs[logs.len() - tail_n..].to_vec()
                        } else {
                            logs
                        };
                        CliResponse::Ok {
                            data: serde_json::to_value(logs).unwrap(),
                        }
                    }
                    Err(e) => CliResponse::Error { message: e },
                },
                Err(e) => CliResponse::Error { message: e },
            }
        }

        CliRequest::GetSettings => match commands::do_get_shell_settings(&state) {
            Ok(settings) => CliResponse::Ok {
                data: serde_json::to_value(settings).unwrap(),
            },
            Err(e) => CliResponse::Error { message: e },
        },

        CliRequest::SetSettings {
            shell_path,
            init_script,
        } => match commands::do_update_shell_settings(shell_path, init_script, &state) {
            Ok(()) => {
                emit_commands_changed(app_handle);
                CliResponse::Ok {
                    data: serde_json::json!("settings updated"),
                }
            }
            Err(e) => CliResponse::Error { message: e },
        },
    }
}

fn emit_commands_changed(app_handle: &tauri::AppHandle) {
    use tauri::Emitter;
    let _ = app_handle.emit("commands-changed", ());
}

// ── Client (used by CLI binary) ──

pub fn send_cli_request(request: &CliRequest) -> Result<CliResponse, String> {
    let path = socket_path();
    if !path.exists() {
        return Err("Termina app is not running (socket not found)".to_string());
    }

    let stream = UnixStream::connect(&path)
        .map_err(|e| format!("Failed to connect to Termina app: {}", e))?;

    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let mut writer = &stream;
    let request_json = serde_json::to_string(request).map_err(|e| e.to_string())?;
    writeln!(writer, "{}", request_json).map_err(|e| format!("Failed to send request: {}", e))?;

    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read response: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        let response: CliResponse =
            serde_json::from_str(&line).map_err(|e| format!("Invalid response: {}", e))?;
        return Ok(response);
    }

    Err("No response from Termina app".to_string())
}
