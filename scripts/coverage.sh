#!/bin/bash

# Flash Coverage Script
# Generates code coverage reports while avoiding long-running benchmarks

set -e

echo "🔍 Generating code coverage for Flash..."

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "❌ cargo-llvm-cov is not installed. Installing..."
    cargo install cargo-llvm-cov
fi

# Backup Cargo.toml
cp Cargo.toml Cargo.toml.backup

# Temporarily disable benchmarks to avoid long execution times
echo "⏸️  Temporarily disabling benchmarks..."
sed -i.bak 's/^\[\[bench\]\]/# [[bench]]/' Cargo.toml
sed -i.bak 's/^name = "file_watcher"/# name = "file_watcher"/' Cargo.toml
sed -i.bak 's/^harness = false/# harness = false/' Cargo.toml
sed -i.bak 's/^required-features = \["benchmarks"\]/# required-features = ["benchmarks"]/' Cargo.toml

# Clean previous coverage data
echo "🧹 Cleaning previous coverage data..."
cargo llvm-cov clean

# Generate coverage reports
echo "📊 Generating coverage reports..."
cargo llvm-cov --all-features --workspace --tests --lcov --output-path lcov.info
cargo llvm-cov --all-features --workspace --tests --html --output-dir coverage-html
echo "📈 Coverage summary:"
cargo llvm-cov --all-features --workspace --tests --summary-only

# Restore Cargo.toml
echo "🔄 Restoring benchmarks configuration..."
mv Cargo.toml.backup Cargo.toml
rm -f Cargo.toml.bak

echo ""
echo "✅ Coverage reports generated successfully!"
echo "📁 HTML report: coverage-html/html/index.html"
echo "📁 LCOV report: lcov.info"
echo ""
echo "🌐 Open HTML report with:"
echo "   open coverage-html/html/index.html"
