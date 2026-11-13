//! gositter `export_test.go`: hooks into the runtime's internals for the integration tests
//! and the fuzz targets. Not part of the API.

use crate::{Language, StateId, Symbol};

impl Language {
    /// Compares the comb small-table lookup with the C scan for every small state and every
    /// symbol value a lookup can carry: (state, symbol, built, ok) of the first difference.
    #[doc(hidden)]
    pub fn dense_mismatch(&self) -> (u32, u32, bool, bool) {
        self.prepare();
        let built = matches!(self.derived.small.get(), Some(Some(_)));
        if !built {
            return (0, 0, false, true); // lookup is the scan itself
        }
        let t = &self.tables;
        for st in t.large_state_count..t.state_count {
            let mut sym = 0u32;
            while sym <= 0xFFFF {
                if sym == t.symbol_count + t.alias_count + 2 {
                    sym = 0xFFF0; // nothing in between; still cover the builtin symbols
                }
                if self.lookup(st as StateId, sym as Symbol)
                    != self.scan_small(st as StateId, sym as Symbol)
                {
                    return (st, sym, built, false);
                }
                sym += 1;
            }
        }
        (0, 0, built, true)
    }

    /// The size of the derived small parse table in bytes.
    #[doc(hidden)]
    pub fn small_table_bytes(&self) -> usize {
        self.prepare();
        match self.derived.small.get() {
            Some(Some(t)) => 4 * (t.base.len() + t.comb.len()),
            _ => 0,
        }
    }

    /// Compares `field_id_for_name` with the C scan on every field name, each of its
    /// prefixes, and each with "x" or "\0" added; the first name that differs.
    #[doc(hidden)]
    pub fn field_id_mismatch(&self) -> Option<String> {
        for field in self.tables.field_names {
            for i in (0..=field.len()).filter(|&i| field.is_char_boundary(i)) {
                let prefix = &field[..i];
                for name in [
                    prefix.to_string(),
                    format!("{prefix}x"),
                    format!("{prefix}\0"),
                ] {
                    if self.field_id_for_name(&name) != self.field_id_for_name_scan(&name) {
                        return Some(name);
                    }
                }
            }
        }
        None
    }
}
