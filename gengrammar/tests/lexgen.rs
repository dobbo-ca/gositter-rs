use gengrammar::lexgen::emit_lex;
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
    let rs = emit_lex(&g).unwrap();
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
