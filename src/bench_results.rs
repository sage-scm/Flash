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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_result_new() {
        let result = WatcherResult::new(25.5, 1024.0, 50.0, 0.5);
        assert_eq!(result.startup_time_ms, 25.5);
        assert_eq!(result.memory_usage_kb, 1024.0);
        assert_eq!(result.change_detection_ms, 50.0);
        assert_eq!(result.idle_cpu_percent, 0.5);
    }

    #[test]
    fn test_bench_results_new() {
        let results = BenchResults::new();
        assert!(results.results.is_empty());
    }

    #[test]
    fn test_bench_results_with_sample_data() {
        let results = BenchResults::with_sample_data();
        assert!(results.results.contains_key("flash"));
        assert!(results.results.contains_key("nodemon"));
        assert!(results.results.contains_key("watchexec"));
        assert!(results.results.contains_key("cargo-watch"));
        assert_eq!(results.results.len(), 4);
    }

    #[test]
    fn test_add_result() {
        let mut results = BenchResults::new();
        let watcher_result = WatcherResult::new(30.0, 2048.0, 60.0, 1.0);

        results.add_result("test-watcher", watcher_result.clone());
        assert!(results.results.contains_key("test-watcher"));

        let stored = results.results.get("test-watcher").unwrap();
        assert_eq!(stored.startup_time_ms, 30.0);
        assert_eq!(stored.memory_usage_kb, 2048.0);
    }

    #[test]
    fn test_best_performer() {
        let mut results = BenchResults::new();
        results.add_result("fast", WatcherResult::new(10.0, 1000.0, 20.0, 0.1));
        results.add_result("slow", WatcherResult::new(50.0, 5000.0, 100.0, 0.5));

        let best_startup = results.best_performer(BenchMetric::StartupTime);
        assert!(best_startup.is_some());
        let (name, value) = best_startup.unwrap();
        assert_eq!(name, "fast");
        assert_eq!(value, 10.0);

        let best_memory = results.best_performer(BenchMetric::MemoryUsage);
        assert!(best_memory.is_some());
        let (name, value) = best_memory.unwrap();
        assert_eq!(name, "fast");
        assert_eq!(value, 1000.0);
    }

    #[test]
    fn test_best_performer_empty() {
        let results = BenchResults::new();
        assert!(results.best_performer(BenchMetric::StartupTime).is_none());
    }

    #[test]
    fn test_flash_improvement() {
        let mut results = BenchResults::new();
        results.add_result("flash", WatcherResult::new(10.0, 1000.0, 20.0, 0.1));
        results.add_result("other1", WatcherResult::new(20.0, 2000.0, 40.0, 0.2));
        results.add_result("other2", WatcherResult::new(30.0, 3000.0, 60.0, 0.3));

        let improvements = results.flash_improvement();

        // Average of others: startup=25.0, memory=2500.0, detection=50.0, cpu=0.25
        // Flash values: startup=10.0, memory=1000.0, detection=20.0, cpu=0.1
        // Improvements: 25/10=2.5, 2500/1000=2.5, 50/20=2.5, 0.25/0.1=2.5

        assert!(improvements.contains_key(&BenchMetric::StartupTime));
        assert_eq!(improvements[&BenchMetric::StartupTime], 2.5);
        assert_eq!(improvements[&BenchMetric::MemoryUsage], 2.5);
        assert_eq!(improvements[&BenchMetric::ChangeDetection], 2.5);
        assert_eq!(improvements[&BenchMetric::CpuUsage], 2.5);
    }

    #[test]
    fn test_flash_improvement_missing_flash() {
        let mut results = BenchResults::new();
        results.add_result("other", WatcherResult::new(20.0, 2000.0, 40.0, 0.2));

        let improvements = results.flash_improvement();
        assert!(improvements.is_empty());
    }

    #[test]
    fn test_flash_improvement_only_flash() {
        let mut results = BenchResults::new();
        results.add_result("flash", WatcherResult::new(10.0, 1000.0, 20.0, 0.1));

        let improvements = results.flash_improvement();
        assert!(improvements.is_empty());
    }

    #[test]
    fn test_bench_metric_display() {
        assert_eq!(format!("{}", BenchMetric::StartupTime), "Startup Time");
        assert_eq!(format!("{}", BenchMetric::MemoryUsage), "Memory Usage");
        assert_eq!(format!("{}", BenchMetric::ChangeDetection), "Change Detection");
        assert_eq!(format!("{}", BenchMetric::CpuUsage), "CPU Usage");
    }
}
