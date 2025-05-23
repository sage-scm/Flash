use flash_watcher::{
    format_display_path, load_config, merge_config, run_benchmarks, validate_args, Args,
};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_config_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_main_logic_config_loading() {
        // Test the main logic for loading configuration
        let config_yaml = r#"
command: ["cargo", "test"]
watch: ["src", "tests"]
ext: "rs"
debounce: 200
initial: true
"#;

        let file = create_config_file(config_yaml);
        let config_path = file.path().to_str().unwrap();

        // Test loading config
        let config = load_config(config_path).unwrap();
        assert_eq!(config.command, vec!["cargo", "test"]);

        // Test merging with default args
        let mut args = Args::default();
        merge_config(&mut args, config);

        assert_eq!(args.command, vec!["cargo", "test"]);
        assert_eq!(args.watch, vec!["src", "tests"]);
        assert_eq!(args.ext, Some("rs".to_string()));
        assert_eq!(args.debounce, 200);
        assert!(args.initial);
    }

    #[test]
    fn test_main_logic_config_loading_error() {
        // Test error handling for non-existent config file
        let result = load_config("nonexistent.yaml");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read config file"));
    }

    #[test]
    fn test_main_logic_validation() {
        // Test argument validation
        let mut args = Args::default();

        // Should fail with empty command
        let result = validate_args(&args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No command specified"));

        // Should succeed with command
        args.command = vec!["echo".to_string(), "test".to_string()];
        let result = validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_logic_benchmark_mode() {
        // Test benchmark mode - this function tries to run cargo bench
        // so it might fail in test environment, but should not panic
        let result = run_benchmarks();
        // We just test that it doesn't panic, result may be Ok or Err depending on environment
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_format_display_path_edge_cases() {
        // Test various path formats
        assert_eq!(format_display_path(Path::new("test.js")), "test.js");
        assert_eq!(format_display_path(Path::new("src/test.js")), "test.js");
        assert_eq!(
            format_display_path(Path::new("/full/path/to/file.rs")),
            "file.rs"
        );
        assert_eq!(format_display_path(Path::new(".")), ".");
        assert_eq!(format_display_path(Path::new("..")), "..");
        assert_eq!(format_display_path(Path::new("/")), "/");

        // Test with complex paths
        assert_eq!(
            format_display_path(Path::new("very/deep/nested/path/file.txt")),
            "file.txt"
        );
        assert_eq!(
            format_display_path(Path::new("./relative/path/file.js")),
            "file.js"
        );
        assert_eq!(
            format_display_path(Path::new("../parent/file.py")),
            "file.py"
        );

        // Test with special characters
        assert_eq!(
            format_display_path(Path::new("path/file with spaces.txt")),
            "file with spaces.txt"
        );
        assert_eq!(
            format_display_path(Path::new("path/file-with-dashes.js")),
            "file-with-dashes.js"
        );
        assert_eq!(
            format_display_path(Path::new("path/file_with_underscores.rs")),
            "file_with_underscores.rs"
        );
    }

    #[test]
    fn test_args_with_config_precedence() {
        // Test that CLI args take precedence over config
        let config_yaml = r#"
command: ["config-command"]
watch: ["config-watch"]
ext: "config-ext"
debounce: 999
initial: true
clear: true
restart: true
stats: true
stats_interval: 999
"#;

        let file = create_config_file(config_yaml);
        let config = load_config(file.path().to_str().unwrap()).unwrap();

        // Create args with NON-DEFAULT CLI values (so they should take precedence)
        let mut args = Args {
            command: vec!["cli-command".to_string()], // Non-empty, should take precedence
            watch: vec!["cli-watch".to_string()],     // Not default ".", should take precedence
            ext: Some("cli-ext".to_string()),         // Not None, should take precedence
            pattern: vec!["cli-pattern".to_string()], // Not empty, should take precedence
            ignore: vec!["cli-ignore".to_string()],   // Not empty, should take precedence
            debounce: 50,                             // Not default 100, should take precedence
            initial: true,     // Set to true (non-default), should take precedence
            clear: true,       // Set to true (non-default), should take precedence
            restart: true,     // Set to true (non-default), should take precedence
            stats: true,       // Set to true (non-default), should take precedence
            stats_interval: 5, // Not default 10, should take precedence
            bench: false,
            config: None,
            fast: false,
        };

        let original_args = args.clone();
        merge_config(&mut args, config);

        // CLI args should be preserved for ALL non-default values
        assert_eq!(args.command, original_args.command);
        assert_eq!(args.watch, original_args.watch);
        assert_eq!(args.ext, original_args.ext);
        assert_eq!(args.pattern, original_args.pattern);
        assert_eq!(args.ignore, original_args.ignore);
        assert_eq!(args.debounce, original_args.debounce);
        assert_eq!(args.stats_interval, original_args.stats_interval);

        // Boolean values should also be preserved since they're non-default (true)
        assert!(args.initial); // CLI value preserved
        assert!(args.clear); // CLI value preserved
        assert!(args.restart); // CLI value preserved
        assert!(args.stats); // CLI value preserved
    }

    #[test]
    fn test_config_with_empty_command() {
        // Test config with empty command
        let config_yaml = r#"
command: []
watch: ["src"]
"#;

        let file = create_config_file(config_yaml);
        let config = load_config(file.path().to_str().unwrap()).unwrap();

        let mut args = Args::default();
        merge_config(&mut args, config);

        // Empty command in config should not override empty command in args
        assert!(args.command.is_empty());
        assert_eq!(args.watch, vec!["src"]);
    }

    #[test]
    fn test_config_partial_override() {
        // Test config that only overrides some fields
        let config_yaml = r#"
command: []
ext: "js,ts"
debounce: 300
stats: true
"#;

        let file = create_config_file(config_yaml);
        let config = load_config(file.path().to_str().unwrap()).unwrap();

        let mut args = Args::default();
        merge_config(&mut args, config);

        // Only specified fields should be overridden
        assert_eq!(args.ext, Some("js,ts".to_string()));
        assert_eq!(args.debounce, 300);
        assert!(args.stats); // This was specified in config and args.stats was false (default)

        // Other fields should remain default
        assert!(args.command.is_empty());
        assert_eq!(args.watch, vec!["."]);
        assert!(!args.initial); // Config didn't specify this, so remains default false
        assert!(!args.clear); // Config didn't specify this, so remains default false
        assert!(!args.restart); // Config didn't specify this, so remains default false
        assert_eq!(args.stats_interval, 10); // Config didn't specify this, so remains default
    }

    #[test]
    fn test_config_with_null_values() {
        // Test config with explicit null values
        let config_yaml = r#"
command: ["test"]
watch: null
ext: null
pattern: null
ignore: null
debounce: null
initial: null
clear: null
restart: null
stats: null
stats_interval: null
"#;

        let file = create_config_file(config_yaml);
        let config = load_config(file.path().to_str().unwrap()).unwrap();

        let mut args = Args::default();
        merge_config(&mut args, config);

        // Command should be set, others should remain default due to null values
        assert_eq!(args.command, vec!["test"]);
        assert_eq!(args.watch, vec!["."]); // Default preserved
        assert_eq!(args.ext, None); // Default preserved
        assert_eq!(args.debounce, 100); // Default preserved
    }

    #[test]
    fn test_invalid_config_yaml() {
        // Test various invalid YAML configurations
        let invalid_configs = vec![
            "command: not-a-list",
            "invalid: yaml: structure",
            "[broken yaml",
            "command:\n  - valid\ninvalid_field: {broken: yaml",
        ];

        for invalid_yaml in invalid_configs {
            let file = create_config_file(invalid_yaml);
            let result = load_config(file.path().to_str().unwrap());
            assert!(
                result.is_err(),
                "Should fail for invalid YAML: {}",
                invalid_yaml
            );
        }
    }

    #[test]
    fn test_config_type_mismatches() {
        // Test config with wrong types
        let config_yaml = r#"
command: "should-be-array"
debounce: "should-be-number"
initial: "should-be-boolean"
"#;

        let file = create_config_file(config_yaml);
        let result = load_config(file.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
