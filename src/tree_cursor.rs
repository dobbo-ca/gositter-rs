//! tree_cursor.c.

use crate::length::{Length, length_add};
use crate::subtree::{F_EXTRA, F_VISIBLE, INL_EXTRA, INL_VISIBLE, Subtree, SubtreeHeap};
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
    pub(crate) fn new(n: Node<'t>) -> TreeCursor<'t> {
        let mut c = TreeCursor {
            tree: n.tree,
            stack: Vec::new(),
        };
        c.init(n);
        c
    }

    /// Go `entrySubtree`: the subtree of stack entry `i`.
    fn entry_subtree(&self, i: usize) -> Subtree {
        self.tree.pool.children[self.stack[i].subtree as usize]
    }

    /// ts_tree_cursor_is_entry_visible, alias read from the entry (Go `isEntryVisible`).
    fn is_entry_visible(&self, index: usize) -> bool {
        let entry = &self.stack[index];
        index == 0
            || entry.alias != 0
            || self
                .tree
                .pool
                .visible(self.tree.pool.children[entry.subtree as usize])
    }

    /// Go `scan`, not a port: the loop of ts_tree_cursor_child_iterator_next over the
    /// children of `h` from `i` on. Pushes the first one that is visible or has visible
    /// children; returns a TREE_CURSOR_STEP_*.
    fn scan(
        &mut self,
        h: &SubtreeHeap,
        mut i: u32,
        mut structural: u32,
        mut position: Length,
    ) -> u32 {
        let tree = self.tree;
        let sp = &tree.pool;
        let lang = tree.language();
        let alias_sequence = lang.alias_sequence(u32::from(h.production_id));
        let children = &sp.children[h.children as usize..(h.children + h.child_count) as usize];
        while i < children.len() as u32 {
            let child = children[i as usize];
            let (padding, size, visible, extra, visible_children) = if child.is_inline() {
                (
                    sp.padding(child),
                    sp.size(child),
                    child.0 & INL_VISIBLE != 0,
                    child.0 & INL_EXTRA != 0,
                    0,
                )
            } else {
                let ch = sp.ptr(child);
                let mut visible_children = 0;
                if ch.child_count > 0 {
                    visible_children = ch.visible_child_count;
                }
                (
                    ch.padding,
                    ch.size,
                    ch.flags & F_VISIBLE != 0,
                    ch.flags & F_EXTRA != 0,
                    visible_children,
                )
            };
            let mut alias: Symbol = 0;
            if !extra && let Some(start) = alias_sequence {
                alias = lang.tables.alias_sequences.get(start + structural as usize);
            }
            if i > 0 {
                position = length_add(position, padding);
            }
            if visible || alias != 0 || visible_children > 0 {
                self.stack.push(TreeCursorEntry {
                    subtree: h.children + i,
                    position,
                    child_index: i,
                    structural_child_index: structural,
                    alias,
                });
                if visible || alias != 0 {
                    return TREE_CURSOR_STEP_VISIBLE;
                }
                return TREE_CURSOR_STEP_HIDDEN;
            }
            if !extra {
                structural += 1;
            }
            position = length_add(position, size);
            i += 1;
        }
        TREE_CURSOR_STEP_NONE
    }

    /// ts_tree_cursor_init, with the root alias kept in the entry (Go `init`).
    fn init(&mut self, n: Node<'t>) {
        self.tree = n.tree;
        let mut alias = n.alias();
        if n.tree.pool.extra(n.subtree()) {
            alias = 0;
        }
        self.stack.clear();
        self.stack.push(TreeCursorEntry {
            subtree: n.id,
            position: Length {
                bytes: n.start_byte(),
                extent: n.start_position(),
            },
            child_index: 0,
            structural_child_index: 0,
            alias,
        });
    }

    /// ts_tree_cursor_goto_sibling_internal (Go `gotoNextSiblingInternal`); a
    /// TREE_CURSOR_STEP_*.
    ///
    /// Go pops by reslicing and restores `c.stack[:initialSize]` from the backing array;
    /// here `n` is that length and the entries stay in place. `scan` pushes after them, and
    /// a found entry is moved down to slot `n`.
    fn goto_next_sibling_internal(&mut self) -> u32 {
        let tree = self.tree;
        let sp = &tree.pool;
        let initial_size = self.stack.len();
        let mut n = initial_size;

        while n > 1 {
            let popped = self.stack[n - 1];
            n -= 1;
            let h = sp.ptr(sp.children[self.stack[n - 1].subtree as usize]);

            let child = sp.children[popped.subtree as usize];
            let (size, visible, extra) = if child.is_inline() {
                (
                    sp.size(child),
                    child.0 & INL_VISIBLE != 0,
                    child.0 & INL_EXTRA != 0,
                )
            } else {
                let ch = sp.ptr(child);
                (ch.size, ch.flags & F_VISIBLE != 0, ch.flags & F_EXTRA != 0)
            };
            if (visible || popped.alias != 0) && n + 1 < initial_size {
                break;
            }
            let mut structural = popped.structural_child_index;
            if !extra {
                structural += 1;
            }
            let step = self.scan(
                h,
                popped.child_index + 1,
                structural,
                length_add(popped.position, size),
            );
            if step != TREE_CURSOR_STEP_NONE {
                self.stack.swap(n, initial_size);
                self.stack.truncate(n + 1);
                return step;
            }
        }

        TREE_CURSOR_STEP_NONE
    }

    /// ts_tree_cursor_reset
    pub fn reset(&mut self, node: Node<'t>) {
        self.init(node);
    }

    /// ts_tree_cursor_current_node
    pub fn node(&self) -> Node<'t> {
        let e = &self.stack[self.stack.len() - 1];
        Node::node_new(self.tree, e.subtree, e.position, e.alias)
    }

    /// ts_tree_cursor_current_field_id. 0 if none.
    pub fn field_id(&self) -> FieldId {
        let sp = &self.tree.pool;
        let lang = self.tree.language();

        // Walk up the tree, visiting the current node and its invisible ancestors.
        for i in (1..self.stack.len()).rev() {
            let entry = &self.stack[i];

            // Stop walking up when another visible node is found.
            if i != self.stack.len() - 1 && self.is_entry_visible(i) {
                break;
            }

            if sp.extra(sp.children[entry.subtree as usize]) {
                break;
            }

            let parent = sp.ptr(self.entry_subtree(i - 1));
            let (start, len) = lang.field_map(u32::from(parent.production_id));
            for j in start..start + len {
                let m = lang.tables.field_map_entries.get(j);
                if !m.inherited && u32::from(m.child_index) == entry.structural_child_index {
                    return m.field_id;
                }
            }
        }
        0
    }

    /// ts_tree_cursor_current_field_name
    pub fn field_name(&self) -> Option<&'static str> {
        let id = self.field_id();
        if id != 0 {
            return self.tree.language().field_name_for_id(id);
        }
        None
    }

    /// ts_tree_cursor_goto_first_child
    pub fn goto_first_child(&mut self) -> bool {
        let tree = self.tree;
        let sp = &tree.pool;
        loop {
            let e = self.stack[self.stack.len() - 1];
            let t = sp.children[e.subtree as usize];
            if t.is_inline() {
                return false;
            }
            let h = sp.ptr(t);
            if h.child_count == 0 {
                return false;
            }
            match self.scan(h, 0, 0, e.position) {
                TREE_CURSOR_STEP_HIDDEN => continue,
                TREE_CURSOR_STEP_VISIBLE => return true,
                _ => return false,
            }
        }
    }

    /// ts_tree_cursor_goto_next_sibling
    pub fn goto_next_sibling(&mut self) -> bool {
        match self.goto_next_sibling_internal() {
            TREE_CURSOR_STEP_HIDDEN => {
                self.goto_first_child();
                true
            }
            TREE_CURSOR_STEP_VISIBLE => true,
            _ => false,
        }
    }

    /// ts_tree_cursor_goto_parent
    pub fn goto_parent(&mut self) -> bool {
        for i in (0..self.stack.len() - 1).rev() {
            if self.is_entry_visible(i) {
                self.stack.truncate(i + 1);
                return true;
            }
        }
        false
    }
}
