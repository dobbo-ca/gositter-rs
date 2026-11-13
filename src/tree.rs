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
        mut pool: SubtreePool,
        language: &'static Language,
        root: Subtree,
    ) -> Tree {
        let off = pool.room(1);
        let slot = pool.bump_children(off, 1);
        pool.children[slot as usize] = root;
        Tree {
            pool,
            language: Some(language),
            root: slot,
        }
    }

    /// ts_tree_root_node
    pub fn root_node(&self) -> Node<'_> {
        Node::node_new(
            self,
            self.root,
            self.pool.padding(self.pool.children[self.root as usize]),
            0,
        )
    }

    /// ts_tree_language
    pub fn language(&self) -> &'static Language {
        self.language.expect("gositter: tree has no language")
    }
}
