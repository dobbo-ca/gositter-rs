//! gengrammar --lock <file> --src <dir> --out <dir> [--update] [-j N] [name ...]
//! gengrammar --lock <file> --src <dir> --oracle <dir> [name ...]
//!
//! The second form compares the tables with the dump of gositter's own Go tables (`oracle.rs`).
//!
//! `--src` holds the grammar modules in the Go module cache layout
//! (`<module>@<version>/<dir>/parser.c`), as the `gen-src` stage of ci/Dockerfile unpacks them.
//! Flow as gositter `cmd/gengrammar/main.go:36-115`.

use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use gengrammar::blob::{check_language, write_blob};
use gengrammar::lock::{LockEntry, check_sum, parse_lock};
use gengrammar::oracle::check_grammar;
use gengrammar::parse::parse_grammar;
use gengrammar::rust::{header, mod_rs};

struct Args {
    lock: String,
    src: PathBuf,
    out: PathBuf,
    oracle: Option<PathBuf>,
    update: bool,
    jobs: usize,
    names: Vec<String>,
}

fn die(msg: &str) -> ! {
    eprintln!("gengrammar: {msg}");
    exit(1)
}

fn args() -> Args {
    let (mut lock, mut src, mut out) = (
        "grammars/grammars.lock".to_string(),
        None,
        PathBuf::from("grammars"),
    );
    let (mut update, mut jobs, mut names, mut oracle) = (false, 4, Vec::new(), None);
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = |name: &str| {
            it.next()
                .unwrap_or_else(|| die(&format!("{name} needs a value")))
        };
        match a.as_str() {
            "--lock" => lock = val("--lock"),
            "--src" => src = Some(PathBuf::from(val("--src"))),
            "--out" => out = PathBuf::from(val("--out")),
            "--oracle" => oracle = Some(PathBuf::from(val("--oracle"))),
            "--update" => update = true,
            "-j" => {
                jobs = val("-j")
                    .parse()
                    .unwrap_or_else(|_| die("-j wants a number"))
            }
            _ if a.starts_with('-') => die(&format!("unknown flag {a}")),
            _ => names.push(a),
        }
    }
    let src = src.unwrap_or_else(|| die("--src is required"));
    Args {
        lock,
        src,
        out,
        oracle,
        update,
        jobs: jobs.max(1),
        names,
    }
}

/// Generates one grammar and returns the sha256 of its parser.c.
fn generate(l: &LockEntry, src: &Path, out: &Path, update: bool) -> Result<String, String> {
    let path = l.parser_c(src);
    let text = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sum = check_sum(&text, &l.sum, update)?;
    let src = std::str::from_utf8(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut g = parse_grammar(src).map_err(|e| e.0)?;
    let info = check_language(&mut g).map_err(|e| e.0)?;
    let blob = write_blob(&g, &info);
    let rs = mod_rs(&header(l, &sum), &g, &info, &blob);
    // Both outputs exist in memory before the first write, so a failure leaves the old files.
    let dir = out.join(&l.pkg);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (name, bytes) in [("tables.bin", &blob.bytes[..]), ("mod.rs", rs.as_bytes())] {
        let p = dir.join(name);
        std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(sum)
}

fn main() {
    let a = args();
    let text =
        std::fs::read_to_string(&a.lock).unwrap_or_else(|e| die(&format!("{}: {e}", a.lock)));
    let lock = parse_lock(&a.lock, &text).unwrap_or_else(|e| die(&e));
    for n in &a.names {
        if !lock.iter().any(|(_, l)| &l.pkg == n) {
            die(&format!("{n} is not in {}", a.lock));
        }
    }
    let todo: Vec<&(usize, LockEntry)> = lock
        .iter()
        .filter(|(_, l)| a.names.is_empty() || a.names.contains(&l.pkg))
        .collect();
    if let Some(o) = &a.oracle {
        let mut failed = false;
        for (_, l) in &todo {
            match check_grammar(l, &a.src, o) {
                Ok(()) => println!("ok {}", l.pkg),
                Err(e) => {
                    eprintln!("gengrammar: {e}");
                    failed = true;
                }
            }
        }
        exit(i32::from(failed));
    }
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    let next = AtomicUsize::new(0);
    let done = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..a.jobs.min(todo.len()) {
            s.spawn(|| {
                while let Some((i, l)) = todo.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let r = generate(l, &a.src, &a.out, a.update);
                    done.lock().unwrap().push((*i, l, r));
                }
            });
        }
    });
    let mut failed = false;
    for (i, l, r) in done.into_inner().unwrap() {
        match r {
            Ok(sum) => lines[i] = l.line(&sum),
            Err(e) => {
                eprintln!("gengrammar: {}: {e}", l.pkg);
                failed = true;
            }
        }
    }
    if failed {
        exit(1);
    }
    if a.update {
        std::fs::write(&a.lock, lines.join("\n"))
            .unwrap_or_else(|e| die(&format!("{}: {e}", a.lock)));
    }
}
