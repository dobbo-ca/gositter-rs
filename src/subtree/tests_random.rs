//! gositter `subtree_test.go`: the random tests.

use super::*;
use crate::export::{SummaryCheck, check_summary, summarize_children_ref};
use crate::test_language::{TABLES, TS_ANON, TS_END, TS_HIDDEN, TS_IDENT, TS_REPEAT, TS_RULE};
use crate::{LanguageTables, Point, SYMBOL_ERROR, SYMBOL_ERROR_REPEAT, U16s, le_bytes};

/// xorshift64 in place of Go's math/rand.
struct Rng(u64);

impl Rng {
    /// rand.New(rand.NewSource(seed)). The seed is mixed so that seed 0 does not stick at 0.
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// rand.Intn
    fn intn(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as u32
    }
}

/// newTestPool
fn new_test_pool() -> SubtreePool {
    let mut sp = SubtreePool::default();
    sp.reset();
    sp
}

/// The alias sequences of newRandomTrees: productions 1 and 2 alias a child.
static ALIAS_SEQUENCES: [u8; 24] =
    le_bytes::<12, 24>(&[0, 0, 0, 0, TS_IDENT, 0, TS_ANON, 0, 0, TS_HIDDEN, 0, 0]);

/// randomTrees: random subtrees of every kind summarize_children tells apart.
struct RandomTrees {
    sp: SubtreePool,
    lang: &'static Language,
    r: Rng,
    made: Vec<Subtree>,
}

impl RandomTrees {
    /// newRandomTrees: testLanguage with two productions that alias a child, and kinds named.
    fn new(seed: u64) -> RandomTrees {
        let lang: &'static Language = Box::leak(Box::new(Language::new(LanguageTables {
            max_alias_sequence_length: 4,
            alias_sequences: U16s::new(&ALIAS_SEQUENCES),
            ..TABLES
        })));
        lang.kind_mask(&["a", "ident", "rule", "ERROR"]);
        RandomTrees {
            sp: new_test_pool(),
            lang,
            r: Rng::new(seed),
            made: Vec::new(),
        }
    }

    /// randomTrees.leaf
    fn leaf(&mut self) -> Subtree {
        let lang = self.lang;
        let sym = [TS_ANON, TS_IDENT, TS_HIDDEN][self.r.intn(3) as usize];
        let mut padding = Length {
            bytes: self.r.intn(9),
            extent: Point {
                row: self.r.intn(3) / 2,
                column: self.r.intn(9),
            },
        };
        let n = self.r.intn(20);
        let size = Length {
            bytes: n,
            extent: Point { row: 0, column: n },
        };
        let state = self.r.intn(50) as StateId;
        let mut s = match self.r.intn(8) {
            0 => {
                // heap, over several rows
                let size = Length {
                    bytes: 300 + n,
                    extent: Point {
                        row: self.r.intn(3),
                        column: n,
                    },
                };
                let lookahead = self.r.intn(40);
                self.sp.new_leaf(
                    sym, padding, size, lookahead, state, false, false, false, lang,
                )
            }
            1 => {
                // external token; the scanner may have read the column
                let lookahead = self.r.intn(4);
                let column = self.r.intn(2) == 0;
                let s = self.sp.new_leaf(
                    sym, padding, size, lookahead, state, true, column, false, lang,
                );
                let change = self.r.intn(2) == 0;
                // C reads repeat_depth from the scanner state of such a leaf.
                let depth = self.r.intn(4) as u16;
                let h = self.sp.ptr_mut(s);
                if change {
                    h.flags |= F_HAS_EXTERNAL_SCANNER_STATE_CHANGE;
                }
                h.repeat_depth = depth;
                s
            }
            2 => {
                if self.r.intn(2) == 0 {
                    padding.extent.row = 20; // not inline
                }
                let lookahead = self.r.intn(4);
                self.sp.new_missing_leaf(sym, padding, lookahead, lang)
            }
            3 => {
                let scanned = self.r.intn(4);
                self.sp
                    .new_error('x' as i32, padding, size, scanned, state, lang)
            }
            4 => self.sp.new_leaf(
                TS_END,
                padding,
                Length::default(),
                0,
                state,
                false,
                false,
                false,
                lang,
            ),
            _ => {
                let lookahead = self.r.intn(4);
                self.sp.new_leaf(
                    sym, padding, size, lookahead, state, false, false, false, lang,
                )
            }
        };
        if self.r.intn(6) == 0 {
            self.sp.set_extra(&mut s, true);
        }
        s
    }

    /// randomTrees.child: a child for a new node: a leaf, a new node or one made before.
    fn child(&mut self, depth: u32) -> Subtree {
        let k = self.r.intn(10);
        if k < 2 && !self.made.is_empty() {
            let i = self.r.intn(self.made.len() as u32) as usize;
            let s = self.made[i];
            self.sp.retain(s);
            return s;
        }
        if k < 5 && depth > 0 {
            return self.node(depth - 1);
        }
        self.leaf()
    }

    /// randomTrees.node
    fn node(&mut self, depth: u32) -> Subtree {
        let lang = self.lang;
        let mut n = 1 + self.r.intn(5);
        if self.r.intn(10) == 0 {
            n = 20 + self.r.intn(60);
        }
        let sym = [
            TS_RULE,
            TS_RULE,
            TS_REPEAT,
            TS_REPEAT,
            TS_HIDDEN,
            SYMBOL_ERROR_REPEAT,
            SYMBOL_ERROR,
        ][self.r.intn(7) as usize];
        let mut arr = SubtreeArray::default();
        for i in 0..n {
            let c = if i == 0 && sym == TS_REPEAT && self.r.intn(2) == 0 {
                // a repeat node below a repeat node: a repeat depth
                let c = self.node(0);
                let h = self.sp.ptr_mut(c);
                h.symbol = TS_REPEAT;
                h.flags &= !(F_VISIBLE | F_NAMED);
                summarize_children_ref(&mut self.sp, c, lang);
                c
            } else {
                self.child(depth)
            };
            self.sp.array_push(&mut arr, c);
        }
        let s = if sym == SYMBOL_ERROR {
            let extra = self.r.intn(2) == 0;
            self.sp.new_error_node(&mut arr, extra, lang)
        } else {
            let production_id = if n <= 4 && sym != SYMBOL_ERROR_REPEAT {
                self.r.intn(4) % 3
            } else {
                0
            };
            let s = self.sp.new_node(sym, &mut arr, production_id, lang);
            // what ts_parser__reduce sets after ts_subtree_new_node
            let extra = self.r.intn(4) == 0;
            let fragile = self.r.intn(3) == 0;
            let state = if fragile {
                TREE_STATE_NONE
            } else {
                self.r.intn(50) as StateId
            };
            let precedence = self.r.intn(3) as i32 - 1;
            let h = self.sp.ptr_mut(s);
            if extra {
                h.flags |= F_EXTRA;
            }
            if fragile {
                h.flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
            }
            h.parse_state = state;
            h.dynamic_precedence += precedence;
            s
        };
        self.sp.retain(s); // made keeps one
        self.made.push(s);
        s
    }

    /// randomTrees.check
    fn check(&mut self, s: Subtree, c: &mut SummaryCheck) {
        check_summary(&mut self.sp, s, self.lang, c);
        if !c.bad.is_empty() {
            panic!(
                "{}:\n{}",
                c.bad,
                self.sp.string(s, 0, false, self.lang, true)
            );
        }
    }

    /// randomTrees.chain: a left-leaning chain of repeat nodes, with extras between its
    /// children and chains inside it.
    fn chain(&mut self, depth: u32) -> Subtree {
        let lang = self.lang;
        let mut arr = SubtreeArray::default();
        let a = self.chain_item(depth);
        self.sp.array_push(&mut arr, a);
        let b = self.chain_item(depth);
        self.sp.array_push(&mut arr, b);
        let mut rep = self.sp.new_node(TS_REPEAT, &mut arr, 0, lang);
        for _ in 0..depth {
            let mut arr = SubtreeArray::default();
            self.sp.array_push(&mut arr, rep);
            if self.r.intn(3) == 0 {
                let mut extra = self.leaf();
                self.sp.set_extra(&mut extra, true);
                self.sp.array_push(&mut arr, extra);
            }
            let item = self.chain_item(depth);
            self.sp.array_push(&mut arr, item);
            rep = self.sp.new_node(TS_REPEAT, &mut arr, 0, lang);
        }
        rep
    }

    /// The item closure of randomTrees.chain.
    fn chain_item(&mut self, depth: u32) -> Subtree {
        if depth > 8 && self.r.intn(8) == 0 {
            let d = self.r.intn(depth / 2);
            return self.chain(d);
        }
        if self.r.intn(4) == 0 {
            let mut arr = SubtreeArray::default();
            let a = self.leaf();
            self.sp.array_push(&mut arr, a);
            let b = self.leaf();
            self.sp.array_push(&mut arr, b);
            return self.sp.new_node(TS_RULE, &mut arr, 0, self.lang);
        }
        self.leaf()
    }
}

/// summarize, summarize_two_heap and both loops of summarize_children against
/// summarize_children_ref, on random nodes.
#[test]
fn summarize_random() {
    let mut c = SummaryCheck::default();
    for seed in 0..300 {
        let mut g = RandomTrees::new(seed);
        for _ in 0..40 {
            let s = g.node(3);
            g.check(s, &mut c);
        }
    }
    assert!(
        c.two_heap >= 1000 && c.column >= 1000 && c.resumed >= 1000,
        "too few cases: {c:?}"
    );
}

/// An ERROR node that takes the children of the one before it and some more, as in
/// ts_parser__recover_to_state, must come out as one summarized from its first child. A
/// summary left behind must not reach any other node.
#[test]
fn summarize_resume() {
    let mut c = SummaryCheck::default();
    for seed in 0..300 {
        let mut g = RandomTrees::new(seed);
        let lang = g.lang;
        let mut arr = SubtreeArray::default();
        let first = g.child(2);
        g.sp.array_push(&mut arr, first);
        let mut error_tree = g.sp.new_error_node(&mut arr, true, lang);
        for _ in 0..30 {
            let mut slice = SubtreeArray::default();
            let mut i = g.r.intn(4);
            while i > 0 {
                let child = g.child(2);
                g.sp.array_push(&mut slice, child);
                i -= 1;
            }
            let mut resume = *g.sp.ptr(error_tree);
            // children_of(error_tree) as a slab range
            g.sp.array_splice(&mut slice, 0, 0, resume.children, resume.child_count);
            for j in 0..resume.child_count {
                let child = g.sp.children[(slice.off + j) as usize];
                g.sp.retain(child);
            }
            g.sp.release(error_tree);
            match g.r.intn(8) {
                0 => {
                    // fewer children than the summary covers
                    if slice.size > 1 {
                        slice.size -= 1;
                        let child = g.sp.children[(slice.off + slice.size) as usize];
                        g.sp.release(child);
                    }
                }
                1 => {
                    // not an ERROR node
                    g.sp.resume = resume;
                    let other = g.sp.new_node(TS_RULE, &mut slice, 0, lang);
                    g.check(other, &mut c);
                    slice = SubtreeArray::default();
                    let oh = *g.sp.ptr(other);
                    g.sp.array_splice(&mut slice, 0, 0, oh.children, oh.child_count);
                    for j in 0..slice.size {
                        let child = g.sp.children[(slice.off + j) as usize];
                        g.sp.retain(child);
                    }
                    g.sp.release(other);
                    resume.child_count = 0;
                }
                _ => {}
            }
            g.sp.resume = resume;
            error_tree = g.sp.new_error_node(&mut slice, true, lang);
            assert!(
                g.sp.resume.child_count == 0,
                "summary to resume from left behind"
            );
            g.check(error_tree, &mut c);
        }
    }
}

/// compressRef: ts_subtree_compress as C has it, three summaries a turn, on
/// summarize_children_ref: the reference for compress. Like compress, it works on
/// `sp.tree_stack`.
fn compress_ref(sp: &mut SubtreePool, start: Subtree, count: u32, lang: &Language) {
    let initial_stack_size = sp.tree_stack.len();

    let mut tree = start;
    let symbol = sp.ptr(tree).symbol;
    for _ in 0..count {
        let th = *sp.ptr(tree);
        if th.ref_count > 1 || th.child_count < 2 {
            break;
        }

        let child = sp.children[th.children as usize];
        if child.is_inline() {
            break;
        }
        let ch = *sp.ptr(child);
        if ch.child_count < 2 || ch.ref_count > 1 || ch.symbol != symbol {
            break;
        }

        let grandchild = sp.children[ch.children as usize];
        if grandchild.is_inline() {
            break;
        }
        let gh = *sp.ptr(grandchild);
        if gh.child_count < 2 || gh.ref_count > 1 || gh.symbol != symbol {
            break;
        }

        let last = (gh.children + gh.child_count - 1) as usize;
        sp.children[th.children as usize] = grandchild;
        sp.children[ch.children as usize] = sp.children[last];
        sp.children[last] = child;
        sp.tree_stack.push(tree);
        tree = grandchild;
    }

    while sp.tree_stack.len() > initial_stack_size {
        let tree = sp.tree_stack.pop().unwrap();
        let child = sp.children[sp.ptr(tree).children as usize];
        let ch = *sp.ptr(child);
        let grandchild = sp.children[(ch.children + ch.child_count - 1) as usize];
        summarize_children_ref(sp, grandchild, lang);
        summarize_children_ref(sp, child, lang);
        summarize_children_ref(sp, tree, lang);
    }
}

/// balanceRef: ts_parser__balance_subtree as C has it, on compress_ref.
fn balance_ref(sp: &mut SubtreePool, root: Subtree, lang: &Language) {
    sp.tree_stack.clear();
    if sp.child_count(root) > 0 && sp.ptr(root).ref_count == 1 {
        sp.tree_stack.push(root);
    }
    while let Some(&tree) = sp.tree_stack.last() {
        let h = *sp.ptr(tree);
        if h.repeat_depth > 0 {
            let child1 = sp.children[h.children as usize];
            let child2 = sp.children[(h.children + h.child_count - 1) as usize];
            let repeat_delta = sp.repeat_depth(child1) as i64 - sp.repeat_depth(child2) as i64;
            if repeat_delta > 0 {
                // C also counts n down by i; nothing reads it.
                let mut i = repeat_delta as u32 / 2;
                while i > 0 {
                    compress_ref(sp, tree, i, lang);
                    i /= 2;
                }
            }
        }
        sp.tree_stack.pop();
        let h = *sp.ptr(tree);
        for k in h.children..h.children + h.child_count {
            let child = sp.children[k as usize];
            if child.is_inline() {
                continue;
            }
            let ch = sp.ptr(child);
            if ch.child_count > 0 && ch.ref_count == 1 {
                sp.tree_stack.push(child);
            }
        }
    }
}

/// Parser.balanceSubtree (parser.go:1657) on a bare pool, until the parser (Task 11) has
/// it: balance_ref with compress, and only children with F_HAS_REPEAT pushed.
fn balance_subtree(sp: &mut SubtreePool, finished_tree: Subtree, lang: &Language) {
    sp.tree_stack.clear();
    if sp.child_count(finished_tree) > 0 && sp.ptr(finished_tree).ref_count == 1 {
        sp.tree_stack.push(finished_tree);
    }
    while let Some(&tree) = sp.tree_stack.last() {
        let h = *sp.ptr(tree);
        if h.repeat_depth > 0 {
            let child1 = sp.children[h.children as usize];
            let child2 = sp.children[(h.children + h.child_count - 1) as usize];
            let repeat_delta = sp.repeat_depth(child1) as i64 - sp.repeat_depth(child2) as i64;
            if repeat_delta > 0 {
                let mut i = repeat_delta as u32 / 2;
                while i > 0 {
                    sp.compress(tree, i, lang);
                    i /= 2;
                }
            }
        }
        sp.tree_stack.pop();
        let h = *sp.ptr(tree);
        for k in h.children..h.children + h.child_count {
            let child = sp.children[k as usize];
            if child.is_inline() {
                continue;
            }
            let ch = sp.ptr(child);
            if (ch.flags & F_HAS_REPEAT) != 0 && ch.ref_count == 1 {
                sp.tree_stack.push(child);
            }
        }
    }
}

/// compress, which leaves out a summary C makes, against compress_ref: the same chains
/// compressed by both must be the same in every field. Half of them are balanced, as the
/// parser does it. The other half get one call: the passes of balancing summarize most
/// nodes again and hide what a call leaves behind.
#[test]
fn compress_random() {
    let mut column = 0;
    for seed in 0..2000u64 {
        let build = || {
            let mut g = RandomTrees::new(seed);
            let depth = 5 + g.r.intn(120);
            let top = g.chain(depth);
            let mut arr = SubtreeArray::default();
            g.sp.array_push(&mut arr, top);
            let last = g.leaf();
            g.sp.array_push(&mut arr, last);
            let root = g.sp.new_node(TS_RULE, &mut arr, 0, g.lang);
            let count = 1 + g.r.intn(depth);
            (g, top, root, count)
        };
        let (mut p, top, root, count) = build();
        let (mut q, _, _, _) = build();
        let lang = p.lang;
        let (a, b) = (&mut p.sp, &mut q.sp);
        if seed % 2 == 0 {
            balance_subtree(a, root, lang);
            balance_ref(b, root, q.lang);
        } else {
            a.compress(top, count, lang);
            compress_ref(b, top, count, q.lang);
        }

        assert!(
            a.heap_len == b.heap_len && a.children.len() == b.children.len(),
            "seed {seed}: slabs differ in length"
        );
        for i in 1..a.heap_len {
            let s = Subtree((i as u64) << 1);
            assert_eq!(a.ptr(s), b.ptr(s), "seed {seed}: node {i}");
            let h = a.ptr(s);
            if (h.flags & F_DEPENDS_ON_COLUMN) != 0 && h.repeat_depth > 0 {
                column += 1;
            }
        }
        for (i, (x, y)) in a.children.iter().zip(&b.children).enumerate() {
            assert!(x == y, "seed {seed}: children slabs differ at {i}");
        }
    }
    assert!(
        column >= 1000,
        "{column} repeat nodes that depend on the column: too few"
    );
}
