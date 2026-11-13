//! tree.c.

use crate::{Language, Node};

/// TSTree. It lives in the parser and owns the subtree pool; its nodes borrow it until
/// the next parse, reset or set_language on that parser.
pub struct Tree {
    _private: (),
}

impl Tree {
    /// ts_tree_root_node
    pub fn root_node(&self) -> Node<'_> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_language
    pub fn language(&self) -> &'static Language {
        todo!("gositter-rs-2f0.10")
    }
}
