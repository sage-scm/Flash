use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

// Integration tests for the Flash watcher
#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a temporary directory with test files
    fn setup_test_dir() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Create a source directory
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create some test files
        let js_file = src_dir.join("test.js");
        let mut file = File::create(js_file).expect("Failed to create test.js");
        writeln!(file, "console.log('hello');").expect("Failed to write to test.js");

        let css_file = src_dir.join("style.css");
        let mut file = File::create(css_file).expect("Failed to create style.css");
        writeln!(file, "body {{ color: black; }}").expect("Failed to write to style.css");

        temp_dir
    }

    /// Helper function to run Flash with specified args
    fn run_flash(args: &[&str], working_dir: &PathBuf) -> (Command, String) {
        // Build the binary path - adjust based on your project structure
        let flash_binary = env::current_dir()
            .expect("Failed to get current dir")
            .join("target/debug/flash");

        // Build the full command
        let mut command = Command::new(&flash_binary);
        command
            .args(args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Return command so caller can spawn it
        let cmd_str = format!("{:?} {:?}", flash_binary, args);
        (command, cmd_str)
    }

    // This test requires the binary to be built but will run by default
    #[test]
    fn test_flash_watches_file_changes() {
        // First, ensure the binary is built
        Command::new("cargo")
            .args(["build"])
            .status()
            .expect("Failed to build flash binary");

        // Setup test directory
        let temp_dir = setup_test_dir();
        let test_dir = temp_dir.path().to_path_buf();

        // Run flash with a simple echo command watching JS files
        let (mut command, cmd_str) = run_flash(
            &[
                "-w",
                "src", // Watch the src directory
                "-e",
                "js",        // Watch only JS files
                "--initial", // Run once on startup
                "echo",
                "File changed",
            ],
            &test_dir,
        );

        println!("Running: {}", cmd_str);

        // Start flash process
        let mut child = command.spawn().expect("Failed to start flash");

        // Give it time to initialize
        thread::sleep(Duration::from_millis(500));

        // Modify a JS file (should trigger the watcher)
        let js_file = test_dir.join("src/test.js");
        let mut file = File::create(js_file).expect("Failed to open test.js");
        writeln!(file, "console.log('updated');").expect("Failed to update test.js");

        // Wait for the file change to be detected
        thread::sleep(Duration::from_millis(1000));

        // Modify a CSS file (should NOT trigger the watcher)
        let css_file = test_dir.join("src/style.css");
        let mut file = File::create(css_file).expect("Failed to open style.css");
        writeln!(file, "body {{ color: blue; }}").expect("Failed to update style.css");

        // Wait a bit more
        thread::sleep(Duration::from_millis(1000));

        // Kill the process
        child.kill().expect("Failed to kill flash process");

        // Collect output
        let output = child.wait_with_output().expect("Failed to wait for output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        println!("Flash stdout: {}", stdout);
        println!("Flash stderr: {}", stderr);

        // Check that flash started correctly
        assert!(stdout.contains("Flash watching for changes"));

        // Check that it detected the JS file change
        assert!(stdout.contains("Change detected:") && stdout.contains("test.js"));

        // It should NOT detect the CSS file change
        assert!(!stdout.contains("style.css"));
    }

    // Simplified test that doesn't actually run the binary but verifies the test setup
    #[test]
    fn test_integration_setup() {
        let temp_dir = setup_test_dir();

        // Verify test files were created
        let js_file = temp_dir.path().join("src/test.js");
        let css_file = temp_dir.path().join("src/style.css");

        assert!(js_file.exists(), "test.js was not created");
        assert!(css_file.exists(), "style.css was not created");
    }
}
