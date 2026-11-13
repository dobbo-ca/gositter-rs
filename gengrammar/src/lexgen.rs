//! Lexer emitter: the body of `grammars/<g>/lex.rs`.
//!
//! It follows the Go lexer gositter emits (`emit.go:254-377`, `run.go`): a state does its run set
//! first (the self-loop `advance_run`), then its statements in C order. A jump advances (or skips)
//! and restarts at the top with a fresh lookahead and `eof`; `Accept` sets the result, the symbol
//! and the mark in that order; `End` returns the result.
//!
//! The form is `loop { match state { N => { ... } _ => return false } }` (plan Task 9: the code form
//! beat the table form, `run_table` over a `lex.bin`, which is in git history).

use std::collections::HashMap;

use crate::cond::{CmpOp, Cond, Operand, char_sets, run_sets, set_contains};
use crate::ctok::GenError;
use crate::parse::{Grammar, LexCase, LexFunc, Stmt};

type Sets = HashMap<String, Vec<(i32, i32)>>;

/// The run set of a case: its index and whether it skips (an advance run wins over a skip run).
type RunId = Option<(usize, bool)>;

fn err<T>(msg: String) -> Result<T, GenError> {
    Err(GenError(msg))
}

fn fn_name(f: &LexFunc) -> Result<&'static str, GenError> {
    match f.cname.as_str() {
        "ts_lex" => Ok("lex"),
        "ts_lex_keywords" => Ok("lex_keywords"),
        c => err(format!("unexpected lex function {c}")),
    }
}

/// One table per distinct run set, in order of first use, and each case's run set.
fn run_ids(g: &Grammar, sets: &Sets) -> (Vec<[bool; 128]>, Vec<Vec<RunId>>) {
    let mut runs: Vec<[bool; 128]> = Vec::new();
    let mut ids = Vec::new();
    for f in &g.lex_fns {
        let mut per_fn = Vec::new();
        for case in &f.cases {
            let (adv, skip) = run_sets(case, sets);
            let pick = if adv.contains(&true) {
                Some((adv, false))
            } else if skip.contains(&true) {
                Some((skip, true))
            } else {
                None
            };
            per_fn.push(pick.map(|(set, skip)| {
                let i = runs.iter().position(|r| *r == set).unwrap_or_else(|| {
                    runs.push(set);
                    runs.len() - 1
                });
                (i, skip)
            }));
        }
        ids.push(per_fn);
    }
    (runs, ids)
}

fn bool_array(name: &str, bits: impl Iterator<Item = bool>, len: usize) -> String {
    let mut s = format!("static {name}: [bool; {len}] = [");
    for (i, b) in bits.enumerate() {
        s.push_str(if i % 16 == 0 { "\n   " } else { "" });
        s.push_str(if b { " true," } else { " false," });
    }
    s.push_str("\n];\n");
    s
}

fn run_static(k: usize, set: &[bool; 128]) -> String {
    let bits = (0..256).map(|c| c < 128 && set[c]);
    bool_array(&format!("LEX_RUN_{k}"), bits, 256)
}

fn set_name(name: &str) -> String {
    name.to_uppercase()
}

fn cond_sets(c: &Cond, out: &mut Vec<String>) {
    match c {
        Cond::Eof | Cond::Cmp(..) => {}
        Cond::Not(x) => cond_sets(x, out),
        Cond::And(a, b) | Cond::Or(a, b) => {
            cond_sets(a, out);
            cond_sets(b, out);
        }
        Cond::SetContains { set, .. } => out.push(set.clone()),
    }
}

fn operand(o: Operand) -> String {
    match o {
        Operand::Lookahead => "lookahead".into(),
        Operand::Lit(v) => v.to_string(),
    }
}

/// The condition as a Rust expression. `and_or` is 1 inside an `&&`, 2 inside a `||`, so that a
/// nested operator of the other kind gets parentheses.
fn cond_rs(c: &Cond, and_or: u8) -> String {
    let wrap = |s: String, own: u8| {
        if and_or != 0 && and_or != own {
            format!("({s})")
        } else {
            s
        }
    };
    match c {
        Cond::Eof => "eof".into(),
        Cond::Not(x) => match **x {
            Cond::Eof | Cond::SetContains { .. } | Cond::Not(_) => format!("!{}", cond_rs(x, 0)),
            _ => format!("!({})", cond_rs(x, 0)),
        },
        Cond::And(a, b) => wrap(format!("{} && {}", cond_rs(a, 1), cond_rs(b, 1)), 1),
        Cond::Or(a, b) => wrap(format!("{} || {}", cond_rs(a, 2), cond_rs(b, 2)), 2),
        Cond::Cmp(op, a, b) => {
            let op = match op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
            };
            format!("{} {op} {}", operand(*a), operand(*b))
        }
        Cond::SetContains { set, .. } => {
            let n = set_name(set);
            format!("set_contains_ascii(&{n}_ASCII, &{n}, lookahead)")
        }
    }
}

/// Go's lex.go compares an int32 lookahead with the literal, so a literal outside int32 does not compile.
fn check_lits(c: &Cond) -> Result<(), GenError> {
    match c {
        Cond::Eof | Cond::SetContains { .. } => Ok(()),
        Cond::Not(x) => check_lits(x),
        Cond::And(a, b) | Cond::Or(a, b) => check_lits(a).and(check_lits(b)),
        Cond::Cmp(_, a, b) => {
            for o in [a, b] {
                if let Operand::Lit(v) = o
                    && i32::try_from(*v).is_err()
                {
                    return err(format!("lex condition: literal {v} does not fit int32"));
                }
            }
            Ok(())
        }
    }
}

fn symbol(g: &Grammar, name: &str) -> Result<i64, GenError> {
    match g.consts.get(name) {
        Some(&v) if (0..=0xFFFF).contains(&v) => Ok(v),
        _ => err(format!("symbol {name} is not a u16 constant")),
    }
}

fn code_case(s: &mut String, g: &Grammar, case: &LexCase, run: RunId) -> Result<(), GenError> {
    s.push_str(&format!("            {} => {{\n", case.state));
    if let Some((k, skip)) = run {
        s.push_str(&format!(
            "                let (lookahead, eof) = if (0..128).contains(&lookahead) && LEX_RUN_{k}[lookahead as usize] {{
                    l.advance_run(&LEX_RUN_{k}, {skip});
                    (l.lookahead(), l.eof())
                }} else {{
                    (lookahead, eof)
                }};\n"
        ));
    }
    for st in &case.stmts {
        match st {
            Stmt::End => s.push_str("                return result;\n"),
            Stmt::Accept(name) => s.push_str(&format!(
                "                result = true;\n                l.set_result_symbol({}); // {name}\n                l.mark_end();\n",
                symbol(g, name)?
            )),
            Stmt::If { cond, skip, target } => s.push_str(&format!(
                "                if {} {{\n                    l.advance({skip});\n                    state = {target};\n                    continue;\n                }}\n",
                cond_rs(cond, 0)
            )),
            Stmt::Map(pairs) => {
                s.push_str("                match lookahead {\n");
                for (key, target) in pairs {
                    s.push_str(&format!(
                        "                    {key} => {{\n                        l.advance(false);\n                        state = {target};\n                        continue;\n                    }}\n"
                    ));
                }
                s.push_str("                    _ => {}\n                }\n");
            }
        }
    }
    s.push_str("            }\n");
    Ok(())
}

fn code_fn(
    s: &mut String,
    g: &Grammar,
    f: &LexFunc,
    name: &str,
    ids: &[RunId],
) -> Result<(), GenError> {
    s.push_str(&format!(
        "\n// {}\n#[allow(\n    unused_mut,\n    unused_variables,\n    clippy::manual_range_contains,\n    clippy::never_loop,\n    clippy::nonminimal_bool,\n    clippy::single_match\n)]\npub fn {name}<L: LexOps>(l: &mut L, mut state: u16) -> bool {{\n    let mut result = false;\n    loop {{\n        let lookahead = l.lookahead();\n        let eof = l.eof();\n        match state {{\n",
        f.cname
    ));
    for (case, run) in f.cases.iter().zip(ids) {
        code_case(s, g, case, *run)?;
    }
    s.push_str("            _ => return false,\n        }\n    }\n}\n");
    Ok(())
}

fn emit_code(g: &Grammar, sets: &Sets) -> Result<String, GenError> {
    let (runs, ids) = run_ids(g, sets);
    let mut used = Vec::new();
    for f in &g.lex_fns {
        fn_name(f)?;
        for st in f.cases.iter().flat_map(|c| &c.stmts) {
            if let Stmt::If { cond, .. } = st {
                cond_sets(cond, &mut used);
            }
        }
    }
    let mut s = if used.is_empty() {
        "use gositter_grammars::lex::LexOps;\n".to_string()
    } else {
        "use gositter_grammars::lex::{LexOps, set_contains_ascii};\n".to_string()
    };
    for name in &g.order {
        let Some(ranges) = sets.get(name).filter(|_| used.contains(name)) else {
            continue;
        };
        let n = set_name(name);
        let list: Vec<String> = ranges.iter().map(|(a, b)| format!("({a}, {b})")).collect();
        s.push_str(&format!(
            "\nstatic {n}: [(i32, i32); {}] = [{}];\n",
            ranges.len(),
            list.join(", ")
        ));
        s.push('\n');
        let bits = (0..128).map(|c| set_contains(ranges, c));
        s.push_str(&bool_array(&format!("{n}_ASCII"), bits, 128));
    }
    for (k, set) in runs.iter().enumerate() {
        s.push('\n');
        s.push_str(&run_static(k, set));
    }
    for (f, ids) in g.lex_fns.iter().zip(&ids) {
        code_fn(&mut s, g, f, fn_name(f)?, ids)?;
    }
    Ok(s)
}

/// The text of `lex.rs` after its header lines.
pub fn emit_lex(g: &Grammar) -> Result<String, GenError> {
    for st in g
        .lex_fns
        .iter()
        .flat_map(|f| &f.cases)
        .flat_map(|c| &c.stmts)
    {
        if let Stmt::If { cond, .. } = st {
            check_lits(cond)?;
        }
    }
    emit_code(g, &char_sets(g))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = include_str!("../../testdata/gengrammar/mini.c");

    fn with_lit(v: i64) -> Grammar {
        let mut g = crate::parse::parse_grammar(MINI).unwrap();
        let cond = Cond::Cmp(CmpOp::Lt, Operand::Lookahead, Operand::Lit(v));
        let stmts = vec![
            Stmt::If {
                cond,
                skip: false,
                target: 0,
            },
            Stmt::End,
        ];
        g.lex_fns[0].cases.push(LexCase {
            state: 60000,
            stmts,
        });
        g
    }

    // Go never adds 1 to the literal (run.go value()), so i64::MAX must not panic; it is refused
    // like Go's lex.go, which does not compile `lookahead < <literal over int32>`.
    #[test]
    fn literal_outside_int32_is_refused() {
        for v in [i64::MAX, i64::from(i32::MAX) + 1, i64::from(i32::MIN) - 1] {
            let e = emit_lex(&with_lit(v)).unwrap_err();
            assert!(e.0.contains("does not fit int32"), "{v}: {}", e.0);
        }
        emit_lex(&with_lit(i64::from(i32::MAX))).unwrap();
    }
}
