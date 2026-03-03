use clap::{Parser, Subcommand};

use crate::socket::{send_cli_request, CliRequest, CliResponse};
use crate::types::CommandType;

#[derive(Parser)]
#[command(name = "termina", about = "Manage Termina commands from the terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// List all commands with status
    List,

    /// Start a command by name or ID
    Start {
        /// Command name or ID
        name: String,
    },

    /// Stop a running command
    Stop {
        /// Command name or ID
        name: String,
    },

    /// Restart a command
    Restart {
        /// Command name or ID
        name: String,
    },

    /// Add a new command
    Add {
        /// Command name
        name: String,

        /// Working directory
        #[arg(long)]
        dir: String,

        /// Shell command to run
        #[arg(long)]
        cmd: String,

        /// Command type: process or onetime
        #[arg(long, value_parser = parse_command_type, default_value = "process")]
        r#type: CommandType,

        /// Tags (can be specified multiple times)
        #[arg(long)]
        tag: Vec<String>,
    },

    /// Show recent logs for a command
    Logs {
        /// Command name or ID
        name: String,

        /// Number of lines to show
        #[arg(short, default_value = "50")]
        n: usize,
    },

    /// Show or update settings
    Settings {
        /// Set shell path
        #[arg(long)]
        shell: Option<String>,

        /// Set init script
        #[arg(long)]
        init_script: Option<String>,
    },
}

fn parse_command_type(s: &str) -> Result<CommandType, String> {
    match s.to_lowercase().as_str() {
        "process" => Ok(CommandType::Process),
        "onetime" | "one-time" | "one_time" => Ok(CommandType::OneTime),
        _ => Err(format!("Invalid type '{}'. Use 'process' or 'onetime'", s)),
    }
}

pub fn run_cli(cli: Cli) -> i32 {
    let request = match &cli.command {
        CliCommand::List => CliRequest::List,
        CliCommand::Start { name } => CliRequest::Start {
            name_or_id: name.clone(),
        },
        CliCommand::Stop { name } => CliRequest::Stop {
            name_or_id: name.clone(),
        },
        CliCommand::Restart { name } => CliRequest::Restart {
            name_or_id: name.clone(),
        },
        CliCommand::Add {
            name,
            dir,
            cmd,
            r#type,
            tag,
        } => CliRequest::Add {
            name: name.clone(),
            dir: dir.clone(),
            command: cmd.clone(),
            command_type: Some(r#type.clone()),
            tags: if tag.is_empty() { None } else { Some(tag.clone()) },
        },
        CliCommand::Logs { name, n } => CliRequest::Logs {
            name_or_id: name.clone(),
            tail: Some(*n),
        },
        CliCommand::Settings { shell, init_script } => {
            if shell.is_none() && init_script.is_none() {
                CliRequest::GetSettings
            } else {
                CliRequest::SetSettings {
                    shell_path: shell.clone(),
                    init_script: init_script.clone(),
                }
            }
        }
    };

    match send_cli_request(&request) {
        Ok(response) => match response {
            CliResponse::Ok { data } => {
                format_output(&cli.command, &data);
                0
            }
            CliResponse::Error { message } => {
                eprintln!("Error: {}", message);
                1
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn format_output(command: &CliCommand, data: &serde_json::Value) {
    match command {
        CliCommand::List => format_list(data),
        CliCommand::Logs { .. } => format_logs(data),
        CliCommand::Settings { shell, init_script } if shell.is_none() && init_script.is_none() => {
            format_settings(data)
        }
        CliCommand::Add { .. } => {
            if let Some(name) = data.get("name").and_then(|n| n.as_str()) {
                println!("Added command '{}'", name);
            } else {
                println!("Command added");
            }
        }
        _ => {
            if let Some(s) = data.as_str() {
                println!("{}", capitalize(s));
            }
        }
    }
}

fn format_list(data: &serde_json::Value) {
    let entries = match data.as_array() {
        Some(arr) => arr,
        None => {
            println!("No commands");
            return;
        }
    };

    if entries.is_empty() {
        println!("No commands configured");
        return;
    }

    // Calculate column widths
    let mut max_name = 4; // "NAME"
    let mut max_status = 6; // "STATUS"
    let mut max_type = 4; // "TYPE"

    for entry in entries {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let status = format_status_str(entry.get("status"));
        let cmd_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Process");

        max_name = max_name.max(name.len());
        max_status = max_status.max(status.len());
        max_type = max_type.max(cmd_type.len());
    }

    // Header
    println!(
        "{:<name_w$}  {:<status_w$}  {:<type_w$}  COMMAND",
        "NAME",
        "STATUS",
        "TYPE",
        name_w = max_name,
        status_w = max_status,
        type_w = max_type,
    );

    for entry in entries {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let status = format_status_str(entry.get("status"));
        let cmd_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Process");
        let command = entry
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        println!(
            "{:<name_w$}  {:<status_w$}  {:<type_w$}  {}",
            name,
            status,
            cmd_type,
            command,
            name_w = max_name,
            status_w = max_status,
            type_w = max_type,
        );
    }
}

fn format_status_str(status: Option<&serde_json::Value>) -> String {
    match status {
        Some(s) => match s.get("type").and_then(|t| t.as_str()) {
            Some("Running") => "Running".to_string(),
            Some("Stopped") => "Stopped".to_string(),
            Some("Exited") => {
                let code = s.get("code").and_then(|c| c.as_i64());
                match code {
                    Some(c) => format!("Exited({})", c),
                    None => "Exited".to_string(),
                }
            }
            Some("Error") => "Error".to_string(),
            _ => "Unknown".to_string(),
        },
        None => "Unknown".to_string(),
    }
}

fn format_logs(data: &serde_json::Value) {
    let logs = match data.as_array() {
        Some(arr) => arr,
        None => return,
    };

    if logs.is_empty() {
        println!("No logs");
        return;
    }

    for log in logs {
        let text = log.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let stream = log.get("stream").and_then(|v| v.as_str()).unwrap_or("stdout");
        let timestamp = log.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);

        let time_str = format_timestamp(timestamp);

        if stream == "stderr" {
            eprintln!("{} [err] {}", time_str, text);
        } else {
            println!("{} {}", time_str, text);
        }
    }
}

fn format_timestamp(ms: u64) -> String {
    let secs = ms / 1000;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, s)
}

fn format_settings(data: &serde_json::Value) {
    let shell = data
        .get("shell_path")
        .and_then(|v| v.as_str())
        .unwrap_or("(default)");
    let init = data
        .get("init_script")
        .and_then(|v| v.as_str())
        .unwrap_or("(none)");

    println!("Shell:       {}", if shell.is_empty() { "(default)" } else { shell });
    println!("Init script: {}", if init.is_empty() { "(none)" } else { init });
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
