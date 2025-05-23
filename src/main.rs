use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use flash_watcher::{
    compile_patterns, load_config, merge_config, run_benchmarks, should_process_path, Args,
    CommandRunner,
};
use notify::{RecursiveMode, Watcher};

mod stats;
use stats::StatsCollector;

/// A blazingly fast file watcher that executes commands when files change
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct CliArgs {
    /// The command to execute when files change
    #[clap(required = false)]
    pub command: Vec<String>,

    /// Paths/patterns to watch (supports glob patterns like "src/**/*.js")
    #[clap(short, long, default_value = ".")]
    pub watch: Vec<String>,

    /// File extensions to watch (e.g., "js,jsx,ts,tsx")
    #[clap(short, long)]
    pub ext: Option<String>,

    /// Specific glob patterns to include (e.g., "src/**/*.{js,ts}")
    #[clap(short = 'p', long)]
    pub pattern: Vec<String>,

    /// Glob patterns to ignore (e.g., "**/node_modules/**", "**/.git/**")
    #[clap(short, long)]
    pub ignore: Vec<String>,

    /// Debounce time in milliseconds
    #[clap(short, long, default_value = "100")]
    pub debounce: u64,

    /// Run command on startup
    #[clap(short = 'n', long)]
    pub initial: bool,

    /// Clear console before each command run
    #[clap(short, long)]
    pub clear: bool,

    /// Use configuration from file
    #[clap(short = 'f', long)]
    pub config: Option<String>,

    /// Restart long-running processes instead of spawning new ones
    #[clap(short, long)]
    pub restart: bool,

    /// Show performance statistics
    #[clap(long)]
    pub stats: bool,

    /// Statistics update interval in seconds
    #[clap(long, default_value = "10")]
    pub stats_interval: u64,

    /// Run benchmark against other file watchers
    #[clap(long)]
    pub bench: bool,

    /// Fast startup mode - minimal output and optimizations
    #[clap(long)]
    pub fast: bool,
}

impl From<CliArgs> for Args {
    fn from(cli: CliArgs) -> Self {
        Args {
            command: cli.command,
            watch: cli.watch,
            ext: cli.ext,
            pattern: cli.pattern,
            ignore: cli.ignore,
            debounce: cli.debounce,
            initial: cli.initial,
            clear: cli.clear,
            restart: cli.restart,
            stats: cli.stats,
            stats_interval: cli.stats_interval,
            bench: cli.bench,
            config: cli.config,
            fast: cli.fast,
        }
    }
}

fn main() -> Result<()> {
    let cli_args = CliArgs::parse();
    let mut args: Args = cli_args.into();

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
    flash_watcher::validate_args(&args)?;

    // Skip startup message for faster startup in fast mode
    if !args.fast && !args.stats {
        println!("{}", "🔥 Flash watching for changes...".bright_green());
    }

    // Create a channel to receive the events
    let (tx, rx) = std::sync::mpsc::channel();

    // Initialize stats collector only if needed
    let stats_collector = if args.stats {
        let collector = Arc::new(Mutex::new(StatsCollector::new()));
        let stats = Arc::clone(&collector);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(args.stats_interval));
            let mut stats = stats.lock().unwrap();
            stats.update_resource_usage();
            stats.display_stats();
        });
        Some(collector)
    } else {
        None
    };

    // Compile glob patterns for better filtering
    let include_patterns = compile_patterns(&args.pattern)?;
    let ignore_patterns = compile_patterns(&args.ignore)?;

    // Create a command runner
    let mut runner = CommandRunner::new(args.command.clone(), args.restart, args.clear);

    // Run the command initially if requested
    if args.initial {
        if let Err(e) = runner.run() {
            eprintln!("{} {}", "Error running initial command:".bright_red(), e);
        }
    }

    // Set up the file watcher
    setup_watcher(&args, tx.clone(), stats_collector.clone())?;

    if !args.fast {
        println!("{}", "Ready! Waiting for changes...".bright_green());
    }

    // Track recently processed paths to avoid duplicates - use PathBuf as key to avoid string allocation
    let mut recently_processed = std::collections::HashMap::new();

    // Listen for events in a loop
    for path in rx {
        if should_process_path(&path, &args.ext, &include_patterns, &ignore_patterns) {
            // Check if we've seen this path recently - use PathBuf directly as key
            let now = std::time::Instant::now();
            if let Some(last_time) = recently_processed.get(&path) {
                if now.duration_since(*last_time).as_millis() < args.debounce as u128 {
                    // Skip this event - too soon after the previous one
                    continue;
                }
            }

            // Update the last processed time for this path
            recently_processed.insert(path.clone(), now);

            // Only format output if not in fast mode and not in stats mode
            if !args.fast && !args.stats {
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
            }

            // Record the file change in stats
            if let Some(ref stats_collector) = stats_collector {
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

fn setup_watcher(
    args: &Args,
    tx: Sender<PathBuf>,
    stats: Option<Arc<Mutex<StatsCollector>>>,
) -> Result<()> {
    // No need to capture stats_enabled since we check the Option directly

    // Create a more direct event handler using standard notify
    let event_tx = tx.clone();
    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Record watcher call in stats
                    if let Some(ref stats) = stats {
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
                if !args.fast {
                    println!("{} {}", "Watching:".bright_blue(), pattern_str);
                }
                watch_count += 1;
            }
        } else {
            // For glob patterns, just watch the current directory and let filtering handle the rest
            // This is much faster than walking the entire directory tree during startup
            let current_dir = Path::new(".");
            if watched_paths.insert(current_dir.to_path_buf()) {
                watcher
                    .watch(current_dir, RecursiveMode::Recursive)
                    .context(format!(
                        "Failed to watch current directory for pattern: {}",
                        pattern_str
                    ))?;
                if !args.fast {
                    println!("{} . (pattern: {})", "Watching:".bright_blue(), pattern_str);
                }
                watch_count += 1;
            }
        }
    }

    if !args.fast {
        if watch_count == 0 {
            println!("{}", "Warning: No paths are being watched!".bright_yellow());
        } else {
            println!("{} {}", "Total watched paths:".bright_blue(), watch_count);
        }
    }

    // Keep the watcher alive by storing it
    std::mem::forget(watcher);

    // Print other settings only if not in fast mode
    if !args.fast {
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
    }

    Ok(())
}
