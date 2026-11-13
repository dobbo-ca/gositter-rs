//! tree.c.

use crate::subtree::{Subtree, SubtreePool};
use crate::{Language, Node};

/// TSTree. It lives in the parser and owns the subtree pool; its nodes borrow it until
/// the next parse, reset or set_language on that parser.
pub struct Tree {
    pub(crate) pool: SubtreePool,
    /// `None` only in the parser's tree before its first parse (Go's nil `*Language`).
    pub(crate) language: Option<&'static Language>,
    /// Slot of the root subtree in pool.children.
    pub(crate) root: u32,
}

impl Tree {
    /// The parser's tree before its first parse: Go's zero `Tree`.
    pub(crate) fn empty() -> Tree {
        Tree {
            pool: SubtreePool::default(),
            language: None,
            root: 0,
        }
    }

    /// ts_tree_new as `parser.go:1837-1842` does it: the root gets a slot in the children
    /// slab (`pool.room(1)` + `bump_children`) so that a Node can address it like any other
    /// subtree.
    pub(crate) fn from_pool(
        _pool: SubtreePool,
        _language: &'static Language,
        _root: Subtree,
    ) -> Tree {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_root_node
    pub fn root_node(&self) -> Node<'_> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_language
    pub fn language(&self) -> &'static Language {
        todo!("gositter-rs-2f0.10")
    }
}
