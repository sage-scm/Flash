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

fn format_duration(duration: Duration) -> String {
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
