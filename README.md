# Termina

A lightweight macOS desktop app for managing long-running shell commands. Start, stop, and monitor your dev servers, build watchers, and background processes from a single place.

## Features

- **Process management** — Start, stop, and restart long-running commands with a click
- **Live logs** — Stream stdout/stderr output in real-time with a scrollable log viewer
- **Health checks** — Configure HTTP health check URLs to monitor service status
- **System tray** — Runs in the background with a tray menu showing active commands
- **Auto-start** — Mark commands to start automatically when Termina launches
- **Notifications** — Get notified when processes exit unexpectedly
- **Launch at login** — Optional auto-start with macOS via LaunchAgent
- **Global shortcut** — Toggle the window with `Cmd+Shift+T` from anywhere
- **One-time commands** — Run-once commands alongside long-running processes
- **Environment variables** — Set custom env vars per command

## Prerequisites

- macOS
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install) (1.77.2+)

## Setup

```bash
# Clone the repo
git clone <repo-url>
cd Termina

# Install frontend dependencies
npm install
```

## Development

```bash
# Start the app in dev mode (Vite dev server + Tauri app)
npm run tauri dev
```

This starts the Vite dev server on `:1420` and launches the Tauri window with hot reload.

## Production Build

```bash
# Build the .app bundle and .dmg
npm run tauri build
```

The output is in `src-tauri/target/release/bundle/macos/`.

## Usage

1. Click **+ New Command** to add a command
2. Give it a name, working directory, and the shell command to run
3. Choose **Process** (long-running) or **One-time** (run once)
4. Optionally set a health check URL and enable auto-start
5. Click the play button on a command card to start it
6. View logs by clicking on a command card
7. Close the window — Termina keeps running in the system tray

## Tech Stack

- **Frontend**: Preact + TypeScript + Preact Signals
- **Backend**: Rust + Tauri 2
- **Build**: Vite + Cargo

## License

MIT
