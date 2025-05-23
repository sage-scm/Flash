use flash_watcher::stats::{format_duration, StatsCollector};
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_collector_new() {
        let stats = StatsCollector::new();
        assert_eq!(stats.file_changes, 0);
        assert_eq!(stats.watcher_calls, 0);
        assert!(stats.start_time.elapsed().as_secs() < 1); // Should be very recent
    }

    #[test]
    fn test_record_file_change() {
        let mut stats = StatsCollector::new();
        assert_eq!(stats.file_changes, 0);

        stats.record_file_change();
        assert_eq!(stats.file_changes, 1);

        stats.record_file_change();
        assert_eq!(stats.file_changes, 2);

        // Test multiple increments
        for _ in 0..10 {
            stats.record_file_change();
        }
        assert_eq!(stats.file_changes, 12);
    }

    #[test]
    fn test_record_watcher_call() {
        let mut stats = StatsCollector::new();
        assert_eq!(stats.watcher_calls, 0);

        stats.record_watcher_call();
        assert_eq!(stats.watcher_calls, 1);

        stats.record_watcher_call();
        assert_eq!(stats.watcher_calls, 2);

        // Test multiple increments
        for _ in 0..100 {
            stats.record_watcher_call();
        }
        assert_eq!(stats.watcher_calls, 102);
    }

    #[test]
    fn test_update_resource_usage() {
        let mut stats = StatsCollector::new();

        // Initial values should be 0
        assert_eq!(stats.last_memory_usage, 0);
        assert_eq!(stats.last_cpu_usage, 0.0);

        // Update resource usage
        stats.update_resource_usage();

        // After update, values should be valid (method doesn't panic)
        // Note: These might still be 0 in some test environments, so we just test that the method doesn't panic
        // Memory usage is u64, so always >= 0, just check it's reasonable
        assert!(stats.last_memory_usage < 1024 * 1024 * 1024); // Less than 1TB in KB
        assert!(stats.last_cpu_usage >= 0.0);
    }

    #[test]
    fn test_display_stats() {
        let mut stats = StatsCollector::new();

        // Add some test data
        stats.record_file_change();
        stats.record_file_change();
        stats.record_watcher_call();
        stats.record_watcher_call();
        stats.record_watcher_call();
        stats.update_resource_usage();

        // This should not panic
        stats.display_stats();

        // Verify the data is still correct after display
        assert_eq!(stats.file_changes, 2);
        assert_eq!(stats.watcher_calls, 3);
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(1)), "1s");
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(61)), "1m 1s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(120)), "2m 0s");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m 0s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h 0m 0s");
        assert_eq!(format_duration(Duration::from_secs(7323)), "2h 2m 3s");
        assert_eq!(format_duration(Duration::from_secs(86400)), "24h 0m 0s");
    }

    #[test]
    fn test_format_duration_edge_cases() {
        // Test very small durations
        assert_eq!(format_duration(Duration::from_millis(500)), "0s");
        assert_eq!(format_duration(Duration::from_millis(999)), "0s");

        // Test large durations
        assert_eq!(format_duration(Duration::from_secs(90061)), "25h 1m 1s");
        assert_eq!(format_duration(Duration::from_secs(359999)), "99h 59m 59s");
    }

    #[test]
    fn test_stats_collector_concurrent_access() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let stats = Arc::new(Mutex::new(StatsCollector::new()));
        let mut handles = vec![];

        // Spawn multiple threads to test concurrent access
        for _ in 0..10 {
            let stats_clone = Arc::clone(&stats);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    {
                        let mut stats = stats_clone.lock().unwrap();
                        stats.record_file_change();
                    }
                    {
                        let mut stats = stats_clone.lock().unwrap();
                        stats.record_watcher_call();
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Check final counts
        let final_stats = stats.lock().unwrap();
        assert_eq!(final_stats.file_changes, 1000); // 10 threads * 100 increments
        assert_eq!(final_stats.watcher_calls, 1000); // 10 threads * 100 increments
    }

    #[test]
    fn test_stats_collector_uptime() {
        let stats = StatsCollector::new();

        // Sleep for a short time
        std::thread::sleep(Duration::from_millis(10));

        // Uptime should be at least the sleep duration
        let uptime = stats.start_time.elapsed();
        assert!(uptime >= Duration::from_millis(10));
        assert!(uptime < Duration::from_secs(1)); // But not too long
    }

    #[test]
    fn test_stats_collector_memory_usage_bounds() {
        let mut stats = StatsCollector::new();
        stats.update_resource_usage();

        // Memory usage should be reasonable (not impossibly large)
        // u64 is always >= 0, so just check upper bound
        assert!(stats.last_memory_usage < 1024 * 1024 * 100); // Less than 100GB in KB
    }

    #[test]
    fn test_stats_collector_cpu_usage_bounds() {
        let mut stats = StatsCollector::new();
        stats.update_resource_usage();

        // CPU usage should be between 0 and 100 (or slightly above due to measurement variations)
        assert!(stats.last_cpu_usage >= 0.0);
        assert!(stats.last_cpu_usage <= 200.0); // Allow some margin for multi-core systems
    }

    #[test]
    fn test_stats_collector_multiple_updates() {
        let mut stats = StatsCollector::new();

        // Multiple updates should not cause issues
        for _ in 0..5 {
            stats.update_resource_usage();
            std::thread::sleep(Duration::from_millis(1));
        }

        // Should still have valid values
        // u64 is always >= 0, so just check it's reasonable
        assert!(stats.last_memory_usage < 1024 * 1024 * 1024); // Less than 1TB in KB
        assert!(stats.last_cpu_usage >= 0.0);
    }
}
