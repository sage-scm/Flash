use flash_watcher::Args;

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(args.config.is_none());
        assert!(args.ext.is_none());
        assert!(args.pattern.is_empty());
        assert!(args.ignore.is_empty());
    }

    #[test]
    fn test_args_clone() {
        let args1 = Args {
            command: vec!["echo".to_string(), "test".to_string()],
            watch: vec!["src".to_string()],
            ext: Some("rs".to_string()),
            pattern: vec!["*.rs".to_string()],
            ignore: vec!["target".to_string()],
            debounce: 200,
            initial: true,
            clear: true,
            restart: true,
            stats: true,
            stats_interval: 5,
            bench: true,
            config: Some("config.yaml".to_string()),
        };

        let args2 = args1.clone();
        assert_eq!(args1, args2);
    }

    #[test]
    fn test_args_partial_eq() {
        let args1 = Args::default();
        let args2 = Args::default();
        assert_eq!(args1, args2);

        let args3 = Args {
            command: vec!["test".to_string()],
            ..Args::default()
        };
        assert_ne!(args1, args3);
    }

    #[test]
    fn test_config_serialization() {
        use flash_watcher::Config;

        let config = Config {
            command: vec!["cargo".to_string(), "test".to_string()],
            watch: Some(vec!["src".to_string()]),
            ext: Some("rs".to_string()),
            pattern: Some(vec!["*.rs".to_string()]),
            ignore: Some(vec!["target".to_string()]),
            debounce: Some(200),
            initial: Some(true),
            clear: Some(true),
            restart: Some(true),
            stats: Some(true),
            stats_interval: Some(5),
        };

        // Test serialization to YAML
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("command:"));
        assert!(yaml.contains("- cargo"));
        assert!(yaml.contains("- test"));

        // Test deserialization from YAML
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_partial_fields() {
        use flash_watcher::Config;

        let yaml = r#"
command: ["npm", "start"]
ext: "js,ts"
debounce: 300
"#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.command, vec!["npm", "start"]);
        assert_eq!(config.ext, Some("js,ts".to_string()));
        assert_eq!(config.debounce, Some(300));
        assert_eq!(config.watch, None);
        assert_eq!(config.pattern, None);
        assert_eq!(config.ignore, None);
        assert_eq!(config.initial, None);
        assert_eq!(config.clear, None);
        assert_eq!(config.restart, None);
        assert_eq!(config.stats, None);
        assert_eq!(config.stats_interval, None);
    }

    #[test]
    fn test_config_empty_command() {
        use flash_watcher::Config;

        let yaml = r#"
command: []
watch: ["src"]
"#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.command.is_empty());
        assert_eq!(config.watch, Some(vec!["src".to_string()]));
    }

    #[test]
    fn test_config_invalid_yaml() {
        use flash_watcher::Config;

        let invalid_yaml = r#"
command: "not-a-list"
invalid_field: true
[broken yaml
"#;

        let result: Result<Config, _> = serde_yaml::from_str(invalid_yaml);
        assert!(result.is_err());
    }
}
