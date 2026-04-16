# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

NEVER accredit or write "written by claude", or "co-authored" in any commits or issues or anything alike.

## Build & Development Commands

```bash
# Development (starts both Vite dev server on :1420 and Tauri app)
npm run tauri dev

# Production build
npm run tauri build

# Frontend only (no Tauri shell)
npm run dev          # Vite dev server
npm run build        # tsc && vite build

# Rust only (type-check without full build)
cd src-tauri && cargo check
```

There are no tests or linting configured in this project.

## Architecture

Tauri 2 desktop app (macOS) with a Preact frontend and Rust backend. Manages long-running shell commands with process lifecycle, health checks, and system tray integration.

### Backend (Rust) — `src-tauri/src/`

- **`lib.rs`** — App setup, plugin registration, background threads (process monitor every 3s, health checker every 15s), window lifecycle, orphaned process cleanup on startup
- **`commands.rs`** — All `#[tauri::command]` handlers (start/stop/restart, CRUD, logs, health, quit). This is the IPC surface between frontend and backend
- **`process_manager.rs`** — Process spawning via `sh -c "exec <cmd>"`, process group creation (`setpgid`), termination via `killpg` (SIGTERM → wait 200ms → SIGKILL), log reader thread spawning
- **`types.rs`** — Shared types: `Command`, `ProcessStatus`, `HealthStatus`, `LogLine`, `LogBuffer` (ring buffer, max 500 lines)
- **`config.rs`** — JSON persistence to `~/.config/termina/config.json` and `running_pids.json`
- **`tray.rs`** — System tray menu with dynamic "Active Commands" submenu

### Frontend (Preact + TypeScript) — `src/`

- **`lib/api.ts`** — Centralized `api` object wrapping all `invoke()` calls to Rust
- **`lib/store.ts`** — Preact Signals for reactive state (`commands`, `statuses`, `healthStatuses`, etc.) and computed values (`runningCount`, `sortedCommands`)
- **`app.tsx`** — Root component, sets up Tauri event listeners (`process-exited`, `health-update`, `open-command`, `confirm-quit`, `send-notification`)
- **`components/`** — CommandCard, CommandDialog, CommandForm, CommandList, ConfirmDialog, FilterBar, LogViewer, SettingsDialog

### Communication

- **Frontend → Backend**: `invoke()` RPC calls (defined in `api.ts`, handled in `commands.rs`)
- **Backend → Frontend**: Tauri events via `app_handle.emit()` — requires `use tauri::Emitter`

## Critical Patterns

### ProcessStatus is an internally-tagged enum
Rust uses `#[serde(tag = "type")]`, so it serializes as `{ type: 'Running' }`, `{ type: 'Exited', code: 0 }`, etc. — **never** plain string literals.

### Process groups
Every spawned process gets its own process group via `setpgid(0, 0)`. Termination uses `killpg()` to kill the entire group. PGIDs are persisted to `running_pids.json` for orphan cleanup on restart.

### spawn_command() signature
Takes 3 arguments: `(cwd: &str, cmd: &str, env: HashMap<String, String>)` — always pass the env map.

### Shared state
Backend uses `Arc<Mutex<...>>` for all shared state (processes, logs, exit info, health). Access patterns are always lock → operate → drop.

### Tauri 2 imports
- `tauri::Emitter` trait for `.emit()` on AppHandle
- Capabilities in `capabilities/default.json` must include permissions for each plugin

### Command types
- **Process**: Long-running, toggleable, supports auto-start and health checks
- **OneTime**: Run-once commands, no auto-start or health checks

## Release & Signing

- Bundle target is `"app"` only — no DMG. No Apple Developer certificate, so DMG builds fail codesigning.
- Git commits are not GPG-signed. Always use `-c commit.gpgsign=false` when committing.
