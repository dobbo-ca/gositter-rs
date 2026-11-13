//! `grammars.lock`: one grammar per line, `<package> <module> <version> <dir of parser.c>
//! <sha256 of parser.c>` (gositter `cmd/gengrammar/main.go:7-11`). `#` lines and blank lines are
//! skipped; the sum may be missing (it is filled by `--update`).

use std::path::{Path, PathBuf};

/// One line of the lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockEntry {
    pub pkg: String,
    pub module: String,
    pub version: String,
    pub dir: String,
    pub sum: String,
}

impl LockEntry {
    /// `<src>/<module>@<version>/<dir>/parser.c`, the Go module cache layout.
    pub fn parser_c(&self, src: &Path) -> PathBuf {
        let _ = src;
        todo!()
    }

    /// The lock line for this entry, with `sum` in the last column.
    pub fn line(&self, sum: &str) -> String {
        let _ = sum;
        todo!()
    }
}

/// Parses the lock. Each entry comes with its 0-based line index. Errors are
/// `<path>:<line>: want 4 or 5 fields`.
pub fn parse_lock(path: &str, text: &str) -> Result<Vec<(usize, LockEntry)>, String> {
    let _ = (path, text);
    todo!()
}

/// Checks `src` (a parser.c) against the lock sum and returns its hex sha256. A mismatch is an
/// error unless `update` is set.
pub fn check_sum(src: &[u8], want: &str, update: bool) -> Result<String, String> {
    let _ = (src, want, update);
    todo!()
}
