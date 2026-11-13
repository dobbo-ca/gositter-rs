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

use crate::length::Length;
use crate::{Language, StateId, Symbol};

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
        todo!("gositter-rs-2f0.8")
    }

    /// setKinds
    pub(crate) fn set_kinds(&mut self, _mask: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// ext: the external scanner state range (off, n) of an external terminal subtree
    /// (child_count == 0 && F_HAS_EXTERNAL_TOKENS).
    pub(crate) fn ext(&self) -> (u32, u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// setExt
    pub(crate) fn set_ext(&mut self, _off: u32, _n: u32) {
        todo!("gositter-rs-2f0.8")
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

/// A reset keeps KEPT_HEAP_CHUNKS chunks and drops the rest.
pub(crate) const KEPT_HEAP_CHUNKS: usize = 8;

/// A reset shrinks a children slab above SHRINK_CHILDREN entries to its initial size when
/// the parse before it used under an eighth of it.
pub(crate) const SHRINK_CHILDREN: usize = 1 << 16;

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
/// per doubling.
pub(crate) const EXACT_CHILD_CLASSES: u32 = 64;
pub(crate) const CHILD_CLASS_COUNT: u32 = 165;

/// errSlabFull: offsets and indexes into the slabs are 32 bits.
pub(crate) const ERR_SLAB_FULL: &str =
    "gositter: input needs a parser slab of more than 2^32 entries";

/// childClass: the class of the smallest block that holds n entries, and the size of that
/// block.
pub(crate) fn child_class(_n: u32) -> (u32, u32) {
    todo!("gositter-rs-2f0.8")
}

/// childClassSize: the block size of a class above EXACT_CHILD_CLASSES.
pub(crate) fn child_class_size(_class: u32) -> u32 {
    todo!("gositter-rs-2f0.8")
}

impl Default for SubtreePool {
    /// The Go zero value `subtreePool{}`; `reset` makes it usable.
    fn default() -> Self {
        todo!("gositter-rs-2f0.8")
    }
}

impl Subtree {
    /// isInline
    pub(crate) fn is_inline(self) -> bool {
        todo!("gositter-rs-2f0.8")
    }
}

impl SubtreePool {
    /// reset
    pub(crate) fn reset(&mut self) {
        todo!("gositter-rs-2f0.8")
    }

    /// ptr
    pub(crate) fn ptr(&self, _s: Subtree) -> &SubtreeHeap {
        todo!("gositter-rs-2f0.8")
    }

    /// ptr, for writing.
    pub(crate) fn ptr_mut(&mut self, _s: Subtree) -> &mut SubtreeHeap {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_pool_allocate
    pub(crate) fn allocate(&mut self) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_pool_free
    pub(crate) fn free(&mut self, _i: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_external_scanner_state. Empty for anything but an external token leaf.
    pub(crate) fn external_scanner_state(&self, _s: Subtree) -> &[u8] {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_external_scanner_state_eq
    pub(crate) fn external_scanner_state_eq(&self, _s: Subtree, _other: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// array
    pub(crate) fn array(&self, _a: SubtreeArray) -> &[Subtree] {
        todo!("gositter-rs-2f0.8")
    }

    /// room: makes space for n slots at the end of the children slab and returns where they
    /// start; bump_children(room(n), n).
    pub(crate) fn room(&mut self, _n: u32) -> usize {
        todo!("gositter-rs-2f0.8")
    }

    /// bumpChildren
    pub(crate) fn bump_children(&mut self, _off: usize, _n: u32) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// growChildren. The capacity never passes 2^32-1 entries.
    pub(crate) fn grow_children(&mut self, _n: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// growFreeBits: sizes the free bits to the slab.
    pub(crate) fn grow_free_bits(&mut self) {
        todo!("gositter-rs-2f0.8")
    }

    /// settleChildren: moves a region at the end of the slab, as pop_linear leaves it, into a
    /// free block.
    pub(crate) fn settle_children(&mut self, _a: &mut SubtreeArray) {
        todo!("gositter-rs-2f0.8")
    }

    /// takeFree: a free block of at least size entries, cut to size, or 0.
    pub(crate) fn take_free(&mut self, _class: u32, _size: u32) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// takeMerged
    pub(crate) fn take_merged(&mut self, _class: u32, _size: u32) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// flushQuick
    pub(crate) fn flush_quick(&mut self) {
        todo!("gositter-rs-2f0.8")
    }
}

/// freeList: the list of a free block.
pub(crate) fn free_list(_size: u32) -> u32 {
    todo!("gositter-rs-2f0.8")
}

impl SubtreePool {
    /// linkFree
    pub(crate) fn link_free(&mut self, _off: u32, _size: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// unlinkFree: takes a free block out of its list and returns its size.
    pub(crate) fn unlink_free(&mut self, _off: u32) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// allocChildren: a block for n > 0 entries, (off, size). C calls malloc here.
    pub(crate) fn alloc_children(&mut self, _n: u32) -> (u32, u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// freeQuick: gives back a block of up to EXACT_CHILD_CLASSES entries.
    pub(crate) fn free_quick(&mut self, _off: u32, _size: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// mergeFree: gives back a block of any size, merged with its free neighbours.
    pub(crate) fn merge_free(&mut self, _off: u32, _size: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// array_reserve
    pub(crate) fn array_reserve(&mut self, _a: &mut SubtreeArray, _new_cap: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// _array__grow
    pub(crate) fn array_grow(&mut self, _a: &mut SubtreeArray, _count: u32) {
        todo!("gositter-rs-2f0.8")
    }

    /// array_push
    pub(crate) fn array_push(&mut self, _a: &mut SubtreeArray, _s: Subtree) {
        todo!("gositter-rs-2f0.8")
    }

    /// array_splice. The elements are the slab entries `[elements_off, elements_off +
    /// elements_len)` (Go passes a slice that may alias the slab; every caller passes
    /// children_of of a live node).
    pub(crate) fn array_splice(
        &mut self,
        _a: &mut SubtreeArray,
        _index: u32,
        _old_count: u32,
        _elements_off: u32,
        _elements_len: u32,
    ) {
        todo!("gositter-rs-2f0.8")
    }

    /// array_delete for a region.
    pub(crate) fn array_free(&mut self, _a: &mut SubtreeArray) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_array_copy
    pub(crate) fn array_copy(&mut self, _array: SubtreeArray) -> SubtreeArray {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_array_delete
    pub(crate) fn array_delete(&mut self, _a: &mut SubtreeArray) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_array_remove_trailing_extras. destination is a parser-owned scratch vector
    /// (trailing_extras).
    pub(crate) fn array_remove_trailing_extras(
        &self,
        _a: &mut SubtreeArray,
        _destination: &mut Vec<Subtree>,
    ) {
        todo!("gositter-rs-2f0.8")
    }
}

/// ts_subtree_array_reverse
pub(crate) fn subtree_slice_reverse(_s: &mut [Subtree]) {
    todo!("gositter-rs-2f0.8")
}

// Subtree accessors (subtree.h).

impl SubtreePool {
    /// ts_subtree_symbol
    pub(crate) fn symbol(&self, _s: Subtree) -> Symbol {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_visible
    pub(crate) fn visible(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_named
    pub(crate) fn named(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_extra
    pub(crate) fn extra(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_missing
    pub(crate) fn missing(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_is_keyword
    pub(crate) fn is_keyword(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_parse_state
    pub(crate) fn parse_state(&self, _s: Subtree) -> StateId {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_lookahead_bytes
    pub(crate) fn lookahead_bytes(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_children
    pub(crate) fn children_of(&self, _s: Subtree) -> &[Subtree] {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_set_extra
    pub(crate) fn set_extra(&mut self, _s: &mut Subtree, _is_extra: bool) {
        todo!("gositter-rs-2f0.8")
    }
}

impl SubtreeHeap {
    /// setFlag
    pub(crate) fn set_flag(&mut self, _f: u16, _on: bool) {
        todo!("gositter-rs-2f0.8")
    }
}

impl SubtreePool {
    /// ts_subtree_leaf_symbol
    pub(crate) fn leaf_symbol(&self, _s: Subtree) -> Symbol {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_leaf_parse_state
    pub(crate) fn leaf_parse_state(&self, _s: Subtree) -> StateId {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_padding
    pub(crate) fn padding(&self, _s: Subtree) -> Length {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_size
    pub(crate) fn size(&self, _s: Subtree) -> Length {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_total_size
    pub(crate) fn total_size(&self, _s: Subtree) -> Length {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_total_bytes
    pub(crate) fn total_bytes(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_child_count
    pub(crate) fn child_count(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_repeat_depth
    pub(crate) fn repeat_depth(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_visible_descendant_count
    pub(crate) fn visible_descendant_count(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_visible_child_count
    pub(crate) fn visible_child_count(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_error_cost
    pub(crate) fn error_cost(&self, _s: Subtree) -> u32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_dynamic_precedence
    pub(crate) fn dynamic_precedence(&self, _s: Subtree) -> i32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_fragile_left
    pub(crate) fn fragile_left(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_fragile_right
    pub(crate) fn fragile_right(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_has_external_tokens
    pub(crate) fn has_external_tokens(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_has_external_scanner_state_change
    pub(crate) fn has_external_scanner_state_change(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_depends_on_column
    pub(crate) fn depends_on_column(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_is_error
    pub(crate) fn is_error(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_is_eof
    pub(crate) fn is_eof(&self, _s: Subtree) -> bool {
        todo!("gositter-rs-2f0.8")
    }
}

// Subtree constructors (subtree.c).

/// ts_subtree_can_inline
pub(crate) fn subtree_can_inline(_padding: Length, _size: Length, _lookahead_bytes: u32) -> bool {
    todo!("gositter-rs-2f0.8")
}

impl SubtreePool {
    /// ts_subtree_new_leaf
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_leaf(
        &mut self,
        _symbol: Symbol,
        _padding: Length,
        _size: Length,
        _lookahead_bytes: u32,
        _parse_state: StateId,
        _has_external_tokens: bool,
        _depends_on_column: bool,
        _is_keyword: bool,
        _lang: &Language,
    ) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_set_symbol
    pub(crate) fn set_symbol(&mut self, _s: &mut Subtree, _symbol: Symbol, _lang: &Language) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_new_error
    pub(crate) fn new_error(
        &mut self,
        _lookahead_char: i32,
        _padding: Length,
        _size: Length,
        _bytes_scanned: u32,
        _parse_state: StateId,
        _lang: &Language,
    ) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_clone
    pub(crate) fn clone(&mut self, _s: Subtree) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_make_mut. Takes ownership; copies if there is another owner.
    pub(crate) fn make_mut(&mut self, _s: Subtree) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_compress, but for the summary of tree where the next turn makes it again.
    /// Go takes the stack as `*[]subtree` and every caller passes `&sp.treeStack`, so this
    /// works on `self.tree_stack`.
    pub(crate) fn compress(&mut self, _tree: Subtree, _count: u32, _lang: &Language) {
        todo!("gositter-rs-2f0.8")
    }

    /// summarize. Not in C: summarize_children, with the most common shape of a node of two
    /// or more children on its own path.
    pub(crate) fn summarize(&mut self, _tree: Subtree, _lang: &Language) {
        todo!("gositter-rs-2f0.8")
    }

    /// summarizeTwoHeap: summarize_children for two heap children a and b, no alias sequence
    /// and not an error symbol. Not in C. Takes no pool: the caller hands it the three heap
    /// entries.
    pub(crate) fn summarize_two_heap(
        _h: &mut SubtreeHeap,
        _a: &SubtreeHeap,
        _b: &SubtreeHeap,
        _lang: &Language,
    ) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_summarize_children
    pub(crate) fn summarize_children(&mut self, _tree: Subtree, _lang: &Language) {
        todo!("gositter-rs-2f0.8")
    }

    /// summarizeOneChild: summarize_children for a node new_node has just made, with one
    /// child and not an error symbol. Not in C. h is not a pool entry: new_node builds it
    /// before storing it. alias_sequence is `Language::alias_sequence` of the production.
    pub(crate) fn summarize_one_child(
        &self,
        _h: &mut SubtreeHeap,
        _flags: u16,
        _alias_sequence: Option<usize>,
        _lang: &Language,
    ) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_new_node. Takes ownership of the children region.
    pub(crate) fn new_node(
        &mut self,
        _symbol: Symbol,
        _children: &mut SubtreeArray,
        _production_id: u32,
        _lang: &Language,
    ) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_new_error_node
    pub(crate) fn new_error_node(
        &mut self,
        _children: &mut SubtreeArray,
        _extra: bool,
        _lang: &Language,
    ) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_new_missing_leaf
    pub(crate) fn new_missing_leaf(
        &mut self,
        _symbol: Symbol,
        _padding: Length,
        _lookahead_bytes: u32,
        _lang: &Language,
    ) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_retain
    pub(crate) fn retain(&mut self, _s: Subtree) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_release
    pub(crate) fn release(&mut self, _s: Subtree) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_compare
    pub(crate) fn compare(&mut self, _left: Subtree, _right: Subtree) -> i32 {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_last_external_token
    pub(crate) fn last_external_token(&self, _tree: Subtree) -> Subtree {
        todo!("gositter-rs-2f0.8")
    }
}

/// ts_subtree__write_char_to_string
pub(crate) fn write_char_to_string(_b: &mut Vec<u8>, _chr: i32) {
    todo!("gositter-rs-2f0.8")
}

impl SubtreePool {
    /// ts_subtree__write_to_string. field_name "" stands for NULL; is_root for the
    /// ROOT_FIELD sentinel.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn write_to_string(
        &self,
        _b: &mut Vec<u8>,
        _tree: Subtree,
        _lang: &Language,
        _include_all: bool,
        _alias_symbol: Symbol,
        _alias_is_named: bool,
        _field_name: &str,
        _is_root: bool,
    ) {
        todo!("gositter-rs-2f0.8")
    }

    /// ts_subtree_string
    pub(crate) fn string(
        &self,
        _tree: Subtree,
        _alias_symbol: Symbol,
        _alias_is_named: bool,
        _lang: &Language,
        _include_all: bool,
    ) -> String {
        todo!("gositter-rs-2f0.8")
    }
}

#[cfg(test)]
mod tests_alloc;
#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_random;
