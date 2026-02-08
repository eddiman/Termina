use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Child;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandType {
    Process,
    OneTime,
}

impl Default for CommandType {
    fn default() -> Self {
        CommandType::Process
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub command: String,
    pub enabled: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub health_check_url: Option<String>,
    #[serde(default)]
    pub command_type: CommandType,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProcessStatus {
    Stopped,
    Running,
    Exited { code: Option<i32> },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessExitInfo {
    pub code: Option<i32>,
    pub command_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub text: String,
    pub stream: String, // "stdout" or "stderr"
    pub timestamp: u64,
}

pub struct LogBuffer {
    pub lines: std::collections::VecDeque<LogLine>,
    pub max_lines: usize,
}

impl LogBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: std::collections::VecDeque::new(),
            max_lines,
        }
    }

    pub fn push(&mut self, line: LogLine) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShellSettings {
    pub shell_path: Option<String>,
    pub init_script: Option<String>,
}

impl ShellSettings {
    pub fn effective_shell(&self) -> String {
        self.shell_path
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()))
    }

    pub fn effective_init_script(&self) -> &str {
        self.init_script
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("")
    }
}

#[derive(Debug)]
pub struct RunningProcess {
    pub pgid: i32,
    pub child: Child,
}

pub struct AppState {
    pub commands: Mutex<Vec<CommandEntry>>,
    pub processes: Mutex<HashMap<String, RunningProcess>>,
    pub exit_info: Mutex<HashMap<String, ProcessExitInfo>>,
    pub logs: std::sync::Arc<Mutex<HashMap<String, LogBuffer>>>,
    pub health: Mutex<HashMap<String, HealthStatus>>,
    pub shell_settings: Mutex<ShellSettings>,
}

impl AppState {
    pub fn new(commands: Vec<CommandEntry>, shell_settings: ShellSettings) -> Self {
        Self {
            commands: Mutex::new(commands),
            processes: Mutex::new(HashMap::new()),
            exit_info: Mutex::new(HashMap::new()),
            logs: std::sync::Arc::new(Mutex::new(HashMap::new())),
            health: Mutex::new(HashMap::new()),
            shell_settings: Mutex::new(shell_settings),
        }
    }
}
