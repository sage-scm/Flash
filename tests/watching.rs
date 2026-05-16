//! End-to-end watching behavior. These tests exercise the binary against a
//! real file system so user-visible regressions (debouncing, filtering,
//! restart behavior, multi-root watches) surface in CI rather than in the
//! field.

mod common;

use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use common::*;

const MAX_E2E: Duration = Duration::from_secs(5);

fn touch_marker_cmd(path: &std::path::Path) -> String {
    format!("printf x >> {}", path.display())
}

/// Build the standard "watch this dir, append to marker on every change"
/// invocation. Keeps the boilerplate out of each test.
fn watcher_command(watch: &str, marker: &std::path::Path, extra: &[&str]) -> Command {
    let mut c = flash();
    c.arg("--fast").args(["--debounce", "10"]);
    for arg in extra {
        c.arg(arg);
    }
    c.args(["-w", watch]).arg(touch_marker_cmd(marker));
    c
}

#[test]
fn file_modification_triggers_the_command() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let mut child = spawn_silent(watcher_command(&workspace.watch_str(), &marker, &[]));

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);

    workspace.write("hello.txt", "v1");

    let fired = wait_for_path(&marker, MAX_E2E);
    let _ = child.kill();
    let _ = child.wait();
    assert!(fired, "modifying a file should run the command");
}

#[test]
fn file_creation_triggers_the_command() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let mut child = spawn_silent(watcher_command(&workspace.watch_str(), &marker, &[]));

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);

    workspace.write("brand-new.txt", "hi");

    let fired = wait_for_path(&marker, MAX_E2E);
    let _ = child.kill();
    let _ = child.wait();
    assert!(fired, "creating a new file should run the command");
}

#[test]
fn extension_filter_keeps_unrelated_files_quiet() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let mut child = spawn_silent(watcher_command(
        &workspace.watch_str(),
        &marker,
        &["-e", "rs"],
    ));

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);

    workspace.write("ignored.md", "no thanks");
    let quiet = wait_for_path_absent(&marker, Duration::from_millis(700));

    workspace.write("kept.rs", "fn main() {}");
    let fired = wait_for_path(&marker, MAX_E2E);

    let _ = child.kill();
    let _ = child.wait();
    assert!(quiet, "extension filter should suppress .md changes");
    assert!(fired, "extension filter should still admit .rs changes");
}

#[test]
fn ignore_pattern_keeps_matching_paths_quiet() {
    let workspace = Workspace::new();
    // Pre-create the ignored directory so its creation event doesn't race
    // with the watcher — this mirrors how users typically ignore long-lived
    // directories like `node_modules` or `target`.
    let skip_dir = workspace.watch_dir().join("skip");
    fs::create_dir(&skip_dir).unwrap();

    let marker = workspace.marker("marker");
    let mut child = spawn_silent(watcher_command(
        &workspace.watch_str(),
        &marker,
        &["-i", "**/skip/**"],
    ));

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);

    fs::write(skip_dir.join("inside.txt"), "should be ignored").unwrap();
    let quiet = wait_for_path_absent(&marker, Duration::from_millis(700));

    workspace.write("not-skipped.txt", "should fire");
    let fired = wait_for_path(&marker, MAX_E2E);

    let _ = child.kill();
    let _ = child.wait();
    assert!(
        quiet,
        "ignored directory contents should not fire the command"
    );
    assert!(fired, "non-ignored change should still fire");
}

#[test]
fn initial_flag_runs_the_command_before_any_change() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let mut child = spawn_silent(watcher_command(
        &workspace.watch_str(),
        &marker,
        &["--initial"],
    ));

    let fired = wait_for_path(&marker, Duration::from_secs(3));
    let _ = child.kill();
    let _ = child.wait();
    assert!(fired, "--initial should run the command once at startup");
}

#[test]
fn debounce_window_coalesces_bursty_writes() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    // A wide debounce window keeps the test stable on slow CI runners — the
    // entire burst has to land inside it, otherwise this test devolves into
    // a measurement of the runner's scheduling jitter.
    let debounce_ms = 600;
    let write_count = 8;

    let mut c = flash();
    c.arg("--fast")
        .args(["--debounce", &debounce_ms.to_string()])
        .args(["-w", &workspace.watch_str()])
        .arg(touch_marker_cmd(&marker));
    let mut child = spawn_silent(c);

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);

    // Burst of writes with no sleeping between them — guarantees they fit
    // inside one debounce window regardless of how slow the host is.
    for i in 0..write_count {
        workspace.write("burst.txt", &format!("v{i}"));
    }

    // Wait for the debounce window to close plus enough slack for the command
    // to actually run.
    thread::sleep(Duration::from_millis(debounce_ms + 600));

    let _ = child.kill();
    let _ = child.wait();

    let bytes = fs::read(&marker).unwrap_or_default();
    assert!(
        !bytes.is_empty(),
        "debouncer should still have fired the command at least once"
    );
    assert!(
        bytes.len() < write_count,
        "burst of {write_count} writes should coalesce into fewer runs; saw {} runs",
        bytes.len()
    );
}

#[test]
fn multiple_watch_roots_are_all_observed() {
    let workspace = Workspace::new();
    let extra_dir = workspace.root.path().join("extra");
    fs::create_dir(&extra_dir).unwrap();
    fs::write(extra_dir.join("seed.txt"), "seed").unwrap();
    let marker = workspace.marker("marker");

    let mut c = flash();
    c.arg("--fast")
        .args(["--debounce", "10"])
        .args(["-w", &workspace.watch_str()])
        .args(["-w", extra_dir.to_str().unwrap()])
        .arg(touch_marker_cmd(&marker));
    let mut child = spawn_silent(c);

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);

    // Change only the *second* watch root and verify the command still fires.
    fs::write(extra_dir.join("trigger.txt"), "v1").unwrap();

    let fired = wait_for_path(&marker, MAX_E2E);
    let _ = child.kill();
    let _ = child.wait();
    assert!(fired, "second watch root should also be observed");
}

#[test]
fn restart_mode_kills_the_previous_long_running_child() {
    let workspace = Workspace::new();
    let stamp = workspace.marker("stamp");

    // The command sleeps long enough that, without restart-mode, a second
    // change would be running while the first is still asleep. Restart mode
    // must kill the first process before launching the second.
    let cmd = format!("echo run >> {stamp} && sleep 2", stamp = stamp.display());
    let mut c = flash();
    c.arg("--fast")
        .args(["--debounce", "10"])
        .arg("--restart")
        .args(["-w", &workspace.watch_str()])
        .arg(&cmd);
    let mut child = spawn_silent(c);

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&stamp);

    workspace.write("a.txt", "1");
    thread::sleep(Duration::from_millis(400));
    workspace.write("a.txt", "2");
    thread::sleep(Duration::from_millis(400));

    let _ = child.kill();
    let _ = child.wait();

    // Two changes ⇒ at least two `run` lines, but each previous run is killed
    // before completing the `sleep`. We just want to see that the command ran
    // multiple times without piling up children indefinitely.
    let log = fs::read_to_string(&stamp).unwrap_or_default();
    let runs = log.lines().filter(|l| l.trim() == "run").count();
    assert!(
        runs >= 2,
        "restart mode should have launched the command at least twice; saw:\n{log}"
    );
}

#[test]
fn fast_mode_does_not_print_the_banner() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let mut c = flash();
    c.arg("--fast")
        .args(["--debounce", "10"])
        .args(["-w", &workspace.watch_str()])
        .arg(touch_marker_cmd(&marker));
    let mut child = spawn_capturing(c);

    thread::sleep(Duration::from_millis(800));
    let _ = child.kill();
    let output = child.wait_with_output().expect("collect output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("watching for changes"),
        "--fast should suppress the banner; got:\n{stdout}"
    );
}

#[test]
fn default_mode_prints_a_banner_with_watch_roots() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let mut c = flash();
    c.args(["--debounce", "10"])
        .args(["-w", &workspace.watch_str()])
        .arg(touch_marker_cmd(&marker));
    let mut child = spawn_capturing(c);

    // Give the banner a moment to print.
    let banner_deadline = Instant::now() + Duration::from_millis(800);
    while Instant::now() < banner_deadline {
        thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let output = child.wait_with_output().expect("collect output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("watching for changes"),
        "default banner missing; got:\n{stdout}"
    );
}
