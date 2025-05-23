use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sysinfo::System;
use tempfile::TempDir;
use which::which;

// Time to wait for file watchers to initialize
const STARTUP_WAIT_MS: u64 = 1000;
// Number of file changes to test
const FILE_CHANGES: usize = 10;
// Time between file changes
const CHANGE_INTERVAL_MS: u64 = 500;

/// Available file watchers to benchmark
enum Watcher {
    Flash,
    Nodemon,
    Watchexec,
    Cargo,
}

impl Watcher {
    fn command(&self, dir: &Path) -> Option<Command> {
        match self {
            Watcher::Flash => {
                let mut cmd =
                    Command::new(std::env::current_dir().ok()?.join("target/release/flash"));
                cmd.args(["--watch", dir.to_str()?]);
                cmd.arg("echo");
                cmd.arg("change");
                Some(cmd)
            }
            Watcher::Nodemon => {
                if which("nodemon").is_err() {
                    return None;
                }
                let mut cmd = Command::new("nodemon");
                cmd.args(["--watch", dir.to_str()?, "--exec"]);
                cmd.arg("echo change");
                Some(cmd)
            }
            Watcher::Watchexec => {
                if which("watchexec").is_err() {
                    return None;
                }
                let mut cmd = Command::new("watchexec");
                cmd.args(["--watch", dir.to_str()?, "--"]);
                cmd.arg("echo");
                cmd.arg("change");
                Some(cmd)
            }
            Watcher::Cargo => {
                if which("cargo").is_err() {
                    return None;
                }
                let mut cmd = Command::new("cargo");
                cmd.current_dir(dir);
                cmd.arg("watch");
                cmd.arg("--exec");
                cmd.arg("echo change");
                Some(cmd)
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Watcher::Flash => "flash",
            Watcher::Nodemon => "nodemon",
            Watcher::Watchexec => "watchexec",
            Watcher::Cargo => "cargo-watch",
        }
    }
}

/// Benchmark startup time of file watchers
fn bench_startup(c: &mut Criterion) {
    // Create a temp dir for testing
    let temp_dir = TempDir::new().unwrap();

    // Build Flash in release mode first
    Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .expect("Failed to build Flash in release mode");

    let mut group = c.benchmark_group("startup_time");

    for watcher in [
        Watcher::Flash,
        Watcher::Nodemon,
        Watcher::Watchexec,
        Watcher::Cargo,
    ] {
        if let Some(mut cmd) = watcher.command(temp_dir.path()) {
            group.bench_with_input(
                BenchmarkId::from_parameter(watcher.name()),
                &watcher,
                |b, _| {
                    b.iter(|| {
                        let start = Instant::now();
                        let mut child = cmd
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .unwrap();
                        sleep(Duration::from_millis(STARTUP_WAIT_MS));
                        let _ = child.kill();
                        let elapsed = start.elapsed();
                        elapsed
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmark memory usage of file watchers
fn bench_memory(c: &mut Criterion) {
    // Create a temp dir for testing
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("memory_usage_kb");

    for watcher in [
        Watcher::Flash,
        Watcher::Nodemon,
        Watcher::Watchexec,
        Watcher::Cargo,
    ] {
        if let Some(mut cmd) = watcher.command(temp_dir.path()) {
            group.bench_with_input(
                BenchmarkId::from_parameter(watcher.name()),
                &watcher,
                |b, _| {
                    b.iter(|| {
                        let mut child = cmd
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .unwrap();

                        sleep(Duration::from_millis(STARTUP_WAIT_MS));

                        let mut system = System::new_all();
                        system.refresh_all();

                        // Get memory usage
                        let memory = system
                            .processes()
                            .values()
                            .find(|p| p.pid().as_u32() == child.id())
                            .map(|p| p.memory())
                            .unwrap_or(0)
                            / 1024; // Convert to KB

                        let _ = child.kill();
                        memory
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmark file change detection latency
fn bench_change_detection(c: &mut Criterion) {
    // Create a temp dir for testing
    let temp_dir = TempDir::new().unwrap();

    // Create a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "initial content").unwrap();

    let mut group = c.benchmark_group("change_detection_ms");

    for watcher in [
        Watcher::Flash,
        Watcher::Nodemon,
        Watcher::Watchexec,
        Watcher::Cargo,
    ] {
        if let Some(mut cmd) = watcher.command(temp_dir.path()) {
            group.bench_with_input(
                BenchmarkId::from_parameter(watcher.name()),
                &watcher,
                |b, _| {
                    b.iter(|| {
                        let mut child = cmd
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .unwrap();

                        // Wait for watcher to initialize
                        sleep(Duration::from_millis(STARTUP_WAIT_MS));

                        let mut total_latency = Duration::new(0, 0);

                        // Make several file changes to get an average
                        for i in 0..FILE_CHANGES {
                            let start = Instant::now();

                            // Modify the test file
                            fs::write(&test_file, format!("content change {}", i)).unwrap();

                            // Wait for the watcher to potentially detect the change
                            sleep(Duration::from_millis(CHANGE_INTERVAL_MS));

                            total_latency += start.elapsed();
                        }

                        let _ = child.kill();

                        // Return average latency in milliseconds
                        total_latency.as_millis() as u64 / FILE_CHANGES as u64
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmark CPU usage during idle
fn bench_idle_cpu(c: &mut Criterion) {
    // Create a temp dir for testing
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("idle_cpu_percent");

    for watcher in [
        Watcher::Flash,
        Watcher::Nodemon,
        Watcher::Watchexec,
        Watcher::Cargo,
    ] {
        if let Some(mut cmd) = watcher.command(temp_dir.path()) {
            group.bench_with_input(
                BenchmarkId::from_parameter(watcher.name()),
                &watcher,
                |b, _| {
                    b.iter(|| {
                        let mut child = cmd
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .unwrap();

                        // Wait for watcher to initialize
                        sleep(Duration::from_millis(STARTUP_WAIT_MS));

                        // Let it run in idle state
                        sleep(Duration::from_secs(2));

                        let mut system = System::new_all();
                        system.refresh_all();

                        // Get CPU usage
                        let cpu_usage = system
                            .processes()
                            .values()
                            .find(|p| p.pid().as_u32() == child.id())
                            .map(|p| p.cpu_usage())
                            .unwrap_or(0.0);

                        let _ = child.kill();
                        cpu_usage
                    })
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_startup,
    bench_memory,
    bench_change_detection,
    bench_idle_cpu
);
criterion_main!(benches);
