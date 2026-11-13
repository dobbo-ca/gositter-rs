//! subtree.c and subtree.h (gositter `subtree.go`).
//!
//! A subtree is the 8-byte C Subtree union. Bit 0 set: the inline leaf, with the
//! little-endian bit layout of SubtreeInlineData. Bit 0 clear: a handle, index << 1 into
//! `SubtreePool.heap`. 0 is NULL_SUBTREE.
//!
//! Not ported (Go GC workarounds, plan Global Constraints): `heapChunks` (sync.Pool),
//! `childSlab`, `childSlabs`, `getChildSlab`, `putChildSlab`, `reserveHeap`, `moveChildren`,
//! `useSlab`, and the pool fields `slab`, `outgrown`, `warm`. The children slab is one `Vec`
//! that a reset truncates and keeps, shrunk by the gositter rule.

use crate::length::{Length, length_add};
use crate::{Language, Point, SYMBOL_END, SYMBOL_ERROR, SYMBOL_ERROR_REPEAT, StateId, Symbol};
use std::sync::atomic::Ordering;

/// Subtree
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Subtree(pub(crate) u64);

/// NULL_SUBTREE
pub(crate) const NULL_SUBTREE: Subtree = Subtree(0);

/// TS_TREE_STATE_NONE
pub(crate) const TREE_STATE_NONE: StateId = 0xFFFF;

/// TS_MAX_INLINE_TREE_LENGTH
pub(crate) const MAX_INLINE_TREE_LENGTH: u32 = 255;

/// ERROR_STATE
pub(crate) const ERROR_STATE: StateId = 0;
/// ERROR_COST_PER_RECOVERY
pub(crate) const ERROR_COST_PER_RECOVERY: u32 = 500;
/// ERROR_COST_PER_MISSING_TREE
pub(crate) const ERROR_COST_PER_MISSING_TREE: u32 = 110;
/// ERROR_COST_PER_SKIPPED_TREE
pub(crate) const ERROR_COST_PER_SKIPPED_TREE: u32 = 100;
/// ERROR_COST_PER_SKIPPED_LINE
pub(crate) const ERROR_COST_PER_SKIPPED_LINE: u32 = 30;
/// ERROR_COST_PER_SKIPPED_CHAR
pub(crate) const ERROR_COST_PER_SKIPPED_CHAR: u32 = 1;

// SubtreeInlineData bits.
pub(crate) const INL_IS_INLINE: u64 = 1 << 0;
pub(crate) const INL_VISIBLE: u64 = 1 << 1;
pub(crate) const INL_NAMED: u64 = 1 << 2;
pub(crate) const INL_EXTRA: u64 = 1 << 3;
pub(crate) const INL_HAS_CHANGES: u64 = 1 << 4;
pub(crate) const INL_IS_MISSING: u64 = 1 << 5;
pub(crate) const INL_IS_KEYWORD: u64 = 1 << 6;

/// uint8
pub(crate) const INL_SYMBOL_SHIFT: u32 = 8;
/// uint16
pub(crate) const INL_PARSE_STATE_SHIFT: u32 = 16;
/// uint8
pub(crate) const INL_PADDING_COLUMNS_SHIFT: u32 = 32;
/// 4 bits
pub(crate) const INL_PADDING_ROWS_SHIFT: u32 = 40;
/// 4 bits
pub(crate) const INL_LOOKAHEAD_BYTES_SHIFT: u32 = 44;
/// uint8
pub(crate) const INL_PADDING_BYTES_SHIFT: u32 = 48;
/// uint8
pub(crate) const INL_SIZE_BYTES_SHIFT: u32 = 56;

// SubtreeHeapData bit flags.
pub(crate) const F_VISIBLE: u16 = 1 << 0;
pub(crate) const F_NAMED: u16 = 1 << 1;
pub(crate) const F_EXTRA: u16 = 1 << 2;
pub(crate) const F_FRAGILE_LEFT: u16 = 1 << 3;
pub(crate) const F_FRAGILE_RIGHT: u16 = 1 << 4;
pub(crate) const F_HAS_CHANGES: u16 = 1 << 5;
pub(crate) const F_HAS_EXTERNAL_TOKENS: u16 = 1 << 6;
pub(crate) const F_HAS_EXTERNAL_SCANNER_STATE_CHANGE: u16 = 1 << 7;
pub(crate) const F_DEPENDS_ON_COLUMN: u16 = 1 << 8;
pub(crate) const F_IS_MISSING: u16 = 1 << 9;
pub(crate) const F_IS_KEYWORD: u16 = 1 << 10;
/// Not in C: this node or one below it has repeat_depth > 0, so balance_subtree has work in
/// here.
pub(crate) const F_HAS_REPEAT: u16 = 1 << 11;

/// SubtreeHeapData. Part of the C union is kept: a leaf has no child counts, so an external
/// token leaf holds its scanner state (a range of `SubtreePool.ext`) in visible_child_count
/// and named_child_count. Readers of those check child_count first, as C does. 80 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SubtreeHeap {
    pub(crate) ref_count: u32,
    pub(crate) padding: Length,
    pub(crate) size: Length,
    pub(crate) lookahead_bytes: u32,
    pub(crate) error_cost: u32,
    pub(crate) child_count: u32,
    pub(crate) symbol: Symbol,
    pub(crate) parse_state: StateId,
    pub(crate) flags: u16,

    // Non-terminal subtrees (child_count > 0).
    /// The one union member C reads without checking the variant (ts_subtree_repeat_depth),
    /// so on an external token leaf it holds bytes 16 and 17 of the inline scanner state.
    pub(crate) repeat_depth: u16,
    pub(crate) production_id: u16,
    pub(crate) first_leaf_symbol: Symbol,
    pub(crate) first_leaf_parse_state: StateId,
    /// Size class of the children block.
    pub(crate) child_class: u8,
    pub(crate) visible_child_count: u32,
    pub(crate) named_child_count: u32,
    pub(crate) visible_descendant_count: u32,
    pub(crate) dynamic_precedence: i32,
    /// Offset into `SubtreePool.children`.
    pub(crate) children: u32,

    /// Error terminal subtrees (child_count == 0 && symbol == SYMBOL_ERROR). A subtree with
    /// children keeps its kind mask in the same four bytes.
    pub(crate) lookahead_char: i32,
}

const _: () = assert!(size_of::<SubtreeHeap>() == 80);

impl SubtreeHeap {
    /// kinds: the KindMask bits of the nodes below a subtree with children, with their
    /// aliases. No C counterpart.
    pub(crate) fn kinds(&self) -> u32 {
        self.lookahead_char as u32
    }

    /// setKinds
    pub(crate) fn set_kinds(&mut self, mask: u32) {
        self.lookahead_char = mask as i32;
    }

    /// ext: the external scanner state range (off, n) of an external terminal subtree
    /// (child_count == 0 && F_HAS_EXTERNAL_TOKENS).
    pub(crate) fn ext(&self) -> (u32, u32) {
        (self.visible_child_count, self.named_child_count)
    }

    /// setExt
    pub(crate) fn set_ext(&mut self, off: u32, n: u32) {
        (self.visible_child_count, self.named_child_count) = (off, n);
    }
}

/// SubtreeArray: a region of `SubtreePool.children`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SubtreeArray {
    pub(crate) off: u32,
    pub(crate) size: u32,
    pub(crate) cap: u32,
}

/// The heap data slab is a list of chunks: it grows without a copy.
pub(crate) const HEAP_CHUNK_BITS: u32 = 11;
pub(crate) const HEAP_CHUNK_MASK: u64 = (1 << HEAP_CHUNK_BITS) - 1;
/// 1 << HEAP_CHUNK_BITS entries per chunk.
pub(crate) const HEAP_CHUNK_SIZE: usize = 1 << HEAP_CHUNK_BITS;

/// heapChunk
pub(crate) type HeapChunk = [SubtreeHeap; HEAP_CHUNK_SIZE];

/// new(heapChunk), built on the heap: a 160 KiB array on the stack could overflow a test
/// thread's.
fn new_heap_chunk() -> Box<HeapChunk> {
    vec![SubtreeHeap::default(); HEAP_CHUNK_SIZE]
        .into_boxed_slice()
        .try_into()
        .unwrap()
}

/// A reset keeps KEPT_HEAP_CHUNKS chunks and drops the rest.
pub const KEPT_HEAP_CHUNKS: usize = 8;

/// A reset shrinks a children slab above SHRINK_CHILDREN entries to its initial size when
/// the parse before it used under an eighth of it.
pub const SHRINK_CHILDREN: usize = 1 << 16;

/// The capacity a children slab starts with (getChildSlab(4096)).
const INITIAL_CHILDREN: usize = 4096;

/// slabpool.go shrinkIndexes: free_trees above this many entries is dropped by the shrink
/// rule.
const SHRINK_FREE_TREES: usize = 1 << 20;

/// SubtreePool, and the arenas behind every subtree of one parser. Slabs are reset, not
/// freed, between parses.
pub(crate) struct SubtreePool {
    /// Entry 0 of chunk 0 unused.
    pub(crate) heap: Vec<Box<HeapChunk>>,
    /// Entries handed out, entry 0 included.
    pub(crate) heap_len: u32,
    /// free_trees
    pub(crate) free_trees: Vec<u32>,
    /// Entry 0 unused.
    pub(crate) children: Vec<Subtree>,
    /// External scanner states.
    pub(crate) ext: Vec<u8>,
    /// tree_stack
    pub(crate) tree_stack: Vec<Subtree>,

    /// Free children blocks, as in a malloc with boundary tags. A block of any size sits in
    /// the list of the largest class that fits in it. Its first entry holds the next block
    /// and, in the high half, the previous one; its second and its last entry hold its size,
    /// unless that is 1.
    pub(crate) free_blocks: [u32; CHILD_CLASS_COUNT as usize],
    /// The non-empty lists.
    pub(crate) free_lists: [u64; 3],
    /// Two bits per slab entry: a free block starts here, a free block ends here.
    pub(crate) free_bits: Vec<u64>,

    /// Freed blocks of up to 64 entries wait here, by size, linked through their first
    /// entry, and serve the next request of their size as they are. flush_quick moves them
    /// to free_blocks.
    pub(crate) quick_blocks: [u32; EXACT_CHILD_CLASSES as usize + 1],

    /// Entries in quick_blocks.
    pub(crate) quick_free: u32,
    /// Entries in free_blocks. reduce_in_place takes free blocks once they are a quarter of
    /// the slab.
    pub(crate) merged_free: u32,

    /// recover_to_state: the ERROR node whose children start the next ERROR node.
    /// child_count 0: none.
    pub(crate) resume: SubtreeHeap,
}

/// Children blocks are handed out in size classes: one class per size up to 64, then four
/// per doubling. Free blocks merge and split, so what an array leaves behind as it outgrows
/// one class after another serves it again.
pub(crate) const EXACT_CHILD_CLASSES: u32 = 64;
pub(crate) const CHILD_CLASS_COUNT: u32 = 165;

/// errSlabFull: offsets and indexes into the slabs are 32 bits.
pub(crate) const ERR_SLAB_FULL: &str =
    "gositter: input needs a parser slab of more than 2^32 entries";

/// childClass: the class of the smallest block that holds n entries, and the size of that
/// block.
pub(crate) fn child_class(n: u32) -> (u32, u32) {
    if n <= EXACT_CHILD_CLASSES {
        return (n, n);
    }
    if n > 1 << 31 {
        panic!("{ERR_SLAB_FULL}");
    }
    // bits.Len32(n-1) - 3; n - 1 >= 64.
    let shift = (n - 1).ilog2() - 2;
    let m = ((n - 1) >> shift) + 1; // 5 to 8
    (shift * 4 + m + 44, m << shift)
}

/// childClassSize: the block size of a class above EXACT_CHILD_CLASSES.
pub(crate) fn child_class_size(class: u32) -> u32 {
    let shift = (class - 49) / 4;
    (class - 44 - shift * 4) << shift
}

impl Default for SubtreePool {
    /// The Go zero value `subtreePool{}`; `reset` makes it usable.
    fn default() -> Self {
        SubtreePool {
            heap: Vec::new(),
            heap_len: 0,
            free_trees: Vec::new(),
            children: Vec::new(),
            ext: Vec::new(),
            tree_stack: Vec::new(),
            free_blocks: [0; CHILD_CLASS_COUNT as usize],
            free_lists: [0; 3],
            free_bits: Vec::new(),
            quick_blocks: [0; EXACT_CHILD_CLASSES as usize + 1],
            quick_free: 0,
            merged_free: 0,
            resume: SubtreeHeap::default(),
        }
    }
}

impl Subtree {
    /// isInline
    pub(crate) fn is_inline(self) -> bool {
        self.0 & INL_IS_INLINE != 0
    }
}

impl SubtreePool {
    /// Makes a new children slab of the initial capacity, with entry 0 in use.
    /// Its free bits grow when a block is freed. Go: useSlab(getChildSlab(4096), 1).
    fn new_children(&mut self) {
        self.children = Vec::with_capacity(INITIAL_CHILDREN);
        self.children.push(NULL_SUBTREE);
        self.free_bits = Vec::new();
    }

    /// reset
    pub(crate) fn reset(&mut self) {
        if self.heap.is_empty() {
            self.heap = vec![new_heap_chunk()];
            self.new_children();
        }
        let c = self.free_trees.capacity();
        if c > SHRINK_FREE_TREES && (self.heap_len as usize) < c / 8 {
            self.free_trees = Vec::new();
        }
        self.heap_len = 1;
        self.heap.truncate(KEPT_HEAP_CHUNKS);
        // len 1: nothing was parsed since the last reset
        let n = self.children.len();
        if n > 1 && self.children.capacity() > SHRINK_CHILDREN && n < self.children.capacity() / 8 {
            self.new_children();
        } else {
            let k = self.free_bits.len().min(n.div_ceil(32));
            self.free_bits[..k].fill(0);
            self.children.truncate(1);
        }
        self.free_blocks = [0; CHILD_CLASS_COUNT as usize];
        self.free_lists = [0; 3];
        self.quick_blocks = [0; EXACT_CHILD_CLASSES as usize + 1];
        (self.quick_free, self.merged_free) = (0, 0);
        self.resume.child_count = 0;
        self.free_trees.clear();
        self.ext.clear();
        self.tree_stack.clear();
    }

    /// ptr
    pub(crate) fn ptr(&self, s: Subtree) -> &SubtreeHeap {
        &self.heap[(s.0 >> (1 + HEAP_CHUNK_BITS)) as usize][((s.0 >> 1) & HEAP_CHUNK_MASK) as usize]
    }

    /// ptr, for writing.
    pub(crate) fn ptr_mut(&mut self, s: Subtree) -> &mut SubtreeHeap {
        &mut self.heap[(s.0 >> (1 + HEAP_CHUNK_BITS)) as usize]
            [((s.0 >> 1) & HEAP_CHUNK_MASK) as usize]
    }

    /// ts_subtree_pool_allocate
    pub(crate) fn allocate(&mut self) -> u32 {
        if let Some(i) = self.free_trees.pop() {
            return i;
        }
        // Every caller overwrites the whole entry, so a reused slot is not zeroed.
        let n = self.heap_len;
        if n >> HEAP_CHUNK_BITS == self.heap.len() as u32 {
            if n == !(HEAP_CHUNK_MASK as u32) {
                // the last chunk would wrap heap_len
                panic!("{ERR_SLAB_FULL}");
            }
            self.heap.push(new_heap_chunk());
        }
        self.heap_len = n + 1;
        n
    }

    /// ts_subtree_pool_free. C caps the pool at TS_MAX_TREE_POOL_SIZE and frees the rest; a
    /// slab index is always reusable.
    pub(crate) fn free(&mut self, i: u32) {
        self.free_trees.push(i);
    }

    /// ts_subtree_external_scanner_state. Empty for anything but an external token leaf.
    pub(crate) fn external_scanner_state(&self, s: Subtree) -> &[u8] {
        if s != NULL_SUBTREE && !s.is_inline() {
            let h = self.ptr(s);
            if h.flags & F_HAS_EXTERNAL_TOKENS != 0 && h.child_count == 0 {
                let (off, n) = h.ext();
                return &self.ext[off as usize..off as usize + n as usize];
            }
        }
        &[]
    }

    /// ts_subtree_external_scanner_state_eq
    pub(crate) fn external_scanner_state_eq(&self, s: Subtree, other: Subtree) -> bool {
        self.external_scanner_state(s) == self.external_scanner_state(other)
    }

    /// array
    pub(crate) fn array(&self, a: SubtreeArray) -> &[Subtree] {
        &self.children[a.off as usize..a.off as usize + a.size as usize]
    }

    /// room: makes space for n slots at the end of the children slab and returns where they
    /// start; bump_children(room(n), n).
    pub(crate) fn room(&mut self, n: u32) -> usize {
        let off = self.children.len();
        if off + n as usize > self.children.capacity() {
            self.grow_children(n);
        }
        off
    }

    /// bumpChildren. Go reslices over stale entries; the new slots here are NULL_SUBTREE.
    /// Every caller writes them before it reads them.
    pub(crate) fn bump_children(&mut self, off: usize, n: u32) -> u32 {
        self.children.resize(off + n as usize, NULL_SUBTREE);
        off as u32
    }

    /// growChildren. The capacity never passes 2^32-1 entries, so every offset and length
    /// of the slab fits in 32 bits.
    pub(crate) fn grow_children(&mut self, n: u32) {
        let need = self.children.len() + n as usize;
        if need > u32::MAX as usize {
            panic!("{ERR_SLAB_FULL}");
        }
        // moveChildren: the Vec moves the children, and the free bits stay as they are.
        let target = (2 * self.children.capacity())
            .max(need)
            .min(u32::MAX as usize);
        self.children.reserve_exact(target - self.children.len());
    }

    /// growFreeBits: sizes the free bits to the slab.
    pub(crate) fn grow_free_bits(&mut self) {
        let n = self.children.capacity().div_ceil(32);
        if n > self.free_bits.len() {
            self.free_bits.resize(n, 0);
        }
    }

    /// settleChildren: moves a region at the end of the slab, as pop_linear leaves it, into a
    /// free block. Without that a parse that frees nodes in error recovery and reduces in
    /// between would never use what it freed. reduce_in_place calls this once much of the
    /// slab is free: until then the end of the slab saves the copy.
    pub(crate) fn settle_children(&mut self, a: &mut SubtreeArray) {
        let n = a.size;
        let (class, size) = child_class(n);
        let mut off = self.take_free(class, size);
        if off == 0 && self.quick_free > 0 {
            self.flush_quick();
            off = self.take_merged(class, size);
        }
        if off == 0 {
            return;
        }
        let from = a.off as usize;
        self.children
            .copy_within(from..from + n as usize, off as usize);
        self.children.truncate(from);
        *a = SubtreeArray {
            off,
            size: n,
            cap: size,
        };
    }

    /// takeFree: a free block of at least size entries, cut to size, or 0. size is the
    /// block size of class.
    pub(crate) fn take_free(&mut self, class: u32, size: u32) -> u32 {
        if class <= EXACT_CHILD_CLASSES {
            let off = self.quick_blocks[class as usize];
            if off != 0 {
                self.quick_blocks[class as usize] = self.children[off as usize].0 as u32;
                self.quick_free -= size;
                return off;
            }
        }
        let mut off = self.take_merged(class, size);
        // Merge what waits in the quick lists before the slab gets a block that they could
        // have made, or grows.
        if off == 0
            && (class > EXACT_CHILD_CLASSES
                || self.children.len() + size as usize > self.children.capacity())
        {
            self.flush_quick();
            off = self.take_merged(class, size);
        }
        off
    }

    /// takeMerged
    pub(crate) fn take_merged(&mut self, class: u32, size: u32) -> u32 {
        let mut list = class;
        if self.free_blocks[list as usize] == 0 {
            // The next non-empty list.
            let mut m = self.free_lists[(list >> 6) as usize] >> (list & 63);
            while m == 0 {
                list = ((list >> 6) + 1) << 6;
                if list >= CHILD_CLASS_COUNT {
                    return 0;
                }
                m = self.free_lists[(list >> 6) as usize];
            }
            list += m.trailing_zeros();
        }
        let off = self.free_blocks[list as usize];
        let rest = self.unlink_free(off) - size;
        if rest > 0 {
            self.link_free(off + size, rest);
        }
        off
    }

    /// flushQuick
    pub(crate) fn flush_quick(&mut self) {
        self.quick_free = 0;
        for size in 1..=EXACT_CHILD_CLASSES {
            let mut off = self.quick_blocks[size as usize];
            self.quick_blocks[size as usize] = 0;
            while off != 0 {
                let next = self.children[off as usize].0 as u32;
                self.merge_free(off, size);
                off = next;
            }
        }
    }
}

/// freeList: the list of a free block.
pub(crate) fn free_list(size: u32) -> u32 {
    if size <= EXACT_CHILD_CLASSES {
        return size;
    }
    if size >= 1 << 31 {
        return CHILD_CLASS_COUNT - 1;
    }
    let (mut class, class_size) = child_class(size);
    if class_size != size {
        class -= 1;
    }
    class
}

impl SubtreePool {
    /// linkFree
    pub(crate) fn link_free(&mut self, off: u32, size: u32) {
        let list = free_list(size);
        let head = self.free_blocks[list as usize];
        self.children[off as usize] = Subtree(u64::from(head));
        if size > 1 {
            self.children[(off + 1) as usize] = Subtree(u64::from(size));
            self.children[(off + size - 1) as usize] = Subtree(u64::from(size));
        }
        if head != 0 {
            self.children[head as usize].0 |= u64::from(off) << 32;
        }
        self.free_blocks[list as usize] = off;
        self.free_lists[(list >> 6) as usize] |= 1 << (list & 63);
        self.merged_free += size;
        let last = off + size - 1;
        self.free_bits[(off >> 5) as usize] |= 1 << ((off & 31) * 2);
        self.free_bits[(last >> 5) as usize] |= 2 << ((last & 31) * 2);
    }

    /// unlinkFree: takes a free block out of its list and returns its size.
    pub(crate) fn unlink_free(&mut self, off: u32) -> u32 {
        let v = self.children[off as usize].0;
        let (next, prev) = (v as u32, (v >> 32) as u32);
        let mut size = 1u32;
        if (self.free_bits[(off >> 5) as usize] >> ((off & 31) * 2)) & 2 == 0 {
            size = self.children[(off + 1) as usize].0 as u32;
        }
        if prev != 0 {
            let p = self.children[prev as usize].0;
            self.children[prev as usize] = Subtree(((p >> 32) << 32) | u64::from(next));
        } else {
            let list = free_list(size);
            self.free_blocks[list as usize] = next;
            if next == 0 {
                self.free_lists[(list >> 6) as usize] &= !(1 << (list & 63));
            }
        }
        if next != 0 {
            let x = self.children[next as usize].0;
            self.children[next as usize] = Subtree(u64::from(x as u32) | (u64::from(prev) << 32));
        }
        let last = off + size - 1;
        self.free_bits[(off >> 5) as usize] &= !(1 << ((off & 31) * 2));
        self.free_bits[(last >> 5) as usize] &= !(2 << ((last & 31) * 2));
        self.merged_free -= size;
        size
    }

    /// allocChildren: a block for n > 0 entries, (off, size): a free one, else new at the
    /// end of the slab. C calls malloc here.
    pub(crate) fn alloc_children(&mut self, n: u32) -> (u32, u32) {
        let (class, size) = child_class(n);
        let off = self.take_free(class, size);
        if off != 0 {
            return (off, size);
        }
        let room = self.room(size);
        (self.bump_children(room, size), size)
    }

    /// freeQuick: gives back a block of up to EXACT_CHILD_CLASSES entries. C calls free
    /// here.
    pub(crate) fn free_quick(&mut self, off: u32, size: u32) {
        self.children[off as usize] = Subtree(u64::from(self.quick_blocks[size as usize]));
        self.quick_blocks[size as usize] = off;
        self.quick_free += size;
    }

    /// mergeFree: gives back a block of any size, or takes one from the quick lists: merges
    /// it with a free block behind it and one in front of it. What then reaches the end of
    /// the slab is cut off, the rest goes on its list.
    pub(crate) fn merge_free(&mut self, mut off: u32, mut size: u32) {
        self.grow_free_bits();
        let end = off + size;
        if (end as usize) < self.children.len()
            && (self.free_bits[(end >> 5) as usize] >> ((end & 31) * 2)) & 1 != 0
        {
            size += self.unlink_free(end);
        }
        let b = self.free_bits[((off - 1) >> 5) as usize] >> (((off - 1) & 31) * 2);
        if b & 2 != 0 {
            let mut before = 1u32;
            if b & 1 == 0 {
                before = self.children[(off - 1) as usize].0 as u32;
            }
            off -= before;
            self.unlink_free(off);
            size += before;
        }
        if (off + size) as usize == self.children.len() {
            self.children.truncate(off as usize);
            return;
        }
        self.link_free(off, size);
    }

    /// array_reserve. A free block comes first: a region at the end of the slab that only
    /// ever grew in place would leave every block freed on the way unused. Without one, such
    /// a region grows in place.
    pub(crate) fn array_reserve(&mut self, a: &mut SubtreeArray, new_cap: u32) {
        if new_cap <= a.cap {
            return;
        }
        let (class, new_cap) = child_class(new_cap);
        let mut off = self.take_free(class, new_cap);
        if off == 0 {
            if a.cap > 0 && (a.off + a.cap) as usize == self.children.len() {
                let room = self.room(new_cap - a.cap);
                self.bump_children(room, new_cap - a.cap);
                a.cap = new_cap;
                return;
            }
            let room = self.room(new_cap);
            off = self.bump_children(room, new_cap);
        }
        let from = a.off as usize;
        self.children
            .copy_within(from..from + a.size as usize, off as usize);
        let size = a.size;
        self.array_free(a);
        *a = SubtreeArray {
            off,
            size,
            cap: new_cap,
        };
    }

    /// _array__grow
    pub(crate) fn array_grow(&mut self, a: &mut SubtreeArray, count: u32) {
        let new_size = a.size + count;
        if new_size > a.cap {
            let new_cap = a.cap.wrapping_mul(2).max(8).max(new_size);
            self.array_reserve(a, new_cap);
        }
    }

    /// array_push
    pub(crate) fn array_push(&mut self, a: &mut SubtreeArray, s: Subtree) {
        self.array_grow(a, 1);
        self.children[(a.off + a.size) as usize] = s;
        a.size += 1;
    }

    /// array_splice. The elements are the slab entries `[elements_off, elements_off +
    /// elements_len)` (Go passes a slice that may alias the slab; every caller passes
    /// children_of of a live node, which array_reserve neither moves nor frees).
    pub(crate) fn array_splice(
        &mut self,
        a: &mut SubtreeArray,
        index: u32,
        old_count: u32,
        elements_off: u32,
        elements_len: u32,
    ) {
        let new_count = elements_len;
        let new_size = a.size + new_count - old_count;
        let old_end = index + old_count;
        let new_end = index + new_count;
        self.array_reserve(a, new_size);
        let c = a.off as usize;
        if a.size > old_end {
            self.children.copy_within(
                c + old_end as usize..c + a.size as usize,
                c + new_end as usize,
            );
        }
        let e = elements_off as usize;
        self.children
            .copy_within(e..e + new_count as usize, c + index as usize);
        a.size = new_size;
    }

    /// array_delete for a region.
    pub(crate) fn array_free(&mut self, a: &mut SubtreeArray) {
        if a.cap > 0 {
            if (a.off + a.cap) as usize == self.children.len() {
                self.children.truncate(a.off as usize);
            } else if a.cap <= EXACT_CHILD_CLASSES {
                self.free_quick(a.off, a.cap);
            } else {
                self.merge_free(a.off, a.cap);
            }
        }
        *a = SubtreeArray::default();
    }

    /// ts_subtree_array_copy
    pub(crate) fn array_copy(&mut self, array: SubtreeArray) -> SubtreeArray {
        let mut dest = array;
        if array.cap > 0 {
            (dest.off, dest.cap) = self.alloc_children(array.cap);
            let from = array.off as usize;
            self.children
                .copy_within(from..from + array.size as usize, dest.off as usize);
            for i in dest.off..dest.off + dest.size {
                let s = self.children[i as usize];
                self.retain(s);
            }
        }
        dest
    }

    /// ts_subtree_array_delete
    pub(crate) fn array_delete(&mut self, a: &mut SubtreeArray) {
        for i in 0..a.size {
            let s = self.children[(a.off + i) as usize];
            self.release(s);
        }
        self.array_free(a);
    }

    /// ts_subtree_array_remove_trailing_extras. destination is a parser-owned scratch vector
    /// (trailing_extras).
    pub(crate) fn array_remove_trailing_extras(
        &self,
        a: &mut SubtreeArray,
        destination: &mut Vec<Subtree>,
    ) {
        destination.clear();
        while a.size > 0 {
            let last = self.children[(a.off + a.size - 1) as usize];
            if self.extra(last) {
                a.size -= 1;
                destination.push(last);
            } else {
                break;
            }
        }
        subtree_slice_reverse(destination);
    }
}

/// ts_subtree_array_reverse
pub(crate) fn subtree_slice_reverse(s: &mut [Subtree]) {
    s.reverse();
}

// Subtree accessors (subtree.h).

impl SubtreePool {
    /// ts_subtree_symbol
    pub(crate) fn symbol(&self, s: Subtree) -> Symbol {
        if s.is_inline() {
            return Symbol::from((s.0 >> INL_SYMBOL_SHIFT) as u8);
        }
        self.ptr(s).symbol
    }

    /// ts_subtree_visible
    pub(crate) fn visible(&self, s: Subtree) -> bool {
        if s.is_inline() {
            return s.0 & INL_VISIBLE != 0;
        }
        self.ptr(s).flags & F_VISIBLE != 0
    }

    /// ts_subtree_named
    pub(crate) fn named(&self, s: Subtree) -> bool {
        if s.is_inline() {
            return s.0 & INL_NAMED != 0;
        }
        self.ptr(s).flags & F_NAMED != 0
    }

    /// ts_subtree_extra
    pub(crate) fn extra(&self, s: Subtree) -> bool {
        if s.is_inline() {
            return s.0 & INL_EXTRA != 0;
        }
        self.ptr(s).flags & F_EXTRA != 0
    }

    /// ts_subtree_missing
    pub(crate) fn missing(&self, s: Subtree) -> bool {
        if s.is_inline() {
            return s.0 & INL_IS_MISSING != 0;
        }
        self.ptr(s).flags & F_IS_MISSING != 0
    }

    /// ts_subtree_is_keyword
    pub(crate) fn is_keyword(&self, s: Subtree) -> bool {
        if s.is_inline() {
            return s.0 & INL_IS_KEYWORD != 0;
        }
        self.ptr(s).flags & F_IS_KEYWORD != 0
    }

    /// ts_subtree_parse_state
    pub(crate) fn parse_state(&self, s: Subtree) -> StateId {
        if s.is_inline() {
            return (s.0 >> INL_PARSE_STATE_SHIFT) as u16;
        }
        self.ptr(s).parse_state
    }

    /// ts_subtree_lookahead_bytes
    pub(crate) fn lookahead_bytes(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            return ((s.0 >> INL_LOOKAHEAD_BYTES_SHIFT) as u32) & 0xF;
        }
        self.ptr(s).lookahead_bytes
    }

    /// ts_subtree_children
    pub(crate) fn children_of(&self, s: Subtree) -> &[Subtree] {
        if s.is_inline() {
            return &[];
        }
        let h = self.ptr(s);
        &self.children[h.children as usize..h.children as usize + h.child_count as usize]
    }

    /// ts_subtree_set_extra
    pub(crate) fn set_extra(&mut self, s: &mut Subtree, is_extra: bool) {
        if s.is_inline() {
            if is_extra {
                s.0 |= INL_EXTRA;
            } else {
                s.0 &= !INL_EXTRA;
            }
        } else {
            self.ptr_mut(*s).set_flag(F_EXTRA, is_extra);
        }
    }
}

impl SubtreeHeap {
    /// setFlag
    pub(crate) fn set_flag(&mut self, f: u16, on: bool) {
        if on {
            self.flags |= f;
        } else {
            self.flags &= !f;
        }
    }
}

impl SubtreePool {
    /// ts_subtree_leaf_symbol
    pub(crate) fn leaf_symbol(&self, s: Subtree) -> Symbol {
        if s.is_inline() {
            return Symbol::from((s.0 >> INL_SYMBOL_SHIFT) as u8);
        }
        let h = self.ptr(s);
        if h.child_count == 0 {
            return h.symbol;
        }
        h.first_leaf_symbol
    }

    /// ts_subtree_leaf_parse_state
    pub(crate) fn leaf_parse_state(&self, s: Subtree) -> StateId {
        if s.is_inline() {
            return (s.0 >> INL_PARSE_STATE_SHIFT) as u16;
        }
        let h = self.ptr(s);
        if h.child_count == 0 {
            return h.parse_state;
        }
        h.first_leaf_parse_state
    }

    /// ts_subtree_padding
    pub(crate) fn padding(&self, s: Subtree) -> Length {
        if s.is_inline() {
            return Length {
                bytes: u32::from((s.0 >> INL_PADDING_BYTES_SHIFT) as u8),
                extent: Point {
                    row: ((s.0 >> INL_PADDING_ROWS_SHIFT) as u32) & 0xF,
                    column: u32::from((s.0 >> INL_PADDING_COLUMNS_SHIFT) as u8),
                },
            };
        }
        self.ptr(s).padding
    }

    /// ts_subtree_size
    pub(crate) fn size(&self, s: Subtree) -> Length {
        if s.is_inline() {
            let n = u32::from((s.0 >> INL_SIZE_BYTES_SHIFT) as u8);
            return Length {
                bytes: n,
                extent: Point { row: 0, column: n },
            };
        }
        self.ptr(s).size
    }

    /// ts_subtree_total_size
    pub(crate) fn total_size(&self, s: Subtree) -> Length {
        length_add(self.padding(s), self.size(s))
    }

    /// ts_subtree_total_bytes
    pub(crate) fn total_bytes(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            return u32::from((s.0 >> INL_PADDING_BYTES_SHIFT) as u8)
                + u32::from((s.0 >> INL_SIZE_BYTES_SHIFT) as u8);
        }
        let h = self.ptr(s);
        h.padding.bytes.wrapping_add(h.size.bytes)
    }

    /// ts_subtree_child_count
    pub(crate) fn child_count(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            return 0;
        }
        self.ptr(s).child_count
    }

    /// ts_subtree_repeat_depth
    pub(crate) fn repeat_depth(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            return 0;
        }
        u32::from(self.ptr(s).repeat_depth)
    }

    /// ts_subtree_visible_descendant_count
    pub(crate) fn visible_descendant_count(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            return 0;
        }
        let h = self.ptr(s);
        if h.child_count == 0 {
            return 0;
        }
        h.visible_descendant_count
    }

    /// ts_subtree_visible_child_count
    pub(crate) fn visible_child_count(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            return 0;
        }
        let h = self.ptr(s);
        if h.child_count > 0 {
            return h.visible_child_count;
        }
        0
    }

    /// ts_subtree_error_cost
    pub(crate) fn error_cost(&self, s: Subtree) -> u32 {
        if s.is_inline() {
            if s.0 & INL_IS_MISSING != 0 {
                return ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
            }
            return 0;
        }
        let h = self.ptr(s);
        if h.flags & F_IS_MISSING != 0 {
            return ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
        }
        h.error_cost
    }

    /// ts_subtree_dynamic_precedence
    pub(crate) fn dynamic_precedence(&self, s: Subtree) -> i32 {
        if s.is_inline() {
            return 0;
        }
        let h = self.ptr(s);
        if h.child_count == 0 {
            return 0;
        }
        h.dynamic_precedence
    }

    /// ts_subtree_fragile_left
    pub(crate) fn fragile_left(&self, s: Subtree) -> bool {
        !s.is_inline() && self.ptr(s).flags & F_FRAGILE_LEFT != 0
    }

    /// ts_subtree_fragile_right
    pub(crate) fn fragile_right(&self, s: Subtree) -> bool {
        !s.is_inline() && self.ptr(s).flags & F_FRAGILE_RIGHT != 0
    }

    /// ts_subtree_has_external_tokens
    pub(crate) fn has_external_tokens(&self, s: Subtree) -> bool {
        !s.is_inline() && self.ptr(s).flags & F_HAS_EXTERNAL_TOKENS != 0
    }

    /// ts_subtree_has_external_scanner_state_change
    pub(crate) fn has_external_scanner_state_change(&self, s: Subtree) -> bool {
        !s.is_inline() && self.ptr(s).flags & F_HAS_EXTERNAL_SCANNER_STATE_CHANGE != 0
    }

    /// ts_subtree_depends_on_column
    pub(crate) fn depends_on_column(&self, s: Subtree) -> bool {
        !s.is_inline() && self.ptr(s).flags & F_DEPENDS_ON_COLUMN != 0
    }

    /// ts_subtree_is_error
    pub(crate) fn is_error(&self, s: Subtree) -> bool {
        self.symbol(s) == SYMBOL_ERROR
    }

    /// ts_subtree_is_eof
    pub(crate) fn is_eof(&self, s: Subtree) -> bool {
        self.symbol(s) == SYMBOL_END
    }
}

// Subtree constructors (subtree.c).

/// ts_subtree_can_inline
pub(crate) fn subtree_can_inline(padding: Length, size: Length, lookahead_bytes: u32) -> bool {
    padding.bytes < MAX_INLINE_TREE_LENGTH
        && padding.extent.row < 16
        && padding.extent.column < MAX_INLINE_TREE_LENGTH
        && size.bytes < MAX_INLINE_TREE_LENGTH
        && size.extent.row == 0
        && size.extent.column < MAX_INLINE_TREE_LENGTH
        && lookahead_bytes < 16
}

impl SubtreePool {
    /// ts_subtree_new_leaf
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_leaf(
        &mut self,
        symbol: Symbol,
        padding: Length,
        size: Length,
        lookahead_bytes: u32,
        parse_state: StateId,
        has_external_tokens: bool,
        depends_on_column: bool,
        is_keyword: bool,
        lang: &Language,
    ) -> Subtree {
        let metadata = lang.symbol_metadata(symbol);
        let extra = symbol == SYMBOL_END;

        let is_inline = symbol <= 255
            && !has_external_tokens
            && subtree_can_inline(padding, size, lookahead_bytes);

        if is_inline {
            let mut s = INL_IS_INLINE
                | (u64::from(symbol) << INL_SYMBOL_SHIFT)
                | (u64::from(parse_state) << INL_PARSE_STATE_SHIFT)
                | (u64::from(padding.extent.column) << INL_PADDING_COLUMNS_SHIFT)
                | (u64::from(padding.extent.row) << INL_PADDING_ROWS_SHIFT)
                | (u64::from(lookahead_bytes) << INL_LOOKAHEAD_BYTES_SHIFT)
                | (u64::from(padding.bytes) << INL_PADDING_BYTES_SHIFT)
                | (u64::from(size.bytes) << INL_SIZE_BYTES_SHIFT);
            if metadata.visible {
                s |= INL_VISIBLE;
            }
            if metadata.named {
                s |= INL_NAMED;
            }
            if extra {
                s |= INL_EXTRA;
            }
            if is_keyword {
                s |= INL_IS_KEYWORD;
            }
            return Subtree(s);
        }

        let mut flags = 0u16;
        if metadata.visible {
            flags |= F_VISIBLE;
        }
        if metadata.named {
            flags |= F_NAMED;
        }
        if extra {
            flags |= F_EXTRA;
        }
        if has_external_tokens {
            flags |= F_HAS_EXTERNAL_TOKENS;
        }
        if depends_on_column {
            flags |= F_DEPENDS_ON_COLUMN;
        }
        if is_keyword {
            flags |= F_IS_KEYWORD;
        }
        let i = self.allocate();
        *self.ptr_mut(Subtree(u64::from(i) << 1)) = SubtreeHeap {
            ref_count: 1,
            padding,
            size,
            lookahead_bytes,
            symbol,
            parse_state,
            flags,
            ..SubtreeHeap::default()
        };
        Subtree(u64::from(i) << 1)
    }

    /// ts_subtree_set_symbol
    pub(crate) fn set_symbol(&mut self, s: &mut Subtree, symbol: Symbol, lang: &Language) {
        let metadata = lang.symbol_metadata(symbol);
        if s.is_inline() {
            let mut v = s.0 & !((0xFFu64 << INL_SYMBOL_SHIFT) | INL_NAMED | INL_VISIBLE);
            v |= u64::from(symbol as u8) << INL_SYMBOL_SHIFT;
            if metadata.named {
                v |= INL_NAMED;
            }
            if metadata.visible {
                v |= INL_VISIBLE;
            }
            *s = Subtree(v);
        } else {
            let h = self.ptr_mut(*s);
            h.symbol = symbol;
            h.set_flag(F_NAMED, metadata.named);
            h.set_flag(F_VISIBLE, metadata.visible);
        }
    }

    /// ts_subtree_new_error
    pub(crate) fn new_error(
        &mut self,
        lookahead_char: i32,
        padding: Length,
        size: Length,
        bytes_scanned: u32,
        parse_state: StateId,
        lang: &Language,
    ) -> Subtree {
        // SYMBOL_ERROR is above 255, so this is never inline.
        let result = self.new_leaf(
            SYMBOL_ERROR,
            padding,
            size,
            bytes_scanned,
            parse_state,
            false,
            false,
            false,
            lang,
        );
        let h = self.ptr_mut(result);
        h.flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
        h.lookahead_char = lookahead_char;
        result
    }

    /// ts_subtree_clone. Go writes the entry first and fixes it up in place; this builds it
    /// in a copy and stores it at the end (nothing between reads the new entry).
    pub(crate) fn clone(&mut self, s: Subtree) -> Subtree {
        let i = self.allocate();
        let mut h = *self.ptr(s);
        if h.child_count > 0 {
            let old = h.children as usize;
            let (off, _) = self.alloc_children(h.child_count);
            self.children
                .copy_within(old..old + h.child_count as usize, off as usize);
            h.children = off;
            let (class, _) = child_class(h.child_count);
            h.child_class = class as u8;
            for j in off..off + h.child_count {
                let child = self.children[j as usize];
                self.retain(child);
            }
        }
        // An external scanner state is immutable and never freed during a parse, so the
        // clone shares the bytes instead of copying them.
        h.ref_count = 1;
        *self.ptr_mut(Subtree(u64::from(i) << 1)) = h;
        Subtree(u64::from(i) << 1)
    }

    /// ts_subtree_make_mut. Takes ownership; copies if there is another owner.
    pub(crate) fn make_mut(&mut self, s: Subtree) -> Subtree {
        if s.is_inline() {
            return s;
        }
        if self.ptr(s).ref_count == 1 {
            return s;
        }
        let result = self.clone(s);
        self.release(s);
        result
    }

    /// ts_subtree_compress, but for the summary of tree where the next turn makes it again.
    /// Go takes the stack as `*[]subtree` and every caller passes `&sp.treeStack`, so this
    /// works on `self.tree_stack`.
    pub(crate) fn compress(&mut self, tree: Subtree, count: u32, lang: &Language) {
        let initial_stack_size = self.tree_stack.len();

        let mut tree = tree;
        let symbol = self.ptr(tree).symbol;
        for _ in 0..count {
            let th = self.ptr(tree);
            if th.ref_count > 1 || th.child_count < 2 {
                break;
            }
            let th_children = th.children as usize;

            let child = self.children[th_children];
            if child.is_inline() {
                break;
            }
            let ch = self.ptr(child);
            if ch.child_count < 2 || ch.ref_count > 1 || ch.symbol != symbol {
                break;
            }
            let ch_children = ch.children as usize;

            let grandchild = self.children[ch_children];
            if grandchild.is_inline() {
                break;
            }
            let gh = self.ptr(grandchild);
            if gh.child_count < 2 || gh.ref_count > 1 || gh.symbol != symbol {
                break;
            }
            let last = (gh.children + gh.child_count - 1) as usize;

            self.children[th_children] = grandchild;
            self.children[ch_children] = self.children[last];
            self.children[last] = child;
            self.tree_stack.push(tree);
            tree = grandchild;
        }

        while self.tree_stack.len() > initial_stack_size {
            let tree = self.tree_stack.pop().unwrap();
            let child = self.children[self.ptr(tree).children as usize];
            let ch = self.ptr(child);
            let grandchild = self.children[(ch.children + ch.child_count - 1) as usize];
            self.summarize(grandchild, lang);
            self.summarize(child, lang);
            // The next turn summarizes tree again, as its child, and nothing reads tree
            // before that. All that summary takes from this one is the size it tests for a
            // first child that depends on the column.
            if self.tree_stack.len() == initial_stack_size
                || self.ptr(child).flags & F_DEPENDS_ON_COLUMN != 0
            {
                self.summarize(tree, lang);
            }
        }
    }

    /// summarize. Not in C: summarize_children, with the most common shape of a node of two
    /// or more children on its own path.
    pub(crate) fn summarize(&mut self, tree: Subtree, lang: &Language) {
        let h = self.ptr(tree);
        if h.child_count == 2 && h.production_id == 0 && h.symbol < SYMBOL_ERROR_REPEAT {
            let c = h.children as usize;
            let (a, b) = (self.children[c], self.children[c + 1]);
            if (a.0 | b.0) & INL_IS_INLINE == 0 {
                let (a, b) = (*self.ptr(a), *self.ptr(b));
                Self::summarize_two_heap(self.ptr_mut(tree), &a, &b, lang);
                return;
            }
        }
        self.summarize_children(tree, lang);
    }

    /// summarizeTwoHeap: summarize_children for two heap children a and b, no alias sequence
    /// and not an error symbol. Each line is the loop of summarize_children with two turns
    /// and the sums folded. Not in C. Takes no pool: the caller hands it the three heap
    /// entries.
    pub(crate) fn summarize_two_heap(
        h: &mut SubtreeHeap,
        a: &SubtreeHeap,
        b: &SubtreeHeap,
        lang: &Language,
    ) {
        let mut flags = h.flags
            & !(F_HAS_EXTERNAL_TOKENS
                | F_DEPENDS_ON_COLUMN
                | F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
                | F_HAS_REPEAT);
        let (af, bf) = (a.flags, b.flags);
        // C tests the old size of h before a sets it.
        if (h.size.extent.row == 0 && af & F_DEPENDS_ON_COLUMN != 0)
            || (a.size.extent.row == 0 && bf & F_DEPENDS_ON_COLUMN != 0)
        {
            flags |= F_DEPENDS_ON_COLUMN;
        }
        flags |= ((af | bf)
            & (F_HAS_EXTERNAL_SCANNER_STATE_CHANGE | F_HAS_EXTERNAL_TOKENS | F_HAS_REPEAT))
            | (af & F_FRAGILE_LEFT)
            | (bf & F_FRAGILE_RIGHT);
        let (as_, bs) = (a.symbol, b.symbol);
        if as_ == SYMBOL_ERROR || bs == SYMBOL_ERROR {
            flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
            h.parse_state = TREE_STATE_NONE;
        }

        let (padding, mut size) = (a.padding, a.size);
        let mut lookahead_end_byte = padding
            .bytes
            .wrapping_add(size.bytes)
            .wrapping_add(a.lookahead_bytes);
        size = length_add(size, length_add(b.padding, b.size));
        let e = padding
            .bytes
            .wrapping_add(size.bytes)
            .wrapping_add(b.lookahead_bytes);
        if e > lookahead_end_byte {
            lookahead_end_byte = e;
        }
        h.padding = padding;
        h.size = size;
        h.lookahead_bytes = lookahead_end_byte
            .wrapping_sub(size.bytes)
            .wrapping_sub(padding.bytes);

        let (mut a_cost, mut b_cost) = (a.error_cost, b.error_cost);
        if af & F_IS_MISSING != 0 {
            a_cost = ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
        }
        if as_ == SYMBOL_ERROR_REPEAT {
            a_cost = 0;
        }
        if bf & F_IS_MISSING != 0 {
            b_cost = ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
        }
        if bs == SYMBOL_ERROR_REPEAT {
            b_cost = 0;
        }
        h.error_cost = a_cost.wrapping_add(b_cost);

        let (mut visible_child_count, mut named_child_count, mut visible_descendant_count) =
            (0u32, 0u32, 0u32);
        let mut dynamic_precedence = 0i32;
        let mut kinds = lang.kind_bit(as_) | lang.kind_bit(bs);
        if a.child_count > 0 {
            visible_descendant_count = a.visible_descendant_count;
            dynamic_precedence = a.dynamic_precedence;
            kinds |= a.kinds();
            h.first_leaf_symbol = a.first_leaf_symbol;
            h.first_leaf_parse_state = a.first_leaf_parse_state;
        } else {
            h.first_leaf_symbol = as_;
            h.first_leaf_parse_state = a.parse_state;
        }
        if af & F_VISIBLE != 0 {
            visible_descendant_count = visible_descendant_count.wrapping_add(1);
            visible_child_count = 1;
            if af & F_NAMED != 0 {
                named_child_count = 1;
            }
        } else if a.child_count > 0 {
            visible_child_count = a.visible_child_count;
            named_child_count = a.named_child_count;
        }
        if b.child_count > 0 {
            visible_descendant_count =
                visible_descendant_count.wrapping_add(b.visible_descendant_count);
            dynamic_precedence = dynamic_precedence.wrapping_add(b.dynamic_precedence);
            kinds |= b.kinds();
        }
        if bf & F_VISIBLE != 0 {
            visible_descendant_count = visible_descendant_count.wrapping_add(1);
            visible_child_count = visible_child_count.wrapping_add(1);
            if bf & F_NAMED != 0 {
                named_child_count = named_child_count.wrapping_add(1);
            }
        } else if b.child_count > 0 {
            visible_child_count = visible_child_count.wrapping_add(b.visible_child_count);
            named_child_count = named_child_count.wrapping_add(b.named_child_count);
        }
        h.visible_child_count = visible_child_count;
        h.named_child_count = named_child_count;
        h.visible_descendant_count = visible_descendant_count;
        h.dynamic_precedence = dynamic_precedence;
        h.set_kinds(kinds);

        h.repeat_depth = 0;
        if flags & (F_VISIBLE | F_NAMED) == 0 && as_ == h.symbol {
            // uint16 in Go: wraps.
            h.repeat_depth = a.repeat_depth.max(b.repeat_depth).wrapping_add(1);
            flags |= F_HAS_REPEAT;
        }
        h.flags = flags;
    }

    /// ts_subtree_summarize_children. Each child is decoded once (inline bits or one heap
    /// load) and the sums are kept in locals. Not line by line: a node without an alias
    /// sequence takes the first loop, which is the second one with the alias test gone and
    /// one branch per kind of child, and an ERROR node made by recover_to_state starts from
    /// the sums of the one it replaces. The node is summarized in a copy of its entry that
    /// is stored at the end.
    pub(crate) fn summarize_children(&mut self, tree: Subtree, lang: &Language) {
        let mut h = *self.ptr(tree);

        let (mut named_child_count, mut visible_child_count) = (0u32, 0u32);
        let (mut error_cost, mut visible_descendant_count) = (0u32, 0u32);
        let mut dynamic_precedence = 0i32;
        let mut kinds = 0u32;
        let mut flags = h.flags
            & !(F_HAS_EXTERNAL_TOKENS
                | F_DEPENDS_ON_COLUMN
                | F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
                | F_HAS_REPEAT);
        h.repeat_depth = 0;

        let mut structural_index = 0usize;
        let alias_sequence = lang.alias_sequence(u32::from(h.production_id));
        let mut lookahead_end_byte = 0u32;
        let self_is_error = h.symbol == SYMBOL_ERROR || h.symbol == SYMBOL_ERROR_REPEAT;
        // Not zero: C tests the old size.extent.row before the first child sets it, and a
        // node without children keeps both.
        let (mut padding, mut size) = (h.padding, h.size);

        let mut start = 0usize;
        let n = self.resume.child_count;
        if n != 0 {
            let r = &self.resume;
            if h.symbol == SYMBOL_ERROR && alias_sequence.is_none() && n <= h.child_count {
                // The sums as the loop left them after the n children of r, which are the
                // first n children of h.
                (padding, size) = (r.padding, r.size);
                lookahead_end_byte = r
                    .lookahead_bytes
                    .wrapping_add(r.size.bytes)
                    .wrapping_add(r.padding.bytes);
                error_cost = r
                    .error_cost
                    .wrapping_sub(ERROR_COST_PER_RECOVERY)
                    .wrapping_sub(ERROR_COST_PER_SKIPPED_CHAR.wrapping_mul(r.size.bytes))
                    .wrapping_sub(ERROR_COST_PER_SKIPPED_LINE.wrapping_mul(r.size.extent.row));
                named_child_count = r.named_child_count;
                visible_child_count = r.visible_child_count;
                visible_descendant_count = r.visible_descendant_count;
                dynamic_precedence = r.dynamic_precedence;
                kinds = r.kinds();
                flags |= r.flags
                    & (F_HAS_EXTERNAL_TOKENS
                        | F_DEPENDS_ON_COLUMN
                        | F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
                        | F_HAS_REPEAT);
                if r.parse_state == TREE_STATE_NONE {
                    h.parse_state = TREE_STATE_NONE;
                }
                start = n as usize;
            }
            self.resume.child_count = 0;
        }

        let children =
            &self.children[h.children as usize..h.children as usize + h.child_count as usize];
        match alias_sequence {
            None => {
                let kind_bits = lang.derived.kind_bits.get();
                let kind_error = lang.derived.kind_error.load(Ordering::Relaxed);
                for (i, &child) in children.iter().enumerate().skip(start) {
                    if child.is_inline() {
                        let n = u32::from((child.0 >> INL_SIZE_BYTES_SHIFT) as u8);
                        let pb = u32::from((child.0 >> INL_PADDING_BYTES_SHIFT) as u8);
                        let pr = ((child.0 >> INL_PADDING_ROWS_SHIFT) as u32) & 0xF;
                        let pc = u32::from((child.0 >> INL_PADDING_COLUMNS_SHIFT) as u8);
                        if i == 0 {
                            padding = Length {
                                bytes: pb,
                                extent: Point {
                                    row: pr,
                                    column: pc,
                                },
                            };
                            size = Length {
                                bytes: n,
                                extent: Point { row: 0, column: n },
                            };
                        } else {
                            // length_add of the padding and the one-row size
                            size.bytes += pb + n;
                            if pr > 0 {
                                size.extent.row += pr;
                                size.extent.column = pc + n;
                            } else {
                                size.extent.column += pc + n;
                            }
                        }
                        let e = padding.bytes
                            + size.bytes
                            + (((child.0 >> INL_LOOKAHEAD_BYTES_SHIFT) as u32) & 0xF);
                        if e > lookahead_end_byte {
                            lookahead_end_byte = e;
                        }
                        if child.0 & INL_IS_MISSING != 0 {
                            error_cost = error_cost.wrapping_add(
                                ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY,
                            );
                        }
                        if child.0 & INL_VISIBLE != 0 {
                            if self_is_error && child.0 & INL_EXTRA == 0 {
                                error_cost = error_cost.wrapping_add(ERROR_COST_PER_SKIPPED_TREE);
                            }
                            visible_descendant_count = visible_descendant_count.wrapping_add(1);
                            visible_child_count = visible_child_count.wrapping_add(1);
                            if child.0 & INL_NAMED != 0 {
                                named_child_count = named_child_count.wrapping_add(1);
                            }
                        }
                        // lang.kind_bit: an inline symbol is never SYMBOL_ERROR
                        let sym = usize::from((child.0 >> INL_SYMBOL_SHIFT) as u8);
                        if let Some(bit) = kind_bits.and_then(|k| k.get(sym)) {
                            kinds |= bit.load(Ordering::Relaxed);
                        }
                        continue;
                    }

                    let ch = self.ptr(child);
                    let cf = ch.flags;
                    if size.extent.row == 0 && cf & F_DEPENDS_ON_COLUMN != 0 {
                        flags |= F_DEPENDS_ON_COLUMN;
                    }
                    flags |= cf
                        & (F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
                            | F_HAS_EXTERNAL_TOKENS
                            | F_HAS_REPEAT);
                    let c_symbol = ch.symbol;
                    if i == 0 {
                        padding = ch.padding;
                        size = ch.size;
                    } else {
                        size = length_add(size, length_add(ch.padding, ch.size));
                    }
                    let e = padding
                        .bytes
                        .wrapping_add(size.bytes)
                        .wrapping_add(ch.lookahead_bytes);
                    if e > lookahead_end_byte {
                        lookahead_end_byte = e;
                    }
                    if c_symbol != SYMBOL_ERROR_REPEAT {
                        if cf & F_IS_MISSING != 0 {
                            error_cost = error_cost.wrapping_add(
                                ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY,
                            );
                        } else {
                            error_cost = error_cost.wrapping_add(ch.error_cost);
                        }
                    }
                    if c_symbol == SYMBOL_ERROR {
                        flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
                        h.parse_state = TREE_STATE_NONE;
                        kinds |= kind_error;
                    } else if let Some(bit) = kind_bits.and_then(|k| k.get(usize::from(c_symbol))) {
                        kinds |= bit.load(Ordering::Relaxed);
                    }
                    let grandchild_count = ch.child_count;
                    if grandchild_count > 0 {
                        kinds |= ch.kinds();
                        dynamic_precedence = dynamic_precedence.wrapping_add(ch.dynamic_precedence);
                        visible_descendant_count =
                            visible_descendant_count.wrapping_add(ch.visible_descendant_count);
                    }
                    if cf & F_VISIBLE != 0 {
                        if self_is_error
                            && cf & F_EXTRA == 0
                            && !(c_symbol == SYMBOL_ERROR && grandchild_count == 0)
                        {
                            error_cost = error_cost.wrapping_add(ERROR_COST_PER_SKIPPED_TREE);
                        }
                        visible_descendant_count = visible_descendant_count.wrapping_add(1);
                        visible_child_count = visible_child_count.wrapping_add(1);
                        if cf & F_NAMED != 0 {
                            named_child_count = named_child_count.wrapping_add(1);
                        }
                    } else if grandchild_count > 0 {
                        if self_is_error && cf & F_EXTRA == 0 {
                            error_cost = error_cost.wrapping_add(
                                ERROR_COST_PER_SKIPPED_TREE.wrapping_mul(ch.visible_child_count),
                            );
                        }
                        visible_child_count =
                            visible_child_count.wrapping_add(ch.visible_child_count);
                        named_child_count = named_child_count.wrapping_add(ch.named_child_count);
                    }
                }
            }
            Some(seq) => {
                for (i, &child) in children.iter().enumerate() {
                    // The child's fields, as ts_subtree_* would return them.
                    let c_padding;
                    let c_size;
                    let c_lookahead_bytes;
                    let c_symbol;
                    let mut c_error_cost = 0u32;
                    let mut grandchild_count = 0u32;
                    let mut c_visible_child_count = 0u32;
                    let mut c_named_child_count = 0u32;
                    let mut c_visible_descendant_count = 0u32;
                    let mut c_dynamic_precedence = 0i32;
                    let child_extra;
                    let child_visible;
                    let child_named;
                    if child.is_inline() {
                        let n = u32::from((child.0 >> INL_SIZE_BYTES_SHIFT) as u8);
                        c_padding = Length {
                            bytes: u32::from((child.0 >> INL_PADDING_BYTES_SHIFT) as u8),
                            extent: Point {
                                row: ((child.0 >> INL_PADDING_ROWS_SHIFT) as u32) & 0xF,
                                column: u32::from((child.0 >> INL_PADDING_COLUMNS_SHIFT) as u8),
                            },
                        };
                        c_size = Length {
                            bytes: n,
                            extent: Point { row: 0, column: n },
                        };
                        c_lookahead_bytes = ((child.0 >> INL_LOOKAHEAD_BYTES_SHIFT) as u32) & 0xF;
                        c_symbol = Symbol::from((child.0 >> INL_SYMBOL_SHIFT) as u8);
                        if child.0 & INL_IS_MISSING != 0 {
                            c_error_cost = ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
                        }
                        child_extra = child.0 & INL_EXTRA != 0;
                        child_visible = child.0 & INL_VISIBLE != 0;
                        child_named = child.0 & INL_NAMED != 0;
                    } else {
                        let ch = self.ptr(child);
                        let cf = ch.flags;
                        if size.extent.row == 0 && cf & F_DEPENDS_ON_COLUMN != 0 {
                            flags |= F_DEPENDS_ON_COLUMN;
                        }
                        flags |= cf
                            & (F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
                                | F_HAS_EXTERNAL_TOKENS
                                | F_HAS_REPEAT);

                        c_padding = ch.padding;
                        c_size = ch.size;
                        c_lookahead_bytes = ch.lookahead_bytes;
                        c_symbol = ch.symbol;
                        c_error_cost = ch.error_cost;
                        if cf & F_IS_MISSING != 0 {
                            c_error_cost = ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
                        }
                        grandchild_count = ch.child_count;
                        if grandchild_count > 0 {
                            c_visible_child_count = ch.visible_child_count;
                            c_named_child_count = ch.named_child_count;
                            c_visible_descendant_count = ch.visible_descendant_count;
                            c_dynamic_precedence = ch.dynamic_precedence;
                            kinds |= ch.kinds();
                        }
                        child_extra = cf & F_EXTRA != 0;
                        child_visible = cf & F_VISIBLE != 0;
                        child_named = cf & F_NAMED != 0;

                        if c_symbol == SYMBOL_ERROR {
                            flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
                            h.parse_state = TREE_STATE_NONE;
                        }
                    }

                    if i == 0 {
                        padding = c_padding;
                        size = c_size;
                    } else {
                        size = length_add(size, length_add(c_padding, c_size));
                    }

                    let child_lookahead_end_byte = padding
                        .bytes
                        .wrapping_add(size.bytes)
                        .wrapping_add(c_lookahead_bytes);
                    if child_lookahead_end_byte > lookahead_end_byte {
                        lookahead_end_byte = child_lookahead_end_byte;
                    }

                    if c_symbol != SYMBOL_ERROR_REPEAT {
                        error_cost = error_cost.wrapping_add(c_error_cost);
                    }

                    if self_is_error
                        && !child_extra
                        && !(c_symbol == SYMBOL_ERROR && grandchild_count == 0)
                    {
                        if child_visible {
                            error_cost = error_cost.wrapping_add(ERROR_COST_PER_SKIPPED_TREE);
                        } else if grandchild_count > 0 {
                            error_cost = error_cost.wrapping_add(
                                ERROR_COST_PER_SKIPPED_TREE.wrapping_mul(c_visible_child_count),
                            );
                        }
                    }

                    dynamic_precedence = dynamic_precedence.wrapping_add(c_dynamic_precedence);
                    visible_descendant_count =
                        visible_descendant_count.wrapping_add(c_visible_descendant_count);
                    kinds |= lang.kind_bit(c_symbol);

                    let alias = lang.tables.alias_sequences.get(seq + structural_index);
                    if alias != 0 && !child_extra {
                        kinds |= lang.kind_bit(alias);
                        visible_descendant_count = visible_descendant_count.wrapping_add(1);
                        visible_child_count = visible_child_count.wrapping_add(1);
                        if lang.symbol_metadata(alias).named {
                            named_child_count = named_child_count.wrapping_add(1);
                        }
                    } else if child_visible {
                        visible_descendant_count = visible_descendant_count.wrapping_add(1);
                        visible_child_count = visible_child_count.wrapping_add(1);
                        if child_named {
                            named_child_count = named_child_count.wrapping_add(1);
                        }
                    } else if grandchild_count > 0 {
                        visible_child_count =
                            visible_child_count.wrapping_add(c_visible_child_count);
                        named_child_count = named_child_count.wrapping_add(c_named_child_count);
                    }

                    if !child_extra {
                        structural_index += 1;
                    }
                }
            }
        }

        h.padding = padding;
        h.size = size;
        h.named_child_count = named_child_count;
        h.visible_child_count = visible_child_count;
        h.visible_descendant_count = visible_descendant_count;
        h.dynamic_precedence = dynamic_precedence;
        h.set_kinds(kinds);
        h.lookahead_bytes = lookahead_end_byte
            .wrapping_sub(size.bytes)
            .wrapping_sub(padding.bytes);

        if self_is_error {
            error_cost = error_cost
                .wrapping_add(ERROR_COST_PER_RECOVERY)
                .wrapping_add(ERROR_COST_PER_SKIPPED_CHAR.wrapping_mul(size.bytes))
                .wrapping_add(ERROR_COST_PER_SKIPPED_LINE.wrapping_mul(size.extent.row));
        }
        h.error_cost = error_cost;

        if h.child_count > 0 {
            let first_child = children[0];
            let last_child = children[h.child_count as usize - 1];

            h.first_leaf_symbol = self.leaf_symbol(first_child);
            h.first_leaf_parse_state = self.leaf_parse_state(first_child);

            if self.fragile_left(first_child) {
                flags |= F_FRAGILE_LEFT;
            }
            if self.fragile_right(last_child) {
                flags |= F_FRAGILE_RIGHT;
            }

            if h.child_count >= 2
                && flags & (F_VISIBLE | F_NAMED) == 0
                && self.symbol(first_child) == h.symbol
            {
                // uint16(...) in Go: truncates.
                if self.repeat_depth(first_child) > self.repeat_depth(last_child) {
                    h.repeat_depth = (self.repeat_depth(first_child) + 1) as u16;
                } else {
                    h.repeat_depth = (self.repeat_depth(last_child) + 1) as u16;
                }
                flags |= F_HAS_REPEAT;
            }
        }
        h.flags = flags;
        *self.ptr_mut(tree) = h;
    }

    /// summarizeOneChild: summarize_children for a node new_node has just made, with one
    /// child and not an error symbol: half of all reduces. Each line is the loop of
    /// summarize_children with one turn and the sums folded. Not in C. h is not a pool
    /// entry: new_node builds it before storing it. alias_sequence is
    /// `Language::alias_sequence` of the production.
    pub(crate) fn summarize_one_child(
        &self,
        h: &mut SubtreeHeap,
        mut flags: u16,
        alias_sequence: Option<usize>,
        lang: &Language,
    ) {
        let child = self.children[h.children as usize];
        let mut visible_child_count = 0u32;
        let mut named_child_count = 0u32;
        let mut kinds;
        let child_extra;
        let child_visible;
        let child_named;
        if child.is_inline() {
            let n = u32::from((child.0 >> INL_SIZE_BYTES_SHIFT) as u8);
            h.padding = Length {
                bytes: u32::from((child.0 >> INL_PADDING_BYTES_SHIFT) as u8),
                extent: Point {
                    row: ((child.0 >> INL_PADDING_ROWS_SHIFT) as u32) & 0xF,
                    column: u32::from((child.0 >> INL_PADDING_COLUMNS_SHIFT) as u8),
                },
            };
            h.size = Length {
                bytes: n,
                extent: Point { row: 0, column: n },
            };
            h.lookahead_bytes = ((child.0 >> INL_LOOKAHEAD_BYTES_SHIFT) as u32) & 0xF;
            h.error_cost = 0;
            if child.0 & INL_IS_MISSING != 0 {
                h.error_cost = ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
            }
            h.visible_descendant_count = 0;
            h.dynamic_precedence = 0;
            h.first_leaf_symbol = Symbol::from((child.0 >> INL_SYMBOL_SHIFT) as u8);
            h.first_leaf_parse_state = (child.0 >> INL_PARSE_STATE_SHIFT) as u16;
            kinds = lang.kind_bit(h.first_leaf_symbol);
            child_extra = child.0 & INL_EXTRA != 0;
            child_visible = child.0 & INL_VISIBLE != 0;
            child_named = child.0 & INL_NAMED != 0;
        } else {
            let ch = self.ptr(child);
            let cf = ch.flags;
            if h.size.extent.row == 0 && cf & F_DEPENDS_ON_COLUMN != 0 {
                flags |= F_DEPENDS_ON_COLUMN;
            }
            flags |= cf
                & (F_HAS_EXTERNAL_SCANNER_STATE_CHANGE
                    | F_HAS_EXTERNAL_TOKENS
                    | F_HAS_REPEAT
                    | F_FRAGILE_LEFT
                    | F_FRAGILE_RIGHT);
            if ch.symbol == SYMBOL_ERROR {
                flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
                h.parse_state = TREE_STATE_NONE;
            }

            h.padding = ch.padding;
            h.size = ch.size;
            h.lookahead_bytes = ch.lookahead_bytes;
            h.error_cost = ch.error_cost;
            if cf & F_IS_MISSING != 0 {
                h.error_cost = ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY;
            }
            if ch.symbol == SYMBOL_ERROR_REPEAT {
                h.error_cost = 0;
            }
            kinds = lang.kind_bit(ch.symbol);
            if ch.child_count > 0 {
                kinds |= ch.kinds();
                visible_child_count = ch.visible_child_count;
                named_child_count = ch.named_child_count;
                h.visible_descendant_count = ch.visible_descendant_count;
                h.dynamic_precedence = ch.dynamic_precedence;
                h.first_leaf_symbol = ch.first_leaf_symbol;
                h.first_leaf_parse_state = ch.first_leaf_parse_state;
            } else {
                h.visible_descendant_count = 0;
                h.dynamic_precedence = 0;
                h.first_leaf_symbol = ch.symbol;
                h.first_leaf_parse_state = ch.parse_state;
            }
            child_extra = cf & F_EXTRA != 0;
            child_visible = cf & F_VISIBLE != 0;
            child_named = cf & F_NAMED != 0;
        }

        let alias = alias_sequence.map_or(0, |seq| lang.tables.alias_sequences.get(seq));
        if alias != 0 && !child_extra {
            kinds |= lang.kind_bit(alias);
            h.visible_descendant_count = h.visible_descendant_count.wrapping_add(1);
            visible_child_count = 1;
            named_child_count = 0;
            if lang.symbol_metadata(alias).named {
                named_child_count = 1;
            }
        } else if child_visible {
            h.visible_descendant_count = h.visible_descendant_count.wrapping_add(1);
            visible_child_count = 1;
            named_child_count = 0;
            if child_named {
                named_child_count = 1;
            }
        }
        h.visible_child_count = visible_child_count;
        h.named_child_count = named_child_count;
        h.set_kinds(kinds);
        h.flags = flags;
    }

    /// ts_subtree_new_node. Takes ownership of the children region. C puts the node's heap
    /// data behind the children in the same allocation; here the region becomes the node's
    /// children block. At the end of the slab it is cut to the class of its size; elsewhere
    /// it keeps its capacity, as in C.
    pub(crate) fn new_node(
        &mut self,
        symbol: Symbol,
        children: &mut SubtreeArray,
        production_id: u32,
        lang: &Language,
    ) -> Subtree {
        let metadata = lang.symbol_metadata(symbol);
        let fragile = symbol == SYMBOL_ERROR || symbol == SYMBOL_ERROR_REPEAT;

        let mut class = 0u32;
        if children.size == 0 {
            self.array_free(children);
        } else if (children.off + children.cap) as usize == self.children.len() {
            let size;
            (class, size) = child_class(children.size);
            if size <= children.cap {
                self.children.truncate((children.off + size) as usize);
            } else {
                let room = self.room(size - children.cap);
                self.bump_children(room, size - children.cap);
            }
        } else {
            (class, _) = child_class(children.cap);
        }

        let mut flags = 0u16;
        if metadata.visible {
            flags |= F_VISIBLE;
        }
        if metadata.named {
            flags |= F_NAMED;
        }
        if fragile {
            flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
        }
        let i = self.allocate();
        let result = Subtree(u64::from(i) << 1);
        let mut h = SubtreeHeap {
            ref_count: 1,
            symbol,
            child_count: children.size,
            flags,
            production_id: production_id as u16,
            children: children.off,
            child_class: class as u8,
            ..SubtreeHeap::default()
        };
        if children.size == 1 && !fragile {
            self.summarize_one_child(&mut h, flags, lang.alias_sequence(production_id), lang);
            *self.ptr_mut(result) = h;
        } else {
            *self.ptr_mut(result) = h;
            self.summarize(result, lang);
        }
        result
    }

    /// ts_subtree_new_error_node
    pub(crate) fn new_error_node(
        &mut self,
        children: &mut SubtreeArray,
        extra: bool,
        lang: &Language,
    ) -> Subtree {
        let result = self.new_node(SYMBOL_ERROR, children, 0, lang);
        self.ptr_mut(result).set_flag(F_EXTRA, extra);
        result
    }

    /// ts_subtree_new_missing_leaf
    pub(crate) fn new_missing_leaf(
        &mut self,
        symbol: Symbol,
        padding: Length,
        lookahead_bytes: u32,
        lang: &Language,
    ) -> Subtree {
        let result = self.new_leaf(
            symbol,
            padding,
            Length::default(),
            lookahead_bytes,
            0,
            false,
            false,
            false,
            lang,
        );
        if result.is_inline() {
            return Subtree(result.0 | INL_IS_MISSING);
        }
        self.ptr_mut(result).flags |= F_IS_MISSING;
        result
    }

    /// ts_subtree_retain
    pub(crate) fn retain(&mut self, s: Subtree) {
        if s.is_inline() {
            return;
        }
        self.ptr_mut(s).ref_count += 1;
    }

    /// ts_subtree_release
    pub(crate) fn release(&mut self, s: Subtree) {
        if s.is_inline() {
            return;
        }
        self.tree_stack.clear();

        let h = self.ptr_mut(s);
        h.ref_count -= 1;
        if h.ref_count == 0 {
            self.tree_stack.push(s);
        }

        while let Some(tree) = self.tree_stack.pop() {
            let th = self.ptr(tree);
            let (off, count, class) = (th.children, th.child_count, u32::from(th.child_class));
            if count > 0 {
                for j in off..off + count {
                    let child = self.children[j as usize];
                    if child.is_inline() {
                        continue;
                    }
                    let ch = self.ptr_mut(child);
                    ch.ref_count -= 1;
                    if ch.ref_count == 0 {
                        self.tree_stack.push(child);
                    }
                }
                if class <= EXACT_CHILD_CLASSES {
                    self.free_quick(off, class);
                } else {
                    self.merge_free(off, child_class_size(class));
                }
            }
            // C frees the children block (which holds a parent's heap data) or returns a
            // leaf to the pool.
            self.free((tree.0 >> 1) as u32);
        }
    }

    /// ts_subtree_compare
    pub(crate) fn compare(&mut self, left: Subtree, right: Subtree) -> i32 {
        self.tree_stack.push(left);
        self.tree_stack.push(right);

        while let Some(right) = self.tree_stack.pop() {
            let left = self.tree_stack.pop().unwrap();

            let mut result = 0;
            if self.symbol(left) < self.symbol(right) {
                result = -1;
            } else if self.symbol(right) < self.symbol(left) {
                result = 1;
            } else if self.child_count(left) < self.child_count(right) {
                result = -1;
            } else if self.child_count(right) < self.child_count(left) {
                result = 1;
            }
            if result != 0 {
                self.tree_stack.clear();
                return result;
            }

            // The child counts are equal here, and 0 for an inline subtree.
            let n = self.child_count(left);
            if n > 0 {
                let (lo, ro) = (self.ptr(left).children, self.ptr(right).children);
                for i in (0..n).rev() {
                    let (l, r) = (
                        self.children[(lo + i) as usize],
                        self.children[(ro + i) as usize],
                    );
                    self.tree_stack.push(l);
                    self.tree_stack.push(r);
                }
            }
        }

        0
    }

    /// ts_subtree_last_external_token
    pub(crate) fn last_external_token(&self, tree: Subtree) -> Subtree {
        if !self.has_external_tokens(tree) {
            return NULL_SUBTREE;
        }
        let mut tree = tree;
        while self.ptr(tree).child_count > 0 {
            for &child in self.children_of(tree).iter().rev() {
                if self.has_external_tokens(child) {
                    tree = child;
                    break;
                }
            }
        }
        tree
    }
}

/// ts_subtree__write_char_to_string
pub(crate) fn write_char_to_string(b: &mut Vec<u8>, chr: i32) {
    match chr {
        -1 => b.extend_from_slice(b"INVALID"),
        0 => b.extend_from_slice(br"'\0'"),
        0x0A => b.extend_from_slice(br"'\n'"),
        0x09 => b.extend_from_slice(br"'\t'"),
        0x0D => b.extend_from_slice(br"'\r'"),
        // 0 < chr < 128 && isprint(chr)
        32..=126 => b.extend_from_slice(&[b'\'', chr as u8, b'\'']),
        _ => b.extend_from_slice(chr.to_string().as_bytes()),
    }
}

impl SubtreePool {
    /// ts_subtree__write_to_string. field_name "" stands for NULL; is_root for the
    /// ROOT_FIELD sentinel.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn write_to_string(
        &self,
        b: &mut Vec<u8>,
        tree: Subtree,
        lang: &Language,
        include_all: bool,
        alias_symbol: Symbol,
        alias_is_named: bool,
        field_name: &str,
        is_root: bool,
    ) {
        if tree == NULL_SUBTREE {
            b.extend_from_slice(b"(NULL)");
            return;
        }

        let mut is_visible = include_all || self.missing(tree);
        if !is_visible {
            if alias_symbol != 0 {
                is_visible = alias_is_named;
            } else {
                is_visible = self.visible(tree) && self.named(tree);
            }
        }

        if is_visible {
            if !is_root {
                b.push(b' ');
                if !field_name.is_empty() {
                    b.extend_from_slice(field_name.as_bytes());
                    b.extend_from_slice(b": ");
                }
            }

            if self.is_error(tree) && self.child_count(tree) == 0 && self.ptr(tree).size.bytes > 0 {
                b.extend_from_slice(b"(UNEXPECTED ");
                write_char_to_string(b, self.ptr(tree).lookahead_char);
            } else {
                let symbol = if alias_symbol != 0 {
                    alias_symbol
                } else {
                    self.symbol(tree)
                };
                let symbol_name = lang.symbol_name(symbol).unwrap_or("");
                if self.missing(tree) {
                    b.extend_from_slice(b"(MISSING ");
                    if alias_is_named || self.named(tree) {
                        b.extend_from_slice(symbol_name.as_bytes());
                    } else {
                        b.push(b'"');
                        b.extend_from_slice(symbol_name.as_bytes());
                        b.push(b'"');
                    }
                } else {
                    b.push(b'(');
                    b.extend_from_slice(symbol_name.as_bytes());
                }
            }
        } else if is_root {
            let symbol = if alias_symbol != 0 {
                alias_symbol
            } else {
                self.symbol(tree)
            };
            let symbol_name = lang.symbol_name(symbol).unwrap_or("");
            if self.child_count(tree) > 0 {
                b.push(b'(');
                b.extend_from_slice(symbol_name.as_bytes());
            } else if self.named(tree) {
                b.push(b'(');
                b.extend_from_slice(symbol_name.as_bytes());
                b.push(b')');
            } else {
                b.extend_from_slice(b"(\"");
                b.extend_from_slice(symbol_name.as_bytes());
                b.extend_from_slice(b"\")");
            }
        }

        let child_count = self.child_count(tree);
        if child_count != 0 {
            let h = self.ptr(tree);
            let children_off = h.children;
            let alias_sequence = lang.alias_sequence(u32::from(h.production_id));
            let (field_map_start, field_map_len) = lang.field_map(u32::from(h.production_id));

            let mut structural_child_index = 0u32;
            for i in 0..child_count {
                let child = self.children[(children_off + i) as usize];
                if self.extra(child) {
                    self.write_to_string(b, child, lang, include_all, 0, false, "", false);
                } else {
                    let subtree_alias_symbol = alias_sequence.map_or(0, |seq| {
                        lang.tables
                            .alias_sequences
                            .get(seq + structural_child_index as usize)
                    });
                    let subtree_alias_is_named = subtree_alias_symbol != 0
                        && lang.symbol_metadata(subtree_alias_symbol).named;

                    let mut child_field_name = if is_visible { "" } else { field_name };
                    // An invisible root hands ROOT_FIELD down, so its child is printed as a
                    // root too.
                    let mut child_is_root_field = is_root && !is_visible;
                    for k in field_map_start..field_map_start + field_map_len {
                        let m = lang.tables.field_map_entries.get(k);
                        if !m.inherited && u32::from(m.child_index) == structural_child_index {
                            child_field_name = lang.tables.field_names[m.field_id as usize];
                            child_is_root_field = false;
                            break;
                        }
                    }

                    self.write_to_string(
                        b,
                        child,
                        lang,
                        include_all,
                        subtree_alias_symbol,
                        subtree_alias_is_named,
                        child_field_name,
                        child_is_root_field,
                    );
                    structural_child_index += 1;
                }
            }
        }

        if is_visible {
            b.push(b')');
        }
    }

    /// ts_subtree_string
    pub(crate) fn string(
        &self,
        tree: Subtree,
        alias_symbol: Symbol,
        alias_is_named: bool,
        lang: &Language,
        include_all: bool,
    ) -> String {
        let mut b = Vec::new();
        self.write_to_string(
            &mut b,
            tree,
            lang,
            include_all,
            alias_symbol,
            alias_is_named,
            "",
            true,
        );
        // Every byte written is ASCII or from a UTF-8 name.
        String::from_utf8_lossy(&b).into_owned()
    }
}

#[cfg(test)]
mod tests_alloc;
#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_random;
