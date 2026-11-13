//! Lexer emitter: the body of `grammars/<g>/lex.rs` in two forms, and `lex.bin` for the table form.
//!
//! Both forms follow the Go lexer gositter emits (`emit.go:254-377`, `run.go`): a state does its
//! run set first (the self-loop `advance_run`), then its statements in C order. A jump advances
//! (or skips) and restarts at the top with a fresh lookahead and `eof`; `Accept` sets the result,
//! the symbol and the mark in that order; `End` returns the result.
//!
//! The code form is `loop { match state { N => { ... } _ => return false } }`. The table form is
//! `run_table` (in `grammars/lex.rs`) over `lex.bin`, which holds one region per lex function
//! (`ts_lex`, then `ts_lex_keywords` if the grammar has it). All numbers are little-endian:
//!
//! ```text
//! region:  u16 ncases
//!          ncases * { u16 state, u32 offset }   sorted by state; offset from the region start
//!          records
//! record:  u16 run          index into the run sets, 0xFFFF for none
//!          u8  run_skip     the run set skips instead of advancing
//!          ops, the last is End
//! op:      u8 0                                          End
//!          u8 1, u16 symbol                              Accept
//!          u8 2, u8 flags, u16 target,                   a jump: advance(flags & 1) to target
//!               u16 n, n * { i32 start, i32 end }        when the lookahead is in the ranges
//!               [if flags & 2: u16 n, n * { i32, i32 }]  the ranges to use at end of input
//! ```
//!
//! The ranges of a jump are those of `cond::ranges_with` over -1..=0x10FFFF, sorted and disjoint;
//! an `ADVANCE_MAP` pair is a jump on one value. Without the second list the first one holds at end
//! of input too. The run sets are the `LEX_RUN_k` statics of `lex.rs`, passed to `run_table`.

use std::collections::HashMap;

use crate::cond::{CmpOp, Cond, Operand, char_sets, ranges_with, run_sets, set_contains};
use crate::ctok::GenError;
use crate::parse::{Grammar, LexCase, LexFunc, Stmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexForm {
    Code,
    Table,
}

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

fn put_u16(b: &mut Vec<u8>, v: usize) -> Result<(), GenError> {
    let v = u16::try_from(v)
        .map_err(|_| GenError(format!("lex table: {v} does not fit in 16 bits")))?;
    b.extend(v.to_le_bytes());
    Ok(())
}

fn put_ranges(b: &mut Vec<u8>, r: &[(i32, i32)]) -> Result<(), GenError> {
    put_u16(b, r.len())?;
    for (start, end) in r {
        b.extend(start.to_le_bytes());
        b.extend(end.to_le_bytes());
    }
    Ok(())
}

fn put_jump(
    b: &mut Vec<u8>,
    skip: bool,
    target: u16,
    ranges: &[(i32, i32)],
    at_eof: Option<&[(i32, i32)]>,
) -> Result<(), GenError> {
    b.extend([2, u8::from(skip) | (u8::from(at_eof.is_some()) << 1)]);
    b.extend(target.to_le_bytes());
    put_ranges(b, ranges)?;
    at_eof.map_or(Ok(()), |e| put_ranges(b, e))
}

fn record(g: &Grammar, case: &LexCase, run: RunId, sets: &Sets) -> Result<Vec<u8>, GenError> {
    let mut b = Vec::new();
    match run {
        Some((k, skip)) => {
            put_u16(&mut b, k)?;
            b.push(u8::from(skip));
        }
        None => b.extend([0xFF, 0xFF, 0]),
    }
    for st in &case.stmts {
        match st {
            Stmt::End => b.push(0),
            Stmt::Accept(name) => {
                b.push(1);
                put_u16(&mut b, symbol(g, name)? as usize)?;
            }
            Stmt::If { cond, skip, target } => {
                let r = ranges_with(cond, sets, false);
                let e = ranges_with(cond, sets, true);
                put_jump(&mut b, *skip, *target, &r, (e != r).then_some(&e[..]))?;
            }
            Stmt::Map(pairs) => {
                for &(key, target) in pairs {
                    put_jump(
                        &mut b,
                        false,
                        target,
                        &[(i32::from(key), i32::from(key))],
                        None,
                    )?;
                }
            }
        }
    }
    Ok(b)
}

fn region(g: &Grammar, f: &LexFunc, ids: &[RunId], sets: &Sets) -> Result<Vec<u8>, GenError> {
    let mut order: Vec<usize> = (0..f.cases.len()).collect();
    order.sort_by_key(|&i| f.cases[i].state);
    let head = 2 + 6 * order.len();
    let (mut dir, mut recs) = (Vec::new(), Vec::new());
    put_u16(&mut dir, order.len())?;
    for i in order {
        dir.extend(f.cases[i].state.to_le_bytes());
        dir.extend(((head + recs.len()) as u32).to_le_bytes());
        recs.extend(record(g, &f.cases[i], ids[i], sets)?);
    }
    dir.extend(recs);
    Ok(dir)
}

fn emit_table(g: &Grammar, sets: &Sets) -> Result<(String, Vec<u8>), GenError> {
    let (runs, ids) = run_ids(g, sets);
    let mut s = "use gositter_grammars::lex::{LexOps, run_table};\n\nstatic DATA: &[u8] = include_bytes!(\"lex.bin\");\n".to_string();
    for (k, set) in runs.iter().enumerate() {
        s.push('\n');
        s.push_str(&run_static(k, set));
    }
    let names: Vec<String> = (0..runs.len()).map(|k| format!("&LEX_RUN_{k}")).collect();
    s.push_str(&format!(
        "\nstatic LEX_RUNS: [&[bool; 256]; {}] = [{}];\n",
        runs.len(),
        names.join(", ")
    ));
    let mut bin = Vec::new();
    for (f, ids) in g.lex_fns.iter().zip(&ids) {
        let name = fn_name(f)?;
        let data = if bin.is_empty() {
            "DATA".to_string()
        } else {
            format!("&DATA[{}..]", bin.len())
        };
        s.push_str(&format!(
            "\n// {}\npub fn {name}<L: LexOps>(l: &mut L, state: u16) -> bool {{\n    run_table({data}, &LEX_RUNS, l, state)\n}}\n",
            f.cname
        ));
        bin.extend(region(g, f, ids, sets)?);
    }
    Ok((s, bin))
}

/// The text of `lex.rs` after its header lines, and for `Table` the bytes of `lex.bin` (empty for
/// `Code`).
pub fn emit_lex(g: &Grammar, form: LexForm) -> Result<(String, Vec<u8>), GenError> {
    let sets = char_sets(g);
    match form {
        LexForm::Code => Ok((emit_code(g, &sets)?, Vec::new())),
        LexForm::Table => emit_table(g, &sets),
    }
}
