//! Behavioral tests for the `flash-watcher` binary's CLI surface.
//!
//! These tests run the compiled binary and assert on user-visible behavior —
//! exit codes, error messages, and help output — so issues like #1 (a missing
//! watch path silently falling back to cwd) cannot regress unnoticed.

mod common;

use std::time::Duration;

use common::*;
use tempfile::TempDir;

#[test]
fn help_lists_every_documented_flag() {
    let output = flash().arg("--help").output().expect("spawn");
    assert!(output.status.success(), "--help should exit zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--watch",
        "--ext",
        "--pattern",
        "--ignore",
        "--debounce",
        "--initial",
        "--clear",
        "--restart",
        "--config",
        "--fast",
        "--stats",
        "--bench",
    ] {
        assert!(stdout.contains(flag), "--help missing {flag}:\n{stdout}");
    }
}

#[test]
fn version_prints_a_semver_string() {
    let output = flash().arg("--version").output().expect("spawn");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .split_whitespace()
            .any(|tok| tok.split('.').count() == 3 && tok.split('.').all(|p| !p.is_empty())),
        "expected a semver token in: {stdout:?}"
    );
}

#[test]
fn unknown_flag_is_rejected_with_clap_styled_error() {
    let output = flash()
        .args(["--definitely-not-a-real-flag", "echo", "hi"])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected") || stderr.contains("unrecognized"),
        "stderr should mention the unknown argument; got:\n{stderr}"
    );
}

#[test]
fn missing_command_errors_with_a_helpful_message() {
    let tmp = TempDir::new().unwrap();
    let output = flash()
        .args(["-w", tmp.path().to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("no command"),
        "stderr should mention a missing command; got:\n{stderr}"
    );
}

#[test]
fn missing_watch_path_errors_instead_of_silently_watching_cwd() {
    // Regression for issue #1: previously, a missing -w argument fell through
    // to watching the current working directory, which caused infinite loops
    // when the command also wrote files.
    let bogus = "flash-test-this-path-does-not-exist-7f3a2";
    let output = flash()
        .args(["-w", bogus, "echo", "should", "not", "run"])
        .output()
        .expect("spawn flash-watcher");

    assert!(
        !output.status.success(),
        "expected non-zero exit when -w points to a missing path"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(bogus) && stderr.contains("does not exist"),
        "stderr should name the missing path; got:\n{stderr}"
    );
}

#[test]
fn missing_watch_path_exits_quickly_and_does_not_run_the_command() {
    // The original bug landed Flash in an infinite loop; make sure the
    // process exits promptly and that any side-effect command never fires.
    let workspace = Workspace::new();
    let marker = workspace.marker("should-never-appear");
    let cmd = format!("printf x > {}", marker.display());

    let mut child = spawn_silent({
        let mut c = flash();
        c.args(["-w", "definitely-not-a-real-watch-path"]).arg(&cmd);
        c
    });

    let status = wait_for_exit(&mut child, Duration::from_secs(2))
        .expect("flash should exit promptly on validation error");
    assert!(!status.success(), "expected non-zero exit");
    assert!(
        !marker.exists(),
        "command must not have been run before the validation error"
    );
}

#[test]
fn glob_with_existing_root_is_accepted() {
    let workspace = Workspace::new();
    let pattern = format!("{}/**/*.rs", workspace.watch_dir().display());
    let marker = workspace.marker("marker");

    let mut child = spawn_silent({
        let mut c = flash();
        c.arg("--fast")
            .args(["--debounce", "10"])
            .args(["-w", &pattern])
            .arg(format!("printf x > {}", marker.display()));
        c
    });

    std::thread::sleep(STEADY_STATE);
    let _ = std::fs::remove_file(&marker);
    workspace.write("module.rs", "fn x() {}");

    let saw = wait_for_path(&marker, Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();
    assert!(saw, "glob watcher should fire on a matching file");
}

#[test]
fn glob_with_missing_root_is_rejected() {
    let output = flash()
        .args(["-w", "no-such-directory/**/*.rs", "echo"])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no existing root"),
        "stderr should call out the bad glob root; got:\n{stderr}"
    );
}

#[test]
fn invalid_glob_pattern_is_rejected() {
    let workspace = Workspace::new();
    let output = flash()
        .args(["-w", &workspace.watch_str()])
        .args(["-p", "[unterminated"])
        .arg("echo")
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid glob") || stderr.contains("compiling include patterns"),
        "stderr should explain the bad pattern; got:\n{stderr}"
    );
}

#[test]
fn invalid_config_path_is_rejected() {
    let output = flash()
        .args(["-f", "this-config-does-not-exist.yaml"])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reading config file"),
        "stderr should mention reading the config file; got:\n{stderr}"
    );
}
