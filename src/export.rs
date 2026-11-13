//! gositter `export_test.go`: hooks into the runtime's internals for the integration tests
//! and the fuzz targets. Not part of the API.

use crate::subtree::{Subtree, SubtreeHeap, SubtreePool};
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

// The hooks below are used only by the subtree tests (src/subtree/tests_*.rs), so a non-test
// build sees them as dead code.

/// SummaryCheck: what summaries_ok went over. `bad` is "" or what differed first.
#[allow(dead_code)]
#[doc(hidden)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SummaryCheck {
    pub nodes: usize,
    pub two_heap: usize,
    pub column: usize,
    pub resumed: usize,
    pub bad: String,
}

/// summarizeChildrenRef: ts_subtree_summarize_children on the ts_subtree_* accessors, a
/// line of C to a line: the reference for summarize, summarize_two_heap and both loops of
/// summarize_children. The kind mask and F_HAS_REPEAT are not in C.
#[allow(dead_code)]
#[doc(hidden)]
pub(crate) fn summarize_children_ref(_sp: &mut SubtreePool, _s: Subtree, _lang: &Language) {
    todo!("gositter-rs-2f0.8")
}

/// summarized: puts fresh in the slot of s, summarizes it with f and puts the node back.
#[allow(dead_code)]
#[doc(hidden)]
pub(crate) fn summarized(
    _sp: &mut SubtreePool,
    _s: Subtree,
    _fresh: SubtreeHeap,
    _lang: &Language,
    _f: fn(&mut SubtreePool, Subtree, &Language),
) -> SubtreeHeap {
    todo!("gositter-rs-2f0.8")
}

/// checkSummary: compares, for one node with children, against summarize_children_ref: the
/// summary as it stands, what summarize makes of the node as new_node leaves it, and what it
/// makes of an ERROR node over the same children, from the start and resumed after some of
/// them.
#[allow(dead_code)]
#[doc(hidden)]
pub(crate) fn check_summary(
    _sp: &mut SubtreePool,
    _s: Subtree,
    _lang: &Language,
    _c: &mut SummaryCheck,
) {
    todo!("gositter-rs-2f0.8")
}
