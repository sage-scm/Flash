use std::collections::HashMap;

// Import the relevant types and structures for testing
// These are simplified versions of what's in src/bench_results.rs
#[derive(Debug, Clone, PartialEq)]
struct WatcherResult {
    startup_time_ms: f64,
    memory_usage_kb: f64,
    change_detection_ms: f64,
    idle_cpu_percent: f64,
}

impl WatcherResult {
    fn new(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BenchMetric {
    StartupTime,
    MemoryUsage,
    ChangeDetection,
    CpuUsage,
}

struct BenchResults {
    results: HashMap<String, WatcherResult>,
}

impl BenchResults {
    fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    fn add_result(&mut self, name: &str, result: WatcherResult) {
        self.results.insert(name.to_string(), result);
    }

    fn best_performer(&self, metric: BenchMetric) -> Option<(&String, f64)> {
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

    fn flash_improvement(&self) -> HashMap<BenchMetric, f64> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_results() -> BenchResults {
        let mut results = BenchResults::new();

        // Flash results (best performance)
        results.add_result("flash", WatcherResult::new(25.6, 5400.0, 32.1, 0.12));

        // nodemon results
        results.add_result("nodemon", WatcherResult::new(156.2, 42800.0, 122.8, 0.85));

        // watchexec results
        results.add_result("watchexec", WatcherResult::new(52.4, 8700.0, 58.4, 0.31));

        results
    }

    #[test]
    fn test_best_performer() {
        let results = create_sample_results();

        // Flash should be best in all categories
        let (best_startup, _) = results.best_performer(BenchMetric::StartupTime).unwrap();
        let (best_memory, _) = results.best_performer(BenchMetric::MemoryUsage).unwrap();
        let (best_detection, _) = results
            .best_performer(BenchMetric::ChangeDetection)
            .unwrap();
        let (best_cpu, _) = results.best_performer(BenchMetric::CpuUsage).unwrap();

        assert_eq!(best_startup, "flash");
        assert_eq!(best_memory, "flash");
        assert_eq!(best_detection, "flash");
        assert_eq!(best_cpu, "flash");
    }

    #[test]
    fn test_flash_improvement() {
        let results = create_sample_results();
        let improvements = results.flash_improvement();

        // Test that all metrics show improvement (factor > 1.0)
        assert!(improvements.contains_key(&BenchMetric::StartupTime));
        assert!(improvements.contains_key(&BenchMetric::MemoryUsage));
        assert!(improvements.contains_key(&BenchMetric::ChangeDetection));
        assert!(improvements.contains_key(&BenchMetric::CpuUsage));

        assert!(improvements[&BenchMetric::StartupTime] > 1.0);
        assert!(improvements[&BenchMetric::MemoryUsage] > 1.0);
        assert!(improvements[&BenchMetric::ChangeDetection] > 1.0);
        assert!(improvements[&BenchMetric::CpuUsage] > 1.0);

        // Calculate expected values manually for verification
        let startup_improvement = (156.2 + 52.4) / 2.0 / 25.6;
        let memory_improvement = (42800.0 + 8700.0) / 2.0 / 5400.0;
        let detection_improvement = (122.8 + 58.4) / 2.0 / 32.1;
        let cpu_improvement = (0.85 + 0.31) / 2.0 / 0.12;

        assert!((improvements[&BenchMetric::StartupTime] - startup_improvement).abs() < 0.001);
        assert!((improvements[&BenchMetric::MemoryUsage] - memory_improvement).abs() < 0.001);
        assert!(
            (improvements[&BenchMetric::ChangeDetection] - detection_improvement).abs() < 0.001
        );
        assert!((improvements[&BenchMetric::CpuUsage] - cpu_improvement).abs() < 0.001);
    }

    #[test]
    fn test_empty_results() {
        let results = BenchResults::new();

        // No best performer when empty
        assert!(results.best_performer(BenchMetric::StartupTime).is_none());

        // No improvements when empty
        let improvements = results.flash_improvement();
        assert!(improvements.is_empty());
    }

    #[test]
    fn test_missing_flash() {
        let mut results = BenchResults::new();

        // Add only non-flash watchers
        results.add_result("nodemon", WatcherResult::new(156.2, 42800.0, 122.8, 0.85));

        // Should still find best performer
        let (best_startup, _) = results.best_performer(BenchMetric::StartupTime).unwrap();
        assert_eq!(best_startup, "nodemon");

        // But no improvements without flash
        let improvements = results.flash_improvement();
        assert!(improvements.is_empty());
    }
}
