# Flash

[![CI](https://github.com/sage-scm/Flash/actions/workflows/ci.yml/badge.svg)](https://github.com/sage-scm/Flash/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/flash-watcher.svg)](https://crates.io/crates/flash-watcher)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fast, predictable file watcher that runs commands when files change. Think
`nodemon`, but a single static binary with no runtime to boot.

```sh
flash-watcher -w src cargo test
```

That's it. Save a file under `src`, your tests rerun.

## Why Flash

- **Tight, native event loop** — built on [`notify`] + [`notify-debouncer-mini`],
  the same battle-tested stack used by watchexec and cargo-watch.
- **Sensible defaults** — debounce coalesces editor write storms, glob filters
  do the obvious thing, and `--restart` keeps long-running servers fresh.
- **Honest about performance** — `flash-watcher --bench` measures Flash against
  every other watcher installed on your machine and prints the numbers. We
  don't ship hard-coded marketing values.
- **Loud when it should be, quiet when it shouldn't** — clear errors when your
  watch path is wrong, a quieter `--fast` mode for tight inner loops.

## Install

```sh
# from crates.io
cargo install flash-watcher

# from source
git clone https://github.com/sage-scm/Flash.git
cd Flash
cargo install --path .
```

## Usage

```text
flash-watcher [OPTIONS] -- <COMMAND>...

  -w, --watch <PATH>          Path or glob to watch (repeat to add more)
  -e, --ext <LIST>            Comma-separated extensions to keep (e.g. "rs,toml")
  -p, --pattern <GLOB>        Include only paths matching this glob
  -i, --ignore <GLOB>         Drop paths matching this glob
  -d, --debounce <MS>         Debounce window in milliseconds [default: 50]
  -n, --initial               Run the command once on startup, before watching
  -c, --clear                 Clear the terminal before each run
  -r, --restart               Restart the previous process instead of spawning anew
  -f, --config <FILE>         Load defaults from a YAML configuration file
      --fast                  Quieter output, leaner startup path
      --stats                 Periodically print live counters
      --stats-interval <S>    How often to refresh statistics [default: 10]
      --bench                 Benchmark Flash against installed watchers, then exit
  -h, --help                  Print help
  -V, --version               Print version
```

### A few recipes

```sh
# Re-run tests when any Rust source changes
flash-watcher -w src -e rs cargo test

# Restart a dev server on save, clearing the terminal each time
flash-watcher -r -c -n npm run dev

# Watch only TypeScript under src/, ignore generated files
flash-watcher -p 'src/**/*.{ts,tsx}' -i '**/*.generated.ts' npm run build

# Load everything from a config file
flash-watcher -f flash.yaml
```

### Configuration file

Every flag has a YAML counterpart:

```yaml
# flash.yaml
command: ["cargo", "test"]
watch:
  - src
  - tests
ext: "rs"
ignore:
  - "**/target/**"
debounce: 200
restart: true
clear: true
initial: true
```

CLI flags always win over the config file — the file fills in whatever you
didn't pass on the command line.

### Glob support

Includes (`-p`) and ignores (`-i`) use [globset], so the obvious things work:

```text
src/**/*.rs              all Rust files under src/
src/**/*.{js,ts,tsx}     brace expansion is supported
**/__snapshots__/**      anything nested in __snapshots__
```

Passing a glob to `-w` is also supported — Flash watches the longest fixed
prefix of the pattern and applies the glob as a filter.

## Performance

Flash sits on the same native event-loop machinery as the fastest watchers in
the ecosystem, so the headline number is: **it stays out of your way**.

Run the bundled, transparent benchmark on your own machine:

```sh
flash-watcher --bench
```

This spawns each watcher it can find on your `PATH`, takes 5 samples per
metric, and reports the median for binary launch time, end-to-end change
detection latency, and resident memory at steady state. Every measurement is
real and reproducible — there are no canned numbers anywhere in the binary.

Representative output (Apple Silicon, macOS, medians across five runs with
all three watchers using their direct-exec paths):

```text
Binary launch (smaller is better)
  flash-watcher     5.21 ms
  cargo-watch       5.25 ms
  watchexec         5.26 ms

Change-detection latency (smaller is better)
  flash-watcher    30.84 ms
  watchexec        33.88 ms
  cargo-watch     527.51 ms

Resident memory (smaller is better)
  flash-watcher    7.11 MiB
  cargo-watch     10.94 MiB
  watchexec       14.03 MiB
```

See [PERFORMANCE.md](PERFORMANCE.md) for a deeper write-up of the methodology.

## Contributing

Bug reports and pull requests are very welcome. See [CONTRIBUTING.md] for the
short version: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`.

## License

MIT — see [LICENSE](LICENSE).

[`notify`]: https://docs.rs/notify
[`notify-debouncer-mini`]: https://docs.rs/notify-debouncer-mini
[globset]: https://docs.rs/globset
[CONTRIBUTING.md]: CONTRIBUTING.md
