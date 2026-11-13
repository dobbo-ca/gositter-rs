//! stack.c (gositter `stack.go`): the graph-structured stack. Nodes live in a slab and are
//! addressed by index; 0 is the NULL node.
//!
//! Not ported (Go GC workarounds, plan Global Constraints): all of `slabpool.go` but the
//! shrink rule (`slabPool`, `keptSlab`, `nodeSlabs`, `linkSlabs`, `resets`, `keepResets`,
//! `keepBytes`), `growNodes` and `growLinks` (a `Vec` doubles), and the stack fields
//! `nodesBox`, `linksBox`, `nodesOut`, `linksOut`. The slabs are per-parser `Vec`s that
//! `reset` truncates, shrunk by the rule of `shrink_slab`. The stack holds no pointer to the
//! subtree pool: a function that needs it takes `sp`.

use crate::length::{Length, length_add};
use crate::subtree::{
    ERROR_COST_PER_MISSING_TREE, ERROR_COST_PER_RECOVERY, ERROR_STATE, F_IS_MISSING, F_VISIBLE,
    INL_IS_MISSING, INL_VISIBLE, NULL_SUBTREE, Subtree, SubtreeArray, SubtreePool,
    subtree_slice_reverse,
};
use crate::{SYMBOL_ERROR_REPEAT, StateId};

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
pub(crate) fn shrink_slab(c: usize, n: usize, max: usize) -> bool {
    c > max && n < c / 8
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
    pub(crate) fn link(&self, node: u32, i: usize) -> &StackLink {
        if i == 0 {
            return &self.nodes[node as usize].links[0];
        }
        &self.more_links[self.more[node as usize] as usize][i - 1]
    }

    /// link, for writing: links[i] of node, for i < link_count.
    pub(crate) fn link_mut(&mut self, node: u32, i: usize) -> &mut StackLink {
        if i == 0 {
            return &mut self.nodes[node as usize].links[0];
        }
        &mut self.more_links[self.more[node as usize] as usize][i - 1]
    }

    /// moreLink: the slot of links[i], i > 0, for writing link i.
    pub(crate) fn more_link(&mut self, node: u32, i: usize) -> &mut StackLink {
        let n = node as usize;
        if self.more.len() <= n {
            self.more.resize(n + 1, 0);
        }
        if self.more[n] == 0 {
            self.more[n] = self.more_links.len() as u32;
            self.more_links.push(MoreLinkSet::default());
        }
        &mut self.more_links[self.more[n] as usize][i - 1]
    }

    /// stack_node_retain
    pub(crate) fn node_retain(&mut self, n: u32) {
        if n == 0 {
            return;
        }
        let node = &mut self.nodes[n as usize];
        node.ref_count = node.ref_count.wrapping_add(1);
    }

    /// stack_node_release
    pub(crate) fn node_release(&mut self, sp: &mut SubtreePool, node: u32) {
        let mut node = node;
        loop {
            let n = &mut self.nodes[node as usize];
            n.ref_count = n.ref_count.wrapping_sub(1);
            if n.ref_count > 0 {
                return;
            }

            let mut first_predecessor = 0;
            let link_count = n.link_count;
            if link_count > 0 {
                for i in (1..link_count as usize).rev() {
                    let link = *self.link(node, i);
                    if link.subtree != NULL_SUBTREE {
                        sp.release(link.subtree);
                    }
                    self.node_release(sp, link.node);
                }
                let link = self.nodes[node as usize].links[0];
                if link.subtree != NULL_SUBTREE {
                    sp.release(link.subtree);
                }
                first_predecessor = link.node;
            }

            // C keeps at most MAX_NODE_POOL_SIZE nodes and frees the rest.
            self.node_pool.push(node);

            if first_predecessor == 0 {
                return;
            }
            node = first_predecessor;
        }
    }

    /// stack__subtree_node_count
    pub(crate) fn subtree_node_count(&self, sp: &SubtreePool, t: Subtree) -> u32 {
        let mut count = sp.visible_descendant_count(t);
        if sp.visible(t) {
            count = count.wrapping_add(1);
        }

        // Count intermediate error nodes even though they are not visible: the node count
        // measures progress since the last error.
        if sp.symbol(t) == SYMBOL_ERROR_REPEAT {
            count = count.wrapping_add(1);
        }

        count
    }

    /// stack_node_new
    pub(crate) fn node_new(
        &mut self,
        sp: &SubtreePool,
        previous_node: u32,
        t: Subtree,
        is_pending: bool,
        state: StateId,
    ) -> u32 {
        let i = match self.node_pool.pop() {
            Some(i) => i,
            None => {
                let i = self.nodes.len();
                // growNodes: the Vec doubles; Go's errSlabFull guard stays.
                if i == self.nodes.capacity() && i > (u32::MAX / 2) as usize {
                    panic!("{}", crate::subtree::ERR_SLAB_FULL);
                }
                self.nodes.push(StackNode::default());
                i as u32
            }
        };
        let mut node = self.nodes[i as usize];
        node.ref_count = 1;
        node.state = state;

        // C zeroes the whole node. Only links[0] is ever read past link_count (by
        // error_cost), so the other seven links keep their stale contents.
        if previous_node == 0 {
            node.link_count = 0;
            node.links[0] = StackLink::default();
            node.position = Length::default();
            node.error_cost = 0;
            node.dynamic_precedence = 0;
            node.node_count = 0;
        } else {
            let prev = &self.nodes[previous_node as usize];
            node.link_count = 1;
            node.links[0] = StackLink {
                node: previous_node,
                subtree: t,
                is_pending,
            };

            node.position = prev.position;
            node.error_cost = prev.error_cost;
            node.dynamic_precedence = prev.dynamic_precedence;
            node.node_count = prev.node_count;

            // ts_subtree_error_cost, ts_subtree_total_size, stack__subtree_node_count and
            // ts_subtree_dynamic_precedence, with the subtree decoded once.
            if t.is_inline() {
                if t.0 & INL_IS_MISSING != 0 {
                    node.error_cost = node
                        .error_cost
                        .wrapping_add(ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY);
                }
                node.position = length_add(node.position, sp.total_size(t));
                if t.0 & INL_VISIBLE != 0 {
                    node.node_count = node.node_count.wrapping_add(1);
                }
            } else if t != NULL_SUBTREE {
                let h = sp.ptr(t);
                if h.flags & F_IS_MISSING != 0 {
                    node.error_cost = node
                        .error_cost
                        .wrapping_add(ERROR_COST_PER_MISSING_TREE + ERROR_COST_PER_RECOVERY);
                } else {
                    node.error_cost = node.error_cost.wrapping_add(h.error_cost);
                }
                node.position = length_add(node.position, length_add(h.padding, h.size));
                if h.child_count > 0 {
                    node.node_count = node.node_count.wrapping_add(h.visible_descendant_count);
                    node.dynamic_precedence =
                        node.dynamic_precedence.wrapping_add(h.dynamic_precedence);
                }
                if h.flags & F_VISIBLE != 0 {
                    node.node_count = node.node_count.wrapping_add(1);
                }
                if h.symbol == SYMBOL_ERROR_REPEAT {
                    node.node_count = node.node_count.wrapping_add(1);
                }
            }
        }
        self.nodes[i as usize] = node;

        i
    }

    /// stack__subtree_is_equivalent
    pub(crate) fn subtree_is_equivalent(
        &self,
        sp: &SubtreePool,
        left: Subtree,
        right: Subtree,
    ) -> bool {
        if left == right {
            return true;
        }
        if left == NULL_SUBTREE || right == NULL_SUBTREE {
            return false;
        }

        // Symbols must match.
        if sp.symbol(left) != sp.symbol(right) {
            return false;
        }

        // If both have errors, don't bother keeping both.
        if sp.error_cost(left) > 0 && sp.error_cost(right) > 0 {
            return true;
        }

        sp.padding(left).bytes == sp.padding(right).bytes
            && sp.size(left).bytes == sp.size(right).bytes
            && sp.child_count(left) == sp.child_count(right)
            && sp.extra(left) == sp.extra(right)
            && sp.external_scanner_state_eq(left, right)
    }

    /// stack_node_add_link. Allocates no node, so node indexes hold.
    pub(crate) fn node_add_link(&mut self, sp: &mut SubtreePool, node: u32, link: StackLink) {
        if link.node == node {
            return;
        }
        let n = node as usize;
        let ln = link.node as usize;

        let mut i = 0;
        while i < self.nodes[n].link_count as usize {
            let existing_link = *self.link(node, i);
            if self.subtree_is_equivalent(sp, existing_link.subtree, link.subtree) {
                // Two links between the same pair of nodes: the ambiguity can be removed
                // ahead of time.
                if existing_link.node == link.node {
                    if sp.dynamic_precedence(link.subtree)
                        > sp.dynamic_precedence(existing_link.subtree)
                    {
                        sp.retain(link.subtree);
                        sp.release(existing_link.subtree);
                        self.link_mut(node, i).subtree = link.subtree;
                        self.nodes[n].dynamic_precedence = self.nodes[ln]
                            .dynamic_precedence
                            .wrapping_add(sp.dynamic_precedence(link.subtree));
                    }
                    return;
                }

                // If the previous nodes are mergeable, merge them recursively.
                let existing = existing_link.node;
                let existing_node = &self.nodes[existing as usize];
                let link_node = &self.nodes[ln];
                if existing_node.state == link_node.state
                    && existing_node.position.bytes == link_node.position.bytes
                    && existing_node.error_cost == link_node.error_cost
                {
                    let mut j = 0;
                    while j < self.nodes[ln].link_count as usize {
                        let l = *self.link(link.node, j);
                        self.node_add_link(sp, existing, l);
                        j += 1;
                    }
                    let mut dynamic_precedence = self.nodes[ln].dynamic_precedence;
                    if link.subtree != NULL_SUBTREE {
                        dynamic_precedence =
                            dynamic_precedence.wrapping_add(sp.dynamic_precedence(link.subtree));
                    }
                    if dynamic_precedence > self.nodes[n].dynamic_precedence {
                        self.nodes[n].dynamic_precedence = dynamic_precedence;
                    }
                    return;
                }
            }
            i += 1;
        }

        if self.nodes[n].link_count == MAX_LINK_COUNT {
            return;
        }

        self.node_retain(link.node);
        let mut node_count = self.nodes[ln].node_count;
        let mut dynamic_precedence = self.nodes[ln].dynamic_precedence;
        let link_count = self.nodes[n].link_count;
        if link_count == 0 {
            self.nodes[n].links[0] = link;
        } else {
            *self.more_link(node, link_count as usize) = link;
        }
        self.nodes[n].link_count += 1;

        if link.subtree != NULL_SUBTREE {
            sp.retain(link.subtree);
            node_count = node_count.wrapping_add(self.subtree_node_count(sp, link.subtree));
            dynamic_precedence =
                dynamic_precedence.wrapping_add(sp.dynamic_precedence(link.subtree));
        }

        if node_count > self.nodes[n].node_count {
            self.nodes[n].node_count = node_count;
        }
        if dynamic_precedence > self.nodes[n].dynamic_precedence {
            self.nodes[n].dynamic_precedence = dynamic_precedence;
        }
    }

    /// summaryFree
    pub(crate) fn summary_free(&mut self, summary: Option<Vec<StackSummaryEntry>>) {
        if let Some(mut summary) = summary {
            summary.clear();
            self.summary_pool.push(summary);
        }
    }

    /// stack_head_delete, of heads[version].
    pub(crate) fn head_delete(&mut self, sp: &mut SubtreePool, version: StackVersion) {
        let h = &mut self.heads[version as usize];
        if h.node != 0 {
            let node = h.node;
            let last_external_token = h.last_external_token;
            let lookahead_when_paused = h.lookahead_when_paused;
            let summary = h.summary.take();
            if last_external_token != NULL_SUBTREE {
                sp.release(last_external_token);
            }
            if lookahead_when_paused != NULL_SUBTREE {
                sp.release(lookahead_when_paused);
            }
            self.summary_free(summary);
            self.node_release(sp, node);
        }
    }

    /// ts_stack__add_version
    pub(crate) fn add_version(
        &mut self,
        sp: &mut SubtreePool,
        original_version: StackVersion,
        node: u32,
    ) -> StackVersion {
        let original = &self.heads[original_version as usize];
        let head = StackHead {
            node,
            summary: None,
            node_count_at_last_error: original.node_count_at_last_error,
            last_external_token: original.last_external_token,
            status: STACK_STATUS_ACTIVE,
            lookahead_when_paused: NULL_SUBTREE,
        };
        let last_external_token = head.last_external_token;
        self.heads.push(head);
        self.node_retain(node);
        if last_external_token != NULL_SUBTREE {
            sp.retain(last_external_token);
        }
        (self.heads.len() - 1) as StackVersion
    }

    /// ts_stack__add_slice
    pub(crate) fn add_slice(
        &mut self,
        sp: &mut SubtreePool,
        original_version: StackVersion,
        node: u32,
        subtrees: SubtreeArray,
    ) {
        let heads = &self.heads;
        let found = self
            .slices
            .iter()
            .rposition(|s| heads[s.version as usize].node == node);
        if let Some(i) = found {
            let version = self.slices[i].version;
            self.slices.insert(i + 1, StackSlice { subtrees, version });
            return;
        }

        let version = self.add_version(sp, original_version, node);
        self.slices.push(StackSlice { subtrees, version });
    }

    /// callback: the StackCallback functions but pop_all_callback, which pop_all has inline.
    pub(crate) fn callback(&mut self, sp: &SubtreePool, cb: u8, iterator: StackIterator) -> u32 {
        match cb {
            // pop_count_callback
            CB_POP_COUNT => {
                if iterator.subtree_count == self.goal_subtree_count {
                    return STACK_ACTION_POP | STACK_ACTION_STOP;
                }
                STACK_ACTION_NONE
            }

            // pop_pending_callback
            CB_POP_PENDING => {
                if iterator.subtree_count >= 1 {
                    if iterator.is_pending {
                        return STACK_ACTION_POP | STACK_ACTION_STOP;
                    }
                    return STACK_ACTION_STOP;
                }
                STACK_ACTION_NONE
            }

            // pop_error_callback
            CB_POP_ERROR => {
                if iterator.subtrees.size > 0 {
                    if !self.found_error && sp.is_error(sp.children[iterator.subtrees.off as usize])
                    {
                        self.found_error = true;
                        return STACK_ACTION_POP | STACK_ACTION_STOP;
                    }
                    return STACK_ACTION_STOP;
                }
                STACK_ACTION_NONE
            }

            // summarize_stack_callback
            _ => {
                if iterator.subtree_count > self.max_depth {
                    return STACK_ACTION_STOP;
                }
                self.summarize(iterator.node, iterator.subtree_count);
                STACK_ACTION_NONE
            }
        }
    }

    /// summarize: the part of summarize_stack_callback below its depth test, for
    /// nodes[node].
    pub(crate) fn summarize(&mut self, node: u32, depth: u32) {
        let node = &self.nodes[node as usize];
        let state = node.state;
        let position = node.position;
        for entry in self.summary.iter().rev() {
            if entry.depth < depth {
                break;
            }
            if entry.depth == depth && entry.state == state {
                return;
            }
        }
        self.summary.push(StackSummaryEntry {
            position,
            depth,
            state,
        });
    }

    /// stack__iter. The returned slice is `self.slices` and is overwritten by the next call,
    /// as in C. goal_subtree_count < 0: no subtrees.
    pub(crate) fn iter(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
        cb: u8,
        goal_subtree_count: i64,
    ) -> &[StackSlice] {
        self.slices.clear();
        self.iterators.clear();

        let mut new_iterator = StackIterator {
            node: self.heads[version as usize].node,
            is_pending: true,
            ..StackIterator::default()
        };

        let include_subtrees = goal_subtree_count >= 0;
        if include_subtrees {
            // C reserves room for the node's heap data too. That lives in another slab
            // here, and a region at the end of the children slab grows in place, so the
            // goal count is enough.
            sp.array_reserve(&mut new_iterator.subtrees, goal_subtree_count as u32);
        }

        self.iterators.push(new_iterator);

        // The common pop: one iterator walking an unbranched chain. Each turn is one turn
        // of the general loop below with no action and one link.
        if cb == CB_POP_COUNT {
            let it = &mut self.iterators[0];
            while it.subtree_count != self.goal_subtree_count {
                let node = &self.nodes[it.node as usize];
                if node.link_count != 1 {
                    break;
                }
                let link = node.links[0];
                it.node = link.node;
                if link.subtree != NULL_SUBTREE {
                    sp.array_push(&mut it.subtrees, link.subtree);
                    sp.retain(link.subtree);
                    if !sp.extra(link.subtree) {
                        it.subtree_count += 1;
                        if !link.is_pending {
                            it.is_pending = false;
                        }
                    }
                } else {
                    it.subtree_count += 1;
                    it.is_pending = false;
                }
            }
        } else if cb == CB_SUMMARIZE {
            // The same for the summary walk, which passes every extra on the chain: after
            // many errors in a row that is most of the stack.
            while self.iterators[0].subtree_count <= self.max_depth {
                let it_node = self.iterators[0].node;
                let node = &self.nodes[it_node as usize];
                if node.link_count != 1 {
                    break;
                }
                let link = node.links[0];
                self.summarize(it_node, self.iterators[0].subtree_count);
                let it = &mut self.iterators[0];
                it.node = link.node;
                if link.subtree == NULL_SUBTREE {
                    it.subtree_count += 1;
                    it.is_pending = false;
                } else if !sp.extra(link.subtree) {
                    it.subtree_count += 1;
                    if !link.is_pending {
                        it.is_pending = false;
                    }
                }
            }
        }

        while !self.iterators.is_empty() {
            let mut i = 0;
            let mut size = self.iterators.len();
            while i < size {
                let iterator = self.iterators[i];
                let node_index = iterator.node;
                // No node is added or relinked in the walk, so the link count holds.
                let link_count = self.nodes[node_index as usize].link_count;

                let action = self.callback(sp, cb, iterator);
                let should_pop = action & STACK_ACTION_POP != 0;
                let should_stop = action & STACK_ACTION_STOP != 0 || link_count == 0;

                if should_pop {
                    let mut subtrees = iterator.subtrees;
                    if !should_stop {
                        subtrees = sp.array_copy(subtrees);
                    }
                    let start = subtrees.off as usize;
                    subtree_slice_reverse(&mut sp.children[start..start + subtrees.size as usize]);
                    self.add_slice(sp, version, node_index, subtrees);
                }

                if should_stop {
                    if !should_pop {
                        let mut subtrees = iterator.subtrees;
                        sp.array_delete(&mut subtrees);
                    }
                    self.iterators.remove(i);
                    size -= 1;
                    continue;
                }

                for j in 1..=link_count {
                    let (link, next) = if j == link_count {
                        (self.nodes[node_index as usize].links[0], i)
                    } else {
                        if self.iterators.len() >= MAX_ITERATOR_COUNT {
                            continue;
                        }
                        let link = *self.link(node_index, j as usize);
                        let copy = self.iterators[i];
                        self.iterators.push(copy);
                        let next = self.iterators.len() - 1;
                        self.iterators[next].subtrees = sp.array_copy(copy.subtrees);
                        (link, next)
                    };

                    let next_iterator = &mut self.iterators[next];
                    next_iterator.node = link.node;
                    if link.subtree != NULL_SUBTREE {
                        if include_subtrees {
                            sp.array_push(&mut next_iterator.subtrees, link.subtree);
                            sp.retain(link.subtree);
                        }

                        if !sp.extra(link.subtree) {
                            next_iterator.subtree_count += 1;
                            if !link.is_pending {
                                next_iterator.is_pending = false;
                            }
                        }
                    } else {
                        next_iterator.subtree_count += 1;
                        next_iterator.is_pending = false;
                    }
                }
                i += 1;
            }
        }

        &self.slices
    }

    /// ts_stack_new, minus the allocations.
    pub(crate) fn init(&mut self, sp: &mut SubtreePool) {
        self.reset(sp);
    }

    /// reset: start of a parse: drop every node and head without walking them. The subtree
    /// slabs are reset at the same time, so no release is owed.
    pub(crate) fn reset(&mut self, sp: &mut SubtreePool) {
        let mut heads = std::mem::take(&mut self.heads);
        for head in &mut heads {
            let summary = head.summary.take();
            self.summary_free(summary);
        }
        heads.clear();
        self.heads = heads;
        self.slices.clear();
        self.iterators.clear();
        // len 2: nothing was parsed since the last reset
        let n = self.nodes.len();
        let parsed = n > 2;
        if parsed && shrink_slab(self.nodes.capacity(), n, SHRINK_NODES) {
            self.nodes = Vec::new();
            if shrink_slab(self.node_pool.capacity(), n, SHRINK_INDEXES) {
                self.node_pool = Vec::new();
            }
            if shrink_slab(self.more.capacity(), self.more.len(), SHRINK_INDEXES) {
                self.more = Vec::new();
            }
        }
        if parsed
            && shrink_slab(
                self.more_links.capacity(),
                self.more_links.len(),
                SHRINK_LINKS,
            )
        {
            self.more_links = Vec::new();
        }
        if self.nodes.capacity() == 0 {
            self.nodes = Vec::with_capacity(64);
        }
        self.nodes.clear();
        self.nodes.push(StackNode::default());
        self.more.clear();
        self.more_links.clear();
        self.more_links.push(MoreLinkSet::default());
        self.node_pool.clear();
        self.base_node = self.node_new(sp, 0, NULL_SUBTREE, false, 1);
        self.clear(sp);
    }

    /// ts_stack_version_count
    pub(crate) fn version_count(&self) -> u32 {
        self.heads.len() as u32
    }

    /// ts_stack_state
    pub(crate) fn state(&self, version: StackVersion) -> StateId {
        self.nodes[self.heads[version as usize].node as usize].state
    }

    /// ts_stack_position
    pub(crate) fn position(&self, version: StackVersion) -> Length {
        self.nodes[self.heads[version as usize].node as usize].position
    }

    /// ts_stack_last_external_token
    pub(crate) fn last_external_token(&self, version: StackVersion) -> Subtree {
        self.heads[version as usize].last_external_token
    }

    /// ts_stack_set_last_external_token
    pub(crate) fn set_last_external_token(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
        token: Subtree,
    ) {
        let head = &mut self.heads[version as usize];
        if token != NULL_SUBTREE {
            sp.retain(token);
        }
        if head.last_external_token != NULL_SUBTREE {
            sp.release(head.last_external_token);
        }
        head.last_external_token = token;
    }

    /// ts_stack_error_cost
    pub(crate) fn error_cost(&self, version: StackVersion) -> u32 {
        let head = &self.heads[version as usize];
        let node = &self.nodes[head.node as usize];
        let mut result = node.error_cost;
        if head.status == STACK_STATUS_PAUSED
            || (node.state == ERROR_STATE && node.links[0].subtree == NULL_SUBTREE)
        {
            result = result.wrapping_add(ERROR_COST_PER_RECOVERY);
        }
        result
    }

    /// ts_stack_node_count_since_error
    pub(crate) fn node_count_since_error(&mut self, version: StackVersion) -> u32 {
        let head = &mut self.heads[version as usize];
        let node_count = self.nodes[head.node as usize].node_count;
        if node_count < head.node_count_at_last_error {
            head.node_count_at_last_error = node_count;
        }
        node_count - head.node_count_at_last_error
    }

    /// ts_stack_push
    pub(crate) fn push(
        &mut self,
        sp: &SubtreePool,
        version: StackVersion,
        t: Subtree,
        pending: bool,
        state: StateId,
    ) {
        let new_node = self.node_new(sp, self.heads[version as usize].node, t, pending, state);
        let node_count = self.nodes[new_node as usize].node_count;
        let head = &mut self.heads[version as usize];
        if t == NULL_SUBTREE {
            head.node_count_at_last_error = node_count;
        }
        head.node = new_node;
    }

    /// ts_stack_pop_count
    pub(crate) fn pop_count(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
        count: u32,
    ) -> &[StackSlice] {
        self.goal_subtree_count = count;
        self.iter(sp, version, CB_POP_COUNT, i64::from(count))
    }

    /// popLinear: ts_stack_pop_count for a head that owns an unbranched chain, without
    /// moving the head. Some((subtrees, trailing, below)), or None, with nothing changed,
    /// if the chain is not of that shape.
    ///
    /// Every node down to the goal has one link and one owner. Then the general pop yields
    /// one slice, and once the caller has moved the new version over this one, exactly
    /// these nodes have been freed. So the subtrees move to the slice without retain and
    /// release and the nodes go back to the pool. The caller checks that the new version
    /// cannot merge and then either sets the head to below or calls undo_pop_linear.
    /// trailing counts the extras on top of the stack, which
    /// ts_subtree_array_remove_trailing_extras would find at the end of the slice.
    pub(crate) fn pop_linear(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
        count: u32,
    ) -> Option<(SubtreeArray, u32, u32)> {
        let mark = sp.children.len();
        let pool_mark = self.node_pool.len();
        let mut trailing = 0u32;
        let mut below = self.heads[version as usize].node;
        let mut counted = 0u32;
        while counted < count {
            let node = &self.nodes[below as usize];
            let link = node.links[0];
            if node.link_count != 1 || node.ref_count != 1 || link.subtree == NULL_SUBTREE {
                sp.children.truncate(mark);
                self.node_pool.truncate(pool_mark);
                return None;
            }
            if !sp.extra(link.subtree) {
                counted += 1;
            } else if counted == 0 {
                trailing += 1;
            }
            let room = sp.room(1);
            let off = sp.bump_children(room, 1);
            sp.children[off as usize] = link.subtree;
            self.node_pool.push(below);
            below = link.node;
        }
        let mut subtrees = SubtreeArray::default();
        let n = (sp.children.len() - mark) as u32;
        if n > 0 {
            subtrees = SubtreeArray {
                off: mark as u32,
                size: n,
                cap: n,
            };
            subtree_slice_reverse(&mut sp.children[mark..]);
        }
        Some((subtrees, trailing, below))
    }

    /// undoPopLinear takes back a pop_linear whose head was not moved.
    pub(crate) fn undo_pop_linear(&mut self, sp: &mut SubtreePool, subtrees: SubtreeArray) {
        if subtrees.size > 0 {
            sp.children.truncate(subtrees.off as usize);
        }
        let n = self.node_pool.len() - subtrees.size as usize;
        self.node_pool.truncate(n);
    }

    /// ts_stack_pop_pending
    pub(crate) fn pop_pending(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
    ) -> &[StackSlice] {
        self.iter(sp, version, CB_POP_PENDING, 0);
        if !self.slices.is_empty() {
            let first = self.slices[0].version;
            self.renumber_version(sp, first, version);
            self.slices[0].version = version;
        }
        &self.slices
    }

    /// ts_stack_pop_error
    pub(crate) fn pop_error(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
    ) -> SubtreeArray {
        let head_node = self.heads[version as usize].node;
        let mut i = 0;
        while i < self.nodes[head_node as usize].link_count as usize {
            let t = self.link(head_node, i).subtree;
            if t != NULL_SUBTREE && sp.is_error(t) {
                self.found_error = false;
                self.iter(sp, version, CB_POP_ERROR, 1);
                if !self.slices.is_empty() {
                    let first = self.slices[0].version;
                    self.renumber_version(sp, first, version);
                    return self.slices[0].subtrees;
                }
                break;
            }
            i += 1;
        }
        SubtreeArray::default()
    }

    /// ts_stack_pop_all, in two steps: the walk keeps no subtrees and records where each
    /// path forked. A slice names its path in subtrees.off.
    ///
    /// C hands back every slice with its own copy of the subtrees, up to 64 arrays as long
    /// as the stack; after an error near the start of a large input they were most of the
    /// memory of the parse. pop_all_subtrees walks one path again, so accept holds one
    /// array at a time. The loop is ts_stack__iter with pop_all_callback and no subtrees.
    pub(crate) fn pop_all(&mut self, sp: &mut SubtreePool, version: StackVersion) -> &[StackSlice] {
        self.slices.clear();
        self.forks.clear();
        self.forks.push(StackFork::default());
        self.iterators.clear();
        self.iterators.push(StackIterator {
            node: self.heads[version as usize].node,
            ..StackIterator::default()
        });

        while !self.iterators.is_empty() {
            let mut i = 0;
            let mut size = self.iterators.len();
            while i < size {
                let node_index = self.iterators[i].node;
                let link_count = self.nodes[node_index as usize].link_count;

                if link_count == 0 {
                    let path = self.iterators[i].path;
                    self.add_slice(
                        sp,
                        version,
                        node_index,
                        SubtreeArray {
                            off: path,
                            ..SubtreeArray::default()
                        },
                    );
                    self.iterators.remove(i);
                    size -= 1;
                    continue;
                }

                for j in 1..=link_count {
                    if j == link_count {
                        self.iterators[i].node = self.nodes[node_index as usize].links[0].node;
                    } else {
                        if self.iterators.len() >= MAX_ITERATOR_COUNT {
                            continue;
                        }
                        self.forks.push(StackFork {
                            parent: self.iterators[i].path,
                            node: node_index,
                            link: j,
                        });
                        let node = self.link(node_index, j as usize).node;
                        self.iterators.push(StackIterator {
                            node,
                            path: (self.forks.len() - 1) as u32,
                            ..StackIterator::default()
                        });
                    }
                }
                i += 1;
            }
        }

        &self.slices
    }

    /// popAllSubtrees: the subtrees of one slice of pop_all, as stack__iter collects them.
    /// The stack must not have changed since pop_all.
    pub(crate) fn pop_all_subtrees(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
        slice: StackSlice,
    ) -> SubtreeArray {
        self.chain.clear();
        let mut path = slice.subtrees.off;
        while path != 0 {
            self.chain.push(path);
            path = self.forks[path as usize].parent;
        }

        let mut subtrees = SubtreeArray::default();
        let mut node_index = self.heads[version as usize].node;
        while self.nodes[node_index as usize].link_count != 0 {
            let mut link = self.nodes[node_index as usize].links[0];
            if let Some(&last) = self.chain.last()
                && self.forks[last as usize].node == node_index
            {
                link = *self.link(node_index, self.forks[last as usize].link as usize);
                self.chain.pop();
            }
            node_index = link.node;
            if link.subtree != NULL_SUBTREE {
                sp.array_push(&mut subtrees, link.subtree);
                sp.retain(link.subtree);
            }
        }
        let start = subtrees.off as usize;
        subtree_slice_reverse(&mut sp.children[start..start + subtrees.size as usize]);
        subtrees
    }

    /// ts_stack_record_summary
    pub(crate) fn record_summary(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
        max_depth: u32,
    ) {
        self.summary = self.summary_pool.pop().unwrap_or_default();
        self.max_depth = max_depth;
        self.iter(sp, version, CB_SUMMARIZE, -1);
        let old = self.heads[version as usize].summary.take();
        self.summary_free(old);
        self.heads[version as usize].summary = Some(std::mem::take(&mut self.summary));
    }

    /// ts_stack_get_summary
    pub(crate) fn get_summary(&self, version: StackVersion) -> Option<&[StackSummaryEntry]> {
        self.heads[version as usize].summary.as_deref()
    }

    /// ts_stack_dynamic_precedence
    pub(crate) fn dynamic_precedence(&self, version: StackVersion) -> i32 {
        self.nodes[self.heads[version as usize].node as usize].dynamic_precedence
    }

    /// ts_stack_has_advanced_since_error
    pub(crate) fn has_advanced_since_error(&self, sp: &SubtreePool, version: StackVersion) -> bool {
        let head = &self.heads[version as usize];
        let mut node = &self.nodes[head.node as usize];
        if node.error_cost == 0 {
            return true;
        }
        loop {
            if node.link_count > 0 {
                let t = node.links[0].subtree;
                if t != NULL_SUBTREE {
                    if sp.total_bytes(t) > 0 {
                        return true;
                    } else if node.node_count > head.node_count_at_last_error
                        && sp.error_cost(t) == 0
                    {
                        let next = node.links[0].node;
                        if next == 0 {
                            break;
                        }
                        node = &self.nodes[next as usize];
                        continue;
                    }
                }
            }
            break;
        }
        false
    }

    /// ts_stack_remove_version
    pub(crate) fn remove_version(&mut self, sp: &mut SubtreePool, version: StackVersion) {
        self.head_delete(sp, version);
        self.heads.remove(version as usize);
    }

    /// ts_stack_renumber_version
    pub(crate) fn renumber_version(
        &mut self,
        sp: &mut SubtreePool,
        v1: StackVersion,
        v2: StackVersion,
    ) {
        if v1 == v2 {
            return;
        }
        let (s, t) = (v1 as usize, v2 as usize);
        if self.heads[t].summary.is_some() && self.heads[s].summary.is_none() {
            self.heads[s].summary = self.heads[t].summary.take();
        }
        self.head_delete(sp, v2);
        self.heads[t] = std::mem::take(&mut self.heads[s]);
        self.heads.remove(s);
    }

    /// ts_stack_swap_versions
    pub(crate) fn swap_versions(&mut self, v1: StackVersion, v2: StackVersion) {
        self.heads.swap(v1 as usize, v2 as usize);
    }

    /// ts_stack_copy_version
    pub(crate) fn copy_version(
        &mut self,
        sp: &mut SubtreePool,
        version: StackVersion,
    ) -> StackVersion {
        let h = &self.heads[version as usize];
        let head = StackHead {
            node: h.node,
            summary: None,
            node_count_at_last_error: h.node_count_at_last_error,
            last_external_token: h.last_external_token,
            lookahead_when_paused: h.lookahead_when_paused,
            status: h.status,
        };
        let (node, last_external_token) = (head.node, head.last_external_token);
        self.heads.push(head);
        self.node_retain(node);
        if last_external_token != NULL_SUBTREE {
            sp.retain(last_external_token);
        }
        (self.heads.len() - 1) as StackVersion
    }

    /// ts_stack_merge
    pub(crate) fn merge(
        &mut self,
        sp: &mut SubtreePool,
        version1: StackVersion,
        version2: StackVersion,
    ) -> bool {
        if !self.can_merge(sp, version1, version2) {
            return false;
        }
        let node1 = self.heads[version1 as usize].node;
        let node2 = self.heads[version2 as usize].node;
        let mut i = 0;
        while i < self.nodes[node2 as usize].link_count as usize {
            let link = *self.link(node2, i);
            self.node_add_link(sp, node1, link);
            i += 1;
        }
        let n1 = &self.nodes[node1 as usize];
        if n1.state == ERROR_STATE {
            self.heads[version1 as usize].node_count_at_last_error = n1.node_count;
        }
        self.remove_version(sp, version2);
        true
    }

    /// ts_stack_can_merge
    pub(crate) fn can_merge(
        &self,
        sp: &SubtreePool,
        version1: StackVersion,
        version2: StackVersion,
    ) -> bool {
        let head1 = &self.heads[version1 as usize];
        let head2 = &self.heads[version2 as usize];
        let node1 = &self.nodes[head1.node as usize];
        let node2 = &self.nodes[head2.node as usize];
        head1.status == STACK_STATUS_ACTIVE
            && head2.status == STACK_STATUS_ACTIVE
            && node1.state == node2.state
            && node1.position.bytes == node2.position.bytes
            && node1.error_cost == node2.error_cost
            && sp.external_scanner_state_eq(head1.last_external_token, head2.last_external_token)
    }

    /// ts_stack_halt
    pub(crate) fn halt(&mut self, version: StackVersion) {
        self.heads[version as usize].status = STACK_STATUS_HALTED;
    }

    /// ts_stack_pause
    pub(crate) fn pause(&mut self, version: StackVersion, lookahead: Subtree) {
        let head = &mut self.heads[version as usize];
        head.status = STACK_STATUS_PAUSED;
        head.lookahead_when_paused = lookahead;
        head.node_count_at_last_error = self.nodes[head.node as usize].node_count;
    }

    /// ts_stack_is_active
    pub(crate) fn is_active(&self, version: StackVersion) -> bool {
        self.heads[version as usize].status == STACK_STATUS_ACTIVE
    }

    /// ts_stack_is_halted
    pub(crate) fn is_halted(&self, version: StackVersion) -> bool {
        self.heads[version as usize].status == STACK_STATUS_HALTED
    }

    /// ts_stack_is_paused
    pub(crate) fn is_paused(&self, version: StackVersion) -> bool {
        self.heads[version as usize].status == STACK_STATUS_PAUSED
    }

    /// ts_stack_resume
    pub(crate) fn resume(&mut self, version: StackVersion) -> Subtree {
        let head = &mut self.heads[version as usize];
        let result = head.lookahead_when_paused;
        head.status = STACK_STATUS_ACTIVE;
        head.lookahead_when_paused = NULL_SUBTREE;
        result
    }

    /// ts_stack_clear
    pub(crate) fn clear(&mut self, sp: &mut SubtreePool) {
        self.node_retain(self.base_node);
        for version in 0..self.heads.len() {
            self.head_delete(sp, version as StackVersion);
        }
        self.heads.clear();
        self.heads.push(StackHead {
            node: self.base_node,
            status: STACK_STATUS_ACTIVE,
            ..StackHead::default()
        });
    }
}

#[cfg(test)]
mod tests;
