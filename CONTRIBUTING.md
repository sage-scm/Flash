# Contributing to Flash

Thanks for taking the time to look at the project. Whether you're filing a bug,
suggesting a feature, or sending a pull request, contributions are welcome and
appreciated.

## Getting started

```sh
git clone https://github.com/sage-scm/Flash.git
cd Flash
cargo build
cargo test
```

Flash targets the latest stable Rust toolchain. The MSRV is recorded in
`Cargo.toml` (`rust-version`).

## Before you push

Run the same checks CI does:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

The integration tests under `tests/` drive the compiled binary against a real
filesystem, so they're the best line of defense against issues like #1
(silently watching the wrong directory). If you change behaviour, please add
or adjust a test that fails without your change.

If you touched anything performance-relevant, run the bench to sanity-check:

```sh
cargo run --release -- --bench
```

It compares Flash against any other watchers (watchexec, nodemon,
cargo-watch, entr) installed on your `PATH` and prints honest, reproducible
numbers. Please don't commit hard-coded performance values to the repo.

## Filing issues

When you report a bug, the most helpful thing you can include is the exact
command you ran and the output you saw. If it's a watching bug, telling us
your OS and which filesystem the affected files live on (APFS, ext4, NTFS,
network mount, …) usually saves a round-trip.

## Pull requests

Small, focused PRs are the easiest to land:

- One topic per PR.
- A short description of what the change accomplishes and *why*.
- Tests for new behaviour or regression coverage for fixes.
- A `CHANGELOG.md` entry under the unreleased section.

## Code style

Idiomatic, formatted Rust with `cargo fmt`. Avoid comments that restate what
the code already says — prefer naming things well. Favour `Result` over panics
in library code; the binary entry point in `src/main.rs` is the one place
errors turn into exit codes.

## License

By contributing, you agree your changes are licensed under the project's MIT
license.
