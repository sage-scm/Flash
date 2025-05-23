# Flash ⚡

[![CI](https://github.com/sage-scm/Flash/workflows/CI/badge.svg)](https://github.com/sage-scm/Flash/actions)
[![Crates.io](https://img.shields.io/crates/v/flash-watcher.svg)](https://crates.io/crates/flash-watcher)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

An impossibly fast file watcher that executes commands when files change.

Think `nodemon`, but more general purpose and written in Rust.

## Features

- ⚡ **Blazingly fast** - Built with Rust for maximum performance
- 🎯 **Flexible filtering** - Support for glob patterns, file extensions, and ignore patterns
- 🔧 **Configurable** - YAML configuration files for complex setups
- 📊 **Performance monitoring** - Built-in statistics and benchmarking
- 🔄 **Process management** - Restart long-running processes or spawn new ones
- 🌍 **Cross-platform** - Works on Windows, macOS, and Linux
- 🎨 **Beautiful output** - Colored terminal output with clear status messages

## Installation

### From Crates.io (Recommended)

```sh
cargo install flash-watcher
```

### From Source

```sh
git clone https://github.com/sage-scm/Flash.git
cd Flash
cargo install --path .
```

### Pre-built Binaries

Download pre-built binaries from the [releases page](https://github.com/sage-scm/Flash/releases).

## Usage

```sh
flash [OPTIONS] <COMMAND>...
```

### Arguments

- `<COMMAND>...`: Command to run when files change

### Options

- `-w, --watch <WATCH>`: Paths/patterns to watch (supports glob patterns like `src/**/*.js`)
- `-e, --ext <EXT>`: File extensions to watch (e.g., "js,jsx,ts,tsx")
- `-p, --pattern <PATTERN>`: Specific glob patterns to include (e.g., "src/**/*.{js,ts}")
- `-i, --ignore <IGNORE>`: Glob patterns to ignore (e.g., "**/node_modules/**")
- `-d, --debounce <DEBOUNCE>`: Debounce time in milliseconds [default: 100]
- `-r, --restart`: Restart long-running processes instead of spawning new ones
- `-c, --clear`: Clear console before each command run
- `-n, --initial`: Run command on startup
- `-f, --config <CONFIG>`: Use configuration from file
- `--stats`: Show performance statistics while running
- `--stats-interval <SECONDS>`: Statistics update interval in seconds [default: 10]
- `--bench`: Run benchmark against other file watchers
- `-h, --help`: Print help
- `-V, --version`: Print version

## Glob Pattern Support

Flash supports powerful glob pattern matching for both watching files and filtering them:

### Watch Patterns (`-w`)

Watch specific file patterns directly:

```sh
# Watch all JavaScript files in src directory
flash -w "src/**/*.js" echo "JS file changed"

# Watch multiple specific patterns
flash -w "src/**/*.js" -w "public/**/*.css" echo "File changed"
```

### Ignore Patterns (`-i`)

Ignore specific directories or files:

```sh
# Ignore node_modules and dist directories anywhere in the tree
flash -w "." -i "**/node_modules/**" -i "**/dist/**" echo "File changed"

# Ignore minified files
flash -w "src" -i "**/*.min.js" echo "File changed"
```

### Include Patterns (`-p`)

Specifically include only certain file patterns:

```sh
# Only include TypeScript files in src and test directories
flash -w "." -p "src/**/*.ts" -p "test/**/*.ts" echo "TS file changed"
```

### Combining Options

The most powerful usage comes from combining these options:

```sh
flash -w "." -e "js,ts" -p "src/**/*.{js,ts}" -i "**/node_modules/**" -i "**/dist/**" echo "File changed"
```

## Examples

Watch current directory and restart a Node.js server when changes occur:
```sh
flash -r node server.js
```

Watch TypeScript files in the src directory and run the build script:
```sh
flash -w src -e ts npm run build
```

Watch multiple directories but ignore node_modules:
```sh
flash -w src -w tests -i "**/node_modules/**" cargo test
```

Watch using glob patterns to include only specific files:
```sh
flash -p "src/**/*.{js,jsx,ts,tsx}" -p "public/**/*.css" npm run build
```

Clear console and run command on startup:
```sh
flash -c -n -r npm start
```

Run with performance statistics:
```sh
flash --stats --stats-interval 5 npm run dev
```

## Configuration File

You can define a configuration file in YAML format to avoid typing long commands:

```yaml
# flash.yaml
command: ["npm", "run", "dev"]
watch:
  - "src/**" # Watch all files in src directory recursively
  - "public/*.html" # Watch HTML files in public directory

ext: "js,jsx,ts,tsx"

pattern:
  - "src/**/*.{js,jsx,ts,tsx}" # JavaScript/TypeScript files in src

ignore:
  - "**/node_modules/**" # Ignore node_modules directory
  - "**/.git/**" # Ignore .git directory
  - "**/*.min.js" # Ignore minified JS files

debounce: 200
initial: true
clear: true
restart: true
```

Then run Flash with:

```sh
flash -f flash.yaml
```

You can also override configuration file settings with command line arguments.

## Common Use Cases

### Web Development

```sh
flash -w "src/**" -w "public/**" -e js,jsx,ts,tsx,css,html -i "**/node_modules/**" -r -c -n npm start
```

### Rust Development

```sh
flash -w "src/**/*.rs" -w "tests/**/*.rs" -i "target/**" -c cargo test
```

### Documentation

```sh
flash -w "docs/**/*.md" -c -n mdbook build
```

## Performance and Benchmarks

Flash is designed to be blazingly fast and resource efficient. To see how it compares to other file watchers:

```sh
flash --bench
```

This will run a series of benchmarks comparing Flash against popular file watchers like nodemon, watchexec, and cargo-watch.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.

## Support

- 📖 **Documentation**: Check the [README](README.md) and [examples](example.flash.yaml)
- 🐛 **Bug Reports**: [Open an issue](https://github.com/sage-scm/Flash/issues/new)
- 💡 **Feature Requests**: [Open an issue](https://github.com/sage-scm/Flash/issues/new)
- 💬 **Questions**: [Start a discussion](https://github.com/sage-scm/Flash/discussions)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.