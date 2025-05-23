use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use glob::Pattern;
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

mod bench_results;
mod stats;
use bench_results::BenchResults;
use stats::StatsCollector;

/// A blazingly fast file watcher that executes commands when files change
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// The command to execute when files change
    #[clap(required = false)]
    command: Vec<String>,

    /// Paths/patterns to watch (supports glob patterns like "src/**/*.js")
    #[clap(short, long, default_value = ".")]
    watch: Vec<String>,

    /// File extensions to watch (e.g., "js,jsx,ts,tsx")
    #[clap(short, long)]
    ext: Option<String>,

    /// Specific glob patterns to include (e.g., "src/**/*.{js,ts}")
    #[clap(short = 'p', long)]
    pattern: Vec<String>,

    /// Glob patterns to ignore (e.g., "**/node_modules/**", "**/.git/**")
    #[clap(short, long)]
    ignore: Vec<String>,

    /// Debounce time in milliseconds
    #[clap(short, long, default_value = "100")]
    debounce: u64,

    /// Run command on startup
    #[clap(short = 'n', long)]
    initial: bool,

    /// Clear console before each command run
    #[clap(short, long)]
    clear: bool,

    /// Use configuration from file
    #[clap(short = 'f', long)]
    config: Option<String>,

    /// Restart long-running processes instead of spawning new ones
    #[clap(short, long)]
    restart: bool,

    /// Show performance statistics
    #[clap(long)]
    stats: bool,

    /// Statistics update interval in seconds
    #[clap(long, default_value = "10")]
    stats_interval: u64,

    /// Run benchmark against other file watchers
    #[clap(long)]
    bench: bool,
}

/// Configuration file format
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    command: Vec<String>,
    watch: Option<Vec<String>>,
    ext: Option<String>,
    pattern: Option<Vec<String>>,
    ignore: Option<Vec<String>>,
    debounce: Option<u64>,
    initial: Option<bool>,
    clear: Option<bool>,
    restart: Option<bool>,
    stats: Option<bool>,
    stats_interval: Option<u64>,
}

struct CommandRunner {
    command: Vec<String>,
    restart: bool,
    clear: bool,
    current_process: Option<Child>,
}

impl CommandRunner {
    fn new(command: Vec<String>, restart: bool, clear: bool) -> Self {
        Self {
            command,
            restart,
            clear,
            current_process: None,
        }
    }

    fn run(&mut self) -> Result<()> {
        // Kill previous process if restart mode is enabled
        if self.restart {
            if let Some(ref mut child) = self.current_process {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        // Clear console if requested
        if self.clear {
            print!("\x1B[2J\x1B[1;1H");
        }

        // Simple feedback for command execution
        println!(
            "{} {}",
            "▶️ Running:".bright_blue(),
            self.command.join(" ").bright_yellow()
        );

        let child = if cfg!(target_os = "windows") {
            Command::new("cmd").arg("/C").args(&self.command).spawn()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(self.command.join(" "))
                .spawn()
        }
        .context("Failed to execute command")?;

        if self.restart {
            self.current_process = Some(child);
        } else {
            let status = child.wait_with_output()?;
            if !status.status.success() {
                println!(
                    "{} {}",
                    "Command exited with code:".bright_red(),
                    status.status
                );
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    // Load configuration file if specified
    if let Some(config_path) = &args.config {
        let config = load_config(config_path)?;
        merge_config(&mut args, config);
    }

    // Run benchmarks if requested
    if args.bench {
        return run_benchmarks();
    }

    // Validate that we have a command to run
    if args.command.is_empty() {
        anyhow::bail!("No command specified. Use CLI arguments or a config file.");
    }

    println!("{}", "🔥 Flash watching for changes...".bright_green());

    // Create a channel to receive the events
    let (tx, rx) = std::sync::mpsc::channel();

    // Initialize stats collector
    let stats_collector = Arc::new(Mutex::new(StatsCollector::new()));

    // Start stats display thread if stats is enabled
    if args.stats {
        let stats = Arc::clone(&stats_collector);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(args.stats_interval));
            let mut stats = stats.lock().unwrap();
            stats.update_resource_usage();
            stats.display_stats();
        });
    }

    // Compile glob patterns for better filtering
    let include_patterns = args
        .pattern
        .iter()
        .map(|p| glob::Pattern::new(p))
        .collect::<Result<Vec<_>, _>>()
        .context("Invalid glob pattern")?;

    let ignore_patterns = args
        .ignore
        .iter()
        .map(|p| glob::Pattern::new(p))
        .collect::<Result<Vec<_>, _>>()
        .context("Invalid ignore pattern")?;

    // Create a command runner
    let mut runner = CommandRunner::new(args.command.clone(), args.restart, args.clear);

    // Run the command initially if requested
    if args.initial {
        if let Err(e) = runner.run() {
            eprintln!("{} {}", "Error running initial command:".bright_red(), e);
        }
    }

    // Set up the file watcher
    setup_watcher(&args, tx.clone(), Arc::clone(&stats_collector))?;

    println!("{}", "Ready! Waiting for changes...".bright_green());

    // Track recently processed paths to avoid duplicates
    let mut recently_processed = std::collections::HashMap::new();

    // Listen for events in a loop
    for path in rx {
        if should_process_path(&path, &args.ext, &include_patterns, &ignore_patterns) {
            // Get a path key for deduplication
            let path_key = path.to_string_lossy().to_string();

            // Check if we've seen this path recently
            let now = std::time::Instant::now();
            if let Some(last_time) = recently_processed.get(&path_key) {
                if now.duration_since(*last_time).as_millis() < args.debounce as u128 {
                    // Skip this event - too soon after the previous one
                    continue;
                }
            }

            // Update the last processed time for this path
            recently_processed.insert(path_key, now);

            // Format the path to be more readable - just show the filename if possible
            let display_path = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or("unknown path"));

            println!(
                "{} {}",
                "📝 Change detected:".bright_blue(),
                display_path.bright_green()
            );

            // Record the file change in stats
            if args.stats {
                let mut stats = stats_collector.lock().unwrap();
                stats.record_file_change();
            }

            if let Err(e) = runner.run() {
                eprintln!("{} {}", "Error running command:".bright_red(), e);
            }

            // Clean up old entries in recently_processed
            recently_processed.retain(|_, time| now.duration_since(*time).as_millis() < 10000);
        }
    }

    Ok(())
}

fn run_benchmarks() -> Result<()> {
    println!("{}", "Running benchmarks...".bright_green());
    println!(
        "{}",
        "This will compare Flash with other file watchers.".bright_yellow()
    );

    // Check if we should run real benchmarks or show sample data
    let has_criterion = Command::new("cargo")
        .args(["bench", "--bench", "file_watcher", "--help"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if has_criterion {
        // Attempt to run real benchmarks
        println!(
            "{}",
            "Running real benchmarks (this may take a few minutes)...".bright_blue()
        );

        let status = Command::new("cargo")
            .args(["bench", "--bench", "file_watcher"])
            .status()
            .context("Failed to run benchmarks")?;

        if !status.success() {
            println!(
                "{}",
                "Benchmark run failed, showing sample data instead...".bright_yellow()
            );
            show_sample_results();
        }
    } else {
        // No criterion benchmarks available, show sample data
        println!(
            "{}",
            "No benchmark suite detected, showing sample data...".bright_yellow()
        );
        show_sample_results();
    }

    Ok(())
}

fn show_sample_results() {
    // Create benchmark results with sample data
    let results = BenchResults::with_sample_data();

    // Display beautiful benchmark report
    results.print_report();

    println!(
        "\n{}",
        "Note: These are simulated results for demonstration.".bright_yellow()
    );
    println!(
        "{}",
        "Run 'cargo bench --bench file_watcher' for real benchmarks.".bright_blue()
    );
}

fn load_config(path: &str) -> Result<Config> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

    serde_yaml::from_str(&content).context(format!("Failed to parse config file: {}", path))
}

fn merge_config(args: &mut Args, config: Config) {
    // Only use config values when CLI args are not provided
    if args.command.is_empty() && !config.command.is_empty() {
        args.command = config.command;
    }

    if args.watch.len() == 1 && args.watch[0] == "." {
        if let Some(watch_dirs) = config.watch {
            args.watch = watch_dirs;
        }
    }

    if args.ext.is_none() {
        args.ext = config.ext;
    }

    if args.pattern.is_empty() {
        if let Some(patterns) = config.pattern {
            args.pattern = patterns;
        }
    }

    if args.ignore.is_empty() {
        if let Some(ignores) = config.ignore {
            args.ignore = ignores;
        }
    }

    if args.debounce == 100 {
        if let Some(debounce) = config.debounce {
            args.debounce = debounce;
        }
    }

    if !args.initial {
        if let Some(initial) = config.initial {
            args.initial = initial;
        }
    }

    if !args.clear {
        if let Some(clear) = config.clear {
            args.clear = clear;
        }
    }

    if !args.restart {
        if let Some(restart) = config.restart {
            args.restart = restart;
        }
    }

    if !args.stats {
        if let Some(stats) = config.stats {
            args.stats = stats;
        }
    }

    if args.stats_interval == 10 {
        if let Some(interval) = config.stats_interval {
            args.stats_interval = interval;
        }
    }
}

fn setup_watcher(
    args: &Args,
    tx: Sender<PathBuf>,
    stats: Arc<Mutex<StatsCollector>>,
) -> Result<()> {
    // Capture only what we need for the event handler
    let stats_enabled = args.stats;

    // Create a more direct event handler using standard notify
    let event_tx = tx.clone();
    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Record watcher call in stats
                    if stats_enabled {
                        let mut stats = stats.lock().unwrap();
                        stats.record_watcher_call();
                    }

                    // Process different event types
                    match event.kind {
                        notify::EventKind::Create(_)
                        | notify::EventKind::Modify(_)
                        | notify::EventKind::Remove(_) => {
                            for path in event.paths {
                                event_tx.send(path).unwrap_or_else(|e| {
                                    eprintln!("{} {}", "Error sending event:".bright_red(), e);
                                });
                            }
                        }
                        _ => {
                            // Ignore other event types like access events
                        }
                    }
                }
                Err(e) => eprintln!("{} {}", "Watcher error:".bright_red(), e),
            }
        })?;

    // Track watched paths to avoid duplicates
    let mut watched_paths = std::collections::HashSet::new();
    let mut watch_count = 0;

    // Add paths to watch
    for pattern_str in &args.watch {
        // First check if it's a plain directory (for backward compatibility)
        let path_obj = Path::new(pattern_str);
        if path_obj.exists() && path_obj.is_dir() {
            // It's a plain directory, watch it directly
            if watched_paths.insert(path_obj.to_path_buf()) {
                watcher
                    .watch(path_obj, RecursiveMode::Recursive)
                    .context(format!("Failed to watch path: {}", pattern_str))?;
                println!("{} {}", "Watching:".bright_blue(), pattern_str);
                watch_count += 1;
            }
        } else {
            // Try to interpret it as a glob pattern
            let pattern = glob::Pattern::new(pattern_str)
                .context(format!("Invalid watch pattern: {}", pattern_str))?;

            // Find all directories that match this pattern
            // Note: We need a way to list directories to apply the glob pattern.
            // For simplicity, we'll start from the current directory.
            let base_dir = ".";
            let walker = WalkDir::new(base_dir)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| !should_skip_dir(e.path(), &args.ignore));

            let mut matched = false;
            for entry in walker.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() && pattern.matches_path(path) && watched_paths.insert(path.to_path_buf()) {
                    watcher
                        .watch(path, RecursiveMode::Recursive)
                        .context(format!("Failed to watch matched path: {}", path.display()))?;
                    println!(
                        "{} {} (from pattern: {})",
                        "Watching:".bright_blue(),
                        path.display(),
                        pattern_str
                    );
                    watch_count += 1;
                    matched = true;
                }
            }

            if !matched {
                println!(
                    "{} {}",
                    "Warning: No directories matched pattern:".bright_yellow(),
                    pattern_str
                );
            }
        }
    }

    if watch_count == 0 {
        println!("{}", "Warning: No paths are being watched!".bright_yellow());
    } else {
        println!("{} {}", "Total watched paths:".bright_blue(), watch_count);
    }

    // Keep the watcher alive by storing it
    std::mem::forget(watcher);

    // Print other settings
    if let Some(ext) = &args.ext {
        println!("{} {}", "File extensions:".bright_blue(), ext);
    }

    if !args.pattern.is_empty() {
        println!(
            "{} {}",
            "Include patterns:".bright_blue(),
            args.pattern.join(", ")
        );
    }

    if !args.ignore.is_empty() {
        println!(
            "{} {}",
            "Ignore patterns:".bright_blue(),
            args.ignore.join(", ")
        );
    }

    // Print command
    println!(
        "{} {}",
        "Will execute:".bright_blue(),
        args.command.join(" ").bright_yellow()
    );

    // Print stats info if enabled
    if args.stats {
        println!(
            "{} {} seconds",
            "Performance stats enabled, interval:".bright_blue(),
            args.stats_interval
        );
    }

    Ok(())
}

/// Check if a directory should be skipped based on ignore patterns
fn should_skip_dir(path: &Path, ignore_patterns: &[String]) -> bool {
    for pattern_str in ignore_patterns {
        // Try to compile the pattern
        if let Ok(pattern) = glob::Pattern::new(pattern_str) {
            if pattern.matches_path(path) {
                return true;
            }
        }
    }
    false
}

// Make the path filtering function public so it can be tested separately
pub fn should_process_path(
    path: &Path,
    extensions: &Option<String>,
    include_patterns: &[Pattern],
    ignore_patterns: &[Pattern],
) -> bool {
    // Check ignore patterns - both exact path match and parent directory matches
    for pattern in ignore_patterns {
        // Try direct path matching first
        if pattern.matches_path(path) {
            return false;
        }

        // Also check if any parent directory matches the ignore pattern
        // This helps with patterns like "**/node_modules/**"
        let mut current = path;
        while let Some(parent) = current.parent() {
            if pattern.matches_path(parent) {
                return false;
            }
            current = parent;
        }
    }

    // If we have include patterns, the path must match at least one
    if !include_patterns.is_empty() {
        let mut matches = false;
        for pattern in include_patterns {
            if pattern.matches_path(path) {
                matches = true;
                break;
            }
        }
        if !matches {
            return false;
        }
    }

    // If no extensions filter is specified, process all files
    let extensions = match extensions {
        Some(ext) => ext,
        None => return true,
    };

    // Check file extension
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return extensions.split(',').any(|e| e.trim() == ext_str);
        }
    }

    false
}
