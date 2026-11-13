//! language.c and language.h (gositter `language.go`). The lookahead iterator, supertype and
//! symbol_for_name functions serve queries only and are not ported.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::{
    FieldId, Language, LexMode, PARSE_ACTION_TYPE_REDUCE, PARSE_ACTION_TYPE_SHIFT, SYMBOL_ERROR,
    SYMBOL_ERROR_REPEAT, StateId, Symbol, SymbolMetadata,
};

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
    pub fn symbol_name(&self, symbol: Symbol) -> Option<&'static str> {
        if symbol == SYMBOL_ERROR {
            Some("ERROR")
        } else if symbol == SYMBOL_ERROR_REPEAT {
            Some("_ERROR")
        } else if (symbol as u32) < self.symbol_count() {
            Some(self.tables.symbol_names[symbol as usize])
        } else {
            None
        }
    }

    /// ts_language_field_name_for_id: `None` for 0 or out of range.
    pub fn field_name_for_id(&self, id: FieldId) -> Option<&'static str> {
        let count = self.tables.field_count;
        if count != 0 && id as u32 <= count {
            Some(self.tables.field_names[id as usize]).filter(|name| !name.is_empty())
        } else {
            None
        }
    }

    /// ts_language_field_id_for_name: 0 if there is no such field. The scan's answer for each
    /// field name is kept; any other name is scanned.
    pub fn field_id_for_name(&self, name: &str) -> FieldId {
        let ids = self.derived.field_ids.get_or_init(|| {
            let count = self.tables.field_count as u16 as usize;
            self.tables.field_names[1..count + 1]
                .iter()
                .map(|&field| (field, self.field_id_for_name_scan(field)))
                .collect::<HashMap<_, _>>()
        });
        match ids.get(name) {
            Some(&id) => id,
            None => self.field_id_for_name_scan(name),
        }
    }

    /// The bits that stand for the named node kinds in `Node::preorder_mask`, giving each new
    /// name the next of 32 bits. A tree knows the kinds named before it was parsed. Panics
    /// with "gositter: more than 32 kinds named to KindMask". No C counterpart.
    pub fn kind_mask(&self, kinds: &[&str]) -> u32 {
        let mut names = self
            .derived
            .kind_names
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut mask = 0u32;
        for &kind in kinds {
            let i = match names.iter().position(|name| name == kind) {
                Some(i) => i,
                None => {
                    let i = names.len();
                    if i == 32 {
                        panic!("gositter: more than 32 kinds named to KindMask");
                    }
                    names.push(kind.to_string());
                    let count = self.symbol_count() as usize;
                    let bits = self
                        .derived
                        .kind_bits
                        .get_or_init(|| (0..count).map(|_| AtomicU32::new(0)).collect());
                    for (s, &name) in self.tables.symbol_names[..count].iter().enumerate() {
                        if name == kind {
                            bits[s].fetch_or(1 << i, Ordering::Relaxed);
                        }
                    }
                    if kind == "ERROR" {
                        self.derived.kind_error.store(1 << i, Ordering::Relaxed);
                    }
                    i
                }
            };
            mask |= 1 << i;
        }
        mask
    }

    /// ts_language_symbol_count
    pub(crate) fn symbol_count(&self) -> u32 {
        self.tables.symbol_count + self.tables.alias_count
    }

    /// ts_language_table_entry: (index of the first action in `parse_actions`, count,
    /// reusable); (0, 0, false) for the error symbols.
    pub(crate) fn table_entry(&self, state: StateId, symbol: Symbol) -> (usize, u8, bool) {
        if symbol == SYMBOL_ERROR || symbol == SYMBOL_ERROR_REPEAT {
            return (0, 0, false);
        }
        let action_index = self.lookup(state, symbol) as usize;
        let entry = self.tables.parse_actions.get(action_index);
        (action_index + 1, entry.count(), entry.reusable())
    }

    /// ts_language_lex_mode_for_state
    pub(crate) fn lex_mode_for_state(&self, state: StateId) -> LexMode {
        let mut mode = self.tables.lex_modes.get(state as usize);
        if self.tables.abi_version < 15 {
            // TSLexMode has no reserved_word_set_id.
            mode.reserved_word_set_id = 0;
        }
        mode
    }

    /// ts_language_is_reserved_word
    pub(crate) fn is_reserved_word(&self, state: StateId, symbol: Symbol) -> bool {
        let lex_mode = self.lex_mode_for_state(state);
        if lex_mode.reserved_word_set_id > 0 {
            let size = self.tables.max_reserved_word_set_size as usize;
            let start = lex_mode.reserved_word_set_id as usize * size;
            for i in start..start + size {
                let word = self.tables.reserved_words.get(i);
                if word == symbol {
                    return true;
                }
                if word == 0 {
                    break;
                }
            }
        }
        false
    }

    /// ts_language_symbol_metadata
    pub(crate) fn symbol_metadata(&self, symbol: Symbol) -> SymbolMetadata {
        if symbol == SYMBOL_ERROR {
            SymbolMetadata {
                visible: true,
                named: true,
                supertype: false,
            }
        } else if symbol == SYMBOL_ERROR_REPEAT {
            SymbolMetadata::default()
        } else {
            self.tables.symbol_metadata.get(symbol as usize)
        }
    }

    /// ts_language_public_symbol
    pub(crate) fn public_symbol(&self, symbol: Symbol) -> Symbol {
        if symbol == SYMBOL_ERROR {
            return symbol;
        }
        self.tables.public_symbol_map.get(symbol as usize)
    }

    /// ts_language_next_state
    pub(crate) fn next_state(&self, state: StateId, symbol: Symbol) -> StateId {
        if symbol == SYMBOL_ERROR || symbol == SYMBOL_ERROR_REPEAT {
            0
        } else if (symbol as u32) < self.tables.token_count {
            let (first, count, _) = self.table_entry(state, symbol);
            if count > 0 {
                let action = self.tables.parse_actions.get(first + count as usize - 1);
                if action.action_type == PARSE_ACTION_TYPE_SHIFT {
                    return if action.extra() {
                        state
                    } else {
                        action.state()
                    };
                }
            }
            0
        } else {
            self.lookup(state, symbol)
        }
    }

    /// The KindMask bit of a symbol; 0 for a kind nobody named.
    pub(crate) fn kind_bit(&self, symbol: Symbol) -> u32 {
        if let Some(bits) = self.derived.kind_bits.get()
            && (symbol as usize) < bits.len()
        {
            return bits[symbol as usize].load(Ordering::Relaxed);
        }
        if symbol == SYMBOL_ERROR {
            return self.derived.kind_error.load(Ordering::Relaxed);
        }
        0
    }

    /// ts_language_field_id_for_name, the C scan (gositter `fieldIDForName`). It is
    /// strncmp(name, field, len(name)); C switches on the result with "case -1", and darwin's
    /// strncmp returns the byte difference, so only a difference of exactly -1 ends the scan
    /// early.
    pub(crate) fn field_id_for_name_scan(&self, name: &str) -> FieldId {
        let count = self.tables.field_count as u16 as u32;
        let name = name.as_bytes();
        for i in 1..count + 1 {
            let field = self.tables.field_names[i as usize].as_bytes();
            let mut cmp = 0i32;
            for (j, &nc) in name.iter().enumerate() {
                let fc = field.get(j).copied().unwrap_or(0);
                if nc != fc {
                    cmp = nc as i32 - fc as i32;
                    break;
                }
                if fc == 0 {
                    break;
                }
            }
            match cmp {
                0 if field.len() <= name.len() => return i as FieldId,
                -1 => return 0,
                _ => {}
            }
        }
        0
    }

    /// ts_language_has_reduce_action
    pub(crate) fn has_reduce_action(&self, state: StateId, symbol: Symbol) -> bool {
        let (first, count, _) = self.table_entry(state, symbol);
        count > 0 && self.tables.parse_actions.get(first).action_type == PARSE_ACTION_TYPE_REDUCE
    }

    /// ts_language_lookup. For non-terminals the value is a successor state, for terminals
    /// an index into `parse_actions`.
    pub(crate) fn lookup(&self, state: StateId, symbol: Symbol) -> u16 {
        if state as u32 >= self.tables.large_state_count {
            return self.lookup_small(state, symbol);
        }
        let index = state as u32 * self.tables.symbol_count + symbol as u32;
        self.tables.parse_table.get(index as usize)
    }

    /// The small-state half of ts_language_lookup. The comb table holds what the scan
    /// returns: two loads instead of a search.
    pub(crate) fn lookup_small(&self, state: StateId, symbol: Symbol) -> u16 {
        let st = state as u32 - self.tables.large_state_count;
        if let Some(Some(t)) = self.derived.small.get()
            && (st as usize) < t.base.len()
            && (symbol as u32) < self.tables.symbol_count
        {
            // A slot belongs to the state whose number it carries; any other slot means the
            // scan would find nothing.
            let e = t.comb[(t.base[st as usize] + symbol as u32) as usize];
            return if e >> 16 == st + 1 { e as u16 } else { 0 };
        }
        self.scan_small(state, symbol)
    }

    /// The small parse table as C scans it. It also counts the entries it walks: the comb
    /// table costs memory and time to build, so a language gets it only once its scans have
    /// walked the small parse table eight times over.
    pub(crate) fn scan_small(&self, state: StateId, symbol: Symbol) -> u16 {
        let table = self.tables.small_parse_table;
        let index =
            self.tables
                .small_parse_table_map
                .get((state as u32 - self.tables.large_state_count) as usize) as usize;
        let group_count = table.get(index);
        let mut pos = 1usize;
        let mut value = 0u16;
        'scan: for _ in 0..group_count {
            let section_value = table.get(index + pos);
            let symbol_count = table.get(index + pos + 1) as usize;
            pos += 2;
            for k in 0..symbol_count {
                if table.get(index + pos + k) == symbol {
                    value = section_value;
                    break 'scan;
                }
            }
            pos += symbol_count;
        }
        let walked = self
            .derived
            .scanned
            .fetch_add(pos as i64, Ordering::Relaxed)
            + pos as i64;
        if walked >= 8 * table.len() as i64 {
            self.prepare();
        }
        value
    }

    /// Builds the comb table once per Language. Parsers that are mid-parse pick it up at
    /// their next lookup: both paths give the same answer.
    pub(crate) fn prepare(&self) {
        self.derived.small.get_or_init(|| self.build_small_table());
    }

    /// The small parse table as a row-displacement ("comb") table: the row of small state st
    /// is laid over one shared array at offset base[st], wherever its non-zero entries hit
    /// only free slots. A slot is (st+1)<<16 | value, and 0 when free. `None` when the state
    /// numbers do not fit in 16 bits.
    fn build_small_table(&self) -> Option<SmallTable> {
        let tables = &self.tables;
        if tables.state_count <= tables.large_state_count {
            return None;
        }
        let small = tables.state_count - tables.large_state_count;
        if small >= 0xFFFF || (tables.small_parse_table_map.len() as u32) < small {
            return None;
        }
        let spt = tables.small_parse_table;
        let symbol_count = tables.symbol_count;

        // The rows: for each state the (symbol, value) pairs the scan can return. The scan
        // returns the first match, and a zero value is the same as no entry.
        let mut entries: Vec<(u16, u16)> = Vec::with_capacity(spt.len());
        let mut row_start = vec![0usize; small as usize + 1];
        let mut seen = vec![0u32; symbol_count as usize];
        for st in 0..small {
            row_start[st as usize] = entries.len();
            let base = tables.small_parse_table_map.get(st as usize) as usize;
            let group_count = spt.get(base);
            let mut pos = 1usize;
            for _ in 0..group_count {
                let section_value = spt.get(base + pos);
                let count = spt.get(base + pos + 1) as usize;
                pos += 2;
                for k in 0..count {
                    let s = spt.get(base + pos + k);
                    if (s as u32) < symbol_count && seen[s as usize] != st + 1 {
                        seen[s as usize] = st + 1;
                        if section_value != 0 {
                            entries.push((s, section_value));
                        }
                    }
                }
                pos += count;
            }
        }
        row_start[small as usize] = entries.len();

        // First fit, longest rows first: they are the hardest to place. sort_by is stable.
        let row_len = |st: u32| row_start[st as usize + 1] - row_start[st as usize];
        let mut order: Vec<u32> = (0..small).collect();
        order.sort_by(|&a, &b| row_len(b).cmp(&row_len(a)));

        let mut base = vec![0u32; small as usize];
        let mut comb = vec![0u32; symbol_count as usize];
        let mut used: Vec<u64> = vec![0; (spt.len() * 7 / 4 + symbol_count as usize) / 64 + 2];
        let mut lowest = 0u32; // no free slot below this index
        let mut long_base = 0u32;
        for &st in &order {
            let row = &entries[row_start[st as usize]..row_start[st as usize + 1]];
            if row.is_empty() {
                continue;
            }
            while (lowest as usize) < comb.len() && comb[lowest as usize] != 0 {
                lowest += 1;
            }
            let min_symbol = row.iter().map(|e| e.0 as u32).min().unwrap_or(0);
            // Short rows search from the lowest free slot and fill holes. Long rows start
            // where the last long row went, and that cursor only moves forward.
            let mut b = lowest.saturating_sub(min_symbol);
            let long = row.len() >= COMB_LONG_ROW;
            if long && long_base > b {
                b = long_base;
            }
            // 64 bases at a time: bit j of taken is set when base b+j puts an entry on a
            // used slot.
            loop {
                let need = ((b + symbol_count) >> 6) as usize + 2;
                if need > used.len() {
                    used.resize(need, 0);
                }
                let mut taken = 0u64;
                for e in row {
                    let i = b + e.0 as u32;
                    let w = (i >> 6) as usize;
                    // Go shifts by 64 to 0; Rust's << would not.
                    let high = used[w + 1].checked_shl(64 - (i & 63)).unwrap_or(0);
                    taken |= (used[w] >> (i & 63)) | high;
                    if taken == u64::MAX {
                        break;
                    }
                }
                if taken != u64::MAX {
                    b += (!taken).trailing_zeros();
                    break;
                }
                b += 64;
            }
            if long {
                long_base = b;
            }
            let need = (b + symbol_count) as usize;
            if need > comb.len() {
                comb.resize(need, 0);
            }
            for e in row {
                let i = b + e.0 as u32;
                comb[i as usize] = ((st + 1) << 16) | e.1 as u32;
                used[(i >> 6) as usize] |= 1 << (i & 63);
            }
            base[st as usize] = b;
        }
        Some(SmallTable { base, comb })
    }

    /// ts_language_has_actions
    pub(crate) fn has_actions(&self, state: StateId, symbol: Symbol) -> bool {
        self.lookup(state, symbol) != 0
    }

    /// ts_language_enabled_external_tokens: (start, length) in `external_scanner.states`;
    /// (0, 0) where C returns NULL.
    pub(crate) fn enabled_external_tokens(&self, external_scanner_state: u32) -> (usize, usize) {
        if external_scanner_state == 0 {
            return (0, 0);
        }
        let n = self.tables.external_token_count as usize;
        (n * external_scanner_state as usize, n)
    }

    /// ts_language_alias_sequence: the start of the sequence in `alias_sequences`, `None`
    /// where C returns NULL. It runs to the end of the table: C indexes it without a bound.
    pub(crate) fn alias_sequence(&self, production_id: u32) -> Option<usize> {
        if production_id == 0 {
            return None;
        }
        Some(production_id as usize * self.tables.max_alias_sequence_length as usize)
    }

    /// ts_language_alias_at
    pub(crate) fn alias_at(&self, production_id: u32, child_index: u32) -> Symbol {
        match self.alias_sequence(production_id) {
            Some(start) => self
                .tables
                .alias_sequences
                .get(start + child_index as usize),
            None => 0,
        }
    }

    /// ts_language_field_map: (start, length) in `field_map_entries`.
    pub(crate) fn field_map(&self, production_id: u32) -> (usize, usize) {
        if self.tables.field_count == 0 {
            return (0, 0);
        }
        let slice = self.tables.field_map_slices.get(production_id as usize);
        (slice.index as usize, slice.length as usize)
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
    use crate::LanguageTables;
    use crate::test_language::*;

    #[test]
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
    #[should_panic(expected = "gositter: more than 32 kinds named to KindMask")]
    fn kind_mask_panics_on_33rd() {
        let lang = test_language();
        let names: Vec<String> = (0..32).map(|i| format!("k{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(lang.kind_mask(&names), u32::MAX);
        lang.kind_mask(&["one too many"]);
    }

    #[test]
    fn kind_mask_repeat_name_same_bit() {
        let lang = test_language();
        assert_eq!(lang.kind_mask(&["ident", "ident"]), 1);
        assert_eq!(lang.kind_mask(&["rule"]), 2);
        assert_eq!(lang.kind_mask(&["rule", "ident"]), 3);
        assert_eq!(lang.kind_bit(SYMBOL_ERROR), 0);
        assert_eq!(lang.kind_bit(TS_END), 0);
    }

    #[test]
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
