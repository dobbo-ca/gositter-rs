//! `grammars.lock`: one grammar per line, `<package> <module> <version> <dir of parser.c>
//! <sha256 of parser.c>` (gositter `cmd/gengrammar/main.go:7-11`). `#` lines and blank lines are
//! skipped; the sum may be missing (it is filled by `--update`).

use std::path::{Path, PathBuf};

use crate::sha256::{hex, sha256};

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
        src.join(format!("{}@{}", self.module, self.version))
            .join(&self.dir)
            .join("parser.c")
    }

    /// The lock line for this entry, with `sum` in the last column.
    pub fn line(&self, sum: &str) -> String {
        [
            self.pkg.as_str(),
            &self.module,
            &self.version,
            &self.dir,
            sum,
        ]
        .join(" ")
    }
}

/// Parses the lock. Each entry comes with its 0-based line index. Errors are
/// `<path>:<line>: want 4 or 5 fields`.
pub fn parse_lock(path: &str, text: &str) -> Result<Vec<(usize, LockEntry)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.is_empty() || f[0].starts_with('#') {
            continue;
        }
        if f.len() < 4 || f.len() > 5 {
            return Err(format!("{path}:{}: want 4 or 5 fields", i + 1));
        }
        out.push((
            i,
            LockEntry {
                pkg: f[0].into(),
                module: f[1].into(),
                version: f[2].into(),
                dir: f[3].into(),
                sum: f.get(4).copied().unwrap_or("").into(),
            },
        ));
    }
    Ok(out)
}

/// Checks `src` (a parser.c) against the lock sum and returns its hex sha256. A mismatch is an
/// error unless `update` is set.
pub fn check_sum(src: &[u8], want: &str, update: bool) -> Result<String, String> {
    let sum = hex(&sha256(src));
    if sum != want && !update {
        return Err(format!(
            "parser.c sha256 is {sum}, the lock says {want:?} (use -update)"
        ));
    }
    Ok(sum)
}
