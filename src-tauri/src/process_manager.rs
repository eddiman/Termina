use crate::types::{LogBuffer, LogLine};
use nix::sys::signal::{killpg, Signal};
use nix::unistd::{setpgid, Pid};
use std::collections::HashMap;
use std::io::BufRead;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct SpawnedProcess {
    pub child: Child,
    pub pgid: i32,
}

pub fn spawn_command(
    cwd: &str,
    cmd: &str,
    env: &HashMap<String, String>,
    shell_path: &str,
    init_script: &str,
) -> Result<SpawnedProcess, String> {
    let full_cmd = if init_script.is_empty() {
        cmd.to_string()
    } else {
        format!("{}; {}", init_script, cmd)
    };
    let mut command = Command::new(shell_path);
    command
        .args(["-l", "-c", &full_cmd])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Apply custom environment variables (inherits parent env by default)
    for (k, v) in env {
        command.env(k, v);
    }

    // CRITICAL: Put child in its own process group for clean termination
    unsafe {
        command.pre_exec(|| {
            setpgid(Pid::from_raw(0), Pid::from_raw(0))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            Ok(())
        });
    }

    let child = command.spawn().map_err(|e| e.to_string())?;
    let pgid = child.id() as i32;

    Ok(SpawnedProcess { child, pgid })
}

pub fn kill_process_group(pgid: i32) -> Result<(), String> {
    let pid = Pid::from_raw(pgid);

    // Try SIGTERM first
    let _ = killpg(pid, Signal::SIGTERM);

    // Give processes a moment to exit gracefully
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Force kill with SIGKILL if still alive
    let _ = killpg(pid, Signal::SIGKILL);

    Ok(())
}

pub enum ProcessCheckResult {
    Running,
    Exited(Option<i32>),
}

pub fn check_process_status(child: &mut Child) -> ProcessCheckResult {
    match child.try_wait() {
        Ok(None) => ProcessCheckResult::Running,
        Ok(Some(status)) => ProcessCheckResult::Exited(status.code()),
        Err(_) => ProcessCheckResult::Exited(None),
    }
}

pub fn spawn_log_reader(
    id: String,
    child: &mut Child,
    logs: Arc<Mutex<HashMap<String, LogBuffer>>>,
) {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if let Some(stdout) = stdout {
        let id_clone = id.clone();
        let logs_clone = logs.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(text) = line {
                    let log_line = LogLine {
                        text,
                        stream: "stdout".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    };
                    if let Ok(mut logs) = logs_clone.lock() {
                        if let Some(buf) = logs.get_mut(&id_clone) {
                            buf.push(log_line);
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    if let Some(stderr) = stderr {
        let id_clone = id;
        let logs_clone = logs;
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(text) = line {
                    let log_line = LogLine {
                        text,
                        stream: "stderr".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    };
                    if let Ok(mut logs) = logs_clone.lock() {
                        if let Some(buf) = logs.get_mut(&id_clone) {
                            buf.push(log_line);
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }
}
