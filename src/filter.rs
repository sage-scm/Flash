use std::path::Path;

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};

/// Decides whether a path should trigger a command run.
///
/// Evaluation order — short-circuit at the first failure:
///   1. Ignore patterns: anything matching is dropped immediately.
///   2. Extension filter: when set, the path's extension must match.
///   3. Include patterns: when set, at least one must match.
pub struct Filter {
    extensions: Vec<String>,
    include: GlobSet,
    ignore: GlobSet,
    include_set: bool,
}

impl Filter {
    pub fn new(extensions: &[String], include: &[String], ignore: &[String]) -> Result<Self> {
        Ok(Self {
            extensions: extensions.to_vec(),
            include: build_set(include).context("compiling include patterns")?,
            ignore: build_set(ignore).context("compiling ignore patterns")?,
            include_set: !include.is_empty(),
        })
    }

    pub fn accepts(&self, path: &Path) -> bool {
        if self.ignore.is_match(path) {
            return false;
        }

        if !self.extensions.is_empty() {
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                return false;
            };
            if !self.extensions.iter().any(|e| e == ext) {
                return false;
            }
        }

        if self.include_set && !self.include.is_match(path) {
            return false;
        }

        true
    }
}

fn build_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).with_context(|| format!("invalid glob '{pattern}'"))?;
        builder.add(glob);
    }
    builder.build().context("building glob set")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn filter(ext: &[&str], inc: &[&str], ign: &[&str]) -> Filter {
        let extensions: Vec<String> = ext.iter().map(|s| s.to_string()).collect();
        let include: Vec<String> = inc.iter().map(|s| s.to_string()).collect();
        let ignore: Vec<String> = ign.iter().map(|s| s.to_string()).collect();
        Filter::new(&extensions, &include, &ignore).expect("valid filter")
    }

    #[test]
    fn extension_match_passes() {
        let f = filter(&["rs"], &[], &[]);
        assert!(f.accepts(&PathBuf::from("src/lib.rs")));
        assert!(!f.accepts(&PathBuf::from("src/lib.toml")));
    }

    #[test]
    fn extension_filter_rejects_files_with_no_extension() {
        let f = filter(&["rs"], &[], &[]);
        assert!(!f.accepts(&PathBuf::from("Makefile")));
    }

    #[test]
    fn ignore_short_circuits_include() {
        let f = filter(&[], &["src/**"], &["**/node_modules/**"]);
        assert!(!f.accepts(&PathBuf::from("src/node_modules/foo.js")));
        assert!(f.accepts(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn include_brace_expansion() {
        let f = filter(&[], &["src/**/*.{js,ts}"], &[]);
        assert!(f.accepts(&PathBuf::from("src/util/app.js")));
        assert!(f.accepts(&PathBuf::from("src/util/app.ts")));
        assert!(!f.accepts(&PathBuf::from("src/util/app.py")));
    }

    #[test]
    fn no_filters_accepts_everything() {
        let f = filter(&[], &[], &[]);
        assert!(f.accepts(&PathBuf::from("anywhere/file.bin")));
    }

    #[test]
    fn invalid_glob_surfaces_error() {
        let bad = Filter::new(&[], &["[unterminated".to_string()], &[]);
        assert!(bad.is_err());
    }
}
