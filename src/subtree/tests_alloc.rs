//! gositter `subtree_test.go`: the alloc tests.

use super::*;
use crate::test_language::{TS_ANON, TS_RULE, test_language};

/// newTestPool
fn new_test_pool() -> SubtreePool {
    let mut sp = SubtreePool::default();
    sp.reset();
    sp
}

/// TestChildClass
#[test]
fn child_class() {
    assert_eq!(
        std::mem::size_of::<SubtreeHeap>(),
        80,
        "subtreeHeap size, want 80"
    );
    let (mut prev_class, mut prev_size) = (0u32, 0u32);
    for n in [
        1u32,
        2,
        63,
        64,
        65,
        80,
        81,
        96,
        97,
        128,
        129,
        1000,
        1 << 20,
        (1 << 20) + 1,
        (1 << 31) - 1,
        1 << 31,
    ] {
        let (class, size) = super::child_class(n);
        assert!(
            size >= n && u64::from(size) * 4 <= u64::from(n) * 5 + 4,
            "childClass({n}): size {size}"
        );
        let (c, s) = super::child_class(size);
        assert!(
            c == class && s == size,
            "childClass({n}) = {class}, {size} but its size maps to {c}, {s}"
        );
        assert!(
            class >= prev_class
                && (class == prev_class) == (size == prev_size)
                && class < CHILD_CLASS_COUNT,
            "childClass({n}) = {class}, {size} after {prev_class}, {prev_size}"
        );
        (prev_class, prev_size) = (class, size);
    }
    let (class, _) = super::child_class(65);
    assert_eq!(class, EXACT_CHILD_CLASSES + 1, "first rounded class");
}

/// The `node` closure of TestChildrenBlockReuse.
fn node(sp: &mut SubtreePool, lang: &Language, leaf: Subtree, n: u32) -> Subtree {
    let mut arr = SubtreeArray::default();
    sp.array_reserve(&mut arr, n);
    for _ in 0..n {
        sp.array_push(&mut arr, leaf);
    }
    sp.new_node(TS_RULE, &mut arr, 0, lang)
}

/// TestChildrenBlockReuse. A released node's children block serves the next
/// request of its class, and an array that is copied one element longer each
/// time, the shape of ts_parser__recover_to_state, keeps the slab linear in its
/// length.
#[test]
fn children_block_reuse() {
    let (mut sp, lang) = (new_test_pool(), test_language());
    let leaf = Subtree((u64::from(TS_ANON) << INL_SYMBOL_SHIFT) | INL_IS_INLINE);
    let a = node(&mut sp, lang, leaf, 3);
    let b = node(&mut sp, lang, leaf, 5);
    let off = sp.ptr(a).children;
    sp.release(a);
    let c = node(&mut sp, lang, leaf, 3);
    assert_eq!(sp.ptr(c).children, off, "freed block not reused");
    sp.release(b);

    const ROUNDS: u32 = 5000;
    let mut error_tree = node(&mut sp, lang, leaf, 1);
    for _ in 0..ROUNDS {
        let mut slice = SubtreeArray::default();
        sp.array_reserve(&mut slice, 1);
        sp.array_push(&mut slice, leaf);
        let h = sp.ptr(error_tree);
        let (elements_off, elements_len) = (h.children, h.child_count);
        sp.array_splice(&mut slice, 0, 0, elements_off, elements_len);
        sp.release(error_tree);
        error_tree = sp.new_node(TS_RULE, &mut slice, 0, lang);
    }
    assert_eq!(sp.ptr(error_tree).child_count, ROUNDS + 1, "child count");
    assert!(
        sp.children.len() <= 40 * ROUNDS as usize,
        "children slab has {} entries for a node of {ROUNDS} children",
        sp.children.len()
    );
}

/// xorshift64 in place of Go's math/rand.
struct Rng(u64);

impl Rng {
    /// rand.Intn
    fn intn(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// The `block` type of TestChildrenAllocator.
#[derive(Clone, Copy)]
struct Block {
    a: SubtreeArray,
    tag: u64,
}

/// The `fill` closure of TestChildrenAllocator.
fn fill(sp: &mut SubtreePool, b: Block, from: u32) {
    let off = b.a.off as usize;
    for i in from..b.a.size {
        sp.children[off + i as usize] = Subtree(b.tag + u64::from(i));
    }
}

/// The `check` closure of TestChildrenAllocator.
fn check(sp: &SubtreePool, b: Block) {
    let off = b.a.off as usize;
    for i in 0..b.a.size {
        assert_eq!(
            sp.children[off + i as usize],
            Subtree(b.tag + u64::from(i)),
            "block at {} size {}: entry {i} overwritten",
            b.a.off,
            b.a.size
        );
    }
}

/// TestChildrenAllocator. Random allocs, frees and array growth on the
/// children slab. Every live block keeps its contents, free blocks merge back
/// (the slab stays within a constant of the live peak) and freeing everything
/// empties the slab.
#[test]
fn children_allocator() {
    let mut sp = new_test_pool();
    // Go seeds math/rand with 1; the case set differs, the invariants do not.
    let mut r = Rng(1);
    let mut live: Vec<Block> = Vec::new();
    let (mut live_cap, mut peak) = (0usize, 0usize);
    for step in 0..200_000u64 {
        let k = r.intn(10);
        if k < 4 || live.is_empty() {
            let mut n = 1 + r.intn(40) as u32;
            if r.intn(4) == 0 {
                n = 1 + r.intn(3000) as u32;
            }
            let mut b = Block {
                a: SubtreeArray::default(),
                tag: step << 32,
            };
            if r.intn(3) == 0 {
                // As reduceInPlace does.
                let room = sp.room(n);
                b.a = SubtreeArray {
                    off: sp.bump_children(room, n),
                    size: n,
                    cap: n,
                };
                fill(&mut sp, b, 0);
                sp.settle_children(&mut b.a);
            } else {
                (b.a.off, b.a.cap) = sp.alloc_children(n);
                b.a.size = n;
                fill(&mut sp, b, 0);
            }
            live.push(b);
            live_cap += b.a.cap as usize;
        } else if k < 7 {
            let i = r.intn(live.len() as u64) as usize;
            let mut b = live[i];
            check(&sp, b);
            live_cap -= b.a.cap as usize;
            let old = b.a.size;
            let grow = 1 + r.intn(u64::from(old) + 8) as u32;
            sp.array_reserve(&mut b.a, old + grow);
            b.a.size = b.a.cap;
            fill(&mut sp, b, old);
            live_cap += b.a.cap as usize;
            live[i] = b;
        } else {
            let i = r.intn(live.len() as u64) as usize;
            check(&sp, live[i]);
            live_cap -= live[i].a.cap as usize;
            sp.array_free(&mut live[i].a);
            live.swap_remove(i);
        }
        peak = peak.max(live_cap);
        let free = (sp.quick_free + sp.merged_free) as usize;
        assert_eq!(
            1 + live_cap + free,
            sp.children.len(),
            "step {step}: slab {} entries, live {live_cap}, counted free {free}",
            sp.children.len()
        );
        assert!(
            sp.children.len() <= 4 * peak + 4096,
            "step {step}: slab {} entries, live peak {peak}",
            sp.children.len()
        );
    }
    for mut b in live {
        check(&sp, b);
        sp.array_free(&mut b.a);
    }
    sp.flush_quick();
    assert!(
        sp.children.len() == 1 && sp.free_lists == [0u64; 3],
        "slab has {} entries and lists {:x?} after freeing everything",
        sp.children.len(),
        sp.free_lists
    );
}
