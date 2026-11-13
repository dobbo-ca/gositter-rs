//! node.c, plus the gositter-only `children`, `preorder` and `preorder_mask`.

use crate::{FieldId, Language, Point, Symbol, Tree, TreeCursor};

/// TSNode: gositter's `{tree, id, x, y}`. Go's null node is `None`. `==` is Go's `==`:
/// the same tree (by address) and the same fields.
#[derive(Clone, Copy)]
pub struct Node<'t> {
    tree: &'t Tree,
    id: u32,
    x: u64,
    y: u64,
}

impl PartialEq for Node<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree)
            && self.id == other.id
            && self.x == other.x
            && self.y == other.y
    }
}

impl Eq for Node<'_> {}

impl<'t> Node<'t> {
    /// ts_node_language
    pub fn language(self) -> &'static Language {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_type
    pub fn kind(self) -> &'static str {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_symbol
    pub fn symbol(self) -> Symbol {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_is_named
    pub fn is_named(self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_is_error
    pub fn is_error(self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_is_missing
    pub fn is_missing(self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_is_extra
    pub fn is_extra(self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_has_error
    pub fn has_error(self) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_start_byte
    pub fn start_byte(self) -> u32 {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_end_byte
    pub fn end_byte(self) -> u32 {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_start_point
    pub fn start_position(self) -> Point {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_end_point
    pub fn end_position(self) -> Point {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_child_count
    pub fn child_count(self) -> u32 {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_child
    pub fn child(self, _index: u32) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_named_child_count
    pub fn named_child_count(self) -> u32 {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_named_child
    pub fn named_child(self, _index: u32) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }

    /// Appends `child(0..child_count())` to `buf` in one pass. No C counterpart.
    pub fn children(self, _buf: &mut Vec<Node<'t>>) {
        todo!("gositter-rs-2f0.10")
    }

    /// Appends `named_child(0..named_child_count())` to `buf` in one pass.
    pub fn named_children(self, _buf: &mut Vec<Node<'t>>) {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_child_by_field_name
    pub fn child_by_field_name(self, _name: &str) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_child_by_field_id
    pub fn child_by_field_id(self, _id: FieldId) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_field_name_for_child. `None` where C returns NULL.
    pub fn field_name_for_child(self, _index: u32) -> Option<&'static str> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_parent: walks down from the root.
    pub fn parent(self) -> Option<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_eq: the same tree and the same subtree.
    pub fn equals(self, _other: Node<'t>) -> bool {
        todo!("gositter-rs-2f0.10")
    }

    /// `src[start_byte..end_byte]`, as go-tree-sitter's `Utf8Text`.
    pub fn utf8_text(self, _src: &[u8]) -> &[u8] {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_node_string, byte-identical.
    pub fn to_sexp(self) -> String {
        todo!("gositter-rs-2f0.10")
    }

    /// Calls `f` for this node and, where `f` returns true, for everything below it, in
    /// document order. No C counterpart.
    pub fn preorder(self, _f: impl FnMut(Node<'t>) -> bool) {
        todo!("gositter-rs-2f0.10")
    }

    /// `preorder` with `f` called only for the kinds in `mask` (`Language::kind_mask`);
    /// subtrees without one of them are skipped.
    pub fn preorder_mask(self, _mask: u32, _f: impl FnMut(Node<'t>) -> bool) {
        todo!("gositter-rs-2f0.10")
    }

    /// ts_tree_cursor_new
    pub fn walk(self) -> TreeCursor<'t> {
        todo!("gositter-rs-2f0.10")
    }

    /// go-tree-sitter's `ChildrenByFieldName`; resets `cursor`.
    pub fn children_by_field_name(
        self,
        _name: &str,
        _cursor: &mut TreeCursor<'t>,
    ) -> Vec<Node<'t>> {
        todo!("gositter-rs-2f0.10")
    }
}
