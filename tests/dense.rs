//! gositter `dense_test.go`: the row-displacement small parse table must answer every lookup
//! as the C scan does, for all 21 grammars.

mod common;

/// Go `TestDenseLookup`'s subtest for one grammar; what failed goes to `errs`.
fn dense_lookup_of(errs: &mut Vec<String>, dir: &str) {
    let l = common::language(dir).unwrap_or_else(|| panic!("{dir} grammar"));
    let (state, symbol, built, ok) = l.dense_mismatch();
    if !ok {
        errs.push(format!(
            "{dir}: state {state} symbol {symbol}: dense lookup differs from the scan"
        ));
        return;
    }
    if !built {
        errs.push(format!("{dir}: small table not built"));
    }
    if let Some(name) = l.field_id_mismatch() {
        errs.push(format!(
            "{dir}: field {name:?}: FieldIDForName differs from the scan"
        ));
    }
    println!("{dir}: {} KiB", l.small_table_bytes() >> 10);
}

#[test]
fn dense_lookup_json() {
    let mut errs = Vec::new();
    dense_lookup_of(&mut errs, "json");
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

/// The wired grammars one by one; `dense_lookup` covers all 21 once every grammar is wired.
#[test]
fn dense_lookup_wired() {
    let mut errs = Vec::new();
    for dir in [
        "c",
        "go",
        "java",
        "javascript",
        "python",
        "tsx",
        "typescript",
        "verilog",
        "zig",
    ] {
        dense_lookup_of(&mut errs, dir);
    }
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn dense_lookup() {
    let mut errs = Vec::new();
    for dir in common::DIRS {
        if dir != "json" {
            dense_lookup_of(&mut errs, dir);
        }
    }
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}
