use std::time::{Duration, Instant};

use colored::Colorize;
use sysinfo::{Pid, System};

/// Lightweight rolling counters and resource samples for the `--stats` flag.
pub struct Stats {
    started_at: Instant,
    changes: u64,
    events: u64,
    memory_bytes: u64,
    cpu_percent: f32,
    system: System,
    pid: Pid,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            changes: 0,
            events: 0,
            memory_bytes: 0,
            cpu_percent: 0.0,
            system: System::new(),
            pid: Pid::from_u32(std::process::id()),
        }
    }

    pub fn record_change(&mut self) {
        self.changes += 1;
    }

    pub fn record_event(&mut self) {
        self.events += 1;
    }

    pub fn refresh(&mut self) {
        self.system.refresh_process(self.pid);
        if let Some(proc) = self.system.process(self.pid) {
            self.memory_bytes = proc.memory();
            self.cpu_percent = proc.cpu_usage();
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn changes(&self) -> u64 {
        self.changes
    }

    pub fn events(&self) -> u64 {
        self.events
    }

    pub fn memory_bytes(&self) -> u64 {
        self.memory_bytes
    }

    pub fn cpu_percent(&self) -> f32 {
        self.cpu_percent
    }

    pub fn render(&self) -> String {
        format!(
            "{header}\n  uptime    {uptime}\n  changes   {changes}\n  events    {events}\n  memory    {memory}\n  cpu       {cpu:.1} %",
            header = "── flash · live stats ──".bright_cyan(),
            uptime = format_duration(self.started_at.elapsed()),
            changes = self.changes,
            events = self.events,
            memory = format_bytes(self.memory_bytes),
            cpu = self.cpu_percent,
        )
    }
}

pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.2} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_increment() {
        let mut s = Stats::new();
        s.record_event();
        s.record_event();
        s.record_change();
        assert_eq!(s.events(), 2);
        assert_eq!(s.changes(), 1);
    }

    #[test]
    fn format_duration_buckets() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 05s");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1h 02m 05s");
    }

    #[test]
    fn format_bytes_buckets() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KiB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MiB");
    }

    #[test]
    fn refresh_does_not_panic() {
        let mut s = Stats::new();
        s.refresh();
        // Memory may be zero on some sandboxes; just ensure refresh is sound.
        assert!(s.cpu_percent() >= 0.0);
    }
}
