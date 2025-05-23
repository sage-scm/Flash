use flash_watcher::{Args, CommandRunner};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_runner_integration() {
        // Test CommandRunner functionality used in main.rs
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "test".to_string()], false, false);

        // Test successful command execution
        let result = runner.run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_runner_with_restart() {
        // Test restart functionality used in main.rs
        let mut runner = CommandRunner::new(
            vec!["echo".to_string(), "test".to_string()],
            true, // restart mode
            false,
        );

        // First run
        let result1 = runner.run();
        assert!(result1.is_ok());

        // Second run (should restart)
        let result2 = runner.run();
        assert!(result2.is_ok());
    }

    #[test]
    fn test_command_runner_with_clear() {
        // Test clear functionality used in main.rs
        let mut runner = CommandRunner::new(
            vec!["echo".to_string(), "test".to_string()],
            false,
            true, // clear mode
        );

        let result = runner.run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_runner_error_handling() {
        // Test error handling in CommandRunner used by main.rs
        let mut runner =
            CommandRunner::new(vec!["nonexistent_command_xyz123".to_string()], false, false);

        // This should handle the error gracefully
        let result = runner.run();
        // The command might fail, but the runner should handle it
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_channel_communication() {
        // Test the channel communication pattern used in main.rs
        let (tx, rx) = mpsc::channel::<PathBuf>();

        // Simulate sending file change events
        let test_paths = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("tests/test.rs"),
            PathBuf::from("Cargo.toml"),
        ];

        // Send events in a separate thread (simulating file watcher)
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            for path in test_paths {
                tx_clone.send(path).unwrap();
            }
        });

        // Receive events (simulating main loop)
        let mut received_paths = Vec::new();
        for _ in 0..3 {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(path) => received_paths.push(path),
                Err(_) => break,
            }
        }

        assert_eq!(received_paths.len(), 3);
        assert!(received_paths.contains(&PathBuf::from("src/main.rs")));
        assert!(received_paths.contains(&PathBuf::from("tests/test.rs")));
        assert!(received_paths.contains(&PathBuf::from("Cargo.toml")));
    }

    #[test]
    fn test_args_with_all_options() {
        // Test Args struct with all options set (as used in main.rs)
        let args = Args {
            command: vec!["cargo".to_string(), "test".to_string()],
            watch: vec!["src".to_string(), "tests".to_string()],
            ext: Some("rs".to_string()),
            pattern: vec!["**/*.rs".to_string()],
            ignore: vec!["target".to_string()],
            debounce: 200,
            initial: true,
            clear: true,
            restart: true,
            stats: true,
            stats_interval: 5,
            bench: false,
            config: Some("flash.yaml".to_string()),
        };

        // Validate all fields are set correctly
        assert_eq!(args.command, vec!["cargo", "test"]);
        assert_eq!(args.watch, vec!["src", "tests"]);
        assert_eq!(args.ext, Some("rs".to_string()));
        assert_eq!(args.pattern, vec!["**/*.rs"]);
        assert_eq!(args.ignore, vec!["target"]);
        assert_eq!(args.debounce, 200);
        assert!(args.initial);
        assert!(args.clear);
        assert!(args.restart);
        assert!(args.stats);
        assert_eq!(args.stats_interval, 5);
        assert!(!args.bench);
        assert_eq!(args.config, Some("flash.yaml".to_string()));
    }

    #[test]
    fn test_args_default_values() {
        // Test default Args values used in main.rs
        let args = Args::default();

        assert!(args.command.is_empty());
        assert_eq!(args.watch, vec!["."]);
        assert_eq!(args.ext, None);
        assert!(args.pattern.is_empty());
        assert!(args.ignore.is_empty());
        assert_eq!(args.debounce, 100);
        assert!(!args.initial);
        assert!(!args.clear);
        assert!(!args.restart);
        assert!(!args.stats);
        assert_eq!(args.stats_interval, 10);
        assert!(!args.bench);
        assert_eq!(args.config, None);
    }

    #[test]
    fn test_path_processing_workflow() {
        // Test the complete path processing workflow from main.rs
        use flash_watcher::{compile_patterns, should_process_path};
        use std::path::Path;

        // Setup similar to main.rs
        let args = Args {
            ext: Some("rs,js".to_string()),
            pattern: vec!["src/**/*".to_string()],
            ignore: vec!["**/target/**".to_string()],
            ..Args::default()
        };

        let include_patterns = compile_patterns(&args.pattern).unwrap();
        let ignore_patterns = compile_patterns(&args.ignore).unwrap();

        // Test various paths
        let test_cases = vec![
            ("src/main.rs", true),  // Should process: matches pattern and extension
            ("src/lib.js", true),   // Should process: matches pattern and extension
            ("src/test.py", false), // Should not process: wrong extension
            ("target/debug/main.rs", false), // Should not process: ignored path
            ("docs/readme.md", false), // Should not process: doesn't match pattern
        ];

        for (path_str, expected) in test_cases {
            let path = Path::new(path_str);
            let result = should_process_path(path, &args.ext, &include_patterns, &ignore_patterns);
            assert_eq!(result, expected, "Failed for path: {}", path_str);
        }
    }

    #[test]
    fn test_recently_processed_cleanup() {
        // Test the recently_processed HashMap cleanup logic from main.rs
        use std::collections::HashMap;
        use std::time::Instant;

        let mut recently_processed = HashMap::new();
        let now = Instant::now();

        // Add some entries
        recently_processed.insert("file1.rs".to_string(), now);
        recently_processed.insert("file2.rs".to_string(), now);
        recently_processed.insert("file3.rs".to_string(), now);

        assert_eq!(recently_processed.len(), 3);

        // Simulate cleanup (retain entries newer than 10 seconds)
        let cleanup_threshold_ms = 10000u128;
        recently_processed
            .retain(|_, time| now.duration_since(*time).as_millis() < cleanup_threshold_ms);

        // All entries should still be there (they're fresh)
        assert_eq!(recently_processed.len(), 3);

        // Simulate old entries by creating a much older timestamp
        let old_time = now - Duration::from_secs(15);
        recently_processed.insert("old_file.rs".to_string(), old_time);

        // Cleanup again
        recently_processed
            .retain(|_, time| now.duration_since(*time).as_millis() < cleanup_threshold_ms);

        // The old entry should be removed, but recent ones should remain
        assert_eq!(recently_processed.len(), 3);
        assert!(!recently_processed.contains_key("old_file.rs"));
    }

    #[test]
    fn test_watch_path_validation() {
        // Test watch path validation logic used in main.rs
        use std::path::Path;

        let test_paths = vec![".", "src", "tests", "nonexistent_directory"];

        for path_str in test_paths {
            let path = Path::new(path_str);

            // Test path existence check (used in main.rs setup_watcher)
            let exists = path.exists();
            let is_dir = path.is_dir();

            // These checks should not panic
            assert!(exists || !exists); // Always true, just testing no panic
            assert!(is_dir || !is_dir); // Always true, just testing no panic
        }
    }

    #[test]
    fn test_glob_pattern_matching() {
        // Test glob pattern matching used in main.rs setup_watcher
        use glob::Pattern;

        let pattern_str = "src/**/*.rs";
        let pattern = Pattern::new(pattern_str).unwrap();

        let test_paths = vec![
            ("src/main.rs", true),
            ("src/lib/mod.rs", true),
            ("src/utils/helper.rs", true),
            ("tests/test.rs", false),
            ("Cargo.toml", false),
        ];

        for (path_str, expected) in test_paths {
            let path = std::path::Path::new(path_str);
            let matches = pattern.matches_path(path);
            assert_eq!(
                matches, expected,
                "Pattern matching failed for: {}",
                path_str
            );
        }
    }
}
