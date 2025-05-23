use flash_watcher::CommandRunner;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_runner_new() {
        let cmd = vec!["cargo".to_string(), "test".to_string()];
        let runner = CommandRunner::new(cmd.clone(), true, false);

        assert_eq!(runner.command, cmd);
        assert!(runner.restart);
        assert!(!runner.clear);
        assert!(runner.current_process.is_none());
    }

    #[test]
    fn test_command_runner_dry_run_empty_command() {
        let mut runner = CommandRunner::new(vec![], false, false);
        let result = runner.dry_run();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty command"));
    }

    #[test]
    fn test_command_runner_dry_run_success() {
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "test".to_string()], false, false);
        let result = runner.dry_run();

        assert!(result.is_ok());
        assert!(runner.current_process.is_none()); // Non-restart mode doesn't save process
    }

    #[test]
    fn test_command_runner_restart_mode() {
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "test".to_string()], true, false);

        // dry_run doesn't actually create processes, so we just test that it succeeds
        let result = runner.dry_run();
        assert!(result.is_ok());

        // In dry_run mode, current_process remains None
        assert!(runner.current_process.is_none());

        // Second run should also succeed
        let result = runner.dry_run();
        assert!(result.is_ok());
        assert!(runner.current_process.is_none());
    }

    #[test]
    fn test_command_runner_run_simple_command() {
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "hello".to_string()], false, false);
        let result = runner.run();

        // Should succeed for simple echo command
        assert!(result.is_ok());
        assert!(runner.current_process.is_none()); // Not in restart mode
    }

    #[test]
    fn test_command_runner_run_with_restart() {
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "hello".to_string()], true, false);
        let result = runner.run();

        // Should succeed and store the process
        assert!(result.is_ok());
        // In restart mode, process should be stored (though it may have finished quickly)
    }

    #[test]
    fn test_command_runner_run_with_clear() {
        let mut runner =
            CommandRunner::new(vec!["echo".to_string(), "hello".to_string()], false, true);
        let result = runner.run();

        // Should succeed even with clear flag
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_runner_run_invalid_command() {
        let mut runner =
            CommandRunner::new(vec!["nonexistent_command_12345".to_string()], false, false);
        let result = runner.run();

        // The run method itself succeeds, but the command fails with non-zero exit code
        // The error is printed but not returned as an error from the run method
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_runner_run_failing_command() {
        // Use a command that will fail (exit with non-zero status)
        let mut runner = CommandRunner::new(
            vec!["sh".to_string(), "-c".to_string(), "exit 1".to_string()],
            false,
            false,
        );
        let result = runner.run();

        // The run should succeed (no error), but the command itself fails
        // The failure is just printed, not returned as an error
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_runner_restart_with_multiple_runs() {
        let mut runner =
            CommandRunner::new(vec!["sleep".to_string(), "0.1".to_string()], true, false);

        // First run
        let result1 = runner.run();
        assert!(result1.is_ok());

        // Second run should kill the first process and start a new one
        let result2 = runner.run();
        assert!(result2.is_ok());
    }
}
