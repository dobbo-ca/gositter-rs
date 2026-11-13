//! Shared by the integration tests and `examples/gsdump.rs`: the grammar registry, the
//! regress corpus and a big-stack runner. Not every test uses every item.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use gositter::Language;

/// The directories under `testdata/regress`, as gositter `regress_test.go:39-61`.
pub const DIRS: [&str; 21] = [
    "bash",
    "c",
    "cpp",
    "c-sharp",
    "go",
    "hcl",
    "java",
    "javascript",
    "json",
    "julia",
    "kotlin",
    "lua",
    "php",
    "python",
    "ruby",
    "rust",
    "scala",
    "tsx",
    "typescript",
    "verilog",
    "zig",
];

/// The language of a regress directory. `None` until the grammar is on main (plan Tasks 11
/// and 15).
pub fn language(_dir: &str) -> Option<&'static Language> {
    None
}

/// `testdata/regress/<dir>`.
pub fn regress_dir(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/regress")
        .join(dir)
}

/// The unzipped goldens (`<case>.dump`) of a directory, sorted by path as Go's
/// `filepath.Glob` returns them. CI unzips the committed `.dump.gz` files first.
pub fn goldens(dir: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(regress_dir(dir))
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "dump"))
        .collect();
    out.sort();
    out
}

/// Runs `f` on a thread with a 1 GiB stack (reserved, not committed): Go grows its stacks,
/// and treedump recurses once per tree level. A panic in `f` is raised again here.
pub fn run_on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let handle = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .expect("spawn");
    match handle.join() {
        Ok(v) => v,
        Err(e) => std::panic::resume_unwind(e),
    }
}
