use clap::Parser;

// Import from the library crate
use termina_lib::cli::{run_cli, Cli};

fn main() {
    let cli = Cli::parse();
    std::process::exit(run_cli(cli));
}
