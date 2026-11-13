//! The generated lex functions against gositter's Go ones: every case of the lex oracle
//! (`gengrammar/oracle/lex`, written to `$GRAMMAR_ORACLE/<g>.lex` and `<g>.lextext`) must give the
//! same trace digest on the mock lexer. Without GRAMMAR_ORACLE it does nothing. The grammars/<g>/lex.rs
//! files are included as they are, whichever form gengrammar wrote.

#[path = "common/lexmock.rs"]
mod lexmock;
#[rustfmt::skip]
#[path = "../bash/lex.rs"]
mod bash;
#[rustfmt::skip]
#[path = "../cpp/lex.rs"]
mod cpp;
#[rustfmt::skip]
#[path = "../json/lex.rs"]
mod json;
#[rustfmt::skip]
#[path = "../verilog/lex.rs"]
mod verilog;

use lexmock::MockLexer;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::rc::Rc;

type LexFn = fn(&mut MockLexer, u16) -> bool;

const OFFSET: u64 = 14695981039346656037;
const PRIME: u64 = 1099511628211;

fn oracle_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("GRAMMAR_ORACLE").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("GRAMMAR_ORACLE is not set: lex oracle skipped");
    }
    dir
}

fn cps(s: &str) -> Vec<i32> {
    if s == "-" || s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|v| v.parse().expect("code point"))
        .collect()
}

fn digest(s: &str) -> u64 {
    u64::from_str_radix(s, 16).expect("hex digest")
}

fn check(g: &str, fns: &[(&str, LexFn)]) {
    let Some(dir) = oracle_dir() else { return };
    let fns: HashMap<&str, LexFn> = fns.iter().copied().collect();
    let lookup = |name: &str| {
        *fns.get(name)
            .unwrap_or_else(|| panic!("{g}: no function {name}"))
    };

    // step cases, folded: points, empty, tail and fold lines
    let path = dir.join(format!("{g}.lex"));
    let mut points: HashMap<String, Vec<i32>> = HashMap::new();
    let mut tails: HashMap<(String, u16, usize), Vec<i32>> = HashMap::new();
    let (mut npoints, mut nempty, mut nfold) = (0usize, 0usize, 0usize);
    for line in BufReader::new(File::open(&path).expect("open .lex")).lines() {
        let line = line.expect("read .lex");
        let mut f = line.split(' ');
        let kind = f.next().expect("kind");
        let name = f.next().expect("fn").to_string();
        match kind {
            "points" => {
                let pts = cps(f.next().expect("points"));
                assert!(!pts.is_empty(), "{g}: empty points line for {name}");
                points.insert(name, pts);
                npoints += 1;
            }
            "empty" => {
                let state = f.next().expect("state").parse().expect("state");
                let want = digest(f.next().expect("digest"));
                let got = MockLexer::run(lookup(&name), state, &[]);
                assert_eq!(
                    got, want,
                    "{g}: {name} state {state} empty input: Go digest {want:016x}, Rust {got:016x}"
                );
                nempty += 1;
            }
            "tail" => {
                let state = f.next().expect("state").parse().expect("state");
                let i = f.next().expect("i").parse().expect("i");
                tails.insert((name, state, i), cps(f.next().expect("tail")));
            }
            "fold" => {
                let state = f.next().expect("state").parse().expect("state");
                let i = f.next().expect("i").parse().expect("i");
                let want = digest(f.next().expect("digest"));
                let tail = tails
                    .get(&(name.clone(), state, i))
                    .unwrap_or_else(|| panic!("{g}: fold {name} {state} {i} has no tail line"));
                let func = lookup(&name);
                let mut input = vec![0];
                input.extend_from_slice(tail);
                let mut h = OFFSET;
                for &p in &points[&name] {
                    input[0] = p;
                    h = (h ^ MockLexer::run(func, state, &input)).wrapping_mul(PRIME);
                }
                assert_eq!(
                    h, want,
                    "{g}: {name} state {state} tail {i} {tail:?}: Go fold {want:016x}, Rust {h:016x}"
                );
                nfold += 1;
            }
            _ => panic!("{g}: unknown .lex line kind {kind}"),
        }
    }
    assert!(
        npoints == fns.len() && nempty >= fns.len() && nfold >= 6 * nempty,
        "{g}: only {npoints} points, {nempty} empty and {nfold} fold lines"
    );

    // text cases: the digests of every suffix of the corpus, folded
    let text = fs::read_to_string(dir.join(format!("{g}.lextext"))).expect("read .lextext");
    let mut lines = text.lines();
    let first = lines.next().expect("text line");
    let corpus: Rc<[i32]> = Rc::from(cps(first.strip_prefix("text ").expect("text prefix")));
    for line in lines {
        let mut f = line.split(' ');
        let (name, state, want) = (
            f.next().expect("fn"),
            f.next().expect("state"),
            f.next().expect("digest"),
        );
        let (state, want) = (state.parse().expect("state"), digest(want));
        let func = lookup(name);
        let mut h = OFFSET;
        for p in 0..=corpus.len() {
            h = (h ^ MockLexer::run_at(func, state, &corpus, p)).wrapping_mul(PRIME);
        }
        assert_eq!(
            h, want,
            "{g}: {name} state {state} text: Go fold {want:016x}, Rust {h:016x}"
        );
    }
}

#[test]
fn bash() {
    check(
        "bash",
        &[
            ("lex", bash::lex::<MockLexer> as LexFn),
            ("kw", bash::lex_keywords::<MockLexer> as LexFn),
        ],
    );
}

#[test]
fn cpp() {
    check(
        "cpp",
        &[
            ("lex", cpp::lex::<MockLexer> as LexFn),
            ("kw", cpp::lex_keywords::<MockLexer> as LexFn),
        ],
    );
}

#[test]
fn json() {
    check("json", &[("lex", json::lex::<MockLexer> as LexFn)]);
}

#[test]
fn verilog() {
    check(
        "verilog",
        &[
            ("lex", verilog::lex::<MockLexer> as LexFn),
            ("kw", verilog::lex_keywords::<MockLexer> as LexFn),
        ],
    );
}
