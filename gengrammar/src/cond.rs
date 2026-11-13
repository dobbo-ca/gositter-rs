//! Lex condition AST. Go keeps a condition as Go source and re-parses it with `go/parser`; the
//! Rust port builds this tree in `parse.rs` from the same accepted grammar. Evaluation comes with the
//! lexer emitter.

use std::collections::HashMap;

use crate::parse::{Grammar, LexCase, Stmt, Table};

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
    let pairs = |t: &Table| {
        t.vals
            .chunks(2)
            .map(|p| (p[0] as i32, p[1] as i32))
            .collect()
    };
    g.tables
        .values()
        .filter(|t| t.ctype == "TSCharacterRange")
        .map(|t| (t.name.clone(), pairs(t)))
        .collect()
}

fn value(o: Operand, lookahead: i64) -> i64 {
    match o {
        Operand::Lookahead => lookahead,
        Operand::Lit(v) => v,
    }
}

/// `run.go` `cond` for one lookahead; `eof` is the `eof` identifier.
pub fn eval(c: &Cond, sets: &HashMap<String, Vec<(i32, i32)>>, lookahead: i64, eof: bool) -> bool {
    match c {
        Cond::Eof => eof,
        Cond::Not(x) => !eval(x, sets, lookahead, eof),
        Cond::And(a, b) => eval(a, sets, lookahead, eof) && eval(b, sets, lookahead, eof),
        Cond::Or(a, b) => eval(a, sets, lookahead, eof) || eval(b, sets, lookahead, eof),
        Cond::Cmp(op, a, b) => {
            let (a, b) = (value(*a, lookahead), value(*b, lookahead));
            match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a < b,
                CmpOp::Le => a <= b,
                CmpOp::Gt => a > b,
                CmpOp::Ge => a >= b,
            }
        }
        Cond::SetContains { set, .. } => sets.get(set).is_some_and(|r| set_contains(r, lookahead)),
    }
}

/// C `set_contains`: the binary search of `emit.go` `setContains`.
pub fn set_contains(ranges: &[(i32, i32)], lookahead: i64) -> bool {
    let (mut index, mut size) = (0, ranges.len());
    if size == 0 {
        return false;
    }
    while size > 1 {
        let half = size / 2;
        let mid = index + half;
        let (start, end) = (ranges[mid].0 as i64, ranges[mid].1 as i64);
        if (start..=end).contains(&lookahead) {
            return true;
        } else if lookahead > end {
            index = mid;
        }
        size -= half;
    }
    (ranges[index].0 as i64..=ranges[index].1 as i64).contains(&lookahead)
}

/// Port of `run.go` `runSets`: the (advance, skip) self-loop sets over characters 1..127.
pub fn run_sets(
    case: &LexCase,
    sets: &HashMap<String, Vec<(i32, i32)>>,
) -> ([bool; 128], [bool; 128]) {
    let (mut adv, mut skip) = ([false; 128], [false; 128]);
    for ch in 1..128usize {
        for s in &case.stmts {
            match s {
                Stmt::If {
                    cond,
                    skip: sk,
                    target,
                } => {
                    if eval(cond, sets, ch as i64, false) {
                        if *target == case.state {
                            if *sk { skip[ch] = true } else { adv[ch] = true }
                        }
                        break;
                    }
                }
                Stmt::Map(pairs) => {
                    if let Some(&(_, t)) = pairs.iter().find(|p| p.0 as usize == ch) {
                        adv[ch] = t == case.state;
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    (adv, skip)
}
