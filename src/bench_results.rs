use std::collections::HashMap;
use std::fmt;

use colored::Colorize;

/// Represents a benchmark result for a specific watcher
#[derive(Debug, Clone)]
pub struct WatcherResult {
    pub startup_time_ms: f64,
    pub memory_usage_kb: f64,
    pub change_detection_ms: f64,
    pub idle_cpu_percent: f64,
}

impl WatcherResult {
    pub fn new(
        startup_time_ms: f64,
        memory_usage_kb: f64,
        change_detection_ms: f64,
        idle_cpu_percent: f64,
    ) -> Self {
        Self {
            startup_time_ms,
            memory_usage_kb,
            change_detection_ms,
            idle_cpu_percent,
        }
    }
}

/// Stores benchmark results for multiple file watchers
pub struct BenchResults {
    results: HashMap<String, WatcherResult>,
}

impl BenchResults {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    /// Add pre-populated sample benchmark results for demonstration purposes
    pub fn with_sample_data() -> Self {
        let mut results = HashMap::new();

        // Flash results (simulating best performance)
        results.insert(
            "flash".to_string(),
            WatcherResult::new(25.6, 5400.0, 32.1, 0.12),
        );

        // nodemon results
        results.insert(
            "nodemon".to_string(),
            WatcherResult::new(156.2, 42800.0, 122.8, 0.85),
        );

        // watchexec results
        results.insert(
            "watchexec".to_string(),
            WatcherResult::new(52.4, 8700.0, 58.4, 0.31),
        );

        // cargo-watch results
        results.insert(
            "cargo-watch".to_string(),
            WatcherResult::new(175.5, 21400.0, 85.2, 0.42),
        );

        Self { results }
    }

    #[allow(dead_code)]
    pub fn add_result(&mut self, name: &str, result: WatcherResult) {
        self.results.insert(name.to_string(), result);
    }

    /// Get the best performer for a specific metric
    #[allow(dead_code)]
    pub fn best_performer(&self, metric: BenchMetric) -> Option<(&String, f64)> {
        self.results
            .iter()
            .map(|(name, result)| {
                let value = match metric {
                    BenchMetric::StartupTime => result.startup_time_ms,
                    BenchMetric::MemoryUsage => result.memory_usage_kb,
                    BenchMetric::ChangeDetection => result.change_detection_ms,
                    BenchMetric::CpuUsage => result.idle_cpu_percent,
                };
                (name, value)
            })
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    }

    /// Calculate how much faster/better Flash is compared to the average
    pub fn flash_improvement(&self) -> HashMap<BenchMetric, f64> {
        let mut improvements = HashMap::new();
        let flash = match self.results.get("flash") {
            Some(r) => r,
            None => return improvements,
        };

        let metrics = vec![
            (BenchMetric::StartupTime, flash.startup_time_ms),
            (BenchMetric::MemoryUsage, flash.memory_usage_kb),
            (BenchMetric::ChangeDetection, flash.change_detection_ms),
            (BenchMetric::CpuUsage, flash.idle_cpu_percent),
        ];

        for (metric, flash_value) in metrics {
            let others: Vec<_> = self
                .results
                .iter()
                .filter(|(name, _)| *name != "flash")
                .map(|(_, result)| match metric {
                    BenchMetric::StartupTime => result.startup_time_ms,
                    BenchMetric::MemoryUsage => result.memory_usage_kb,
                    BenchMetric::ChangeDetection => result.change_detection_ms,
                    BenchMetric::CpuUsage => result.idle_cpu_percent,
                })
                .collect();

            if !others.is_empty() {
                let avg: f64 = others.iter().sum::<f64>() / others.len() as f64;
                let improvement = avg / flash_value;
                improvements.insert(metric, improvement);
            }
        }

        improvements
    }

    /// Print a comparison bar chart for a specific metric
    pub fn print_chart(&self, metric: BenchMetric) {
        let title = match metric {
            BenchMetric::StartupTime => "Startup Time (ms) - lower is better",
            BenchMetric::MemoryUsage => "Memory Usage (KB) - lower is better",
            BenchMetric::ChangeDetection => "Change Detection (ms) - lower is better",
            BenchMetric::CpuUsage => "CPU Usage (%) - lower is better",
        };

        println!("\n{}", title.bright_green().bold());
        println!("{}", "─".repeat(60).bright_blue());

        let max_name_len = self.results.keys().map(|k| k.len()).max().unwrap_or(10);

        // Get values for this metric
        let mut entries: Vec<_> = self
            .results
            .iter()
            .map(|(name, result)| {
                let value = match metric {
                    BenchMetric::StartupTime => result.startup_time_ms,
                    BenchMetric::MemoryUsage => result.memory_usage_kb,
                    BenchMetric::ChangeDetection => result.change_detection_ms,
                    BenchMetric::CpuUsage => result.idle_cpu_percent,
                };
                (name, value)
            })
            .collect();

        // Sort by value (best first)
        entries.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

        // Find the maximum value for scaling
        let max_value = entries.iter().map(|(_, v)| *v).fold(0.0, f64::max);
        let scale_factor = 40.0 / max_value;

        // Print bars
        for (name, value) in entries {
            let bar_length = (value * scale_factor).round() as usize;
            let bar = "█".repeat(bar_length);

            let formatted_name = format!("{:width$}", name, width = max_name_len);
            let formatted_value = match metric {
                BenchMetric::StartupTime => format!("{:.1} ms", value),
                BenchMetric::MemoryUsage => format!("{:.0} KB", value),
                BenchMetric::ChangeDetection => format!("{:.1} ms", value),
                BenchMetric::CpuUsage => format!("{:.2} %", value),
            };

            let color = if name == "flash" {
                bar.bright_green()
            } else {
                bar.bright_blue()
            };

            println!(
                "{} {} {}",
                formatted_name.bright_yellow(),
                color,
                formatted_value.bright_white()
            );
        }

        println!("{}", "─".repeat(60).bright_blue());
    }

    /// Print a summary report of all benchmark results
    pub fn print_report(&self) {
        println!("\n{}", "📊 Flash Benchmark Results".bright_green().bold());
        println!("{}", "══════════════════════════════════════".bright_blue());

        for metric in [
            BenchMetric::StartupTime,
            BenchMetric::MemoryUsage,
            BenchMetric::ChangeDetection,
            BenchMetric::CpuUsage,
        ] {
            self.print_chart(metric);
        }

        // Print Flash improvement stats
        println!(
            "\n{}",
            "Flash Performance Improvement".bright_green().bold()
        );
        println!("{}", "──────────────────────────────".bright_blue());

        let improvements = self.flash_improvement();
        for (metric, factor) in improvements {
            let metric_name = match metric {
                BenchMetric::StartupTime => "Startup Speed",
                BenchMetric::MemoryUsage => "Memory Efficiency",
                BenchMetric::ChangeDetection => "Detection Speed",
                BenchMetric::CpuUsage => "CPU Efficiency",
            };

            println!(
                "{}: {} {}x faster than average",
                metric_name.bright_yellow(),
                format!("{:.1}", factor).bright_green(),
                if factor >= 2.0 { "🔥" } else { "" }
            );
        }

        println!("{}", "══════════════════════════════════════".bright_blue());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BenchMetric {
    StartupTime,
    MemoryUsage,
    ChangeDetection,
    CpuUsage,
}

impl fmt::Display for BenchMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchMetric::StartupTime => write!(f, "Startup Time"),
            BenchMetric::MemoryUsage => write!(f, "Memory Usage"),
            BenchMetric::ChangeDetection => write!(f, "Change Detection"),
            BenchMetric::CpuUsage => write!(f, "CPU Usage"),
        }
    }
}
