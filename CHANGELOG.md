# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of Flash file watcher
- Command-line interface with comprehensive options
- Glob pattern support for watching and filtering files
- Configuration file support (YAML format)
- Performance statistics and monitoring
- Benchmark suite comparing against other file watchers
- Debouncing to prevent excessive command execution
- Process restart capability for long-running commands
- Console clearing option
- Initial command execution option
- Comprehensive test suite

### Features
- **Fast file watching** using Rust's notify library
- **Glob pattern matching** for flexible file filtering
- **Multiple watch directories** support
- **File extension filtering** (e.g., "js,jsx,ts,tsx")
- **Ignore patterns** to exclude directories like node_modules
- **Configuration files** for complex setups
- **Performance monitoring** with real-time statistics
- **Cross-platform support** (Windows, macOS, Linux)

## [0.1.0] - 2024-01-XX

### Added
- Initial implementation of Flash file watcher
- Core file watching functionality
- Command execution on file changes
- Basic CLI interface
- Documentation and examples
