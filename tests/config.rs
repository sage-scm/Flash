use std::fs;
use std::io::Write;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

// Mock version of the Config struct from main.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// Mock version of Args struct from main.rs
#[derive(Debug, Clone, PartialEq)]
struct Args {
    command: Vec<String>,
    watch: Vec<String>,
    ext: Option<String>,
    pattern: Vec<String>,
    ignore: Vec<String>,
    debounce: u64,
    initial: bool,
    clear: bool,
    restart: bool,
    stats: bool,
    stats_interval: u64,
    bench: bool,
}

// Functions to test
fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;

    serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_config_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    fn default_args() -> Args {
        Args {
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
        }
    }

    #[test]
    fn test_load_config() {
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

        let file = create_config_file(config_yaml);
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
    fn test_merge_config_empty_args() {
        let mut args = default_args();

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
        // Args with CLI-provided values
        let mut args = Args {
            command: vec!["echo".to_string(), "hello".to_string()],
            watch: vec!["src".to_string()], // Not default
            ext: Some("js".to_string()),
            pattern: vec!["custom-pattern".to_string()],
            ignore: vec!["custom-ignore".to_string()],
            debounce: 50, // Not default
            initial: true,
            clear: true,
            restart: true,
            stats: true,
            stats_interval: 15, // Not default
            bench: false,
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
    fn test_merge_config_partial() {
        let mut args = default_args();

        // Only some config values provided
        let config = Config {
            command: vec!["cargo".to_string(), "test".to_string()],
            watch: None,
            ext: Some("rs".to_string()),
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

        assert_eq!(args.command, vec!["cargo", "test"]);
        assert_eq!(args.watch, vec!["."]); // Default unchanged
        assert_eq!(args.ext, Some("rs".to_string()));
        assert_eq!(args.pattern, Vec::<String>::new()); // Default unchanged
        assert_eq!(args.ignore, Vec::<String>::new()); // Default unchanged
        assert_eq!(args.debounce, 100); // Default unchanged
    }

    #[test]
    fn test_load_invalid_config() {
        let invalid_yaml = r#"
command: "not-a-list"
invalid: true
"#;

        let file = create_config_file(invalid_yaml);
        let result: anyhow::Result<Config> = load_config(file.path().to_str().unwrap());

        assert!(result.is_err());
    }
}
