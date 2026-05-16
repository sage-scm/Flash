//! Shared helpers for the integration test suite.
//!
//! Each integration test binary includes this module from source, so a helper
//! used by only one binary looks "dead" to the others. `#![allow(dead_code)]`
//! silences that without weakening warnings in production code.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

/// Wrapper around a temporary directory plus a `watch/` subdirectory and a
/// pre-allocated `marker` path. Every integration test uses the same shape so
/// the helpers below can stay short and obvious.
pub struct Workspace {
    pub root: TempDir,
}

impl Workspace {
    pub fn new() -> Self {
        let root = TempDir::new().expect("create temp workspace");
        std::fs::create_dir(root.path().join("watch")).expect("create watch dir");
        std::fs::write(root.path().join("watch").join("seed.txt"), "seed").expect("seed file");
        Self { root }
    }

    pub fn watch_dir(&self) -> PathBuf {
        self.root.path().join("watch")
    }

    pub fn watch_str(&self) -> String {
        self.watch_dir().to_string_lossy().into_owned()
    }

    pub fn marker(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    pub fn write(&self, rel: &str, contents: &str) {
        let path = self.watch_dir().join(rel);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, contents).expect("write file");
    }
}

pub fn flash() -> Command {
    Command::new(env!("CARGO_BIN_EXE_flash-watcher"))
}

/// Spawn `flash-watcher` with output suppressed.
pub fn spawn_silent(mut cmd: Command) -> Child {
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn flash-watcher")
}

/// Spawn `flash-watcher` keeping output, so tests can assert on stderr/stdout.
pub fn spawn_capturing(mut cmd: Command) -> Child {
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn flash-watcher")
}

pub fn wait_for_path(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

pub fn wait_for_path_absent(path: &Path, wait: Duration) -> bool {
    // We are looking for the absence of a side effect — give the watcher the
    // full window before concluding nothing happened.
    thread::sleep(wait);
    !path.exists()
}

/// Time-bounded wait for a child to exit. Returns the exit status, or `None`
/// if the child outlived the timeout (in which case it is killed and reaped).
pub fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

/// Sleep long enough for the watcher to reach steady state and for any
/// initial-scan events to drain. Roughly the same as the `--bench` settle
/// window.
pub const STEADY_STATE: Duration = Duration::from_millis(1500);
