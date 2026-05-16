//! Real benchmarks against the file watchers installed on this machine.
//!
//! No measurement is hard-coded. If a competitor isn't installed, it isn't in
//! the table; if a sample times out, that row reports `timeout`. The
//! methodology block at the bottom of the output documents exactly what was
//! measured so anyone can reproduce — or argue with — the numbers.
//!
//! The bench is intentionally simple: 5 samples per measurement, report the
//! median, no statistical claims beyond that. Users who want rigorous numbers
//! reach for `hyperfine`; this is for a quick, honest sanity-check.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use sysinfo::{Pid, System};
use tempfile::TempDir;

const SAMPLES: usize = 5;
const READY_WAIT: Duration = Duration::from_millis(1500);
const DETECT_TIMEOUT: Duration = Duration::from_secs(5);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

pub fn run() -> Result<()> {
    let flash = locate_flash().context("locating the flash-watcher binary")?;
    let competitors = discover_competitors(&flash);

    print_header(&competitors);

    let startups = bench_startup(&competitors)?;
    let detections = bench_detection(&competitors)?;
    let memories = bench_memory(&competitors)?;

    print_table(
        "Binary launch (smaller is better)",
        "ms",
        &competitors,
        &startups,
    );
    print_table(
        "Change-detection latency (smaller is better)",
        "ms",
        &competitors,
        &detections,
    );
    print_memory(&competitors, &memories);

    print_methodology();
    Ok(())
}

// ─── competitor discovery ────────────────────────────────────────────────────

struct Competitor {
    name: &'static str,
    binary: PathBuf,
    version: String,
    /// Build the watcher command. Args: watch root, marker path the watcher's
    /// command should `touch` on every file change.
    invoke: fn(&Path, &Path) -> Command,
}

fn discover_competitors(flash: &Path) -> Vec<Competitor> {
    let mut found = Vec::new();

    found.push(Competitor {
        name: "flash-watcher",
        binary: flash.to_path_buf(),
        version: query_version(flash, &["--version"]).unwrap_or_else(|| "local".to_string()),
        invoke: invoke_flash,
    });

    if let Some(p) = which("watchexec") {
        let version = query_version(&p, &["--version"]).unwrap_or_default();
        found.push(Competitor {
            name: "watchexec",
            binary: p,
            version,
            invoke: invoke_watchexec,
        });
    }
    if let Some(p) = which("nodemon") {
        let version = query_version(&p, &["--version"]).unwrap_or_default();
        found.push(Competitor {
            name: "nodemon",
            binary: p,
            version,
            invoke: invoke_nodemon,
        });
    }
    if let Some(p) = which("cargo-watch") {
        let version = query_version(&p, &["--version"]).unwrap_or_default();
        found.push(Competitor {
            name: "cargo-watch",
            binary: p,
            version,
            invoke: invoke_cargo_watch,
        });
    }
    if let Some(p) = which("entr") {
        let version = query_version(&p, &["-v"]).unwrap_or_default();
        found.push(Competitor {
            name: "entr",
            binary: p,
            version,
            invoke: invoke_entr,
        });
    }
    found
}

fn invoke_flash(watch: &Path, marker: &Path) -> Command {
    // Pass the marker command as separate args so flash's shell-skip kicks in.
    let mut c = Command::new(crate_invocation_self());
    c.arg("--fast")
        .arg("--debounce")
        .arg("10")
        .arg("-w")
        .arg(watch)
        .arg("touch")
        .arg(marker);
    c
}

fn invoke_watchexec(watch: &Path, marker: &Path) -> Command {
    // `watchexec` already direct-execs anything after `--`, so we hand it
    // the command as separate args too — apples-to-apples with Flash.
    let mut c = Command::new("watchexec");
    c.arg("--watch")
        .arg(watch)
        .arg("--debounce")
        .arg("10ms")
        .arg("--")
        .arg("touch")
        .arg(marker);
    c
}

fn invoke_nodemon(watch: &Path, marker: &Path) -> Command {
    // nodemon's --exec always wraps in a shell; nothing we can do about it.
    let mut c = Command::new("nodemon");
    c.arg("--quiet")
        .arg("--watch")
        .arg(watch)
        .arg("--exec")
        .arg(format!("touch {}", shell_quote(&marker.to_string_lossy())));
    c
}

fn invoke_cargo_watch(watch: &Path, marker: &Path) -> Command {
    // cargo-watch's `-s` likewise always goes through the shell.
    let mut c = Command::new("cargo-watch");
    c.arg("-w")
        .arg(watch)
        .arg("-s")
        .arg(format!("touch {}", shell_quote(&marker.to_string_lossy())));
    c
}

fn invoke_entr(watch: &Path, marker: &Path) -> Command {
    // `entr` reads paths from stdin and re-runs the command when any of those
    // paths change. The shell pipeline is part of how entr is invoked, not an
    // unfair addition.
    let cmd = format!("touch {}", shell_quote(&marker.to_string_lossy()));
    let script = format!(
        "find {watch} -type f | entr -n -s {cmd:?}",
        watch = watch.display(),
    );
    let mut c = Command::new("sh");
    c.arg("-c").arg(script);
    c
}

fn crate_invocation_self() -> PathBuf {
    // Use the running binary so users can compare debug vs. release fairly.
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("flash-watcher"))
}

fn which(binary: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {binary}"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?;
    let trimmed = path.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn query_version(binary: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new(binary).args(args).output().ok()?;
    let raw = if !out.stdout.is_empty() {
        out.stdout
    } else {
        out.stderr
    };
    String::from_utf8(raw)
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
}

// ─── measurements ────────────────────────────────────────────────────────────

fn bench_startup(competitors: &[Competitor]) -> Result<Vec<Option<Duration>>> {
    competitors
        .iter()
        .map(|c| measure_startup(&c.binary))
        .collect()
}

/// Time `binary --help` from `spawn()` to process exit. A clean measurement of
/// the "what does it cost to even launch this thing" overhead.
fn measure_startup(binary: &Path) -> Result<Option<Duration>> {
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let mut child = match Command::new(binary)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        match wait_with_timeout(&mut child, STARTUP_TIMEOUT) {
            Some(_) => samples.push(start.elapsed()),
            None => return Ok(None),
        }
    }
    Ok(Some(median(samples)))
}

fn bench_detection(competitors: &[Competitor]) -> Result<Vec<Option<Duration>>> {
    competitors.iter().map(measure_detection).collect()
}

fn measure_detection(c: &Competitor) -> Result<Option<Duration>> {
    let mut samples = Vec::new();
    for _ in 0..SAMPLES {
        let Some(sample) = detection_once(c)? else {
            return Ok(None);
        };
        samples.push(sample);
    }
    Ok(Some(median(samples)))
}

fn detection_once(c: &Competitor) -> Result<Option<Duration>> {
    let workspace = TempDir::new().context("creating bench workspace")?;
    let watch_dir = workspace.path().join("watch");
    fs::create_dir(&watch_dir)?;
    let marker = workspace.path().join("marker");

    // Pre-create a file so the initial scan has something to settle on.
    fs::write(watch_dir.join("seed.txt"), "seed")?;

    let mut child = (c.invoke)(&watch_dir, &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawning competitor")?;

    // Let the watcher reach steady state and absorb any initial-scan events.
    thread::sleep(READY_WAIT);
    let _ = fs::remove_file(&marker);

    let trigger = watch_dir.join("trigger.txt");
    let started = Instant::now();
    fs::write(&trigger, "hello")?;

    let result = poll_for(&marker, DETECT_TIMEOUT);
    let _ = child.kill();
    let _ = child.wait();

    Ok(result.map(|_| started.elapsed()))
}

fn bench_memory(competitors: &[Competitor]) -> Result<Vec<Option<u64>>> {
    competitors.iter().map(measure_memory).collect()
}

fn measure_memory(c: &Competitor) -> Result<Option<u64>> {
    let mut samples = Vec::new();
    for _ in 0..SAMPLES {
        let workspace = TempDir::new()?;
        let watch_dir = workspace.path().join("watch");
        fs::create_dir(&watch_dir)?;
        fs::write(watch_dir.join("seed.txt"), "seed")?;
        // Throwaway marker; the memory probe never triggers the watcher's
        // command, but the invoke API still wants somewhere to point.
        let marker = workspace.path().join("unused");

        let mut child = (c.invoke)(&watch_dir, &marker)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawning competitor for memory probe")?;

        thread::sleep(READY_WAIT);
        let pid = Pid::from_u32(child.id());
        let mut sys = System::new();
        sys.refresh_process(pid);
        let bytes = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

        let _ = child.kill();
        let _ = child.wait();

        if bytes == 0 {
            return Ok(None);
        }
        samples.push(bytes);
    }
    samples.sort_unstable();
    Ok(samples.get(samples.len() / 2).copied())
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
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
                thread::sleep(Duration::from_millis(2));
            }
            Err(_) => return None,
        }
    }
}

fn poll_for(path: &Path, timeout: Duration) -> Option<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return Some(());
        }
        thread::sleep(Duration::from_millis(2));
    }
    None
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn shell_quote(s: &str) -> String {
    // Tiny single-quote escaping, enough for the temp paths we generate.
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ─── output ──────────────────────────────────────────────────────────────────

fn print_header(competitors: &[Competitor]) {
    println!("{}", "flash · benchmark".bold().bright_cyan());
    println!("{}", "─".repeat(58).bright_black());
    println!(
        "{} {} {}",
        "host".bright_black(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!(
        "{} {SAMPLES} per metric, reporting median",
        "samples".bright_black()
    );

    if std::env::current_exe()
        .map(|p| p.to_string_lossy().contains("/debug/"))
        .unwrap_or(false)
    {
        println!(
            "{}",
            "note: running from a debug build — rebuild with --release for representative numbers"
                .yellow()
        );
    }

    println!("\n{}", "Watchers detected".bright_white().bold());
    for c in competitors {
        println!(
            "  {}  {}  {}",
            format!("{:<14}", c.name).bright_green(),
            c.binary.display().to_string().bright_black(),
            c.version.bright_black()
        );
    }
    if competitors.len() == 1 {
        println!(
            "  {}",
            "(no other watchers found on PATH; install some to compare)".bright_black()
        );
    }
}

fn print_table(title: &str, unit: &str, competitors: &[Competitor], samples: &[Option<Duration>]) {
    println!("\n{}", title.bright_white().bold());
    let width = competitors.iter().map(|c| c.name.len()).max().unwrap_or(8);

    let mut rows: Vec<(usize, String)> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let value = match s {
                Some(d) => format!("{:>7.2} {unit}", d.as_secs_f64() * 1000.0),
                None => "    n/a   ".to_string(),
            };
            (i, value)
        })
        .collect();

    // Order rows by elapsed time so winners read first.
    rows.sort_by_key(|(i, _)| samples[*i].unwrap_or(Duration::from_secs(u64::MAX / 2)));

    for (i, value) in rows {
        let name = competitors[i].name;
        let label = format!("{name:<width$}", width = width);
        let painted = if name == "flash-watcher" {
            label.bright_green().to_string()
        } else {
            label.normal().to_string()
        };
        println!("  {}  {}", painted, value);
    }
}

fn print_memory(competitors: &[Competitor], samples: &[Option<u64>]) {
    println!(
        "\n{}",
        "Resident memory (smaller is better)".bright_white().bold()
    );
    let width = competitors.iter().map(|c| c.name.len()).max().unwrap_or(8);

    let mut rows: Vec<(usize, String, u64)> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| match s {
            Some(b) => (i, crate::stats::format_bytes(*b), *b),
            None => (i, "    n/a   ".to_string(), u64::MAX),
        })
        .collect();
    rows.sort_by_key(|(_, _, raw)| *raw);

    for (i, value, _) in rows {
        let name = competitors[i].name;
        let label = format!("{name:<width$}", width = width);
        let painted = if name == "flash-watcher" {
            label.bright_green().to_string()
        } else {
            label.normal().to_string()
        };
        println!("  {}  {:>10}", painted, value);
    }
}

fn print_methodology() {
    println!("\n{}", "Methodology".bright_white().bold());
    println!(
        "  {}",
        "launch     time from spawn() to process exit on `--help`".bright_black()
    );
    println!(
        "  {}",
        "detection  time from a file write to the watcher's command writing a marker"
            .bright_black()
    );
    println!(
        "  {}",
        "memory     resident set size after 1.5 s of steady state".bright_black()
    );
    println!(
        "  {}",
        "samples    5; reporting median to dampen one-off scheduler noise".bright_black()
    );
}

// ─── locate flash binary ────────────────────────────────────────────────────

fn locate_flash() -> Result<PathBuf> {
    if let Ok(p) = std::env::current_exe() {
        if p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("flash-watcher"))
        {
            return Ok(p);
        }
    }
    if let Some(p) = which("flash-watcher") {
        return Ok(p);
    }
    bail!("could not locate the flash-watcher binary on PATH")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_picks_middle() {
        let samples = vec![
            Duration::from_millis(50),
            Duration::from_millis(10),
            Duration::from_millis(30),
        ];
        assert_eq!(median(samples), Duration::from_millis(30));
    }

    #[test]
    fn shell_quote_escapes_quotes() {
        assert_eq!(shell_quote("ab'cd"), "'ab'\\''cd'");
    }
}
