use crate::types::CommandEntry;
use std::fs;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("termina");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn get_pids_path() -> PathBuf {
    config_dir().join("running_pids.json")
}

pub fn save_running_pids(pgids: &[i32]) {
    let path = get_pids_path();
    if pgids.is_empty() {
        let _ = fs::remove_file(&path);
    } else if let Ok(json) = serde_json::to_string(pgids) {
        let _ = fs::write(&path, json);
    }
}

pub fn load_running_pids() -> Vec<i32> {
    let path = get_pids_path();
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn clear_running_pids() {
    let _ = fs::remove_file(get_pids_path());
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Config {
    commands: Vec<CommandEntry>,
}

pub fn load_commands() -> Vec<CommandEntry> {
    let path = get_config_path();

    if !path.exists() {
        return Vec::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<Config>(&content)
            .map(|c| c.commands)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_commands(commands: &[CommandEntry]) -> Result<(), String> {
    let path = get_config_path();
    let config = Config {
        commands: commands.to_vec(),
    };

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}
