# Contributing to Flash ⚡

Thank you for your interest in contributing to Flash! We welcome contributions from everyone.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/flash.git
   cd flash
   ```
3. **Create a new branch** for your feature or bugfix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Benchmarks

```bash
cargo bench
```

## Making Changes

1. **Write tests** for your changes when applicable
2. **Ensure all tests pass**: `cargo test`
3. **Check formatting**: `cargo fmt`
4. **Run clippy**: `cargo clippy`
5. **Update documentation** if needed

## Submitting Changes

1. **Commit your changes** with a clear commit message:
   ```bash
   git commit -m "Add feature: description of your changes"
   ```
2. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```
3. **Create a Pull Request** on GitHub

## Pull Request Guidelines

- **Describe your changes** clearly in the PR description
- **Reference any related issues** using `#issue-number`
- **Keep PRs focused** - one feature or fix per PR
- **Update tests** and documentation as needed
- **Ensure CI passes** before requesting review

## Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and small

## Reporting Issues

When reporting issues, please include:

- **Operating system** and version
- **Rust version** (`rustc --version`)
- **Flash version** or commit hash
- **Steps to reproduce** the issue
- **Expected vs actual behavior**
- **Error messages** or logs if applicable

## Feature Requests

We welcome feature requests! Please:

- **Check existing issues** to avoid duplicates
- **Describe the use case** clearly
- **Explain why** the feature would be valuable
- **Consider implementation** if you're willing to contribute

## Questions?

Feel free to open an issue for questions or join discussions in existing issues.

Thank you for contributing! 🚀
