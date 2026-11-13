//! The json `language()` as generated: counts, names, and the (state, terminal) lookups of gositter
//! `cmd/gengrammar/check/main.go`, each of which must land on a parse action header.

use gositter_grammars::json::{self, SYMBOL_NAMES};

#[test]
fn json_language_tables() {
    let l = json::language();
    let t = &l.tables;
    assert_eq!(
        (t.abi_version, t.symbol_count, t.token_count, t.field_count),
        (14, 25, 15, 2)
    );
    assert_eq!(
        (t.state_count, t.large_state_count, t.production_id_count),
        (32, 7, 2)
    );
    assert_eq!(t.name, "json");
    assert!(t.external_scanner.create.is_none());
    assert!(t.keyword_lex_fn.is_none()); // json's lex.rs has no lex_keywords
    for s in 0..t.symbol_count as u16 {
        assert_eq!(l.symbol_name(s), Some(SYMBOL_NAMES[s as usize]));
    }
    assert_eq!(l.field_name_for_id(1), Some("key"));
    assert_eq!(l.field_name_for_id(2), Some("value"));
}

/// ts_language_lookup, small states as `scanSmall` of gositter language.go.
fn lookup(t: &gositter::LanguageTables, state: u32, symbol: u32) -> u16 {
    if state < t.large_state_count {
        return t
            .parse_table
            .get((state * t.symbol_count + symbol) as usize);
    }
    let mut pos = t
        .small_parse_table_map
        .get((state - t.large_state_count) as usize) as usize;
    let groups = t.small_parse_table.get(pos);
    pos += 1;
    for _ in 0..groups {
        let value = t.small_parse_table.get(pos);
        let n = t.small_parse_table.get(pos + 1) as usize;
        pos += 2;
        if (pos..pos + n).any(|i| u32::from(t.small_parse_table.get(i)) == symbol) {
            return value;
        }
        pos += n;
    }
    0
}

#[test]
fn json_terminal_lookups_hit_action_headers() {
    let t = &json::language().tables;
    let mut header = vec![false; t.parse_actions.len()];
    let mut i = 0;
    while i < header.len() {
        header[i] = true;
        i += 1 + t.parse_actions.get(i).count() as usize;
    }
    assert_eq!(
        i,
        header.len(),
        "the last header's actions run past the table"
    );
    for state in 0..t.state_count {
        for symbol in 0..t.token_count {
            let v = lookup(t, state, symbol) as usize;
            assert!(
                header.get(v).copied().unwrap_or(false),
                "lookup({state}, {symbol}) = {v} is not a parse action header"
            );
        }
    }
}
