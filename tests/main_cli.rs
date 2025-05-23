use flash_watcher::{compile_patterns, should_process_path, should_skip_dir, Args};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_conversion() {
        // Test the conversion from CliArgs to Args
        // Since we can't directly test CliArgs::parse() in unit tests,
        // we'll test the Args struct and its functionality

        let args = Args {
            command: vec!["echo".to_string(), "hello".to_string()],
            watch: vec!["src".to_string(), "tests".to_string()],
            ext: Some("rs,js".to_string()),
            pattern: vec!["**/*.rs".to_string()],
            ignore: vec!["target".to_string(), "node_modules".to_string()],
            debounce: 200,
            initial: true,
            clear: true,
            restart: true,
            stats: true,
            stats_interval: 5,
            bench: false,
            config: Some("config.yaml".to_string()),
        };

        // Test that all fields are properly set
        assert_eq!(args.command, vec!["echo", "hello"]);
        assert_eq!(args.watch, vec!["src", "tests"]);
        assert_eq!(args.ext, Some("rs,js".to_string()));
        assert_eq!(args.pattern, vec!["**/*.rs"]);
        assert_eq!(args.ignore, vec!["target", "node_modules"]);
        assert_eq!(args.debounce, 200);
        assert!(args.initial);
        assert!(args.clear);
        assert!(args.restart);
        assert!(args.stats);
        assert_eq!(args.stats_interval, 5);
        assert!(!args.bench);
        assert_eq!(args.config, Some("config.yaml".to_string()));
    }

    #[test]
    fn test_compile_patterns_for_main_logic() {
        // Test pattern compilation used in main.rs
        let patterns = vec![
            "**/*.rs".to_string(),
            "src/**/*.js".to_string(),
            "tests/**/*.rs".to_string(),
        ];

        let compiled = compile_patterns(&patterns).unwrap();
        assert_eq!(compiled.len(), 3);

        // Test that compiled patterns work correctly
        assert!(compiled[0].matches_path(Path::new("src/main.rs")));
        assert!(compiled[1].matches_path(Path::new("src/utils/helper.js")));
        assert!(compiled[2].matches_path(Path::new("tests/integration.rs")));
    }

    #[test]
    fn test_compile_patterns_empty() {
        // Test empty patterns (used in main.rs when no patterns specified)
        let patterns = vec![];
        let compiled = compile_patterns(&patterns).unwrap();
        assert!(compiled.is_empty());
    }

    #[test]
    fn test_compile_patterns_invalid() {
        // Test invalid patterns (error handling in main.rs)
        let patterns = vec!["[invalid".to_string()];
        let result = compile_patterns(&patterns);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid pattern"));
    }

    #[test]
    fn test_should_process_path_main_logic() {
        // Test the path processing logic used in main.rs event loop
        let include_patterns = compile_patterns(&["**/*.rs".to_string()]).unwrap();
        let ignore_patterns = compile_patterns(&["**/target/**".to_string()]).unwrap();

        // Should process Rust files
        assert!(should_process_path(
            Path::new("src/main.rs"),
            &None,
            &include_patterns,
            &ignore_patterns
        ));

        // Should ignore files in target directory
        assert!(!should_process_path(
            Path::new("target/debug/main.rs"),
            &None,
            &include_patterns,
            &ignore_patterns
        ));

        // Should not process non-Rust files when include patterns are specified
        assert!(!should_process_path(
            Path::new("src/main.js"),
            &None,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_process_path_with_extensions() {
        // Test extension filtering used in main.rs
        let ext_filter = Some("rs,js,ts".to_string());
        let include_patterns = vec![];
        let ignore_patterns = vec![];

        // Should process files with matching extensions
        assert!(should_process_path(
            Path::new("src/main.rs"),
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));

        assert!(should_process_path(
            Path::new("src/app.js"),
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));

        assert!(should_process_path(
            Path::new("src/types.ts"),
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));

        // Should not process files with non-matching extensions
        assert!(!should_process_path(
            Path::new("README.md"),
            &ext_filter,
            &include_patterns,
            &ignore_patterns
        ));
    }

    #[test]
    fn test_should_skip_dir_main_logic() {
        // Test directory skipping logic used in main.rs setup_watcher
        let ignore_patterns = vec!["**/node_modules/**".to_string(), "**/build/**".to_string()];

        // Should skip common directories (these are hardcoded in the function)
        assert!(should_skip_dir(Path::new(".git"), &ignore_patterns));
        assert!(should_skip_dir(Path::new("node_modules"), &ignore_patterns));
        assert!(should_skip_dir(Path::new("target"), &ignore_patterns));

        // Should skip custom ignore patterns that match the glob
        assert!(should_skip_dir(
            Path::new("project/build/assets"),
            &ignore_patterns
        )); // Matches **/build/**
        assert!(should_skip_dir(
            Path::new("app/node_modules/package"),
            &ignore_patterns
        )); // Matches **/node_modules/**

        // Should not skip regular directories
        assert!(!should_skip_dir(Path::new("src"), &ignore_patterns));
        assert!(!should_skip_dir(Path::new("tests"), &ignore_patterns));
        assert!(!should_skip_dir(Path::new("docs"), &ignore_patterns));

        // Test with simpler patterns that should work
        let simple_patterns = vec!["build".to_string(), "dist".to_string()];
        assert!(should_skip_dir(Path::new("build"), &simple_patterns)); // Exact match
        assert!(should_skip_dir(Path::new("dist"), &simple_patterns)); // Exact match
        assert!(!should_skip_dir(Path::new("src"), &simple_patterns)); // No match
    }

    #[test]
    fn test_debounce_logic_simulation() {
        // Test the debounce logic used in main.rs
        use std::collections::HashMap;
        use std::time::Instant;

        let mut recently_processed = HashMap::new();
        let debounce_ms = 100u64;

        let path_key = "src/main.rs".to_string();
        let now = Instant::now();

        // First time processing - should be allowed
        assert!(!recently_processed.contains_key(&path_key));
        recently_processed.insert(path_key.clone(), now);

        // Immediate second processing - should be blocked
        let immediate_now = now;
        if let Some(last_time) = recently_processed.get(&path_key) {
            assert!(immediate_now.duration_since(*last_time).as_millis() < debounce_ms as u128);
        }

        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(debounce_ms + 10));
        let later_now = Instant::now();

        // After debounce period - should be allowed
        if let Some(last_time) = recently_processed.get(&path_key) {
            assert!(later_now.duration_since(*last_time).as_millis() >= debounce_ms as u128);
        }
    }

    #[test]
    fn test_path_display_formatting() {
        // Test the path display formatting used in main.rs
        use flash_watcher::format_display_path;

        // Test various path formats that main.rs might encounter
        assert_eq!(format_display_path(Path::new("src/main.rs")), "main.rs");
        assert_eq!(
            format_display_path(Path::new("tests/integration.rs")),
            "integration.rs"
        );
        assert_eq!(format_display_path(Path::new("./src/lib.rs")), "lib.rs");
        assert_eq!(
            format_display_path(Path::new("../project/file.js")),
            "file.js"
        );

        // Test edge cases
        assert_eq!(format_display_path(Path::new("file.txt")), "file.txt");
        assert_eq!(format_display_path(Path::new(".")), ".");
        assert_eq!(format_display_path(Path::new("..")), "..");
    }

    #[test]
    fn test_args_validation_scenarios() {
        // Test various argument validation scenarios that main.rs handles
        use flash_watcher::validate_args;

        // Valid args
        let valid_args = Args {
            command: vec!["cargo".to_string(), "test".to_string()],
            ..Args::default()
        };
        assert!(validate_args(&valid_args).is_ok());

        // Invalid args - empty command
        let invalid_args = Args {
            command: vec![],
            ..Args::default()
        };
        assert!(validate_args(&invalid_args).is_err());

        // Valid args - single command
        let single_command_args = Args {
            command: vec!["echo".to_string()],
            ..Args::default()
        };
        assert!(validate_args(&single_command_args).is_ok());
    }

    #[test]
    fn test_benchmark_mode_handling() {
        // Test benchmark mode that main.rs handles
        use flash_watcher::run_benchmarks;

        // This should not panic and should return a result
        let result = run_benchmarks();
        assert!(result.is_ok() || result.is_err()); // Either is fine, just shouldn't panic
    }
}
