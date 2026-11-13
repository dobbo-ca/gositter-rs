//! tree_cursor.c.

use crate::{FieldId, Node, Tree};

/// TSTreeCursor.
pub struct TreeCursor<'t> {
    _tree: &'t Tree,
}

impl<'t> TreeCursor<'t> {
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
