//! tree_cursor.c.

use crate::length::Length;
use crate::subtree::{Subtree, SubtreeHeap};
use crate::{FieldId, Node, Symbol, Tree};

/// TreeCursorEntry. `subtree` is a slot in the children slab.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TreeCursorEntry {
    pub(crate) subtree: u32,
    pub(crate) position: Length,
    pub(crate) child_index: u32,
    pub(crate) structural_child_index: u32,
    /// Under its parent; 0 for an extra.
    pub(crate) alias: Symbol,
}

/// TreeCursorStep: TreeCursorStepNone
pub(crate) const TREE_CURSOR_STEP_NONE: u32 = 0;
/// TreeCursorStepHidden
pub(crate) const TREE_CURSOR_STEP_HIDDEN: u32 = 1;
/// TreeCursorStepVisible
pub(crate) const TREE_CURSOR_STEP_VISIBLE: u32 = 2;

/// TSTreeCursor.
pub struct TreeCursor<'t> {
    tree: &'t Tree,
    stack: Vec<TreeCursorEntry>,
}

impl<'t> TreeCursor<'t> {
    /// ts_tree_cursor_new: a cursor on `n` (Go `Walk` builds the struct, then `init`).
    pub(crate) fn new(_n: Node<'t>) -> TreeCursor<'t> {
        todo!("gositter-rs-2f0.10")
    }

    /// Go `entrySubtree`: the subtree of stack entry `i`.
    fn entry_subtree(&self, _i: usize) -> Subtree {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_is_entry_visible, alias read from the entry (Go `isEntryVisible`).
    fn is_entry_visible(&self, _index: usize) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// Go `scan`, not a port: the loop of ts_tree_cursor_child_iterator_next over the
    /// children of `h` from `i` on. Pushes the first one that is visible or has visible
    /// children; returns a TREE_CURSOR_STEP_*.
    fn scan(&mut self, _h: &SubtreeHeap, _i: u32, _structural: u32, _position: Length) -> u32 {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_init, with the root alias kept in the entry (Go `init`).
    fn init(&mut self, _n: Node<'t>) {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_goto_sibling_internal (Go `gotoNextSiblingInternal`); a
    /// TREE_CURSOR_STEP_*.
    fn goto_next_sibling_internal(&mut self) -> u32 {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_reset
    pub fn reset(&mut self, _node: Node<'t>) {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_current_node
    pub fn node(&self) -> Node<'t> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_current_field_id. 0 if none.
    pub fn field_id(&self) -> FieldId {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_current_field_name
    pub fn field_name(&self) -> Option<&'static str> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_goto_first_child
    pub fn goto_first_child(&mut self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_goto_next_sibling
    pub fn goto_next_sibling(&mut self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_goto_parent
    pub fn goto_parent(&mut self) -> bool {
        todo!("gositter-rs-2f0.10")
    }
}
