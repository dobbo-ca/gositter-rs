//! language.c and language.h (gositter `language.go`). The lookahead iterator, supertype and
//! symbol_for_name functions serve queries only and are not ported.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU32};
use std::sync::{Mutex, OnceLock};

use crate::{FieldId, Language, LexMode, StateId, Symbol, SymbolMetadata};

/// Entries; see `prepare`.
pub(crate) const COMB_LONG_ROW: usize = 4;

/// The small parse table as a row-displacement ("comb") table (`prepare`).
pub(crate) struct SmallTable {
    /// Per small state: its offset in `comb`.
    pub(crate) base: Vec<u32>,
    /// `(state + 1) << 16 | value`, 0 if free.
    pub(crate) comb: Vec<u32>,
}

/// What the runtime derives from a language's tables (gositter `Language`'s private fields).
pub(crate) struct Derived {
    /// Entries walked by `scan_small`.
    pub(crate) scanned: AtomicI64,
    /// Built by `prepare`; `Some(None)` when the table does not fit its encoding.
    pub(crate) small: OnceLock<Option<SmallTable>>,
    /// The scan's answer for each field name (`field_id_for_name`).
    pub(crate) field_ids: OnceLock<HashMap<&'static str, FieldId>>,
    /// The names given to `kind_mask`, in bit order.
    pub(crate) kind_names: Mutex<Vec<String>>,
    /// The kind bits of each symbol (`symbol_count + alias_count` entries).
    pub(crate) kind_bits: OnceLock<Box<[AtomicU32]>>,
    /// The kind bit of ERROR.
    pub(crate) kind_error: AtomicU32,
}

impl Derived {
    pub(crate) const fn new() -> Derived {
        Derived {
            scanned: AtomicI64::new(0),
            small: OnceLock::new(),
            field_ids: OnceLock::new(),
            kind_names: Mutex::new(Vec::new()),
            kind_bits: OnceLock::new(),
            kind_error: AtomicU32::new(0),
        }
    }
}

impl Language {
    /// ts_language_symbol_name: "ERROR" and "_ERROR" for the builtins, `None` out of range.
    pub fn symbol_name(&self, _symbol: Symbol) -> Option<&'static str> {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_field_name_for_id: `None` for 0 or out of range.
    pub fn field_name_for_id(&self, _id: FieldId) -> Option<&'static str> {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_field_id_for_name: 0 if there is no such field. The scan's answer for each
    /// field name is kept; any other name is scanned.
    pub fn field_id_for_name(&self, _name: &str) -> FieldId {
        todo!("gositter-rs-2f0.7")
    }

    /// The bits that stand for the named node kinds in `Node::preorder_mask`, giving each new
    /// name the next of 32 bits. A tree knows the kinds named before it was parsed. Panics
    /// with "gositter: more than 32 kinds named to KindMask". No C counterpart.
    pub fn kind_mask(&self, _kinds: &[&str]) -> u32 {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_symbol_count
    pub(crate) fn symbol_count(&self) -> u32 {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_table_entry: (index of the first action in `parse_actions`, count,
    /// reusable); (0, 0, false) for the error symbols.
    pub(crate) fn table_entry(&self, _state: StateId, _symbol: Symbol) -> (usize, u8, bool) {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_lex_mode_for_state
    pub(crate) fn lex_mode_for_state(&self, _state: StateId) -> LexMode {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_is_reserved_word
    pub(crate) fn is_reserved_word(&self, _state: StateId, _symbol: Symbol) -> bool {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_symbol_metadata
    pub(crate) fn symbol_metadata(&self, _symbol: Symbol) -> SymbolMetadata {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_public_symbol
    pub(crate) fn public_symbol(&self, _symbol: Symbol) -> Symbol {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_next_state
    pub(crate) fn next_state(&self, _state: StateId, _symbol: Symbol) -> StateId {
        todo!("gositter-rs-2f0.7")
    }

    /// The KindMask bit of a symbol; 0 for a kind nobody named.
    pub(crate) fn kind_bit(&self, _symbol: Symbol) -> u32 {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_field_id_for_name, the C scan (gositter `fieldIDForName`).
    pub(crate) fn field_id_for_name_scan(&self, _name: &str) -> FieldId {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_has_reduce_action
    pub(crate) fn has_reduce_action(&self, _state: StateId, _symbol: Symbol) -> bool {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_lookup. For non-terminals the value is a successor state, for terminals
    /// an index into `parse_actions`.
    pub(crate) fn lookup(&self, _state: StateId, _symbol: Symbol) -> u16 {
        todo!("gositter-rs-2f0.7")
    }

    /// The small-state half of ts_language_lookup.
    pub(crate) fn lookup_small(&self, _state: StateId, _symbol: Symbol) -> u16 {
        todo!("gositter-rs-2f0.7")
    }

    /// The small parse table as C scans it; counts the entries it walks and builds the comb
    /// table once they reach eight times the table.
    pub(crate) fn scan_small(&self, _state: StateId, _symbol: Symbol) -> u16 {
        todo!("gositter-rs-2f0.7")
    }

    /// Builds the comb table once per Language.
    pub(crate) fn prepare(&self) {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_has_actions
    pub(crate) fn has_actions(&self, _state: StateId, _symbol: Symbol) -> bool {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_enabled_external_tokens: (start, length) in `external_scanner.states`;
    /// (0, 0) where C returns NULL.
    pub(crate) fn enabled_external_tokens(&self, _external_scanner_state: u32) -> (usize, usize) {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_alias_sequence: the start of the sequence in `alias_sequences`, `None`
    /// where C returns NULL. It runs to the end of the table: C indexes it without a bound.
    pub(crate) fn alias_sequence(&self, _production_id: u32) -> Option<usize> {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_alias_at
    pub(crate) fn alias_at(&self, _production_id: u32, _child_index: u32) -> Symbol {
        todo!("gositter-rs-2f0.7")
    }

    /// ts_language_field_map: (start, length) in `field_map_entries`.
    pub(crate) fn field_map(&self, _production_id: u32) -> (usize, usize) {
        todo!("gositter-rs-2f0.7")
    }
}

impl Clone for Language {
    fn clone(&self) -> Self {
        Language::new(self.tables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_language::*;
    use crate::{LanguageTables, SYMBOL_ERROR, SYMBOL_ERROR_REPEAT};

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn kind_mask_threads() {
        let lang = test_language();
        let kinds = ["rule", "ERROR", "ident"];
        let want = lang.kind_mask(&kinds);
        assert_eq!(want, 0b111);
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    for _ in 0..1000 {
                        assert_eq!(lang.kind_mask(&kinds), want);
                        assert_eq!(lang.kind_bit(TS_RULE), 0b001);
                        assert_eq!(lang.kind_bit(SYMBOL_ERROR), 0b010);
                        assert_eq!(lang.kind_bit(TS_IDENT), 0b100);
                        assert_eq!(lang.kind_bit(TS_ANON), 0);
                    }
                });
            }
        });
        // A new name after the threads takes the next bit.
        assert_eq!(lang.kind_mask(&["a", "rule"]), 0b1001);
        assert_eq!(lang.kind_bit(TS_ANON), 0b1000);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    #[should_panic(expected = "gositter: more than 32 kinds named to KindMask")]
    fn kind_mask_panics_on_33rd() {
        let lang = test_language();
        let names: Vec<String> = (0..32).map(|i| format!("k{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(lang.kind_mask(&names), u32::MAX);
        lang.kind_mask(&["one too many"]);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn kind_mask_repeat_name_same_bit() {
        let lang = test_language();
        assert_eq!(lang.kind_mask(&["ident", "ident"]), 1);
        assert_eq!(lang.kind_mask(&["rule"]), 2);
        assert_eq!(lang.kind_mask(&["rule", "ident"]), 3);
        assert_eq!(lang.kind_bit(SYMBOL_ERROR), 0);
        assert_eq!(lang.kind_bit(TS_END), 0);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn symbol_name_builtins() {
        let lang = test_language();
        assert_eq!(lang.symbol_name(SYMBOL_ERROR), Some("ERROR"));
        assert_eq!(lang.symbol_name(SYMBOL_ERROR_REPEAT), Some("_ERROR"));
        assert_eq!(lang.symbol_name(TS_HIDDEN), Some("_hidden"));
        assert_eq!(lang.symbol_name(TS_REPEAT), Some("rule_repeat1"));
        assert_eq!(lang.symbol_name(6), None);
        assert_eq!(lang.symbol_count(), 6);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn field_names_and_ids() {
        let lang = test_language();
        assert_eq!(lang.field_name_for_id(0), None);
        assert_eq!(lang.field_name_for_id(1), Some("key"));
        assert_eq!(lang.field_name_for_id(2), Some("value"));
        assert_eq!(lang.field_name_for_id(3), None);
        // gositter language.go:175-206 worked by hand: strncmp(name, field, len(name)), and
        // darwin's byte difference.
        for (name, want) in [
            ("", 0),
            ("valu", 0),
            ("value", 2),
            ("value\0", 2),
            ("valuex", 0),
            ("k", 0),
            ("key", 1),
        ] {
            assert_eq!(lang.field_id_for_name(name), want, "{name:?}");
            assert_eq!(lang.field_id_for_name_scan(name), want, "{name:?} (scan)");
        }
        assert_eq!(lang.field_id_mismatch(), None);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn field_id_scan_stops_at_minus_one() {
        // C switches on strncmp with "case -1": "a" against field 1 "b" is exactly -1, so
        // the scan ends before it reaches field 2 "a".
        let lang = Box::leak(Box::new(Language::new(LanguageTables {
            field_names: &["", "b", "a"],
            ..TABLES
        })));
        assert_eq!(lang.field_id_for_name("a"), 0);
        assert_eq!(lang.field_id_for_name("b"), 1);
        assert_eq!(lang.field_id_mismatch(), None);
    }

    /// (state, symbol, value) of every non-zero lookup in the test language.
    const LOOKUPS: [(StateId, Symbol, u16); 9] = [
        (0, 1, 1),
        (0, 4, 2),
        (1, 1, 5),
        (1, 2, 5),
        (1, 4, 7),
        (2, 0, 3),
        (2, 1, 3),
        (2, 5, 3),
        (3, 3, 9),
    ];

    fn check_lookups(lang: &Language) {
        for state in 0..4 {
            // C asks a large state only for symbols below symbol_count.
            for symbol in 0..6 {
                let want = LOOKUPS
                    .iter()
                    .find(|&&(st, sy, _)| st == state && sy == symbol)
                    .map_or(0, |&(_, _, v)| v);
                assert_eq!(
                    lang.lookup(state, symbol),
                    want,
                    "state {state} symbol {symbol}"
                );
                assert_eq!(lang.has_actions(state, symbol), want != 0);
            }
        }
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn small_table_lookup() {
        let lang = test_language();
        // Before and after the comb table exists (gositter dense_test.go on a small table).
        check_lookups(lang);
        let (state, symbol, built, ok) = lang.dense_mismatch();
        assert!(built, "comb table not built");
        assert!(
            ok,
            "state {state} symbol {symbol}: the comb table differs from the scan"
        );
        assert!(lang.small_table_bytes() > 0);
        check_lookups(lang);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn comb_table_builds_after_eight_passes() {
        let lang = test_language();
        assert!(lang.derived.small.get().is_none());
        // Each scan of state 3 for symbol 0 walks the 8 entries of its row.
        for _ in 0..21 {
            lang.scan_small(3, 0);
        }
        assert!(
            lang.derived.small.get().is_none(),
            "built before 8 x 22 entries"
        );
        lang.scan_small(3, 0);
        assert!(
            lang.derived.small.get().is_some(),
            "not built at 8 x 22 entries"
        );
    }

    #[test]
    #[ignore = "gositter-rs-2f0.7"]
    fn table_entry_of_error_symbols() {
        let lang = test_language();
        assert_eq!(lang.table_entry(1, SYMBOL_ERROR), (0, 0, false));
        assert_eq!(lang.table_entry(1, SYMBOL_ERROR_REPEAT), (0, 0, false));
        assert_eq!(lang.next_state(1, SYMBOL_ERROR), 0);
        assert_eq!(
            lang.symbol_metadata(SYMBOL_ERROR),
            SymbolMetadata {
                visible: true,
                named: true,
                supertype: false
            }
        );
        assert_eq!(
            lang.symbol_metadata(SYMBOL_ERROR_REPEAT),
            SymbolMetadata::default()
        );
        assert_eq!(lang.public_symbol(SYMBOL_ERROR), SYMBOL_ERROR);
        assert_eq!(lang.public_symbol(TS_RULE), TS_RULE);
        assert_eq!(lang.alias_sequence(0), None);
        assert_eq!(lang.alias_at(0, 3), 0);
        assert_eq!(lang.enabled_external_tokens(0), (0, 0));
    }
}
