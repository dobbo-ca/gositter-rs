//! stack.c (gositter `stack.go`): the graph-structured stack. Nodes live in a slab and are
//! addressed by index; 0 is the NULL node.
//!
//! Not ported (Go GC workarounds, plan Global Constraints): all of `slabpool.go` but the
//! shrink rule (`slabPool`, `keptSlab`, `nodeSlabs`, `linkSlabs`, `resets`, `keepResets`,
//! `keepBytes`), `growNodes` and `growLinks` (a `Vec` doubles), and the stack fields
//! `nodesBox`, `linksBox`, `nodesOut`, `linksOut`. The slabs are per-parser `Vec`s that
//! `reset` truncates, shrunk by the rule of `shrink_slab`. The stack holds no pointer to the
//! subtree pool: a function that needs it takes `sp`.

use crate::StateId;
use crate::length::Length;
use crate::subtree::{Subtree, SubtreeArray, SubtreePool};

/// MAX_LINK_COUNT
pub(crate) const MAX_LINK_COUNT: u16 = 8;
/// MAX_ITERATOR_COUNT
pub(crate) const MAX_ITERATOR_COUNT: usize = 64;

/// StackVersion
pub(crate) type StackVersion = u32;
/// STACK_VERSION_NONE
pub(crate) const STACK_VERSION_NONE: StackVersion = u32::MAX;

/// StackLink. Field order differs from C: 16 bytes instead of 24.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackLink {
    pub(crate) subtree: Subtree,
    pub(crate) node: u32,
    pub(crate) is_pending: bool,
}

/// StackNode. 48 bytes: most nodes have one link, so links 1 to 7 are kept in
/// `Stack.more_links`. Use `Stack::link` for any link but `links[0]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackNode {
    pub(crate) state: StateId,
    pub(crate) link_count: u16,
    pub(crate) ref_count: u32,
    pub(crate) error_cost: u32,
    pub(crate) node_count: u32,
    pub(crate) dynamic_precedence: i32,
    pub(crate) position: Length,
    pub(crate) links: [StackLink; 1],
}

const _: () = assert!(size_of::<StackNode>() == 48);

/// moreLinkSet: links 1 to 7 of one stack node.
pub(crate) type MoreLinkSet = [StackLink; MAX_LINK_COUNT as usize - 1];

/// StackIterator
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackIterator {
    pub(crate) node: u32,
    pub(crate) subtrees: SubtreeArray,
    pub(crate) subtree_count: u32,
    pub(crate) is_pending: bool,
    /// Index into `Stack.forks`, pop_all only.
    pub(crate) path: u32,
}

/// stackFork: where a path of pop_all left the path it was copied from: at node, by
/// links[link]. Entry 0 is the path that starts at the head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackFork {
    pub(crate) parent: u32,
    pub(crate) node: u32,
    pub(crate) link: u16,
}

/// StackStatus: active.
pub(crate) const STACK_STATUS_ACTIVE: u8 = 0;
/// StackStatus: paused.
pub(crate) const STACK_STATUS_PAUSED: u8 = 1;
/// StackStatus: halted.
pub(crate) const STACK_STATUS_HALTED: u8 = 2;

/// StackSummaryEntry
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackSummaryEntry {
    pub(crate) position: Length,
    pub(crate) depth: u32,
    pub(crate) state: StateId,
}

/// StackHead. summary is None where C has NULL; a recorded summary always has at least the
/// entry for the head node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackHead {
    pub(crate) node: u32,
    pub(crate) summary: Option<Vec<StackSummaryEntry>>,
    pub(crate) node_count_at_last_error: u32,
    pub(crate) last_external_token: Subtree,
    pub(crate) lookahead_when_paused: Subtree,
    pub(crate) status: u8,
}

/// StackSlice
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StackSlice {
    pub(crate) subtrees: SubtreeArray,
    pub(crate) version: StackVersion,
}

/// StackAction: none.
pub(crate) const STACK_ACTION_NONE: u32 = 0;
/// StackAction: stop.
pub(crate) const STACK_ACTION_STOP: u32 = 1;
/// StackAction: pop.
pub(crate) const STACK_ACTION_POP: u32 = 2;

/// The StackCallback functions. C passes a function pointer and a payload; here iter
/// switches on the callback id and the payload sits in the stack. pop_count_callback.
pub(crate) const CB_POP_COUNT: u8 = 0;
/// pop_pending_callback
pub(crate) const CB_POP_PENDING: u8 = 1;
/// pop_error_callback
pub(crate) const CB_POP_ERROR: u8 = 2;
/// summarize_stack_callback
pub(crate) const CB_SUMMARIZE: u8 = 3;

/// slabpool.go shrinkNodes: a reset shrinks a node slab above this many entries that the
/// parse before used under an eighth of.
pub(crate) const SHRINK_NODES: usize = 1 << 14;
/// slabpool.go shrinkLinks: the same for the more_links slab.
pub(crate) const SHRINK_LINKS: usize = 1 << 12;
/// slabpool.go shrinkIndexes: the same for the u32 slabs node_pool and more.
pub(crate) const SHRINK_INDEXES: usize = 1 << 20;

/// shrinkSlab: a slab of capacity c of which n entries were used shrinks.
pub(crate) fn shrink_slab(_c: usize, _n: usize, _max: usize) -> bool {
    todo!("gositter-rs-2f0.9")
}

/// Stack
#[derive(Debug, Default)]
pub(crate) struct Stack {
    pub(crate) heads: Vec<StackHead>,
    pub(crate) slices: Vec<StackSlice>,
    pub(crate) iterators: Vec<StackIterator>,
    /// pop_all
    pub(crate) forks: Vec<StackFork>,
    /// pop_all_subtrees scratch
    pub(crate) chain: Vec<u32>,
    /// Entry 0 unused.
    pub(crate) nodes: Vec<StackNode>,
    /// Links 1 to 7 of the nodes that have them: `more_links[more[node]]`, where 0 means
    /// none yet. A node index keeps its entry until reset.
    pub(crate) more: Vec<u32>,
    /// Entry 0 unused.
    pub(crate) more_links: Vec<MoreLinkSet>,
    /// node_pool
    pub(crate) node_pool: Vec<u32>,
    pub(crate) base_node: u32,

    // Callback payloads.
    /// pop_count_callback
    pub(crate) goal_subtree_count: u32,
    /// pop_error_callback
    pub(crate) found_error: bool,
    /// SummarizeStackSession
    pub(crate) summary: Vec<StackSummaryEntry>,
    /// SummarizeStackSession
    pub(crate) max_depth: u32,
    /// Freed summaries, for reuse.
    pub(crate) summary_pool: Vec<Vec<StackSummaryEntry>>,
}

impl Stack {
    /// link: links[i] of node, for i < link_count.
    pub(crate) fn link(&self, _node: u32, _i: usize) -> &StackLink {
        todo!("gositter-rs-2f0.9")
    }

    /// link, for writing: links[i] of node, for i < link_count.
    pub(crate) fn link_mut(&mut self, _node: u32, _i: usize) -> &mut StackLink {
        todo!("gositter-rs-2f0.9")
    }

    /// moreLink: the slot of links[i], i > 0, for writing link i.
    pub(crate) fn more_link(&mut self, _node: u32, _i: usize) -> &mut StackLink {
        todo!("gositter-rs-2f0.9")
    }

    /// stack_node_retain
    pub(crate) fn node_retain(&mut self, _n: u32) {
        todo!("gositter-rs-2f0.9")
    }

    /// stack_node_release
    pub(crate) fn node_release(&mut self, _sp: &mut SubtreePool, _node: u32) {
        todo!("gositter-rs-2f0.9")
    }

    /// stack__subtree_node_count
    pub(crate) fn subtree_node_count(&self, _sp: &SubtreePool, _t: Subtree) -> u32 {
        todo!("gositter-rs-2f0.9")
    }

    /// stack_node_new
    pub(crate) fn node_new(
        &mut self,
        _sp: &SubtreePool,
        _previous_node: u32,
        _t: Subtree,
        _is_pending: bool,
        _state: StateId,
    ) -> u32 {
        todo!("gositter-rs-2f0.9")
    }

    /// stack__subtree_is_equivalent
    pub(crate) fn subtree_is_equivalent(
        &self,
        _sp: &SubtreePool,
        _left: Subtree,
        _right: Subtree,
    ) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// stack_node_add_link
    pub(crate) fn node_add_link(&mut self, _sp: &mut SubtreePool, _node: u32, _link: StackLink) {
        todo!("gositter-rs-2f0.9")
    }

    /// summaryFree
    pub(crate) fn summary_free(&mut self, _summary: Option<Vec<StackSummaryEntry>>) {
        todo!("gositter-rs-2f0.9")
    }

    /// stack_head_delete, of heads[version].
    pub(crate) fn head_delete(&mut self, _sp: &mut SubtreePool, _version: StackVersion) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack__add_version
    pub(crate) fn add_version(
        &mut self,
        _sp: &mut SubtreePool,
        _original_version: StackVersion,
        _node: u32,
    ) -> StackVersion {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack__add_slice
    pub(crate) fn add_slice(
        &mut self,
        _sp: &mut SubtreePool,
        _original_version: StackVersion,
        _node: u32,
        _subtrees: SubtreeArray,
    ) {
        todo!("gositter-rs-2f0.9")
    }

    /// callback: the StackCallback functions but pop_all_callback, which pop_all has inline.
    pub(crate) fn callback(&mut self, _sp: &SubtreePool, _cb: u8, _iterator: StackIterator) -> u32 {
        todo!("gositter-rs-2f0.9")
    }

    /// summarize: the part of summarize_stack_callback below its depth test, for
    /// nodes[node].
    pub(crate) fn summarize(&mut self, _node: u32, _depth: u32) {
        todo!("gositter-rs-2f0.9")
    }

    /// stack__iter. The returned slice is `self.slices` and is overwritten by the next call,
    /// as in C. goal_subtree_count < 0: no subtrees.
    pub(crate) fn iter(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
        _cb: u8,
        _goal_subtree_count: i64,
    ) -> &[StackSlice] {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_new, minus the allocations.
    pub(crate) fn init(&mut self, _sp: &mut SubtreePool) {
        todo!("gositter-rs-2f0.9")
    }

    /// reset: start of a parse: drop every node and head without walking them. The subtree
    /// slabs are reset at the same time, so no release is owed.
    pub(crate) fn reset(&mut self, _sp: &mut SubtreePool) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_version_count
    pub(crate) fn version_count(&self) -> u32 {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_state
    pub(crate) fn state(&self, _version: StackVersion) -> StateId {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_position
    pub(crate) fn position(&self, _version: StackVersion) -> Length {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_last_external_token
    pub(crate) fn last_external_token(&self, _version: StackVersion) -> Subtree {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_set_last_external_token
    pub(crate) fn set_last_external_token(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
        _token: Subtree,
    ) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_error_cost
    pub(crate) fn error_cost(&self, _version: StackVersion) -> u32 {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_node_count_since_error
    pub(crate) fn node_count_since_error(&mut self, _version: StackVersion) -> u32 {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_push
    pub(crate) fn push(
        &mut self,
        _sp: &SubtreePool,
        _version: StackVersion,
        _t: Subtree,
        _pending: bool,
        _state: StateId,
    ) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_pop_count
    pub(crate) fn pop_count(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
        _count: u32,
    ) -> &[StackSlice] {
        todo!("gositter-rs-2f0.9")
    }

    /// popLinear: ts_stack_pop_count for a head that owns an unbranched chain, without
    /// moving the head. Some((subtrees, trailing, below)), or None, with nothing changed,
    /// if the chain is not of that shape.
    pub(crate) fn pop_linear(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
        _count: u32,
    ) -> Option<(SubtreeArray, u32, u32)> {
        todo!("gositter-rs-2f0.9")
    }

    /// undoPopLinear takes back a pop_linear whose head was not moved.
    pub(crate) fn undo_pop_linear(&mut self, _sp: &mut SubtreePool, _subtrees: SubtreeArray) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_pop_pending
    pub(crate) fn pop_pending(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
    ) -> &[StackSlice] {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_pop_error
    pub(crate) fn pop_error(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
    ) -> SubtreeArray {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_pop_all, in two steps: the walk keeps no subtrees and records where each
    /// path forked. A slice names its path in subtrees.off.
    pub(crate) fn pop_all(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
    ) -> &[StackSlice] {
        todo!("gositter-rs-2f0.9")
    }

    /// popAllSubtrees: the subtrees of one slice of pop_all, as stack__iter collects them.
    /// The stack must not have changed since pop_all.
    pub(crate) fn pop_all_subtrees(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
        _slice: StackSlice,
    ) -> SubtreeArray {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_record_summary
    pub(crate) fn record_summary(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
        _max_depth: u32,
    ) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_get_summary
    pub(crate) fn get_summary(&self, _version: StackVersion) -> Option<&[StackSummaryEntry]> {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_dynamic_precedence
    pub(crate) fn dynamic_precedence(&self, _version: StackVersion) -> i32 {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_has_advanced_since_error
    pub(crate) fn has_advanced_since_error(
        &self,
        _sp: &SubtreePool,
        _version: StackVersion,
    ) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_remove_version
    pub(crate) fn remove_version(&mut self, _sp: &mut SubtreePool, _version: StackVersion) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_renumber_version
    pub(crate) fn renumber_version(
        &mut self,
        _sp: &mut SubtreePool,
        _v1: StackVersion,
        _v2: StackVersion,
    ) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_swap_versions
    pub(crate) fn swap_versions(&mut self, _v1: StackVersion, _v2: StackVersion) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_copy_version
    pub(crate) fn copy_version(
        &mut self,
        _sp: &mut SubtreePool,
        _version: StackVersion,
    ) -> StackVersion {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_merge
    pub(crate) fn merge(
        &mut self,
        _sp: &mut SubtreePool,
        _version1: StackVersion,
        _version2: StackVersion,
    ) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_can_merge
    pub(crate) fn can_merge(
        &self,
        _sp: &SubtreePool,
        _version1: StackVersion,
        _version2: StackVersion,
    ) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_halt
    pub(crate) fn halt(&mut self, _version: StackVersion) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_pause
    pub(crate) fn pause(&mut self, _version: StackVersion, _lookahead: Subtree) {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_is_active
    pub(crate) fn is_active(&self, _version: StackVersion) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_is_halted
    pub(crate) fn is_halted(&self, _version: StackVersion) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_is_paused
    pub(crate) fn is_paused(&self, _version: StackVersion) -> bool {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_resume
    pub(crate) fn resume(&mut self, _version: StackVersion) -> Subtree {
        todo!("gositter-rs-2f0.9")
    }

    /// ts_stack_clear
    pub(crate) fn clear(&mut self, _sp: &mut SubtreePool) {
        todo!("gositter-rs-2f0.9")
    }
}

#[cfg(test)]
mod tests;
