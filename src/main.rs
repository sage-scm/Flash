use std::process::ExitCode;

use clap::Parser;
use flash_watcher::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match flash_watcher::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("flash-watcher: {err:#}");
            ExitCode::FAILURE
        }
    }
}
