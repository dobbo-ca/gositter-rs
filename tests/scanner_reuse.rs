//! gositter `scanner_reuse_test.go`: the kept scanner serves the next parse of its language
//! only.

mod common;

use gositter::{Language, Parser};

fn language(dir: &str) -> &'static Language {
    common::language(dir).unwrap_or_else(|| panic!("{dir} grammar"))
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn scanner_reuse_across_languages() {
    let (python, bash, go) = (language("python"), language("bash"), language("go"));
    let py: &[u8] = b"if a:\n    x = f\"a{b}\"\n";
    let sh: &[u8] = b"cat <<EOF\nbody\nEOF\n";
    let fresh = |l: &'static Language, src: &[u8]| {
        let mut p = Parser::new();
        p.set_language(l).unwrap();
        p.parse(src).unwrap().root_node().to_sexp()
    };
    let mut errs = Vec::new();
    let mut p = Parser::new();
    p.set_language(python).unwrap();
    p.parse(py);
    let first = p.kept_scanner_id();
    assert!(first.is_some(), "no scanner kept");
    p.parse(py);
    if p.kept_scanner_id() != first {
        errs.push("scanner not reused for the same language");
    }
    p.set_language(bash).unwrap();
    let got = p.parse(sh).unwrap().root_node().to_sexp();
    if got != fresh(bash, sh) {
        errs.push("bash tree differs after python");
    }
    if p.kept_scanner_id() == first {
        errs.push("python scanner handed to bash");
    }
    p.set_language(go).unwrap(); // no scanner of its own
    p.parse(b"package p\n");
    p.set_language(python).unwrap();
    let got = p.parse(py).unwrap().root_node().to_sexp();
    if got != fresh(python, py) {
        errs.push("python tree differs after bash and go");
    }
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn set_language_drops_kept_scanner() {
    let mut p = Parser::new();
    p.set_language(language("bash")).unwrap();
    p.parse(b"cat <<A\nx\nA\n");
    p.set_language(language("go")).unwrap();
    assert!(
        p.kept_scanner_id().is_none(),
        "SetLanguage kept the bash scanner"
    );
}
