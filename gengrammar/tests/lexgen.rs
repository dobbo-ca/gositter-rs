use gengrammar::lexgen::{LexForm, emit_lex};
use gengrammar::parse::parse_grammar;

const MINI: &str = include_str!("../../testdata/gengrammar/mini.c");

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Asserts that the needles occur in `text` in this order (whitespace-insensitive).
fn in_order(text: &str, needles: &[&str]) {
    let text = norm(text);
    let mut from = 0;
    for n in needles {
        let n = norm(n);
        match text[from..].find(&n) {
            Some(i) => from += i + n.len(),
            None => panic!("{n:?} not found (in order) after byte {from} of:\n{text}"),
        }
    }
}

#[test]
fn code_form_of_mini() {
    let g = parse_grammar(MINI).unwrap();
    let (rs, bin) = emit_lex(&g, LexForm::Code).unwrap();
    assert!(bin.is_empty());
    assert!(!rs.contains("lex_keywords"));
    in_order(
        &rs,
        &[
            "use gositter_grammars::lex::{LexOps, set_contains_ascii};",
            "static SET_1: [(i32, i32); 2] = [(97, 122), (170, 170)];",
            "static SET_1_ASCII: [bool; 128] = [",
            "static LEX_RUN_0: [bool; 256] = [",
            "pub fn lex<L: LexOps>(l: &mut L, mut state: u16) -> bool {",
            // state 0: the skip run first, then the statements in C order
            "0 => {",
            "l.advance_run(&LEX_RUN_0, true);",
            "if eof { l.advance(false); state = 1; continue; }",
            "match lookahead {
                34 => { l.advance(false); state = 2; continue; }
                92 => { l.advance(false); state = 3; continue; }
                160 => { l.advance(false); state = 4; continue; }
                _ => {}
            }",
            "if (9 <= lookahead && lookahead <= 13) || lookahead == 32 {
                l.advance(true); state = 0; continue; }",
            "if !eof && set_contains_ascii(&SET_1_ASCII, &SET_1, lookahead) {
                l.advance(false); state = 5; continue; }",
            "if lookahead != 0 && (lookahead < 40 || 41 < lookahead) {
                l.advance(false); state = 6; continue; }",
            "return result;",
            // state 1: result, symbol, mark_end, in that order
            "1 => { result = true; l.set_result_symbol(0); // ts_builtin_sym_end
                l.mark_end(); return result; }",
            "_ => return false,",
        ],
    );
}

// The test's own reader of lex.bin, written from the format in lexgen.rs.
#[derive(Debug, PartialEq)]
enum Op {
    End,
    Accept(u16),
    If {
        skip: bool,
        target: u16,
        ranges: Vec<(i32, i32)>,
        at_eof: Option<Vec<(i32, i32)>>,
    },
}

#[derive(Debug, PartialEq)]
struct Rec {
    state: u16,
    run: Option<(u16, bool)>,
    ops: Vec<Op>,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn ranges_at(b: &[u8], o: usize) -> (Vec<(i32, i32)>, usize) {
    let n = u16_at(b, o) as usize;
    let v = (0..n)
        .map(|i| (i32_at(b, o + 2 + 8 * i), i32_at(b, o + 6 + 8 * i)))
        .collect();
    (v, o + 2 + 8 * n)
}

/// The records of the region of `b` that starts at `base`, in directory (state) order.
fn region(b: &[u8], base: usize) -> Vec<Rec> {
    let r = &b[base..];
    let n = u16_at(r, 0) as usize;
    (0..n)
        .map(|i| {
            let d = 2 + 6 * i;
            let state = u16_at(r, d);
            let mut p = u32::from_le_bytes([r[d + 2], r[d + 3], r[d + 4], r[d + 5]]) as usize;
            let run = u16_at(r, p);
            let run = (run != 0xFFFF).then(|| (run, r[p + 2] != 0));
            p += 3;
            let mut ops = Vec::new();
            loop {
                let tag = r[p];
                p += 1;
                match tag {
                    0 => {
                        ops.push(Op::End);
                        break;
                    }
                    1 => {
                        ops.push(Op::Accept(u16_at(r, p)));
                        p += 2;
                    }
                    2 => {
                        let flags = r[p];
                        let target = u16_at(r, p + 1);
                        let (ranges, q) = ranges_at(r, p + 3);
                        p = q;
                        let mut at_eof = None;
                        if flags & 2 != 0 {
                            let (v, q) = ranges_at(r, p);
                            at_eof = Some(v);
                            p = q;
                        }
                        ops.push(Op::If {
                            skip: flags & 1 != 0,
                            target,
                            ranges,
                            at_eof,
                        });
                    }
                    t => panic!("bad op tag {t}"),
                }
            }
            Rec { state, run, ops }
        })
        .collect()
}

fn jump(skip: bool, target: u16, ranges: &[(i32, i32)]) -> Op {
    Op::If {
        skip,
        target,
        ranges: ranges.to_vec(),
        at_eof: None,
    }
}

#[test]
fn table_form_of_mini() {
    const MAX: i32 = 0x10FFFF;
    let g = parse_grammar(MINI).unwrap();
    let (rs, bin) = emit_lex(&g, LexForm::Table).unwrap();
    in_order(
        &rs,
        &[
            "use gositter_grammars::lex::{LexOps, run_table};",
            "static DATA: &[u8] = include_bytes!(\"lex.bin\");",
            "static LEX_RUN_0: [bool; 256] = [",
            "static LEX_RUNS: [&[bool; 256]; 1] = [&LEX_RUN_0];",
            "pub fn lex<L: LexOps>(l: &mut L, state: u16) -> bool {
                run_table(DATA, &LEX_RUNS, l, state) }",
        ],
    );
    assert!(!rs.contains("lex_keywords"));
    assert!(!rs.contains("SET_1"), "the table form has no sets: {rs}");
    let recs = region(&bin, 0);
    assert_eq!(
        recs,
        [
            Rec {
                state: 0,
                // the skip run on LEX_RUN_0
                run: Some((0, true)),
                ops: vec![
                    // `if (eof)`: no lookahead matches when not at end, every one at end
                    Op::If {
                        skip: false,
                        target: 1,
                        ranges: vec![],
                        at_eof: Some(vec![(-1, MAX)]),
                    },
                    jump(false, 2, &[(34, 34)]),
                    jump(false, 3, &[(92, 92)]),
                    jump(false, 4, &[(0xa0, 0xa0)]),
                    jump(true, 0, &[(9, 13), (32, 32)]),
                    // `!eof && set_contains(..)`: never at end of input
                    Op::If {
                        skip: false,
                        target: 5,
                        ranges: vec![(97, 122), (0xaa, 0xaa)],
                        at_eof: Some(vec![]),
                    },
                    jump(false, 6, &[(-1, -1), (1, 39), (42, MAX)]),
                    Op::End,
                ],
            },
            Rec {
                state: 1,
                run: None,
                ops: vec![Op::Accept(0), Op::End],
            },
        ]
    );
}
