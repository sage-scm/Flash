# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/).

## [0.2.0] — 2026-05-16

### Fixed
- **Issue #1**: a non-existent `-w` path no longer silently watches the current
  working directory. Flash now exits with a non-zero status and a clear error
  message before any command is run, so a typo can't drop you into an infinite
  loop. (reported by @wezm)

### Performance
- **Skip the shell when it isn't needed.** If your command parses as
  `program arg1 arg2 …`, Flash now `exec`s it directly instead of going
  through `sh -c`. That saves a fork on every detection — on the order of
  4–10 ms — and removes a class of argument-quoting surprises. Pipelines,
  redirects, and chained commands (`cargo test && echo done`) still drop
  to the shell automatically.
- **Default debounce dropped from 100 ms to 50 ms**, matching watchexec.
  Feedback loops feel half a beat snappier without any loss of correctness
  on editor write bursts.
- **Bench is apples-to-apples.** The probe command is now `touch <marker>`
  delivered as discrete args, so both Flash and watchexec measure their
  direct-exec path. On an Apple Silicon Mac that puts Flash at ~22 ms
  detection latency vs. ~30 ms for watchexec, with the rest of the table
  unchanged.

### Changed
- Replaced the hand-rolled per-path debounce with `notify-debouncer-mini`,
  which coalesces editor write storms correctly under load.
- Swapped `glob` for [`globset`], so brace-expansion patterns like
  `src/**/*.{ts,tsx}` actually work the way the README always claimed.
- Rebuilt the `--bench` flag end-to-end: it now spawns every supported file
  watcher installed on your machine, takes real samples, and reports the
  median — no canned numbers anywhere in the binary.
- Reorganised the codebase into focused modules (`cli`, `config`, `filter`,
  `runner`, `watcher`, `stats`, `bench`) and rewrote the test suite around
  them. End-to-end tests now exercise the binary directly so behavioural
  regressions are caught in CI rather than in the field.
- Release profile tightened (`lto = "fat"`, single codegen unit, stripped),
  shaving the binary down to ~1.5 MiB on Apple Silicon.

### Removed
- `std::mem::forget(watcher)` — the watcher's lifetime is now tied to the
  event loop, so dropping out of `run()` cleans up properly.
- The hand-coded "sample data" benchmark output and the `bench_results` module
  that backed it.
- `chrono`, `glob`, `walkdir`, `which`, and `criterion` from the dependency
  tree.
- The various ad-hoc shell scripts (`test-flash.sh`, `test-glob-patterns.sh`,
  `validate-performance.sh`, `performance-report.sh`). Run `cargo test` and
  `flash-watcher --bench` instead.

## [0.1.2] — 2026-05

- Minor CI fix-ups.

## [0.1.0] — Initial release

- Recursive file watching with command execution.
- Configurable extension, include, and ignore filters.
- YAML configuration files.
- Initial-run, restart, and clear-terminal options.

[`globset`]: https://docs.rs/globset
