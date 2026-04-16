// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use termina_lib::cli::{run_cli, Cli};

fn main() {
    // If launched with CLI subcommand args, run as CLI.
    // If launched with no args (e.g. double-clicked), run the Tauri app.
    if std::env::args().len() > 1 {
        let cli = Cli::parse();
        std::process::exit(run_cli(cli));
    } else {
        termina_lib::run();
    }
}
