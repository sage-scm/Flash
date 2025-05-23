#!/bin/bash

# Flash Benchmark Script
# Runs performance benchmarks with proper feature flags

set -e

echo "🚀 Running Flash benchmarks..."

# Check if criterion is available
if ! grep -q "criterion" Cargo.toml; then
    echo "❌ Criterion not found in dependencies. Please add it to [dev-dependencies]."
    exit 1
fi

# Check if gnuplot is available for better charts
if command -v gnuplot &> /dev/null; then
    echo "📊 Gnuplot detected - will generate enhanced charts"
else
    echo "⚠️  Gnuplot not found - using plotters backend for charts"
    echo "   Install gnuplot for better charts: brew install gnuplot (macOS) or apt-get install gnuplot (Ubuntu)"
fi

echo ""
echo "⏱️  Running benchmarks (this may take several minutes)..."
echo "   Use Ctrl+C to cancel if needed"
echo ""

# Run benchmarks with the benchmarks feature enabled
cargo bench --features benchmarks --verbose

echo ""
echo "✅ Benchmarks completed!"
echo ""
echo "📁 Results saved to: target/criterion/"
echo "🌐 Open benchmark report with:"
echo "   open target/criterion/report/index.html"
echo ""
echo "💡 To run specific benchmarks:"
echo "   cargo bench --features benchmarks startup_time"
echo ""
echo "📊 To compare with other file watchers:"
echo "   flash --bench"
