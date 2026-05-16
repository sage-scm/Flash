//! End-to-end tests covering the YAML configuration file.

mod common;

use std::fs;
use std::thread;
use std::time::Duration;

use common::*;

#[test]
fn config_supplies_command_and_watch_paths() {
    let workspace = Workspace::new();
    let marker = workspace.marker("marker");
    let config = workspace.root.path().join("flash.yaml");
    fs::write(
        &config,
        format!(
            "command:\n  - 'printf x > {marker}'\nwatch:\n  - {watch}\ndebounce: 10\n",
            marker = marker.display(),
            watch = workspace.watch_dir().display(),
        ),
    )
    .unwrap();

    let mut c = flash();
    c.arg("--fast").args(["-f", config.to_str().unwrap()]);
    let mut child = spawn_silent(c);

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&marker);
    workspace.write("change.txt", "go");

    let fired = wait_for_path(&marker, Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();
    assert!(fired, "config-driven invocation should fire the command");
}

#[test]
fn cli_overrides_command_from_config() {
    let workspace = Workspace::new();
    let from_config = workspace.marker("from-config");
    let from_cli = workspace.marker("from-cli");
    let config = workspace.root.path().join("flash.yaml");

    fs::write(
        &config,
        format!(
            "command:\n  - 'printf x > {fc}'\nwatch:\n  - {watch}\ndebounce: 10\n",
            fc = from_config.display(),
            watch = workspace.watch_dir().display(),
        ),
    )
    .unwrap();

    let mut c = flash();
    c.arg("--fast")
        .args(["-f", config.to_str().unwrap()])
        .arg(format!("printf x > {}", from_cli.display()));
    let mut child = spawn_silent(c);

    thread::sleep(STEADY_STATE);
    let _ = fs::remove_file(&from_config);
    let _ = fs::remove_file(&from_cli);
    workspace.write("change.txt", "go");

    let cli_fired = wait_for_path(&from_cli, Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();

    assert!(
        cli_fired,
        "CLI command should win — expected marker at {}",
        from_cli.display()
    );
    assert!(
        !from_config.exists(),
        "the config's command must not run when the CLI overrides it"
    );
}

#[test]
fn invalid_config_yaml_is_rejected_with_context() {
    let workspace = Workspace::new();
    let config = workspace.root.path().join("flash.yaml");
    fs::write(&config, "command: [unterminated\n").unwrap();

    let output = flash()
        .args(["-f", config.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("parsing config file"),
        "stderr should explain which file failed; got:\n{stderr}"
    );
}

#[test]
fn unknown_config_keys_are_rejected() {
    let workspace = Workspace::new();
    let config = workspace.root.path().join("flash.yaml");
    fs::write(&config, "command:\n  - echo\nmystery_field: yes\n").unwrap();

    let output = flash()
        .args(["-f", config.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        !output.status.success(),
        "unknown keys in config should fail the run"
    );
}
