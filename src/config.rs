use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;

const DEFAULT_DEBOUNCE_MS: u64 = 50;
const DEFAULT_STATS_INTERVAL_SECS: u64 = 10;

/// On-disk YAML representation. Every field is optional so users can write the
/// shortest config that does the job.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
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

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("reading config file '{}'", path.display()))?;
        serde_yaml::from_str(&contents)
            .with_context(|| format!("parsing config file '{}'", path.display()))
    }
}

/// Fully-merged runtime configuration. CLI flags win; the config file fills any
/// values the user did not supply on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub command: Vec<String>,
    pub watch: Vec<String>,
    pub extensions: Vec<String>,
    pub include: Vec<String>,
    pub ignore: Vec<String>,
    pub debounce: Duration,
    pub initial: bool,
    pub clear: bool,
    pub restart: bool,
    pub stats: bool,
    pub stats_interval: Duration,
    pub fast: bool,
}

impl Settings {
    /// Combine CLI arguments with any referenced config file. The CLI is always
    /// authoritative; the config file only fills gaps.
    pub fn build(cli: Cli) -> Result<Self> {
        let config = match cli.config.as_deref() {
            Some(path) => Some(Config::load(path)?),
            None => None,
        };
        Ok(Self::merge(cli, config))
    }

    pub(crate) fn merge(cli: Cli, config: Option<Config>) -> Self {
        let cfg = config.unwrap_or_default();

        let command = if cli.command.is_empty() {
            cfg.command
        } else {
            cli.command
        };

        let watch = if !cli.watch.is_empty() {
            cli.watch
        } else {
            cfg.watch.unwrap_or_else(|| vec![".".to_string()])
        };

        let extensions = cli
            .ext
            .or(cfg.ext)
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        let include = if !cli.pattern.is_empty() {
            cli.pattern
        } else {
            cfg.pattern.unwrap_or_default()
        };

        let ignore = if !cli.ignore.is_empty() {
            cli.ignore
        } else {
            cfg.ignore.unwrap_or_default()
        };

        let debounce_ms = cli.debounce.or(cfg.debounce).unwrap_or(DEFAULT_DEBOUNCE_MS);

        let stats_interval_secs = cli
            .stats_interval
            .or(cfg.stats_interval)
            .unwrap_or(DEFAULT_STATS_INTERVAL_SECS);

        Self {
            command,
            watch,
            extensions,
            include,
            ignore,
            debounce: Duration::from_millis(debounce_ms),
            initial: cli.initial || cfg.initial.unwrap_or(false),
            clear: cli.clear || cfg.clear.unwrap_or(false),
            restart: cli.restart || cfg.restart.unwrap_or(false),
            stats: cli.stats || cfg.stats.unwrap_or(false),
            stats_interval: Duration::from_secs(stats_interval_secs.max(1)),
            fast: cli.fast,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            watch: vec![".".to_string()],
            extensions: Vec::new(),
            include: Vec::new(),
            ignore: Vec::new(),
            debounce: Duration::from_millis(DEFAULT_DEBOUNCE_MS),
            initial: false,
            clear: false,
            restart: false,
            stats: false,
            stats_interval: Duration::from_secs(DEFAULT_STATS_INTERVAL_SECS),
            fast: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn cli() -> Cli {
        Cli {
            command: Vec::new(),
            watch: Vec::new(),
            ext: None,
            pattern: Vec::new(),
            ignore: Vec::new(),
            debounce: None,
            initial: false,
            clear: false,
            restart: false,
            config: None,
            fast: false,
            stats: false,
            stats_interval: None,
            bench: false,
        }
    }

    fn write_config(yaml: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        file
    }

    #[test]
    fn defaults_when_neither_cli_nor_config_provides_values() {
        let s = Settings::merge(cli(), None);
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn extensions_are_split_and_trimmed() {
        let mut c = cli();
        c.ext = Some("rs, toml , md".into());
        let s = Settings::merge(c, None);
        assert_eq!(s.extensions, vec!["rs", "toml", "md"]);
    }

    #[test]
    fn cli_command_wins_over_config_command() {
        let mut c = cli();
        c.command = vec!["cargo".into(), "check".into()];
        let cfg = Config {
            command: vec!["cargo".into(), "test".into()],
            ..Config::default()
        };
        let s = Settings::merge(c, Some(cfg));
        assert_eq!(s.command, vec!["cargo", "check"]);
    }

    #[test]
    fn config_fills_in_when_cli_is_silent() {
        let cfg = Config {
            command: vec!["cargo".into(), "test".into()],
            debounce: Some(250),
            restart: Some(true),
            ext: Some("rs".into()),
            ..Config::default()
        };
        let s = Settings::merge(cli(), Some(cfg));
        assert_eq!(s.command, vec!["cargo", "test"]);
        assert_eq!(s.debounce, Duration::from_millis(250));
        assert!(s.restart);
        assert_eq!(s.extensions, vec!["rs"]);
    }

    #[test]
    fn cli_debounce_overrides_config() {
        let mut c = cli();
        c.debounce = Some(50);
        let cfg = Config {
            debounce: Some(999),
            ..Config::default()
        };
        let s = Settings::merge(c, Some(cfg));
        assert_eq!(s.debounce, Duration::from_millis(50));
    }

    #[test]
    fn cli_boolean_true_sticks_even_when_config_says_false() {
        let mut c = cli();
        c.restart = true;
        let cfg = Config {
            restart: Some(false),
            ..Config::default()
        };
        let s = Settings::merge(c, Some(cfg));
        assert!(s.restart);
    }

    #[test]
    fn config_load_round_trip() {
        let yaml = r#"
command:
  - cargo
  - test
watch:
  - src
ext: "rs"
debounce: 200
restart: true
"#;
        let file = write_config(yaml);
        let config = Config::load(file.path()).unwrap();
        assert_eq!(config.command, vec!["cargo", "test"]);
        assert_eq!(config.watch, Some(vec!["src".to_string()]));
        assert_eq!(config.ext, Some("rs".to_string()));
        assert_eq!(config.debounce, Some(200));
        assert_eq!(config.restart, Some(true));
    }

    #[test]
    fn config_load_rejects_unknown_fields() {
        let file = write_config("command: [echo]\nmystery: 1\n");
        let err = Config::load(file.path()).expect_err("should fail");
        assert!(err.to_string().contains("parsing config file"));
    }

    #[test]
    fn config_load_rejects_invalid_yaml() {
        let file = write_config("command: [unterminated");
        assert!(Config::load(file.path()).is_err());
    }

    #[test]
    fn stats_interval_is_clamped_to_at_least_one_second() {
        let mut c = cli();
        c.stats_interval = Some(0);
        let s = Settings::merge(c, None);
        assert_eq!(s.stats_interval, Duration::from_secs(1));
    }
}
