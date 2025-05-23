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
}
