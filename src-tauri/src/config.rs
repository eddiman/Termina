use crate::types::{CommandEntry, ShellSettings};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .expect("Could not determine system config directory")
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

fn write_private(path: &PathBuf, content: &str) -> Result<(), std::io::Error> {
    fs::write(path, content)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

pub fn save_running_pids(pgids: &[i32]) {
    let path = get_pids_path();
    if pgids.is_empty() {
        let _ = fs::remove_file(&path);
    } else if let Ok(json) = serde_json::to_string(pgids) {
        let _ = write_private(&path, &json);
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
    #[serde(default)]
    shell_settings: ShellSettings,
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
    // Load existing config to preserve shell_settings
    let existing = load_config();
    let config = Config {
        commands: commands.to_vec(),
        shell_settings: existing.shell_settings,
    };

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    write_private(&path, &json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

fn load_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        return Config::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn load_shell_settings() -> ShellSettings {
    load_config().shell_settings
}

pub fn save_shell_settings(settings: &ShellSettings) -> Result<(), String> {
    let path = get_config_path();
    let mut config = load_config();
    config.shell_settings = settings.clone();

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    write_private(&path, &json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}
