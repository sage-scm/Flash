# Flash performance

> tl;dr — Flash uses the same native event-loop stack as every fast watcher
> in this ecosystem. On the metrics that matter to a feedback loop (memory
> footprint, change-to-command latency) it lands at or near the top. The
> numbers below are not marketing copy; they were produced by running
> `flash-watcher --bench` on the author's machine. You can run it on yours.

## Run the benchmark yourself

```sh
cargo install flash-watcher          # or build from source: cargo build --release
flash-watcher --bench
```

`--bench` discovers every supported watcher on your `PATH`, then for each one
takes five samples of three quantities:

1. **Binary launch** — wall time from `spawn()` to process exit on `--help`.
   A clean proxy for "what does it cost to even start this thing".
2. **Change-detection latency** — wall time from writing a file in the watched
   directory to the watcher's command writing its output. Measures the
   complete end-to-end loop, including debouncer + scheduler + fork.
3. **Resident memory** — resident set size sampled after 1.5 s of steady state.

The reported value is the median, to dampen one-off scheduler noise. No
warmup runs, no statistical claims beyond "median of five". Users who want
rigorous numbers should reach for `hyperfine` (for launch time) or build a
domain-specific harness.

Currently supported on the comparison side: `watchexec`, `nodemon`,
`cargo-watch`, `entr`. Anything not on `PATH` is skipped, not faked.

## Reference numbers

Captured 2026-05 on an Apple Silicon Mac running macOS. Both Flash and
watchexec are measured with their direct-exec paths; cargo-watch always wraps
the command in a shell.

| Metric                     | flash-watcher | watchexec | cargo-watch |
| -------------------------- | ------------: | --------: | ----------: |
| Binary launch (median, ms) |          5.22 |      5.28 |        5.23 |
| Detection latency (ms)     |         22.49 |     30.73 |      513.45 |
| Resident memory (steady)   |       7.1 MiB |  14.0 MiB |    10.9 MiB |

Some observations:

- **Detection latency is where day-to-day feedback lives**, and Flash now
  shaves ~8 ms off watchexec by skipping the shell for plain commands. That
  margin holds across runs on this hardware.
- **Binary launch is effectively a three-way tie.** All three are within
  scheduler noise of each other; if startup speed is what you care about, any
  of them is a good answer.
- **Memory footprint** is the cleanest win. Flash sits at ~7 MiB resident
  versus 11 MiB (cargo-watch) and 14 MiB (watchexec), because we keep the
  dependency graph deliberately small.

Numbers for `nodemon` and `entr` are omitted from this table because they were
not installed on the reference machine. Install them and they will appear in
your own `--bench` output.

## Methodology notes

A few decisions that matter when reading the numbers:

- **Detection includes the debounce window.** Flash defaults to 50 ms; the
  bench shrinks it to 10 ms so all watchers are compared on similar footing.
  In day-to-day use, the debounce window dominates "perceived" latency, so
  tuning it matters.
- **The probe command is `touch <marker>` delivered as discrete args**, which
  is the cheapest observable side-effect we can ask each watcher to perform.
  Both Flash and watchexec execute it directly (no shell layer); nodemon,
  cargo-watch and entr wrap it in `sh -c` because their argument APIs
  require a single string.
- **Memory is RSS, not heap.** RSS is what `top` shows you and what your
  laptop's battery cares about. It includes shared libraries and JIT'd code,
  so node-based watchers are at a structural disadvantage here.
- **Launch is `--help`, not "first event observed".** Measuring the latter
  fairly across watchers is hard, because each defines "ready" differently
  and many emit phantom initial-scan events. `--help` is a clean, comparable
  proxy that captures binary load + arg parsing + clean exit.

## Why we replaced the old benchmark

Earlier versions of this document quoted numbers like "2.1 ms startup" and
"1.7× faster than watchexec." Those came from a script that measured
`--help` invocations and folded them into the headline as "startup time".
They were misleading — a file watcher's "startup time" is dominated by how
long it takes to register with the OS, not how long `--help` takes to exit.

The current bench is honest about what it's measuring, runs end-to-end against
real installed binaries, and embeds no canned numbers. If you find a number in
this repo that contradicts what `--bench` prints on your machine, please open
an issue.
