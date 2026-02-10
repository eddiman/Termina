# Security Audit Report

**Date:** 2026-02-10
**Scope:** Termina — Tauri 2 macOS desktop app for managing long-running shell commands

---

## Summary

9 security findings were identified and fixed across 4 files. The attack surface is primarily local, but several patterns could cause unintended damage or be exploited if the frontend were ever compromised (e.g., via a malicious dependency or XSS).

| # | Severity | Finding | File | Status |
|---|----------|---------|------|--------|
| 1 | HIGH | `open_url` accepts arbitrary input | `commands.rs` | Fixed |
| 2 | HIGH | `kill_by_ports` can terminate any system process | `commands.rs` | Fixed |
| 3 | HIGH | CSP is disabled | `tauri.conf.json` | Fixed |
| 4 | MEDIUM | Orphan cleanup PID reuse risk (TOCTOU) | `commands.rs`, `lib.rs` | Fixed |
| 5 | MEDIUM | Health check URL allows SSRF | `lib.rs` | Fixed |
| 6 | MEDIUM | Config files have no explicit permissions | `config.rs` | Fixed |
| 7 | MEDIUM | No health check timeout | `lib.rs` | Fixed |
| 8 | LOW | `config_dir()` falls back to current directory | `config.rs` | Fixed |
| 9 | LOW | Shell path fallback hardcodes `/bin/zsh` | `lib.rs` | Fixed |

---

## HIGH Severity

### 1. `open_url` accepts arbitrary input

The macOS `open` command will open any path or URL scheme — `file:///etc/passwd`, `ssh://`, arbitrary apps. While only called with one hardcoded GitHub URL from the frontend, nothing prevented the IPC from being called with anything else. If the frontend were ever compromised, this becomes an arbitrary file/app opener.

**Fix:** Validate that `url` starts with `https://` before passing to `open`.

### 2. `kill_by_ports` can terminate any system process

Used `lsof` to find PIDs on user-supplied ports, then sent SIGTERM + SIGKILL to every PID found. No check that the process belonged to Termina or was spawned by it. A compromised frontend could kill SSH servers, databases, or system daemons.

**Fix:** Scope kills to only processes belonging to known Termina process groups by checking each PID's process group ID against tracked PGIDs via `getpgid`.

### 3. CSP is disabled

Content Security Policy was explicitly set to `null`. If any XSS vector existed in the frontend, there would be zero mitigation. For a Tauri app that executes shell commands via IPC, XSS would mean arbitrary command execution.

**Fix:** Set a restrictive CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: https://asset.localhost`.

---

## MEDIUM Severity

### 4. Orphan cleanup TOCTOU (PID reuse)

`kill_orphaned_processes()` and `cleanup_orphaned_processes()` loaded PGIDs from disk and killed them. Between an app crash and restart, the OS could reuse those PIDs for unrelated processes. The signal-0 check only confirmed the PID exists, not that it's the same process.

**Fix:** Before killing, verify the process command line starts with `sh -c` (Termina's spawn pattern) using `ps -p <pid> -o command=`.

### 5. Health check URL allows SSRF

The `health_check_url` field accepted any string and `ureq::get(url).call()` would make HTTP requests to it — including internal services, cloud metadata endpoints (`169.254.169.254`), or non-HTTP protocols.

**Fix:** Validate that health check URLs use `http://` or `https://` scheme before making the request.

### 6. Config files have no explicit permissions

`fs::write()` created files with the default umask. Config contains command definitions, environment variables (which may include secrets/tokens), and shell init scripts. Other local users could read these.

**Fix:** Set file permissions to `0600` after writing using a `write_private()` helper with `std::os::unix::fs::PermissionsExt`.

### 7. No health check timeout

`ureq::get(url).call()` had no timeout configured. A hanging endpoint would block the health checker thread indefinitely, preventing health checks for all other commands.

**Fix:** Create a `ureq::AgentBuilder` with a 5-second timeout, reused across all health check requests.

---

## LOW Severity

### 8. `config_dir()` falls back to current directory

If `dirs::config_dir()` returned `None`, config would be written to whatever the CWD happened to be. This could place sensitive files in unexpected locations.

**Fix:** Use `expect()` with a clear error message instead of silently falling back to `"."`.

### 9. Shell path fallback hardcodes `/bin/zsh`

Not a real issue on macOS where zsh is always available, but `/bin/sh` is a more portable POSIX fallback.

**Fix:** Changed fallback from `/bin/zsh` to `/bin/sh`.

---

## Dismissed findings

The following were evaluated and determined to **not** be vulnerabilities:

- **Command injection via `sh -c`** — By design. Termina is a command runner; the user types shell commands and they get executed.
- **Shell settings (init_script, shell_path)** — The user configures their own shell. Equivalent to editing `.zshrc`.
- **XSS in log viewer / error messages** — Preact escapes text content by default. No `dangerouslySetInnerHTML` exists in the codebase.
- **Lock poisoning / TOCTOU in start_command** — Theoretical race requiring two simultaneous IPC calls for the same command ID. Frontend serializes these calls.
