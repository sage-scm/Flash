use std::path::PathBuf;

use clap::Parser;

// Optional fields use `Option<T>` rather than clap's `default_value` so the
// merge logic in `Settings::build` can tell "the user set it to the default"
// apart from "the user did not provide a value." That distinction matters when
// a YAML config file is also in play.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "flash-watcher",
    version,
    about = "A fast, predictable file watcher that runs commands when files change.",
    after_long_help = EXAMPLES,
    disable_help_subcommand = true,
)]
pub struct Cli {
    /// Command (and arguments) to execute when matching files change.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,

    /// Path or glob to watch. Pass repeatedly to watch several locations.
    #[arg(short, long, value_name = "PATH")]
    pub watch: Vec<String>,

    /// File extensions to keep, comma-separated. Example: "rs,toml".
    #[arg(short, long, value_name = "LIST")]
    pub ext: Option<String>,

    /// Glob patterns to include. Files matching none of these are ignored.
    #[arg(short = 'p', long, value_name = "GLOB")]
    pub pattern: Vec<String>,

    /// Glob patterns to ignore (evaluated before include patterns).
    #[arg(short, long, value_name = "GLOB")]
    pub ignore: Vec<String>,

    /// Debounce window in milliseconds. Defaults to 50.
    #[arg(short, long, value_name = "MS")]
    pub debounce: Option<u64>,

    /// Run the command once before watching.
    #[arg(short = 'n', long)]
    pub initial: bool,

    /// Clear the terminal before each run.
    #[arg(short, long)]
    pub clear: bool,

    /// Restart the previous process on change instead of spawning a new one.
    #[arg(short, long)]
    pub restart: bool,

    /// Load defaults from a YAML configuration file.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Quieter output and a leaner startup path. Useful for tight feedback loops.
    #[arg(long)]
    pub fast: bool,

    /// Print periodic performance statistics.
    #[arg(long)]
    pub stats: bool,

    /// How often to refresh statistics, in seconds. Defaults to 10.
    #[arg(long, value_name = "SECONDS")]
    pub stats_interval: Option<u64>,

    /// Benchmark Flash against other watchers installed on this machine, then exit.
    #[arg(long)]
    pub bench: bool,
}

const EXAMPLES: &str = "\
EXAMPLES:
    flash-watcher -w src cargo test
    flash-watcher -r -c -n npm run dev
    flash-watcher -e rs --debounce 250 cargo check
    flash-watcher -f flash.yaml
";
