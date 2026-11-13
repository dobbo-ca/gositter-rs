//! Lex condition AST. Go keeps a condition as Go source and re-parses it with `go/parser`; the
//! Rust port builds this tree in `parse.rs` from the same accepted grammar. Evaluation and range
//! lists come with the lexer emitter.

use std::collections::HashMap;

use crate::parse::{Grammar, LexCase};

/// A comparison operator of a lex condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// One side of a comparison: `lookahead` or a literal (char literals are their value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    Lookahead,
    Lit(i64),
}

/// A lex condition: `eof`, `!`, `&&`, `||`, a comparison, or `set_contains(set, n, lookahead)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cond {
    Eof,
    Not(Box<Cond>),
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
    Cmp(CmpOp, Operand, Operand),
    SetContains { set: String, n: usize },
}

/// The `TSCharacterRange` tables of a grammar by name, as `(start, end)` pairs.
pub fn char_sets(g: &Grammar) -> HashMap<String, Vec<(i32, i32)>> {
    let _ = g;
    todo!()
}

/// `run.go` `cond` for one lookahead; `eof` is the `eof` identifier.
pub fn eval(c: &Cond, sets: &HashMap<String, Vec<(i32, i32)>>, lookahead: i64, eof: bool) -> bool {
    let _ = (c, sets, lookahead, eof);
    todo!()
}

/// The lookaheads in -1..=0x10FFFF that satisfy `c` when not at end of input, as sorted ranges.
pub fn ranges(c: &Cond, sets: &HashMap<String, Vec<(i32, i32)>>) -> Vec<(i32, i32)> {
    let _ = (c, sets);
    todo!()
}

/// C `set_contains`: the binary search of `emit.go` `setContains`.
pub fn set_contains(ranges: &[(i32, i32)], lookahead: i64) -> bool {
    let _ = (ranges, lookahead);
    todo!()
}

/// Port of `run.go` `runSets`: the (advance, skip) self-loop sets over characters 1..127.
pub fn run_sets(
    case: &LexCase,
    sets: &HashMap<String, Vec<(i32, i32)>>,
) -> ([bool; 128], [bool; 128]) {
    let _ = (case, sets);
    todo!()
}
