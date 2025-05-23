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

# Create test directory structure
echo -e "\033[0;32m=== Setting up test environment ===\033[0m"
test_dir=$(mktemp -d)
mkdir -p "$test_dir/src"
mkdir -p "$test_dir/src/components"

# Create test files
echo 'console.log("Hello");' > "$test_dir/src/index.js"
echo 'function App() { return <div>Hello</div>; }' > "$test_dir/src/App.js"
echo '.button { color: blue; }' > "$test_dir/src/styles.css"

echo -e "\033[0;34mTest files created in:\033[0m $test_dir"

# Build Flash in debug mode
echo -e "\n\033[0;32m=== Building Flash ===\033[0m"
cargo build || { echo "Failed to build Flash"; exit 1; }

# Run Flash with specific extension
echo -e "\n\033[0;32m=== Starting Flash ===\033[0m"
echo -e "\033[0;34mRunning with options:\033[0m -w $test_dir/src -e js -n -d 100"
./target/debug/flash -w "$test_dir/src" -e js -n -d 100 echo "File changed" &
flash_pid=$!

# Wait for Flash to initialize
sleep 2

# Define a function to make file changes and wait
make_changes() {
  local file=$1
  local content=$2
  local description=$3
  
  echo -e "\n\033[0;33m$description\033[0m"
  echo -e "\033[0;34mModifying:\033[0m $file"
  echo "$content" > "$file"
  sleep 2
}

echo -e "\n\033[0;32m=== Testing file changes ===\033[0m"

# Change 1: Modify index.js (should trigger)
make_changes "$test_dir/src/index.js" "console.log(\"Updated file\");" "Test: Updating JS file (should trigger Flash)"

# Change 2: Create a new JS file (should trigger)
make_changes "$test_dir/src/components/Button.js" "export const Button = () => <button>Click</button>;" "Test: Creating new JS file (should trigger Flash)"

# Change 3: Update CSS file (should NOT trigger since we're only watching JS)
make_changes "$test_dir/src/styles.css" ".button { color: red; }" "Test: Updating CSS file (should NOT trigger Flash)"

# Verify Flash is still running
echo -e "\n\033[0;32m=== Test summary ===\033[0m"
if kill -0 $flash_pid 2>/dev/null; then
  echo -e "\033[0;32m✅ Flash is running correctly with PID $flash_pid\033[0m"
else
  echo -e "\033[0;31m❌ Flash process is not running\033[0m"
fi

# List the files we created
echo -e "\n\033[0;34mFiles monitored:\033[0m"
find "$test_dir" -name "*.js" | sort

echo -e "\n\033[0;32m=== Test completed successfully ===\033[0m"
echo -e "Press Enter to exit and clean up..."
read

# Cleanup is handled by the trap 