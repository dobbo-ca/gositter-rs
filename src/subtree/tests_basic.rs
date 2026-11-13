//! gositter `subtree_test.go`: the basic tests.

use super::*;
use crate::test_language::{TS_ANON, TS_END, TS_HIDDEN, TS_IDENT, TS_RULE, test_language};
use crate::{Point, SYMBOL_ERROR};

/// newTestPool
fn new_test_pool() -> SubtreePool {
    let mut sp = SubtreePool::default();
    sp.reset();
    sp
}

/// Go's `length{bytes, Point{row, column}}`.
fn len(bytes: u32, row: u32, column: u32) -> Length {
    Length {
        bytes,
        extent: Point { row, column },
    }
}

/// TestSubtreeInlineLayout
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_inline_layout() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let padding = len(7, 3, 5);
    let size = len(9, 0, 9);
    let s = sp.new_leaf(
        TS_IDENT, padding, size, 11, 0x1234, false, false, true, lang,
    );
    assert!(s.is_inline(), "want inline");
    // The C bit layout of SubtreeInlineData, little-endian.
    let want = Subtree(
        1 | (1 << 1) | (1 << 2) | (1 << 6) // is_inline, visible, named, is_keyword
            | (2 << 8)
            | (0x1234 << 16)
            | (5 << 32)
            | (3 << 40)
            | (11 << 44)
            | (7 << 48)
            | (9 << 56),
    );
    assert_eq!(s, want, "bits = {:#x}, want {:#x}", s.0, want.0);
    assert!(
        sp.symbol(s) == TS_IDENT
            && sp.parse_state(s) == 0x1234
            && sp.padding(s) == padding
            && sp.size(s) == size
            && sp.lookahead_bytes(s) == 11
            && sp.is_keyword(s)
            && sp.visible(s)
            && sp.named(s)
            && !sp.extra(s)
            && sp.total_bytes(s) == 16,
        "accessor mismatch"
    );
    assert_eq!(sp.heap_len, 1, "inline leaf allocated");
}

/// TestSubtreeCanInline
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_can_inline() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let ok = len(1, 0, 1);
    // (name, symbol, padding, size, lookahead, external, inline)
    let cases = [
        ("small", TS_ANON, ok, ok, 1, false, true),
        (
            "padding 254",
            TS_ANON,
            len(254, 15, 254),
            ok,
            15,
            false,
            true,
        ),
        (
            "padding bytes 255",
            TS_ANON,
            len(255, 0, 1),
            ok,
            1,
            false,
            false,
        ),
        (
            "padding rows 16",
            TS_ANON,
            len(20, 16, 0),
            ok,
            1,
            false,
            false,
        ),
        (
            "padding columns 255",
            TS_ANON,
            len(20, 0, 255),
            ok,
            1,
            false,
            false,
        ),
        ("size 255", TS_ANON, ok, len(255, 0, 255), 1, false, false),
        (
            "multi-line size",
            TS_ANON,
            ok,
            len(3, 1, 0),
            1,
            false,
            false,
        ),
        ("lookahead 16", TS_ANON, ok, ok, 16, false, false),
        ("external token", TS_ANON, ok, ok, 1, true, false),
        ("error symbol", SYMBOL_ERROR, ok, ok, 1, false, false),
    ];
    for (name, symbol, padding, size, lookahead, external, inline) in cases {
        let s = sp.new_leaf(
            symbol, padding, size, lookahead, 1, external, false, false, lang,
        );
        assert_eq!(s.is_inline(), inline, "{name}: inline");
        assert!(
            sp.padding(s) == padding && sp.size(s) == size && sp.lookahead_bytes(s) == lookahead,
            "{name}: round trip failed"
        );
    }
    // The end symbol is extra.
    let eof = sp.new_leaf(
        TS_END,
        ok,
        Length::default(),
        1,
        1,
        false,
        false,
        false,
        lang,
    );
    assert!(sp.extra(eof), "eof leaf is not extra");
}

/// TestSubtreeMissingAndErrorCost
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_missing_and_error_cost() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let m = sp.new_missing_leaf(TS_ANON, Length::default(), 3, lang);
    assert!(
        m.is_inline()
            && sp.missing(m)
            && sp.error_cost(m) == ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY,
        "missing leaf: inline={} cost={}",
        m.is_inline(),
        sp.error_cost(m)
    );
    let e = sp.new_error('x' as i32, Length::default(), len(2, 0, 2), 1, 0, lang);
    let h = sp.ptr(e);
    assert!(
        !e.is_inline()
            && h.lookahead_char == 'x' as i32
            && sp.fragile_left(e)
            && sp.fragile_right(e),
        "error leaf fields"
    );
    let mut arr = SubtreeArray::default();
    sp.array_push(&mut arr, e);
    let node = sp.new_error_node(&mut arr, false, lang);
    // ERROR_COST_PER_RECOVERY + chars; an error leaf child adds no tree cost.
    assert_eq!(
        sp.error_cost(node),
        ERROR_COST_PER_RECOVERY + 2,
        "error node cost"
    );
    assert_eq!(
        sp.string(node, 0, false, lang, false),
        "(ERROR (UNEXPECTED 'x'))"
    );
}

/// TestSubtreeNewNodeSummary
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_new_node_summary() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let a = sp.new_leaf(
        TS_ANON,
        len(2, 0, 2),
        len(1, 0, 1),
        1,
        1,
        false,
        false,
        false,
        lang,
    );
    let id = sp.new_leaf(
        TS_IDENT,
        len(1, 1, 0),
        len(3, 0, 3),
        2,
        1,
        false,
        false,
        false,
        lang,
    );
    let hid = sp.new_leaf(
        TS_HIDDEN,
        Length::default(),
        len(1, 0, 1),
        1,
        1,
        false,
        false,
        false,
        lang,
    );

    let mut arr = SubtreeArray::default();
    for c in [a, id, hid] {
        sp.array_push(&mut arr, c);
    }
    let node = sp.new_node(TS_RULE, &mut arr, 0, lang);
    let h = *sp.ptr(node);
    assert_eq!(h.padding, len(2, 0, 2), "padding");
    // a(1) + [newline, ident(3)] + hidden(1): the size crosses one row.
    assert_eq!(h.size, len(6, 1, 4), "size");
    assert!(
        h.child_count == 3
            && h.visible_child_count == 2
            && h.named_child_count == 1
            && h.visible_descendant_count == 2,
        "counts = {} {} {} {}",
        h.child_count,
        h.visible_child_count,
        h.named_child_count,
        h.visible_descendant_count
    );
    assert!(
        h.first_leaf_symbol == TS_ANON && h.ref_count == 1 && h.lookahead_bytes == 1,
        "firstLeaf={} refCount={} lookahead={}",
        h.first_leaf_symbol,
        h.ref_count,
        h.lookahead_bytes
    );
    assert_eq!(sp.string(node, 0, false, lang, false), "(rule (ident))");
    // The unused array capacity went back to the slab.
    assert_eq!(
        sp.children.len() as u32,
        h.children + h.child_count,
        "slab not trimmed"
    );
}

/// TestSubtreeMakeMutAndRelease
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_make_mut_and_release() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let big = len(300, 0, 300);
    let leaf = sp.new_leaf(
        TS_IDENT,
        Length::default(),
        big,
        1,
        1,
        false,
        false,
        false,
        lang,
    );
    let mut arr = SubtreeArray::default();
    sp.array_push(&mut arr, leaf);
    let node = sp.new_node(TS_RULE, &mut arr, 0, lang);

    // One owner: mutable in place.
    assert_eq!(
        sp.make_mut(node),
        node,
        "make_mut cloned a subtree with one owner"
    );

    // Two owners: a clone, the original loses one reference, and the shared
    // child gains one.
    sp.retain(node);
    let clone = sp.make_mut(node);
    assert_ne!(clone, node, "make_mut did not clone a shared subtree");
    assert!(
        sp.ptr(node).ref_count == 1 && sp.ptr(clone).ref_count == 1 && sp.ptr(leaf).ref_count == 2,
        "refcounts: node={} clone={} leaf={}",
        sp.ptr(node).ref_count,
        sp.ptr(clone).ref_count,
        sp.ptr(leaf).ref_count
    );
    assert_ne!(
        sp.ptr(clone).children,
        sp.ptr(node).children,
        "clone shares the children region"
    );
    assert_eq!(sp.compare(node, clone), 0, "clone compares unequal");

    // Releasing both frees node, clone and then the leaf.
    sp.release(node);
    assert!(
        sp.ptr(leaf).ref_count == 1 && sp.free_trees.len() == 1,
        "after first release: leaf={} free={}",
        sp.ptr(leaf).ref_count,
        sp.free_trees.len()
    );
    sp.release(clone);
    assert_eq!(sp.free_trees.len(), 3, "free list");
    // A freed index is reused.
    let before = sp.heap_len;
    sp.new_leaf(
        TS_IDENT,
        Length::default(),
        big,
        1,
        1,
        false,
        false,
        false,
        lang,
    );
    assert!(
        sp.heap_len == before && sp.free_trees.len() == 2,
        "free index not reused"
    );
}

/// TestSubtreeCompare: leaf
fn compare_leaf(sp: &mut SubtreePool, lang: &Language, sym: Symbol) -> Subtree {
    sp.new_leaf(
        sym,
        Length::default(),
        len(1, 0, 1),
        1,
        1,
        false,
        false,
        false,
        lang,
    )
}

/// TestSubtreeCompare: node
fn compare_node(sp: &mut SubtreePool, lang: &Language, children: &[Subtree]) -> Subtree {
    let mut arr = SubtreeArray::default();
    for &c in children {
        sp.array_push(&mut arr, c);
    }
    sp.new_node(TS_RULE, &mut arr, 0, lang)
}

/// TestSubtreeCompare
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_compare() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let (la, li) = (
        compare_leaf(&mut sp, lang, TS_ANON),
        compare_leaf(&mut sp, lang, TS_IDENT),
    );
    let a = compare_node(&mut sp, lang, &[la, li]);
    let (lb, lh) = (
        compare_leaf(&mut sp, lang, TS_ANON),
        compare_leaf(&mut sp, lang, TS_HIDDEN),
    );
    let b = compare_node(&mut sp, lang, &[lb, lh]);
    assert!(
        sp.compare(a, b) == -1 && sp.compare(b, a) == 1 && sp.compare(a, a) == 0,
        "compare by child symbol"
    );
    let lc = compare_leaf(&mut sp, lang, TS_ANON);
    let c = compare_node(&mut sp, lang, &[lc]);
    assert_eq!(sp.compare(c, a), -1, "compare by child count");
    assert!(
        sp.tree_stack.is_empty(),
        "tree stack not empty after compare"
    );
}

/// TestSubtreeArraySplice
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_array_splice() {
    let mut sp = new_test_pool();
    let mut arr = SubtreeArray::default();
    for i in 1..=4u64 {
        sp.array_push(&mut arr, Subtree((i << 8) | 1));
    }
    // Another region behind it, so growth has to move the array.
    let mut other = SubtreeArray::default();
    sp.array_push(&mut other, Subtree((99 << 8) | 1));

    // array_splice takes a region of the slab, not a slice: push ins into its own array.
    let mut ins = SubtreeArray::default();
    for i in 10..=15u64 {
        sp.array_push(&mut ins, Subtree((i << 8) | 1));
    }
    sp.array_splice(&mut arr, 1, 1, ins.off, ins.size);
    let want: [u64; 9] = [1, 10, 11, 12, 13, 14, 15, 3, 4];
    let got = sp.array(arr);
    assert_eq!(got.len(), want.len(), "len");
    for (i, (g, &w)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(g.0 >> 8, w, "elem {i}");
    }
    assert_eq!(sp.array(other)[0].0 >> 8, 99, "neighbour overwritten");
}

/// TestSubtreeRepeatDepthOfLeaves
///
/// The union quirk: C reads repeat_depth from an external token's inline
/// scanner state. (*Parser).lex stores those two bytes in repeatDepth.
#[test]
#[ignore = "gositter-rs-2f0.8"]
fn subtree_repeat_depth_of_leaves() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let inline = sp.new_leaf(
        TS_ANON,
        Length::default(),
        len(1, 0, 1),
        1,
        1,
        false,
        false,
        false,
        lang,
    );
    let ext = sp.new_leaf(
        TS_ANON,
        Length::default(),
        len(1, 0, 1),
        1,
        1,
        true,
        false,
        false,
        lang,
    );
    assert!(
        sp.repeat_depth(inline) == 0 && sp.repeat_depth(ext) == 0,
        "fresh leaves have a repeat depth"
    );
    // Go checks for nil; a Rust slice has no nil, so empty stands in for it.
    assert!(
        sp.external_scanner_state(ext).is_empty()
            && sp.external_scanner_state_eq(ext, NULL_SUBTREE),
        "empty scanner state must equal the state of NULL"
    );
    // The state range shares the child count fields of the union.
    sp.ext.extend_from_slice(&[1, 2, 3]);
    sp.ptr_mut(ext).set_ext(1, 2);
    assert!(
        sp.external_scanner_state(ext) == b"\x02\x03" && sp.visible_child_count(ext) == 0,
        "scanner state range of a leaf"
    );
}
