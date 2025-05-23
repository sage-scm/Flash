use std::time::{Duration, Instant};

use chrono::Local;
use colored::Colorize;
use sysinfo::{Pid, System};

/// Stats collector for Flash performance metrics
pub struct StatsCollector {
    pub start_time: Instant,
    pub file_changes: usize,
    pub watcher_calls: usize,
    pub last_memory_usage: u64,
    pub last_cpu_usage: f32,
    system: System,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            file_changes: 0,
            watcher_calls: 0,
            last_memory_usage: 0,
            last_cpu_usage: 0.0,
            system: System::new_all(),
        }
    }

    pub fn record_file_change(&mut self) {
        self.file_changes += 1;
    }

    pub fn record_watcher_call(&mut self) {
        self.watcher_calls += 1;
    }

    pub fn update_resource_usage(&mut self) {
        self.system.refresh_all();

        let pid = std::process::id();
        if let Some(process) = self.system.process(Pid::from_u32(pid)) {
            self.last_memory_usage = process.memory() / 1024; // KB
            self.last_cpu_usage = process.cpu_usage();
        }
    }

    pub fn display_stats(&self) {
        let elapsed = self.start_time.elapsed();
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        println!("{}", "── Flash Performance Stats ──".bright_green());
        println!("{} {}", "Time:".bright_blue(), timestamp);
        println!("{} {}", "Uptime:".bright_blue(), format_duration(elapsed));
        println!("{} {}", "File changes:".bright_blue(), self.file_changes);
        println!("{} {}", "Watcher calls:".bright_blue(), self.watcher_calls);
        println!(
            "{} {} KB",
            "Memory usage:".bright_blue(),
            self.last_memory_usage
        );
        println!("{} {:.1}%", "CPU usage:".bright_blue(), self.last_cpu_usage);
        println!("{}", "────────────────────────────".bright_green());
    }
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!(
            "{}h {}m {}s",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_stats_collector_new() {
        let stats = StatsCollector::new();
        assert_eq!(stats.file_changes, 0);
        assert_eq!(stats.watcher_calls, 0);
        assert_eq!(stats.last_memory_usage, 0);
        assert_eq!(stats.last_cpu_usage, 0.0);
    }

    #[test]
    fn test_record_file_change() {
        let mut stats = StatsCollector::new();
        assert_eq!(stats.file_changes, 0);

        stats.record_file_change();
        assert_eq!(stats.file_changes, 1);

        stats.record_file_change();
        assert_eq!(stats.file_changes, 2);
    }

    #[test]
    fn test_record_watcher_call() {
        let mut stats = StatsCollector::new();
        assert_eq!(stats.watcher_calls, 0);

        stats.record_watcher_call();
        assert_eq!(stats.watcher_calls, 1);

        stats.record_watcher_call();
        assert_eq!(stats.watcher_calls, 2);
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m 0s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
        assert_eq!(format_duration(Duration::from_secs(7323)), "2h 2m 3s");
    }

    #[test]
    fn test_update_resource_usage() {
        let mut stats = StatsCollector::new();
        // This test just ensures the method doesn't panic
        // Actual values depend on system state
        stats.update_resource_usage();
        // Memory usage should be updated (non-zero for a running process)
        // Note: This might be 0 in some test environments, so we just check it doesn't panic
    }
}
