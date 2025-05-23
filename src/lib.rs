use std::path::Path;
use std::process::{Child, Command};

use anyhow::{Context, Result};
use colored::Colorize;
use glob::Pattern;
use serde::{Deserialize, Serialize};

pub mod bench_results;
pub mod stats;

/// Configuration file format
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub command: Vec<String>,
    pub watch: Option<Vec<String>>,
    pub ext: Option<String>,
    pub pattern: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
    pub debounce: Option<u64>,
    pub initial: Option<bool>,
    pub clear: Option<bool>,
    pub restart: Option<bool>,
    pub stats: Option<bool>,
    pub stats_interval: Option<u64>,
}

/// Command line arguments structure
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub command: Vec<String>,
    pub watch: Vec<String>,
    pub ext: Option<String>,
    pub pattern: Vec<String>,
    pub ignore: Vec<String>,
    pub debounce: u64,
    pub initial: bool,
    pub clear: bool,
    pub restart: bool,
    pub stats: bool,
    pub stats_interval: u64,
    pub bench: bool,
    pub config: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            command: vec![],
            watch: vec![".".to_string()],
            ext: None,
            pattern: vec![],
            ignore: vec![],
            debounce: 100,
            initial: false,
            clear: false,
            restart: false,
            stats: false,
            stats_interval: 10,
            bench: false,
            config: None,
        }
    }
}

/// Command runner for executing commands when files change
pub struct CommandRunner {
    pub command: Vec<String>,
    pub restart: bool,
    pub clear: bool,
    pub current_process: Option<Child>,
}

impl CommandRunner {
    pub fn new(command: Vec<String>, restart: bool, clear: bool) -> Self {
        Self {
            command,
            restart,
            clear,
            current_process: None,
        }
    }

    pub fn run(&mut self) -> Result<()> {
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

    /// Dry run for testing - doesn't actually execute commands
    pub fn dry_run(&mut self) -> Result<()> {
        if self.restart && self.current_process.is_some() {
            self.current_process = None;
        }

        if self.command.is_empty() {
            anyhow::bail!("Empty command");
        }

        Ok(())
    }
}

/// Load configuration from a YAML file
pub fn load_config(path: &str) -> Result<Config> {
    let content =
        std::fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

    serde_yaml::from_str(&content).context(format!("Failed to parse config file: {}", path))
}

/// Merge configuration file settings with command line arguments
pub fn merge_config(args: &mut Args, config: Config) {
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

/// Check if a path should be processed based on filters
pub fn should_process_path(
    path: &Path,
    ext_filter: &Option<String>,
    include_patterns: &[Pattern],
    ignore_patterns: &[Pattern],
) -> bool {
    // Check ignore patterns first
    for pattern in ignore_patterns {
        if pattern.matches_path(path) {
            return false;
        }
    }

    // Check extension filter
    if let Some(ext_list) = ext_filter {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let extensions: Vec<&str> = ext_list.split(',').map(|s| s.trim()).collect();
            if !extensions.contains(&extension) {
                return false;
            }
        } else {
            // No extension, but we have an extension filter
            return false;
        }
    }

    // Check include patterns
    if !include_patterns.is_empty() {
        for pattern in include_patterns {
            if pattern.matches_path(path) {
                return true;
            }
        }
        return false;
    }

    true
}

/// Check if a directory should be skipped during traversal
pub fn should_skip_dir(path: &Path, ignore_patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();

    // Skip common directories that should be ignored
    let common_ignores = [".git", "node_modules", "target", ".svn", ".hg"];

    for ignore in &common_ignores {
        if path_str.contains(ignore) {
            return true;
        }
    }

    // Check user-defined ignore patterns
    for pattern_str in ignore_patterns {
        if let Ok(pattern) = glob::Pattern::new(pattern_str) {
            if pattern.matches_path(path) {
                return true;
            }
        }
    }

    false
}

/// Run benchmarks and display results
pub fn run_benchmarks() -> Result<()> {
    println!("{}", "Running benchmarks...".bright_green());
    println!(
        "{}",
        "This will compare Flash with other file watchers.".bright_yellow()
    );

    // Check if benchmarks are available with the benchmarks feature
    let has_criterion = Command::new("cargo")
        .args([
            "bench",
            "--features",
            "benchmarks",
            "--bench",
            "file_watcher",
            "--help",
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if has_criterion {
        // Attempt to run real benchmarks with feature flag
        println!(
            "{}",
            "Running real benchmarks (this may take a few minutes)...".bright_blue()
        );

        let status = Command::new("cargo")
            .args([
                "bench",
                "--features",
                "benchmarks",
                "--bench",
                "file_watcher",
            ])
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
            "Benchmarks require the 'benchmarks' feature. Showing sample data...".bright_yellow()
        );
        println!(
            "{}",
            "To run real benchmarks: cargo bench --features benchmarks".bright_blue()
        );
        show_sample_results();
    }

    Ok(())
}

/// Show sample benchmark results
pub fn show_sample_results() {
    use crate::bench_results::BenchResults;

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

/// Compile glob patterns from string patterns
pub fn compile_patterns(patterns: &[String]) -> Result<Vec<Pattern>> {
    patterns
        .iter()
        .map(|p| Pattern::new(p).context(format!("Invalid pattern: {}", p)))
        .collect()
}

/// Validate command line arguments
pub fn validate_args(args: &Args) -> Result<()> {
    if args.command.is_empty() {
        anyhow::bail!("No command specified. Use CLI arguments or a config file.");
    }
    Ok(())
}

/// Format a path for display (show just filename if possible)
pub fn format_display_path(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("unknown path"))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_config_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_args_default() {
        let args = Args::default();
        assert!(args.command.is_empty());
        assert_eq!(args.watch, vec!["."]);
        assert_eq!(args.debounce, 100);
        assert!(!args.initial);
        assert!(!args.clear);
        assert!(!args.restart);
        assert!(!args.stats);
        assert_eq!(args.stats_interval, 10);
        assert!(!args.bench);
    }

    #[test]
    fn test_command_runner_new() {
        let command = vec!["echo".to_string(), "hello".to_string()];
        let runner = CommandRunner::new(command.clone(), true, false);

        assert_eq!(runner.command, command);
        assert!(runner.restart);
        assert!(!runner.clear);
        assert!(runner.current_process.is_none());
    }

    #[test]
    fn test_command_runner_dry_run_success() {
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "test".to_string()], false, false);
        assert!(runner.dry_run().is_ok());
    }

    #[test]
    fn test_command_runner_dry_run_empty_command() {
        let mut runner = CommandRunner::new(vec![], false, false);
        assert!(runner.dry_run().is_err());
    }

    #[test]
    fn test_command_runner_dry_run_restart_mode() {
        let mut runner = CommandRunner::new(vec!["echo".to_string()], true, false);
        // Simulate having a current process
        runner.current_process = None; // Would be Some(child) in real scenario
        assert!(runner.dry_run().is_ok());
        assert!(runner.current_process.is_none());
    }

    #[test]
    fn test_load_config_valid() {
        let config_yaml = r#"
command: ["npm", "run", "dev"]
watch:
  - "src"
  - "public"
ext: "js,jsx,ts,tsx"
pattern:
  - "src/**/*.{js,jsx,ts,tsx}"
ignore:
  - "node_modules"
  - ".git"
debounce: 200
initial: true
clear: true
restart: true
stats: true
stats_interval: 5
"#;

        let file = create_test_config_file(config_yaml);
        let config = load_config(file.path().to_str().unwrap()).unwrap();

        assert_eq!(config.command, vec!["npm", "run", "dev"]);
        assert_eq!(
            config.watch,
            Some(vec!["src".to_string(), "public".to_string()])
        );
        assert_eq!(config.ext, Some("js,jsx,ts,tsx".to_string()));
        assert_eq!(
            config.pattern,
            Some(vec!["src/**/*.{js,jsx,ts,tsx}".to_string()])
        );
        assert_eq!(
            config.ignore,
            Some(vec!["node_modules".to_string(), ".git".to_string()])
        );
        assert_eq!(config.debounce, Some(200));
        assert_eq!(config.initial, Some(true));
        assert_eq!(config.clear, Some(true));
        assert_eq!(config.restart, Some(true));
        assert_eq!(config.stats, Some(true));
        assert_eq!(config.stats_interval, Some(5));
    }

    #[test]
    fn test_load_config_invalid() {
        let invalid_yaml = r#"
command: "not-a-list"
invalid: true
"#;

        let file = create_test_config_file(invalid_yaml);
        let result = load_config(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_nonexistent_file() {
        let result = load_config("nonexistent.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_config_empty_args() {
        let mut args = Args::default();
        let config = Config {
            command: vec!["cargo".to_string(), "test".to_string()],
            watch: Some(vec!["src".to_string(), "tests".to_string()]),
            ext: Some("rs".to_string()),
            pattern: Some(vec!["src/**/*.rs".to_string()]),
            ignore: Some(vec!["target".to_string()]),
            debounce: Some(200),
            initial: Some(true),
            clear: Some(true),
            restart: Some(true),
            stats: Some(true),
            stats_interval: Some(5),
        };

        merge_config(&mut args, config);

        assert_eq!(args.command, vec!["cargo", "test"]);
        assert_eq!(args.watch, vec!["src", "tests"]);
        assert_eq!(args.ext, Some("rs".to_string()));
        assert_eq!(args.pattern, vec!["src/**/*.rs"]);
        assert_eq!(args.ignore, vec!["target"]);
        assert_eq!(args.debounce, 200);
        assert!(args.initial);
        assert!(args.clear);
        assert!(args.restart);
        assert!(args.stats);
        assert_eq!(args.stats_interval, 5);
    }

    #[test]
    fn test_merge_config_cli_override() {
        let mut args = Args {
            command: vec!["echo".to_string(), "hello".to_string()],
            watch: vec!["src".to_string()],
            ext: Some("js".to_string()),
            pattern: vec!["custom-pattern".to_string()],
            ignore: vec!["custom-ignore".to_string()],
            debounce: 50,
            initial: true,
            clear: true,
            restart: true,
            stats: true,
            stats_interval: 15,
            bench: false,
            config: None,
        };

        let config = Config {
            command: vec!["cargo".to_string(), "test".to_string()],
            watch: Some(vec!["src".to_string(), "tests".to_string()]),
            ext: Some("rs".to_string()),
            pattern: Some(vec!["src/**/*.rs".to_string()]),
            ignore: Some(vec!["target".to_string()]),
            debounce: Some(200),
            initial: Some(false),
            clear: Some(false),
            restart: Some(false),
            stats: Some(false),
            stats_interval: Some(5),
        };

        let args_before = args.clone();
        merge_config(&mut args, config);

        // CLI args should take precedence
        assert_eq!(args, args_before);
    }

    #[test]
    fn test_should_process_path_no_filters() {
        let path = Path::new("test.txt");
        let ext_filter = None;
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        assert!(should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_extension_filter_match() {
        let path = Path::new("test.js");
        let ext_filter = Some("js,ts".to_string());
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        assert!(should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_extension_filter_no_match() {
        let path = Path::new("test.py");
        let ext_filter = Some("js,ts".to_string());
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        assert!(!should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_ignore_pattern() {
        let path = Path::new("node_modules/test.js");
        let ext_filter = None;
        let include_patterns = vec![];
        let ignore_patterns = vec![Pattern::new("**/node_modules/**").unwrap()];

        assert!(!should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_include_pattern_match() {
        let path = Path::new("src/test.js");
        let ext_filter = None;
        let include_patterns = vec![Pattern::new("src/**/*.js").unwrap()];
        let ignore_patterns = vec![];

        assert!(should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_include_pattern_no_match() {
        let path = Path::new("docs/test.md");
        let ext_filter = None;
        let include_patterns = vec![Pattern::new("src/**/*.js").unwrap()];
        let ignore_patterns = vec![];

        assert!(!should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_skip_dir_common_ignores() {
        assert!(should_skip_dir(Path::new(".git"), &[]));
        assert!(should_skip_dir(Path::new("node_modules"), &[]));
        assert!(should_skip_dir(Path::new("target"), &[]));
        assert!(should_skip_dir(Path::new("project/.git/hooks"), &[]));
        assert!(should_skip_dir(
            Path::new("project/node_modules/package"),
            &[]
        ));
    }

    #[test]
    fn test_should_skip_dir_custom_patterns() {
        let ignore_patterns = vec!["build".to_string(), "dist".to_string()];
        assert!(should_skip_dir(Path::new("build"), &ignore_patterns));
        assert!(should_skip_dir(Path::new("dist"), &ignore_patterns));
        assert!(!should_skip_dir(Path::new("src"), &ignore_patterns));
    }

    #[test]
    fn test_should_skip_dir_no_match() {
        assert!(!should_skip_dir(Path::new("src"), &[]));
        assert!(!should_skip_dir(Path::new("tests"), &[]));
        assert!(!should_skip_dir(Path::new("docs"), &[]));
    }

    #[test]
    fn test_run_benchmarks() {
        // This test just ensures the function doesn't panic
        // In a real scenario, it would check for cargo bench availability
        let result = run_benchmarks();
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_sample_results() {
        // This test just ensures the function doesn't panic
        // It should print sample benchmark results
        show_sample_results();
    }

    #[test]
    fn test_compile_patterns_valid() {
        let patterns = vec!["*.js".to_string(), "src/**/*.rs".to_string()];
        let result = compile_patterns(&patterns);
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.len(), 2);
    }

    #[test]
    fn test_compile_patterns_invalid() {
        let patterns = vec!["[invalid".to_string()];
        let result = compile_patterns(&patterns);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_patterns_empty() {
        let patterns = vec![];
        let result = compile_patterns(&patterns);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_validate_args_valid() {
        let args = Args {
            command: vec!["echo".to_string(), "hello".to_string()],
            ..Args::default()
        };
        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn test_validate_args_empty_command() {
        let args = Args::default();
        assert!(validate_args(&args).is_err());
    }

    #[test]
    fn test_format_display_path() {
        assert_eq!(format_display_path(Path::new("test.js")), "test.js");
        assert_eq!(format_display_path(Path::new("src/test.js")), "test.js");
        assert_eq!(
            format_display_path(Path::new("/full/path/to/file.rs")),
            "file.rs"
        );
        assert_eq!(format_display_path(Path::new(".")), ".");
    }

    #[test]
    fn test_should_process_path_file_without_extension() {
        let path = Path::new("Makefile");
        let ext_filter = Some("js,ts".to_string());
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        // File without extension should be rejected when extension filter is present
        assert!(!should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_extension_with_spaces() {
        let path = Path::new("test.js");
        let ext_filter = Some("js, ts, jsx ".to_string()); // Extensions with spaces
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        // Should handle extensions with spaces correctly
        assert!(should_process_path(
            path,
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_skip_dir_invalid_glob_pattern() {
        // Test with invalid glob pattern that can't be compiled
        let invalid_patterns = vec!["[invalid".to_string()];

        // Should not skip directories when pattern is invalid
        assert!(!should_skip_dir(Path::new("some-dir"), &invalid_patterns));
    }

    #[test]
    fn test_merge_config_edge_cases() {
        let mut args = Args {
            command: vec![],              // Empty command
            watch: vec![".".to_string()], // Default watch
            ext: None,
            pattern: vec![],
            ignore: vec![],
            debounce: 100, // Default debounce
            initial: false,
            clear: false,
            restart: false,
            stats: false,
            stats_interval: 10, // Default stats interval
            bench: false,
            config: None,
        };

        let config = Config {
            command: vec![], // Empty command in config too
            watch: None,
            ext: None,
            pattern: None,
            ignore: None,
            debounce: None,
            initial: None,
            clear: None,
            restart: None,
            stats: None,
            stats_interval: None,
        };

        merge_config(&mut args, config);

        // Args should remain unchanged when config has no values
        assert!(args.command.is_empty());
        assert_eq!(args.watch, vec!["."]);
        assert_eq!(args.debounce, 100);
        assert_eq!(args.stats_interval, 10);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original_config = Config {
            command: vec!["cargo".to_string(), "test".to_string()],
            watch: Some(vec!["src".to_string(), "tests".to_string()]),
            ext: Some("rs".to_string()),
            pattern: Some(vec!["**/*.rs".to_string()]),
            ignore: Some(vec!["target".to_string()]),
            debounce: Some(200),
            initial: Some(true),
            clear: Some(false),
            restart: Some(true),
            stats: Some(false),
            stats_interval: Some(5),
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&original_config).unwrap();

        // Deserialize back
        let deserialized_config: Config = serde_yaml::from_str(&yaml).unwrap();

        // Should be identical
        assert_eq!(original_config, deserialized_config);
    }

    #[test]
    fn test_args_debug_format() {
        let args = Args {
            command: vec!["echo".to_string(), "test".to_string()],
            watch: vec!["src".to_string()],
            ext: Some("rs".to_string()),
            pattern: vec!["*.rs".to_string()],
            ignore: vec!["target".to_string()],
            debounce: 200,
            initial: true,
            clear: false,
            restart: true,
            stats: false,
            stats_interval: 5,
            bench: false,
            config: Some("config.yaml".to_string()),
        };

        let debug_str = format!("{:?}", args);
        assert!(debug_str.contains("command"));
        assert!(debug_str.contains("echo"));
        assert!(debug_str.contains("test"));
    }
}
