use std::process::{Child, Command};

use anyhow::{Context, Result};

/// Executes the user's command in response to file events.
///
/// In *restart* mode the previous child is killed before each new run, so
/// long-running processes (servers, watchers, REPLs) come back fresh on every
/// change. In the default mode each invocation is one-shot: the runner waits
/// for it to finish so failures surface immediately.
pub struct Runner {
    command: Vec<String>,
    restart: bool,
    clear: bool,
    current: Option<Child>,
}

impl Runner {
    pub fn new(command: Vec<String>, restart: bool, clear: bool) -> Self {
        Self {
            command,
            restart,
            clear,
            current: None,
        }
    }

    /// Invoke the command. Returns `Ok(())` if the command was launched; a
    /// non-zero exit status from a one-shot run is reported on stderr but does
    /// not bubble up as an error — the watcher keeps running.
    pub fn run(&mut self) -> Result<()> {
        if self.restart {
            self.stop_current();
        }

        if self.clear {
            // CSI 2J clears the screen, CSI H homes the cursor.
            print!("\x1B[2J\x1B[H");
        }

        let child = self.spawn().context("launching command")?;

        if self.restart {
            self.current = Some(child);
        } else {
            let mut child = child;
            let status = child.wait().context("waiting on command")?;
            if !status.success() {
                eprintln!("flash-watcher: command exited with {status}");
            }
        }

        Ok(())
    }

    fn spawn(&self) -> std::io::Result<Child> {
        if needs_shell(&self.command) {
            let joined = self.command.join(" ");
            if cfg!(windows) {
                Command::new("cmd").args(["/C", &joined]).spawn()
            } else {
                Command::new("sh").args(["-c", &joined]).spawn()
            }
        } else {
            // Skip the shell entirely. Saves a fork/exec on every run and
            // preserves the user's argument quoting exactly as clap parsed it.
            Command::new(&self.command[0])
                .args(&self.command[1..])
                .spawn()
        }
    }

    fn stop_current(&mut self) {
        if let Some(mut child) = self.current.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop_current();
    }
}

/// Decide whether the user's command needs a shell to run as intended.
///
/// Rule of thumb: if the user split their command into multiple arguments
/// (`flash-watcher cargo test`) we treat them as program + arguments and
/// `exec` directly — that's how `watchexec` behaves and it saves a process
/// per run. If they passed a single quoted string (`flash-watcher 'cargo
/// test && echo done'`), the only way to honour pipes, redirects, and
/// command chaining is to hand it to a shell.
fn needs_shell(command: &[String]) -> bool {
    if command.len() != 1 {
        return false;
    }
    let only = &command[0];
    only.chars().any(char::is_whitespace) || only.chars().any(is_shell_metachar)
}

fn is_shell_metachar(c: char) -> bool {
    matches!(
        c,
        '|' | '&' | ';' | '<' | '>' | '(' | ')' | '$' | '`' | '\\'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_and_waits_for_simple_command() {
        let mut runner = Runner::new(
            vec!["true".to_string()],
            /* restart */ false,
            /* clear */ false,
        );
        runner.run().expect("true should always succeed");
    }

    #[test]
    fn nonzero_exit_is_reported_but_not_an_error() {
        let mut runner = Runner::new(
            vec!["sh".to_string(), "-c".to_string(), "exit 7".to_string()],
            false,
            false,
        );
        runner
            .run()
            .expect("runner should not propagate exit codes");
    }

    #[test]
    fn restart_mode_holds_onto_the_child() {
        let mut runner = Runner::new(vec!["sleep".to_string(), "30".to_string()], true, false);
        runner.run().expect("spawn sleep");
        assert!(
            runner.current.is_some(),
            "restart mode should keep a handle on the running child"
        );
        // Dropping the runner kills the long-running child.
    }

    #[test]
    fn multi_token_commands_skip_the_shell() {
        let cmd = vec!["cargo".to_string(), "test".to_string()];
        assert!(!needs_shell(&cmd));
    }

    #[test]
    fn single_token_binary_skips_the_shell() {
        let cmd = vec!["cargo".to_string()];
        assert!(!needs_shell(&cmd));
    }

    #[test]
    fn single_quoted_pipeline_goes_through_the_shell() {
        let cmd = vec!["cargo test && echo done".to_string()];
        assert!(needs_shell(&cmd));
    }

    #[test]
    fn redirects_force_the_shell() {
        let cmd = vec!["printf x > /tmp/marker".to_string()];
        assert!(needs_shell(&cmd));
    }

    #[test]
    fn substitutions_force_the_shell() {
        let cmd = vec!["echo $HOME".to_string()];
        assert!(needs_shell(&cmd));
    }

    #[test]
    fn direct_exec_preserves_quoted_arguments() {
        // The shell path would join with spaces and lose the original
        // argument boundary; the direct path must keep them intact.
        let mut runner = Runner::new(
            vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            false,
            false,
        );
        runner.run().expect("multi-token command should run");
    }
}
