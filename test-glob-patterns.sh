#!/bin/bash

# Clean up when script exits
cleanup() {
  echo -e "\n\033[0;34mCleaning up test environment...\033[0m"
  if [ -n "$flash_pid" ]; then
    kill $flash_pid 2>/dev/null
  fi
  rm -rf "$test_dir"
}

trap cleanup EXIT
trap cleanup INT

# Create a test directory structure with nested dirs and various file types
echo -e "\033[0;32m=== Setting up test environment ===\033[0m"
test_dir=$(mktemp -d)

# Create main source directories
mkdir -p "$test_dir/src/components"
mkdir -p "$test_dir/src/utils"
mkdir -p "$test_dir/public/css"
mkdir -p "$test_dir/public/js"
mkdir -p "$test_dir/node_modules/some-package"
mkdir -p "$test_dir/dist"

# Create test files of different types in various locations
echo 'console.log("Main entry");' > "$test_dir/src/index.js"
echo 'export const Button = () => {};' > "$test_dir/src/components/Button.jsx"
echo 'export const utils = {};' > "$test_dir/src/utils/helpers.ts"
echo 'body { color: black; }' > "$test_dir/public/css/style.css"
echo 'function main() {}' > "$test_dir/public/js/main.js"
echo 'console.log("Minified");' > "$test_dir/public/js/app.min.js"
echo 'module.exports = {};' > "$test_dir/node_modules/some-package/index.js"
echo 'const bundled = {};' > "$test_dir/dist/bundle.js"

echo -e "\033[0;34mTest files created in:\033[0m $test_dir"
echo -e "\033[0;34mDirectory structure:\033[0m"
find "$test_dir" -type f | sort

# Build Flash in debug mode
echo -e "\n\033[0;32m=== Building Flash ===\033[0m"
cargo build || { echo "Failed to build Flash"; exit 1; }

# Test glob pattern examples
run_test() {
  local name=$1
  local cmd=$2
  
  echo -e "\n\033[0;32m=== Testing: $name ===\033[0m"
  echo -e "\033[0;34mRunning:\033[0m $cmd"
  
  # Run Flash with the specified glob patterns
  eval "$cmd" &
  flash_pid=$!
  
  # Wait for Flash to initialize and show initial output
  sleep 2
  
  # Make a simple change to trigger the watcher
  echo 'console.log("Updated");' > "$test_dir/src/index.js"
  sleep 2
  
  # Terminate Flash
  kill $flash_pid 2>/dev/null
  wait $flash_pid 2>/dev/null
  echo -e "\033[0;34mTest completed.\033[0m"
}

# Run tests with different glob pattern configurations
run_test "Watch all JS files" "./target/debug/flash -w \"$test_dir/src/**/*.js\" -w \"$test_dir/public/**/*.js\" -i \"**/node_modules/**\" -i \"**/dist/**\" -i \"**/*.min.js\" echo \"Change detected\""

run_test "Watch specific extensions" "./target/debug/flash -w \"$test_dir\" -e js,jsx,ts -i \"**/node_modules/**\" -i \"**/dist/**\" echo \"Change detected\""

run_test "Custom include patterns" "./target/debug/flash -w \"$test_dir\" -p \"$test_dir/src/**/*.{js,ts}\" -p \"$test_dir/public/js/*.js\" -i \"**/*.min.js\" echo \"Change detected\""

echo -e "\n\033[0;32m=== All tests completed ===\033[0m"
echo -e "Press Enter to exit and clean up..."
read

# Cleanup is handled by the trap 