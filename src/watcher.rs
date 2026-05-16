use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

use crate::cli::Cli;
use crate::config::Settings;
use crate::filter::Filter;
use crate::runner::Runner;
use crate::stats::Stats;

/// Entry point used by both the binary and the integration tests.
///
/// Loads [`Settings`], validates them, sets up a debounced watcher, and runs
/// the event loop until the channel from notify closes (i.e. the watcher is
/// dropped or the process is killed).
pub fn run(cli: Cli) -> Result<()> {
    if cli.bench {
        return crate::bench::run();
    }

    let settings = Settings::build(cli)?;
    if settings.command.is_empty() {
        bail!("no command specified — pass one after the flags, or set `command:` in your config");
    }

    let filter = Filter::new(&settings.extensions, &settings.include, &settings.ignore)?;
    let watch_roots = resolve_watch_roots(&settings.watch)?;

    let stats = settings.stats.then(|| Arc::new(Mutex::new(Stats::new())));
    if let Some(stats) = stats.clone() {
        spawn_stats_thread(stats, settings.stats_interval);
    }

    print_banner(&settings, &watch_roots);

    let mut runner = Runner::new(settings.command.clone(), settings.restart, settings.clear);
    if settings.initial {
        if let Err(err) = runner.run() {
            eprintln!("flash-watcher: initial run failed: {err:#}");
        }
    }

    let (tx, rx) = channel::<PathBuf>();
    let stats_for_events = stats.clone();
    let mut debouncer =
        new_debouncer(
            settings.debounce,
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    if let Some(stats) = stats_for_events.as_ref() {
                        if let Ok(mut s) = stats.lock() {
                            for _ in &events {
                                s.record_event();
                            }
                        }
                    }
                    for event in events {
                        let _ = tx.send(event.path);
                    }
                }
                Err(err) => eprintln!("flash-watcher: watcher error: {err}"),
            },
        )
        .context("creating debounced watcher")?;

    for root in &watch_roots {
        debouncer
            .watcher()
            .watch(root, RecursiveMode::Recursive)
            .with_context(|| format!("watching '{}'", root.display()))?;
    }

    if !settings.fast {
        println!("{}", "ready · waiting for changes".bright_green());
    }

    loop {
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(path) => {
                if !filter.accepts(&path) {
                    continue;
                }
                if let Some(stats) = stats.as_ref() {
                    if let Ok(mut s) = stats.lock() {
                        s.record_change();
                    }
                }
                if !settings.fast && !settings.stats {
                    println!(
                        "{}  {}",
                        "↻".bright_blue(),
                        display_path(&path).bright_white()
                    );
                }
                if let Err(err) = runner.run() {
                    eprintln!("flash-watcher: command failed: {err:#}");
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

/// Translate the user's `--watch` arguments into actual paths to hand to
/// `notify`. Globs are accepted: their fixed prefix becomes the root, and the
/// pattern itself is enforced by the [`Filter`]. Literal paths that do not
/// exist are an error — issue #1.
fn resolve_watch_roots(specs: &[String]) -> Result<Vec<PathBuf>> {
    if specs.is_empty() {
        return Ok(vec![PathBuf::from(".")]);
    }

    let mut roots: Vec<PathBuf> = Vec::with_capacity(specs.len());
    for spec in specs {
        let root = if looks_like_glob(spec) {
            glob_root(spec)
        } else {
            PathBuf::from(spec)
        };

        if !root.exists() {
            if looks_like_glob(spec) {
                bail!(
                    "watch pattern '{spec}' has no existing root directory (tried '{}')",
                    root.display()
                );
            }
            bail!("watch path '{spec}' does not exist");
        }

        let canonical = root.canonicalize().unwrap_or(root);
        if !roots.iter().any(|p| p == &canonical) {
            roots.push(canonical);
        }
    }
    Ok(roots)
}

fn looks_like_glob(spec: &str) -> bool {
    spec.chars().any(|c| matches!(c, '*' | '?' | '[' | '{'))
}

/// Take the longest leading prefix of the glob that contains no wildcard
/// characters. `src/**/*.rs` -> `src`; `**/foo` -> `.`; `a/b/*.c` -> `a/b`.
fn glob_root(pattern: &str) -> PathBuf {
    let mut root = PathBuf::new();
    for component in Path::new(pattern).components() {
        let s = component.as_os_str().to_string_lossy();
        if looks_like_glob(&s) {
            break;
        }
        root.push(component);
    }
    if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root
    }
}

fn display_path(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(stripped) = path.strip_prefix(&cwd) {
            return stripped.display().to_string();
        }
    }
    path.display().to_string()
}

fn print_banner(settings: &Settings, roots: &[PathBuf]) {
    if settings.fast || settings.stats {
        return;
    }
    println!("{}", "flash · watching for changes".bright_green().bold());
    for root in roots {
        println!("  {} {}", "•".bright_blue(), display_path(root));
    }
    if !settings.extensions.is_empty() {
        println!(
            "  {} {}",
            "ext".bright_blue(),
            settings.extensions.join(",")
        );
    }
    if !settings.include.is_empty() {
        println!(
            "  {} {}",
            "include".bright_blue(),
            settings.include.join(", ")
        );
    }
    if !settings.ignore.is_empty() {
        println!(
            "  {} {}",
            "ignore".bright_blue(),
            settings.ignore.join(", ")
        );
    }
    println!(
        "  {} {}",
        "run".bright_blue(),
        settings.command.join(" ").bright_yellow()
    );
}

fn spawn_stats_thread(stats: Arc<Mutex<Stats>>, interval: Duration) {
    thread::spawn(move || loop {
        thread::sleep(interval);
        let mut s = match stats.lock() {
            Ok(s) => s,
            Err(_) => break,
        };
        s.refresh();
        println!("{}", s.render());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn looks_like_glob_detects_wildcards() {
        assert!(looks_like_glob("src/**/*.rs"));
        assert!(looks_like_glob("a?b"));
        assert!(looks_like_glob("a[bc]"));
        assert!(looks_like_glob("a/{b,c}.txt"));
        assert!(!looks_like_glob("plain/path"));
    }

    #[test]
    fn glob_root_takes_fixed_prefix() {
        assert_eq!(glob_root("src/**/*.rs"), PathBuf::from("src"));
        assert_eq!(glob_root("a/b/c/*.x"), PathBuf::from("a/b/c"));
        assert_eq!(glob_root("**/*.rs"), PathBuf::from("."));
        assert_eq!(glob_root("*.rs"), PathBuf::from("."));
    }

    #[test]
    fn resolve_watch_roots_errors_on_missing_path() {
        let err = resolve_watch_roots(&["definitely-does-not-exist-12345".into()])
            .expect_err("should reject missing path");
        let msg = err.to_string();
        assert!(
            msg.contains("does not exist"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn resolve_watch_roots_accepts_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let resolved = resolve_watch_roots(&[tmp.path().to_string_lossy().into_owned()]).unwrap();
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn resolve_watch_roots_dedupes() {
        let tmp = TempDir::new().unwrap();
        let s = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_watch_roots(&[s.clone(), s]).unwrap();
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn resolve_watch_roots_accepts_glob_with_existing_prefix() {
        let tmp = TempDir::new().unwrap();
        let pattern = format!("{}/**/*.rs", tmp.path().display());
        let resolved = resolve_watch_roots(&[pattern]).unwrap();
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn resolve_watch_roots_rejects_glob_with_missing_prefix() {
        let err = resolve_watch_roots(&["nope/**/*.rs".into()])
            .expect_err("missing glob prefix should error");
        assert!(err.to_string().contains("no existing root"));
    }
}
