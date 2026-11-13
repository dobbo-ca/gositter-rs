//! The Rust side of the parity check (plan Task 12): the commands of `parity/go/gsdump`
//! (without `gen`), with the same output bytes. CI diffs the two.
//!
//! ```text
//! gsdump list                               grammar dirs this build has, one per line
//! gsdump dump  <dir> <in> <out>             <out>/<file>.dump for each file in <in>
//! gsdump log   <dir> <in> <out> <max-bytes> <out>/<file>.log: "L "/"P " + LOG message
//! gsdump stall <dir> <in> <out> <max-bytes> <out>/<file>.stall: smallest max_stall, or none
//! gsdump kinds <dir>                        Go strconv.Quote of each symbol name
//! gsdump bench <dir> <in> <reps>            <file>\t<bytes>\t<best ns of reps parses>
//! ```
//!
//! `<in>` is read flat: every regular file whose name does not end in `.dump` or `.dump.gz`,
//! in name order, on one parser per command.

#[path = "../tests/common/mod.rs"]
mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use gositter::{Language, LogType, Parser};

fn die(msg: &str) -> ! {
    eprintln!("gsdump: {msg}");
    std::process::exit(1)
}

fn language(dir: &str) -> &'static Language {
    common::language(dir).unwrap_or_else(|| die(&format!("unknown dir {dir:?}")))
}

fn parser(l: &'static Language) -> Parser {
    let mut p = Parser::new();
    p.set_language(l).unwrap_or_else(|e| die(&format!("{e:?}")));
    p
}

/// The names of the input files in `dir`, sorted by bytes as Go `os.ReadDir`.
fn inputs(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| die(&format!("{}: {e}", dir.display())))
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| e.file_name().into_string().unwrap())
        .filter(|n| !n.ends_with(".dump") && !n.ends_with(".dump.gz"))
        .collect();
    names.sort();
    names
}

fn write(path: &Path, data: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, data).unwrap_or_else(|e| die(&format!("{}: {e}", path.display())));
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| die(&format!("{}: {e}", path.display())))
}

fn dump(dir: &str, input: &Path, out: &Path) {
    let l = language(dir);
    // verilog has no fields and an empty name table.
    let fields: &[&str] = l.tables.field_names.get(1..).unwrap_or_default();
    let mut p = parser(l);
    for name in inputs(input) {
        let src = read(&input.join(&name));
        let mut b = Vec::new();
        let tree = p.parse(&src).unwrap();
        gositter_treedump::dump(&mut b, tree.root_node(), fields, &src);
        write(&out.join(format!("{name}.dump")), &b);
    }
}

fn log(dir: &str, input: &Path, out: &Path, limit: usize) {
    let mut p = parser(language(dir));
    let buf = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buf);
    p.set_logger(Some(Box::new(move |t, msg| {
        let mut b = sink.lock().unwrap();
        b.extend_from_slice(if t == LogType::Lex { b"L " } else { b"P " });
        b.extend_from_slice(msg);
        b.push(b'\n');
    })));
    for name in inputs(input) {
        let src = read(&input.join(&name));
        if src.len() > limit {
            continue;
        }
        buf.lock().unwrap().clear();
        p.parse(&src);
        write(&out.join(format!("{name}.log")), &buf.lock().unwrap());
    }
}

/// The smallest `max_stall` in `[1, 1 << 20]` that lets `parse_budget` finish, found by
/// bisection (as Go `stallOf`), or `none`.
fn stall(dir: &str, input: &Path, out: &Path, limit: usize) {
    let mut p = parser(language(dir));
    for name in inputs(input) {
        let src = read(&input.join(&name));
        if src.len() > limit {
            continue;
        }
        let (mut lo, mut hi) = (1u32, 1u32 << 20);
        let res = if p.parse_budget(&src, hi).is_some() {
            while lo < hi {
                let mid = (lo + hi) / 2;
                if p.parse_budget(&src, mid).is_some() {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            lo.to_string()
        } else {
            "none".to_string()
        };
        write(
            &out.join(format!("{name}.stall")),
            format!("{res}\n").as_bytes(),
        );
    }
}

fn kinds(dir: &str) {
    let mut out = String::new();
    for name in language(dir).tables.symbol_names {
        out.push_str(&gositter_treedump::go_quote(name));
        out.push('\n');
    }
    print!("{out}");
}

/// Parses each input `reps` times on one parser and prints the fastest parse (plan Task 14).
fn bench(dir: &str, input: &Path, reps: usize) {
    let mut p = parser(language(dir));
    let mut out = String::new();
    for name in inputs(input) {
        let src = read(&input.join(&name));
        let mut best = u128::MAX;
        for _ in 0..reps {
            let t = std::time::Instant::now();
            p.parse(&src);
            best = best.min(t.elapsed().as_nanos());
        }
        out.push_str(&format!("{name}\t{}\t{best}\n", src.len()));
    }
    print!("{out}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let limit = |s: &str| {
        s.parse()
            .unwrap_or_else(|_| die(&format!("bad max-bytes {s:?}")))
    };
    // treedump recurses once per tree level, as Go grows its stacks.
    common::run_on_big_stack(
        move || match *args.iter().map(String::as_str).collect::<Vec<_>>() {
            ["list"] => {
                for d in common::DIRS
                    .iter()
                    .filter(|d| common::language(d).is_some())
                {
                    println!("{d}");
                }
            }
            ["dump", d, i, o] => dump(d, Path::new(i), Path::new(o)),
            ["log", d, i, o, m] => log(d, Path::new(i), Path::new(o), limit(m)),
            ["stall", d, i, o, m] => stall(d, Path::new(i), Path::new(o), limit(m)),
            ["kinds", d] => kinds(d),
            ["bench", d, i, r] => bench(d, Path::new(i), limit(r)),
            _ => die(
                "usage: gsdump list | dump <dir> <in> <out> | log|stall <dir> <in> <out> <max-bytes> | kinds <dir> | bench <dir> <in> <reps>",
            ),
        },
    );
}
