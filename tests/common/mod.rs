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

/// The language of a regress directory: `None` until its grammar is in gositter-grammars
/// (plan Task 15 adds each one as it lands).
pub fn language(dir: &str) -> Option<&'static Language> {
    match dir {
        "bash" => Some(gositter_grammars::bash::language()),
        "c" => Some(gositter_grammars::c::language()),
        "c-sharp" => Some(gositter_grammars::csharp::language()),
        "cpp" => Some(gositter_grammars::cpp::language()),
        "go" => Some(gositter_grammars::golang::language()),
        "hcl" => Some(gositter_grammars::hcl::language()),
        "java" => Some(gositter_grammars::java::language()),
        "javascript" => Some(gositter_grammars::javascript::language()),
        "json" => Some(gositter_grammars::json::language()),
        "julia" => Some(gositter_grammars::julia::language()),
        "kotlin" => Some(gositter_grammars::kotlin::language()),
        "lua" => Some(gositter_grammars::lua::language()),
        "php" => Some(gositter_grammars::php::language()),
        "python" => Some(gositter_grammars::python::language()),
        "ruby" => Some(gositter_grammars::ruby::language()),
        "rust" => Some(gositter_grammars::rust::language()),
        "scala" => Some(gositter_grammars::scala::language()),
        "tsx" => Some(gositter_grammars::tsx::language()),
        "typescript" => Some(gositter_grammars::typescript::language()),
        "verilog" => Some(gositter_grammars::verilog::language()),
        "zig" => Some(gositter_grammars::zig::language()),
        _ => None,
    }
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
