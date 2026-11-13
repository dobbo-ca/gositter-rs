//! Node, TreeCursor and Tree over trees built with `new_leaf`/`new_node` (plan Task 10 Step 1;
//! there is no parser yet). Expected values follow node.go and the summary rules of
//! subtree.go: a hidden node is transparent, and its visible children count in its parent.

use crate::length::Length;
use crate::subtree::{Subtree, SubtreeArray, SubtreePool};
use crate::test_language::{TS_ANON, TS_HIDDEN, TS_IDENT, TS_RULE, test_language};
use crate::{Language, Node, Point, Tree};

/// Go's `length{bytes, Point{row, column}}`.
fn len(bytes: u32, row: u32, column: u32) -> Length {
    Length {
        bytes,
        extent: Point { row, column },
    }
}

fn pt(row: u32, column: u32) -> Point {
    Point { row, column }
}

fn new_pool() -> SubtreePool {
    let mut sp = SubtreePool::default();
    sp.reset();
    sp
}

fn leaf(
    sp: &mut SubtreePool,
    symbol: u16,
    padding: Length,
    size: Length,
    lang: &Language,
) -> Subtree {
    sp.new_leaf(symbol, padding, size, 1, 0, false, false, false, lang)
}

fn node(sp: &mut SubtreePool, symbol: u16, children: &[Subtree], lang: &Language) -> Subtree {
    let mut arr = SubtreeArray::default();
    for &c in children {
        sp.array_push(&mut arr, c);
    }
    sp.new_node(symbol, &mut arr, 0, lang)
}

/// The source `SRC` as the parser would leave it:
///
/// ```text
/// rule                 bytes 0..9, (0,0)-(1,3)
///   a      "a"         padding 0          size 1 (0,1)  -> bytes 0..1, (0,0)-(0,1)
///   ident  "foo"       padding " " (0,1)  size 3 (0,3)  -> bytes 2..5, (0,2)-(0,5)
///   _hidden            padding "\n" (1,0) size 3 (0,3)  -> bytes 6..9, (1,0)-(1,3)
///     ident "bar"      padding "\n" (1,0) size 3 (0,3)  -> bytes 6..9, (1,0)-(1,3)
/// ```
///
/// The rule's size is summarize's sum: size(a) + total(foo) + total(_hidden) =
/// (1,(0,1)) + (4,(0,4)) + (4,(1,3)) = (9,(1,3)) (length_add: a row step resets the
/// column to the right operand's). Its padding is a's, 0, so it starts at (0,0).
const SRC: &[u8] = b"a foo\nbar";

fn built_tree(lang: &'static Language) -> Tree {
    let mut sp = new_pool();
    let a = leaf(&mut sp, TS_ANON, len(0, 0, 0), len(1, 0, 1), lang);
    let foo = leaf(&mut sp, TS_IDENT, len(1, 0, 1), len(3, 0, 3), lang);
    let bar = leaf(&mut sp, TS_IDENT, len(1, 1, 0), len(3, 0, 3), lang);
    let hidden = node(&mut sp, TS_HIDDEN, &[bar], lang);
    let root = node(&mut sp, TS_RULE, &[a, foo, hidden], lang);
    Tree::from_pool(sp, lang, root)
}

#[test]
fn node_api_built_tree() {
    let lang = test_language();
    let tree = built_tree(lang);
    let root = tree.root_node();
    assert!(std::ptr::eq(tree.language(), lang));
    assert!(std::ptr::eq(root.language(), lang));
    // RootNode: nodeNew(tree, root slot, padding(root), 0), the same Node on every call.
    assert!(tree.root_node() == root);

    assert_eq!(root.kind(), "rule");
    assert_eq!(root.symbol(), TS_RULE);
    assert!(root.is_named() && !root.is_error() && !root.is_missing() && !root.is_extra());
    assert!(!root.has_error()); // error_cost 0: no ERROR or MISSING below
    assert_eq!((root.start_byte(), root.end_byte()), (0, 9));
    assert_eq!(root.start_position(), pt(0, 0));
    assert_eq!(root.end_position(), pt(1, 3));

    // visible_child_count: a (visible) + foo (visible) + _hidden's own visible_child_count
    // (1, bar), as _hidden is neither visible nor aliased. named_child_count: foo + bar; a
    // is not named.
    assert_eq!(root.child_count(), 3);
    assert_eq!(root.named_child_count(), 2);

    let a = root.child(0).unwrap();
    let foo = root.child(1).unwrap();
    // ts_node__child descends into _hidden for index 2: 2 - 2 < its 1 visible child.
    let bar = root.child(2).unwrap();
    assert_eq!(a.kind(), "a");
    assert!(!a.is_named());
    assert_eq!((a.start_byte(), a.end_byte()), (0, 1));
    assert_eq!((a.start_position(), a.end_position()), (pt(0, 0), pt(0, 1)));
    assert_eq!(a.utf8_text(SRC), b"a");
    // foo: position 1 after a, plus its padding (1,(0,1)) -> 2 at (0,2); +3 -> 5 at (0,5).
    assert_eq!(foo.kind(), "ident");
    assert!(foo.is_named());
    assert_eq!((foo.start_byte(), foo.end_byte()), (2, 5));
    assert_eq!(
        (foo.start_position(), foo.end_position()),
        (pt(0, 2), pt(0, 5))
    );
    assert_eq!(foo.utf8_text(SRC), b"foo");
    // bar: _hidden starts at 5 + its padding (1,(1,0)) = 6 at (1,0); bar is _hidden's first
    // child, so the iterator adds no padding again.
    assert_eq!(bar.kind(), "ident");
    assert_eq!(bar.symbol(), TS_IDENT);
    assert_eq!((bar.start_byte(), bar.end_byte()), (6, 9));
    assert_eq!(
        (bar.start_position(), bar.end_position()),
        (pt(1, 0), pt(1, 3))
    );
    assert_eq!(bar.utf8_text(SRC), b"bar");
    assert_eq!(bar.child_count(), 0);
    assert!(bar.child(0).is_none());
    assert!(root.child(3).is_none());

    // Named: a is skipped, so named_child(0) is foo and named_child(1) the ident inside
    // _hidden; the same Node values (slot, position, alias 0) as child(1) and child(2).
    assert!(root.named_child(0) == Some(foo));
    assert!(root.named_child(1) == Some(bar));
    assert!(root.named_child(2).is_none());

    // ts_node_parent: childWithDescendant(root, bar) passes a and foo (they end before 9),
    // enters _hidden (not relevant) and finds bar there, so bar's parent is the root.
    assert!(bar.parent() == Some(root));
    assert!(foo.parent() == Some(root));
    assert!(a.parent() == Some(root));
    assert!(root.parent().is_none());

    // equals (same tree and slot) and == (also x and y) agree on a node read twice.
    let foo2 = root.child(1).unwrap();
    assert!(foo == foo2 && foo.equals(foo2));
    assert!(foo != bar && !foo.equals(bar));

    let mut buf = Vec::new();
    root.children(&mut buf);
    assert!(buf == [a, foo, bar]);
    buf.clear();
    root.named_children(&mut buf);
    assert!(buf == [foo, bar]);

    // ts_node_string leaves out the anonymous a and the hidden _hidden.
    assert_eq!(root.to_sexp(), "(rule (ident) (ident))");

    // The test language has no fields.
    assert!(root.child_by_field_name("key").is_none());
    assert!(root.field_name_for_child(1).is_none());

    // The references agree with child and named_child (no alias sequence here).
    for i in 0..4 {
        assert!(
            root.child_ref(i, true) == root.child(i),
            "child_ref({i}, true)"
        );
        assert!(
            root.child_ref(i, false) == root.named_child(i),
            "child_ref({i}, false)"
        );
    }
}

#[test]
fn cursor_built_tree() {
    let lang = test_language();
    let tree = built_tree(lang);
    let root = tree.root_node();
    let (a, foo, bar) = (
        root.child(0).unwrap(),
        root.child(1).unwrap(),
        root.child(2).unwrap(),
    );

    // preorder visits the visible nodes in document order; _hidden is not one of them.
    let mut want = Vec::new();
    root.preorder(|n| {
        want.push(n);
        true
    });
    assert!(want == [root, a, foo, bar]);

    // A depth-first walk with the cursor. Its goto_next_sibling from foo lands on _hidden
    // (TreeCursorStepHidden) and goes on to bar; goto_parent from bar skips _hidden (not
    // visible) and returns to the root.
    let mut c = root.walk();
    let mut got = vec![c.node()];
    let mut parents: Vec<Node<'_>> = Vec::new();
    'walk: loop {
        let here = c.node();
        if c.goto_first_child() {
            parents.push(here);
            got.push(c.node());
            assert_eq!(c.field_id(), 0);
            assert!(c.field_name().is_none());
            continue;
        }
        loop {
            if c.goto_next_sibling() {
                got.push(c.node());
                continue 'walk;
            }
            if !c.goto_parent() {
                break 'walk;
            }
            assert!(Some(c.node()) == parents.pop(), "goto_parent");
        }
    }
    assert!(got == want);
    assert!(parents.is_empty());
    // The walk ends at the root, where goto_parent is false.
    assert!(c.node() == root);
    assert!(!c.goto_parent());

    // reset makes foo the cursor's root (a one-entry stack): a leaf with no children, and
    // gotoNextSiblingInternal and goto_parent need a second entry, so it cannot leave foo.
    c.reset(foo);
    assert!(c.node() == foo);
    assert!(!c.goto_first_child() && !c.goto_next_sibling() && !c.goto_parent());
    assert!(c.node() == foo);
}

/// Review Focus 2: preorder, preorder_mask and children keep their own stack, so a chain
/// far deeper than a 512 KiB thread stack could recurse through is walked.
#[test]
fn preorder_deep_small_stack() {
    const DEPTH: u32 = 100_000;
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let lang = test_language();
            // Named before the tree is built: summarize records only kinds already named.
            let mask = lang.kind_mask(&["rule"]);
            let mut sp = new_pool();
            let mut top = leaf(&mut sp, TS_IDENT, len(0, 0, 0), len(1, 0, 1), lang);
            for _ in 0..DEPTH {
                top = node(&mut sp, TS_RULE, &[top], lang);
            }
            let tree = Tree::from_pool(sp, lang, top);
            let root = tree.root_node();

            // Every rule and the ident are visible: DEPTH + 1 nodes.
            let mut n = 0u32;
            root.preorder(|_| {
                n += 1;
                true
            });
            assert_eq!(n, DEPTH + 1);

            // Only the rules have the "rule" bit; the ident's kind is not in the mask.
            let mut m = 0u32;
            root.preorder_mask(mask, |nd| {
                assert_eq!(nd.kind(), "rule");
                m += 1;
                true
            });
            assert_eq!(m, DEPTH);

            let mut buf = Vec::new();
            root.children(&mut buf);
            assert_eq!(buf.len(), 1);
            assert_eq!(buf[0].kind(), "rule");
            assert_eq!((buf[0].start_byte(), buf[0].end_byte()), (0, 1));
        })
        .unwrap()
        .join()
        .unwrap();
}
