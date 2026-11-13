//! The regress corpus: 1,423 cases with the C runtime's dump of each (gositter
//! `regress_test.go`). The replay against the goldens comes with the parser (plan Tasks 5,
//! 11 and 15); this file checks the corpus that CI unzips.

mod common;

/// Cases per directory, from gositter v0.2.0 `testdata/regress`.
const CASES: [(&str, usize); 21] = [
    ("bash", 65),
    ("c", 50),
    ("cpp", 85),
    ("c-sharp", 73),
    ("go", 51),
    ("hcl", 71),
    ("java", 53),
    ("javascript", 85),
    ("json", 50),
    ("julia", 68),
    ("kotlin", 79),
    ("lua", 80),
    ("php", 66),
    ("python", 63),
    ("ruby", 70),
    ("rust", 85),
    ("scala", 78),
    ("tsx", 78),
    ("typescript", 65),
    ("verilog", 61),
    ("zig", 47),
];

#[test]
fn corpus_is_complete_and_unzipped() {
    let mut total = 0;
    for (dir, want) in CASES {
        assert!(common::DIRS.contains(&dir), "{dir}");
        let goldens = common::goldens(dir);
        assert_eq!(goldens.len(), want, "{dir}: unzipped goldens");
        for golden in &goldens {
            let source = golden.with_extension("");
            assert!(source.is_file(), "{}: no source", golden.display());
            let text = std::fs::read(golden).unwrap();
            assert!(
                text.starts_with(b"0 \""),
                "{}: not a dump",
                golden.display()
            );
            assert!(
                text.ends_with(b"\n"),
                "{}: no final newline",
                golden.display()
            );
        }
        total += goldens.len();
    }
    assert_eq!(total, 1423);
    assert_eq!(CASES.len(), common::DIRS.len());
}

#[test]
fn goldens_are_sorted_by_path() {
    let g = common::goldens("json");
    let mut sorted = g.clone();
    sorted.sort();
    assert_eq!(g, sorted);
}
