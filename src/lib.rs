//! Flash — a fast, predictable file watcher.
//!
//! The library is intentionally small. The CLI is a thin shell around
//! [`run`], which loads the merged [`Settings`], compiles the path
//! [`Filter`], wires up a debounced [`notify`] watcher, and dispatches
//! changes to a [`Runner`].

mod bench;
mod cli;
mod config;
mod filter;
mod runner;
mod stats;
mod watcher;

pub use cli::Cli;
pub use config::{Config, Settings};
pub use filter::Filter;
pub use runner::Runner;
pub use stats::Stats;
pub use watcher::run;
