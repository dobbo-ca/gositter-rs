//! gositter `export_test.go`: hooks into the runtime's internals for the integration tests
//! and the fuzz targets. Not part of the API.

use crate::length::length_add;
use crate::subtree::{
    ERROR_COST_PER_RECOVERY, ERROR_COST_PER_SKIPPED_CHAR, ERROR_COST_PER_SKIPPED_LINE,
    ERROR_COST_PER_SKIPPED_TREE, F_DEPENDS_ON_COLUMN, F_FRAGILE_LEFT, F_FRAGILE_RIGHT,
    F_HAS_EXTERNAL_SCANNER_STATE_CHANGE, F_HAS_EXTERNAL_TOKENS, F_HAS_REPEAT, F_NAMED, F_VISIBLE,
    Subtree, SubtreeHeap, SubtreePool, TREE_STATE_NONE,
};
use crate::{
    ExternalScanner, FieldId, Language, Node, Parser, SYMBOL_ERROR, SYMBOL_ERROR_REPEAT, StateId,
    Symbol,
};

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
pub struct SummaryCheck {
    pub nodes: usize,
    pub two_heap: usize,
    pub column: usize,
    pub resumed: usize,
    pub bad: String,
}

/// summarizeChildrenRef: ts_subtree_summarize_children on the ts_subtree_* accessors, a
/// line of C to a line: the reference for summarize, summarize_two_heap and both loops of
/// summarize_children. The kind mask and F_HAS_REPEAT are not in C. Go writes through the
/// heap pointer; here the node is copied out and written back at the end (no child is the
/// node itself, so no read sees the difference).
#[allow(dead_code)]
#[doc(hidden)]
pub(crate) fn summarize_children_ref(sp: &mut SubtreePool, s: Subtree, lang: &Language) {
    let mut h = *sp.ptr(s);
    h.named_child_count = 0;
    h.visible_child_count = 0;
    h.error_cost = 0;
    h.repeat_depth = 0;
    h.visible_descendant_count = 0;
    h.flags &= !(F_HAS_EXTERNAL_TOKENS
        | F_DEPENDS_ON_COLUMN
        | F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
        | F_HAS_REPEAT);
    h.dynamic_precedence = 0;
    let mut kinds = 0u32;

    let mut structural_index = 0usize;
    let alias_sequence = lang.alias_sequence(u32::from(h.production_id));
    let mut lookahead_end_byte = 0u32;
    let self_is_error = h.symbol == SYMBOL_ERROR || h.symbol == SYMBOL_ERROR_REPEAT;

    let children: Vec<Subtree> =
        sp.children[h.children as usize..(h.children + h.child_count) as usize].to_vec();
    for (i, &child) in children.iter().enumerate() {
        if h.size.extent.row == 0 && sp.depends_on_column(child) {
            h.flags |= F_DEPENDS_ON_COLUMN;
        }
        if sp.has_external_scanner_state_change(child) {
            h.flags |= F_HAS_EXTERNAL_SCANNER_STATE_CHANGE;
        }

        if i == 0 {
            h.padding = sp.padding(child);
            h.size = sp.size(child);
        } else {
            h.size = length_add(h.size, sp.total_size(child));
        }

        let child_lookahead_end_byte = h
            .padding
            .bytes
            .wrapping_add(h.size.bytes)
            .wrapping_add(sp.lookahead_bytes(child));
        if child_lookahead_end_byte > lookahead_end_byte {
            lookahead_end_byte = child_lookahead_end_byte;
        }

        if sp.symbol(child) != SYMBOL_ERROR_REPEAT {
            h.error_cost = h.error_cost.wrapping_add(sp.error_cost(child));
        }

        let grandchild_count = sp.child_count(child);
        if self_is_error && !sp.extra(child) && !(sp.is_error(child) && grandchild_count == 0) {
            if sp.visible(child) {
                h.error_cost = h.error_cost.wrapping_add(ERROR_COST_PER_SKIPPED_TREE);
            } else if grandchild_count > 0 {
                h.error_cost = h.error_cost.wrapping_add(
                    ERROR_COST_PER_SKIPPED_TREE.wrapping_mul(sp.ptr(child).visible_child_count),
                );
            }
        }

        h.dynamic_precedence = h
            .dynamic_precedence
            .wrapping_add(sp.dynamic_precedence(child));
        h.visible_descendant_count = h
            .visible_descendant_count
            .wrapping_add(sp.visible_descendant_count(child));
        kinds |= lang.kind_bit(sp.symbol(child));
        if grandchild_count > 0 {
            kinds |= sp.ptr(child).kinds();
        }
        if !child.is_inline() {
            h.flags |= sp.ptr(child).flags & F_HAS_REPEAT;
        }

        let alias = match alias_sequence {
            Some(start) => lang.tables.alias_sequences.get(start + structural_index),
            None => 0,
        };
        if alias_sequence.is_some() && alias != 0 && !sp.extra(child) {
            kinds |= lang.kind_bit(alias);
            h.visible_descendant_count = h.visible_descendant_count.wrapping_add(1);
            h.visible_child_count = h.visible_child_count.wrapping_add(1);
            if lang.symbol_metadata(alias).named {
                h.named_child_count = h.named_child_count.wrapping_add(1);
            }
        } else if sp.visible(child) {
            h.visible_descendant_count = h.visible_descendant_count.wrapping_add(1);
            h.visible_child_count = h.visible_child_count.wrapping_add(1);
            if sp.named(child) {
                h.named_child_count = h.named_child_count.wrapping_add(1);
            }
        } else if grandchild_count > 0 {
            h.visible_child_count = h
                .visible_child_count
                .wrapping_add(sp.ptr(child).visible_child_count);
            h.named_child_count = h
                .named_child_count
                .wrapping_add(sp.ptr(child).named_child_count);
        }

        if sp.has_external_tokens(child) {
            h.flags |= F_HAS_EXTERNAL_TOKENS;
        }

        if sp.is_error(child) {
            h.flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
            h.parse_state = TREE_STATE_NONE;
        }

        if !sp.extra(child) {
            structural_index += 1;
        }
    }

    h.lookahead_bytes = lookahead_end_byte
        .wrapping_sub(h.size.bytes)
        .wrapping_sub(h.padding.bytes);

    if self_is_error {
        h.error_cost = h.error_cost.wrapping_add(
            ERROR_COST_PER_RECOVERY
                .wrapping_add(ERROR_COST_PER_SKIPPED_CHAR.wrapping_mul(h.size.bytes))
                .wrapping_add(ERROR_COST_PER_SKIPPED_LINE.wrapping_mul(h.size.extent.row)),
        );
    }

    if h.child_count > 0 {
        let first_child = children[0];
        let last_child = children[h.child_count as usize - 1];

        h.first_leaf_symbol = sp.leaf_symbol(first_child);
        h.first_leaf_parse_state = sp.leaf_parse_state(first_child);

        if sp.fragile_left(first_child) {
            h.flags |= F_FRAGILE_LEFT;
        }
        if sp.fragile_right(last_child) {
            h.flags |= F_FRAGILE_RIGHT;
        }

        if h.child_count >= 2
            && h.flags & (F_VISIBLE | F_NAMED) == 0
            && sp.symbol(first_child) == h.symbol
        {
            if sp.repeat_depth(first_child) > sp.repeat_depth(last_child) {
                h.repeat_depth = (sp.repeat_depth(first_child) + 1) as u16;
            } else {
                h.repeat_depth = (sp.repeat_depth(last_child) + 1) as u16;
            }
            h.flags |= F_HAS_REPEAT;
        }
    }
    h.set_kinds(kinds);
    *sp.ptr_mut(s) = h;
}

/// summarized: puts fresh in the slot of s, summarizes it with f and puts the node back.
#[allow(dead_code)]
#[doc(hidden)]
pub(crate) fn summarized(
    sp: &mut SubtreePool,
    s: Subtree,
    fresh: SubtreeHeap,
    lang: &Language,
    f: fn(&mut SubtreePool, Subtree, &Language),
) -> SubtreeHeap {
    let saved = *sp.ptr(s);
    *sp.ptr_mut(s) = fresh;
    f(sp, s, lang);
    let got = *sp.ptr(s);
    *sp.ptr_mut(s) = saved;
    got
}

/// checkSummary: compares, for one node with children, against summarize_children_ref: the
/// summary as it stands, what summarize makes of the node as new_node leaves it, and what it
/// makes of an ERROR node over the same children, from the start and resumed after some of
/// them.
#[allow(dead_code)]
#[doc(hidden)]
pub(crate) fn check_summary(
    sp: &mut SubtreePool,
    s: Subtree,
    lang: &Language,
    c: &mut SummaryCheck,
) {
    let h = *sp.ptr(s);
    let children: Vec<Subtree> = sp.children_of(s).to_vec();
    let mut fresh = SubtreeHeap {
        ref_count: 1,
        symbol: h.symbol,
        child_count: h.child_count,
        flags: h.flags & (F_VISIBLE | F_NAMED),
        production_id: h.production_id,
        children: h.children,
        child_class: h.child_class,
        ..SubtreeHeap::default()
    };
    let self_is_error = h.symbol == SYMBOL_ERROR || h.symbol == SYMBOL_ERROR_REPEAT;
    if self_is_error {
        fresh.flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
    }
    c.nodes += 1;
    if h.child_count == 2 && !children[0].is_inline() && !children[1].is_inline() {
        c.two_heap += 1;
    }
    if h.flags & F_DEPENDS_ON_COLUMN != 0 {
        c.column += 1;
    }

    let mut want = summarized(sp, s, fresh, lang, summarize_children_ref);
    let got = summarized(sp, s, fresh, lang, SubtreePool::summarize);
    if got != want {
        c.bad = "summarize differs from the reference".to_string();
        return;
    }

    // What the children alone decide. The parser sets the parse state, the fragile bits and
    // extra, and adds the precedence of the production, on any node but an ERROR node.
    let pure = |sp: &SubtreePool, mut x: SubtreeHeap| -> SubtreeHeap {
        x.ref_count = 0;
        let mut keep = F_HAS_EXTERNAL_TOKENS
            | F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
            | F_HAS_REPEAT
            | F_DEPENDS_ON_COLUMN;
        // Not the column bit where balancing may have been at work. C leaves it as it was
        // above a repeat node that compress changed, and compress summarizes a node over the
        // size it had before, which C tests for a first child that depends on the column.
        if h.flags & F_HAS_REPEAT != 0
            || x.flags & (F_VISIBLE | F_NAMED) == 0 && sp.depends_on_column(children[0])
        {
            keep &= !F_DEPENDS_ON_COLUMN;
        }
        x.flags &= keep;
        if x.symbol != SYMBOL_ERROR {
            x.parse_state = 0;
            x.dynamic_precedence = 0;
        }
        x
    };
    if pure(sp, h) != pure(sp, want) {
        c.bad = "summary in the tree differs from the reference".to_string();
        return;
    }

    let mut e = fresh;
    e.symbol = SYMBOL_ERROR;
    e.production_id = 0;
    e.flags = F_VISIBLE | F_NAMED | F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
    want = summarized(sp, s, e, lang, summarize_children_ref);
    let got = summarized(sp, s, e, lang, SubtreePool::summarize);
    if got != want {
        c.bad = "summarize of an ERROR node differs from the reference".to_string();
        return;
    }
    let n = e.child_count;
    for k in [1, n / 2, n.wrapping_sub(1), n] {
        if k == 0 {
            continue;
        }
        let mut r = e;
        r.child_count = k;
        sp.resume = summarized(sp, s, r, lang, summarize_children_ref);
        let got = summarized(sp, s, e, lang, SubtreePool::summarize);
        if got != want || sp.resume.child_count != 0 {
            c.bad = "resumed summary of an ERROR node differs from the reference".to_string();
            return;
        }
        c.resumed += 1;
    }
}

impl Parser {
    /// Capacity of the subtree children slab (gositter `ChildrenSlabCap`).
    #[doc(hidden)]
    pub fn children_slab_cap(&self) -> usize {
        todo!("gositter-rs-2f0.11")
    }

    /// Capacity of the stack node slab (gositter `StackSlabCap`).
    #[doc(hidden)]
    pub fn stack_slab_cap(&self) -> usize {
        todo!("gositter-rs-2f0.11")
    }

    /// Capacity of the stack link slab (gositter `LinkSlabCap`).
    #[doc(hidden)]
    pub fn link_slab_cap(&self) -> usize {
        todo!("gositter-rs-2f0.11")
    }

    /// Number of subtree heap chunks held (gositter `HeapChunks`).
    #[doc(hidden)]
    pub fn heap_chunks(&self) -> usize {
        todo!("gositter-rs-2f0.11")
    }

    /// Whether every node's repeat flag matches its children (gositter `RepeatFlagsOK`).
    #[doc(hidden)]
    pub fn repeat_flags_ok(&self) -> bool {
        todo!("gositter-rs-2f0.11")
    }

    /// (nodes checked, ok) for the one-child summaries (gositter `OneChildSummaryOK`).
    #[doc(hidden)]
    pub fn one_child_summary_ok(&self) -> (usize, bool) {
        todo!("gositter-rs-2f0.11")
    }

    /// (nodes checked, ok) for the error scans (gositter `ErrorScansOK`).
    #[doc(hidden)]
    pub fn error_scans_ok(&self) -> (usize, bool) {
        todo!("gositter-rs-2f0.11")
    }

    /// check_summary over every node of the last tree (gositter `SummariesOK`).
    #[doc(hidden)]
    pub fn summaries_ok(&self) -> SummaryCheck {
        todo!("gositter-rs-2f0.11")
    }

    /// Generation number of the kept scanner, bumped per scanner construction (gositter
    /// `KeptScannerID`); None when no scanner is kept.
    #[doc(hidden)]
    pub fn kept_scanner_id(&self) -> Option<u64> {
        todo!("gositter-rs-2f0.11")
    }

    /// The scanner the parser keeps for its next parse in the same language (gositter
    /// `KeptScanner`). A test upcasts it to `&dyn Any` and downcasts it to the grammar's
    /// scanner type.
    #[doc(hidden)]
    pub fn kept_scanner(&self) -> Option<&dyn ExternalScanner> {
        todo!("gositter-rs-2f0.11")
    }
}

impl<'t> Node<'t> {
    /// ChildRef (`export_test.go:268`): `child` as it was with one loop for every node, the
    /// reference for the loop of a node with no alias sequence.
    #[doc(hidden)]
    pub fn child_ref(self, _i: u32, _include_anonymous: bool) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }

    /// ChildByFieldIDRef (`export_test.go:186`): ts_node_child_by_field_id as first ported,
    /// on the child iterator; the reference for `child_by_field_id`.
    #[doc(hidden)]
    pub fn child_by_field_id_ref(self, _id: FieldId) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }
}
