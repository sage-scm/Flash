# Code Coverage

Flash uses `cargo-llvm-cov` to generate code coverage reports. Coverage is automatically generated on every push to the main branch.

## Viewing Coverage Reports

### GitHub Actions Artifacts

Coverage reports are automatically generated and uploaded as artifacts in GitHub Actions:

1. Go to the [Actions tab](https://github.com/sage-scm/Flash/actions)
2. Click on the latest CI run for the main branch
3. Download the "coverage-reports" artifact
4. Extract and open `coverage-html/index.html` in your browser

### Local Coverage Generation

To generate coverage reports locally:

```bash
# Install cargo-llvm-cov if not already installed
cargo install cargo-llvm-cov

# Generate HTML coverage report
cargo llvm-cov --all-features --workspace --html --output-dir coverage-html

# Generate LCOV format (for external tools)
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Show coverage summary in terminal
cargo llvm-cov --all-features --workspace --summary-only
```

## Coverage Goals

We aim to maintain:
- **Overall coverage**: > 80%
- **Critical paths**: > 90%
- **New features**: 100% coverage required

## Excluded Files

The following files are excluded from coverage:
- Benchmark files (`benches/`)
- Example files
- Generated code

## Coverage in CI/CD

Coverage is generated automatically in GitHub Actions without requiring external services like Codecov. Reports are stored as artifacts for 30 days and can be downloaded for analysis.

This approach provides:
- ✅ **Free**: No external service costs
- ✅ **Private**: Coverage data stays in your repository
- ✅ **Accessible**: Download reports directly from GitHub
- ✅ **Automated**: Generated on every main branch push
