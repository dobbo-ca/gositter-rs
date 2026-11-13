//! node.c, plus the gositter-only `children`, `preorder` and `preorder_mask`.

use crate::length::{Length, length_add, point_add};
use crate::subtree::{
    F_EXTRA, F_NAMED, F_VISIBLE, INL_EXTRA, INL_NAMED, INL_VISIBLE, NULL_SUBTREE, Subtree,
};
use crate::{FieldId, Language, Point, SYMBOL_ERROR, Symbol, Tree, TreeCursor};

/// TSNode: gositter's `{tree, id, x, y}`. Go's null node is `None`. `==` is Go's `==`:
/// the same tree (by address) and the same fields.
#[derive(Clone, Copy)]
pub struct Node<'t> {
    pub(crate) tree: &'t Tree,
    /// Slot in `tree.pool.children`.
    pub(crate) id: u32,
    /// Start byte in the low 32 bits, alias symbol in the high 32 bits.
    pub(crate) x: u64,
    /// Start row in the low 32 bits, start column in the high 32 bits.
    pub(crate) y: u64,
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

/// NodeChildIterator. `children` is the parent's children as a slab offset;
/// `alias_sequence` is `Language::alias_sequence` of the parent's production.
pub(crate) struct NodeChildIterator<'t> {
    pub(crate) tree: &'t Tree,
    pub(crate) parent: Subtree,
    pub(crate) position: Length,
    pub(crate) child_index: u32,
    pub(crate) structural_child_index: u32,
    pub(crate) alias_sequence: Option<usize>,
    pub(crate) children: u32,
    pub(crate) child_count: u32,
}

impl<'t> Iterator for NodeChildIterator<'t> {
    type Item = Node<'t>;

    /// ts_node_child_iterator_next (Go `next`, which returns the node through a pointer).
    fn next(&mut self) -> Option<Node<'t>> {
        if self.parent == NULL_SUBTREE || self.child_index == self.child_count {
            return None;
        }
        let tree = self.tree;
        let sp = &tree.pool;
        let slot = self.children + self.child_index;
        let child = sp.children[slot as usize];
        let mut alias_symbol: Symbol = 0;
        if !sp.extra(child) {
            if let Some(start) = self.alias_sequence {
                alias_symbol = tree
                    .language()
                    .tables
                    .alias_sequences
                    .get(start + self.structural_child_index as usize);
            }
            self.structural_child_index += 1;
        }
        if self.child_index > 0 {
            self.position = length_add(self.position, sp.padding(child));
        }
        let result = Node::node_new(tree, slot, self.position, alias_symbol);
        self.position = length_add(self.position, sp.size(child));
        self.child_index += 1;
        Some(result)
    }
}

impl<'t> Node<'t> {
    /// ts_node_new (Go `nodeNew`)
    pub(crate) fn node_new(tree: &'t Tree, id: u32, position: Length, alias: Symbol) -> Node<'t> {
        Node {
            tree,
            id,
            x: u64::from(position.bytes) | (u64::from(alias) << 32),
            y: u64::from(position.extent.row) | (u64::from(position.extent.column) << 32),
        }
    }

    /// ts_node__alias (Go `alias`)
    pub(crate) fn alias(self) -> Symbol {
        (self.x >> 32) as Symbol
    }

    /// ts_node__subtree (Go `subtree`)
    pub(crate) fn subtree(self) -> Subtree {
        self.tree.pool.children[self.id as usize]
    }

    /// Go's `length{uint32(n.x), Point{uint32(n.y), uint32(n.y >> 32)}}`.
    fn position(self) -> Length {
        Length {
            bytes: self.x as u32,
            extent: Point {
                row: self.y as u32,
                column: (self.y >> 32) as u32,
            },
        }
    }

    /// ts_node_iterate_children (Go `iterateChildren`)
    pub(crate) fn iterate_children(self) -> NodeChildIterator<'t> {
        let sp = &self.tree.pool;
        let t = self.subtree();
        if sp.child_count(t) == 0 {
            return NodeChildIterator {
                tree: self.tree,
                parent: NULL_SUBTREE,
                position: Length::default(),
                child_index: 0,
                structural_child_index: 0,
                alias_sequence: None,
                children: 0,
                child_count: 0,
            };
        }
        let h = sp.ptr(t);
        NodeChildIterator {
            tree: self.tree,
            parent: t,
            position: self.position(),
            child_index: 0,
            structural_child_index: 0,
            alias_sequence: self.language().alias_sequence(u32::from(h.production_id)),
            children: h.children,
            child_count: h.child_count,
        }
    }

    /// ts_node__is_relevant (Go `isRelevant`)
    pub(crate) fn is_relevant(self, include_anonymous: bool) -> bool {
        let sp = &self.tree.pool;
        let t = self.subtree();
        if include_anonymous {
            return sp.visible(t) || self.alias() != 0;
        }
        let alias = self.alias();
        if alias != 0 {
            return self.language().symbol_metadata(alias).named;
        }
        sp.visible(t) && sp.named(t)
    }

    /// ts_node__relevant_child_count (Go `relevantChildCount`)
    pub(crate) fn relevant_child_count(self, include_anonymous: bool) -> u32 {
        let sp = &self.tree.pool;
        let t = self.subtree();
        if sp.child_count(t) > 0 {
            if include_anonymous {
                return sp.ptr(t).visible_child_count;
            }
            return sp.ptr(t).named_child_count;
        }
        0
    }

    /// ts_node__child (Go `child`; renamed, as `child` is ts_node_child here), with the loop
    /// of ts_node_child_iterator_next written out.
    pub(crate) fn child_inner(
        self,
        mut child_index: u32,
        include_anonymous: bool,
    ) -> Option<Node<'t>> {
        let tree = self.tree;
        let sp = &tree.pool;
        let lang = tree.language();
        let mut result = self;
        let mut did_descend = true;

        while did_descend {
            did_descend = false;

            let t = result.subtree();
            if t.is_inline() {
                break;
            }
            let h = sp.ptr(t);
            if h.child_count == 0 {
                break;
            }
            let alias_sequence = lang.alias_sequence(u32::from(h.production_id));
            let mut position = result.position();
            let mut structural_child_index = 0usize;
            let mut index = 0u32;
            let children = &sp.children[h.children as usize..(h.children + h.child_count) as usize];
            if alias_sequence.is_none() {
                for (i, &child) in children.iter().enumerate() {
                    if child.is_inline() {
                        if i > 0 {
                            position = length_add(position, sp.padding(child));
                        }
                        if child.0 & INL_VISIBLE != 0
                            && (include_anonymous || child.0 & INL_NAMED != 0)
                        {
                            if index == child_index {
                                return Some(Node::node_new(
                                    tree,
                                    h.children + i as u32,
                                    position,
                                    0,
                                ));
                            }
                            index += 1;
                        }
                        position = length_add(position, sp.size(child));
                        continue;
                    }
                    let c = sp.ptr(child);
                    if i > 0 {
                        position = length_add(position, c.padding);
                    }
                    if c.flags & F_VISIBLE != 0 && (include_anonymous || c.flags & F_NAMED != 0) {
                        if index == child_index {
                            return Some(Node::node_new(tree, h.children + i as u32, position, 0));
                        }
                        index += 1;
                    } else if c.child_count > 0 {
                        // ts_node__relevant_child_count
                        let mut grandchild_count = c.named_child_count;
                        if include_anonymous {
                            grandchild_count = c.visible_child_count;
                        }
                        let grandchild_index = child_index.wrapping_sub(index);
                        if grandchild_index < grandchild_count {
                            did_descend = true;
                            result = Node::node_new(tree, h.children + i as u32, position, 0);
                            child_index = grandchild_index;
                            break;
                        }
                        index += grandchild_count;
                    }
                    position = length_add(position, c.size);
                }
                continue;
            }
            for (i, &child) in children.iter().enumerate() {
                let (padding, size, visible, named, extra, grandchild_count) = if child.is_inline()
                {
                    (
                        sp.padding(child),
                        sp.size(child),
                        child.0 & INL_VISIBLE != 0,
                        child.0 & INL_NAMED != 0,
                        child.0 & INL_EXTRA != 0,
                        0,
                    )
                } else {
                    let c = sp.ptr(child);
                    // ts_node__relevant_child_count
                    let mut grandchild_count = 0;
                    if c.child_count > 0 {
                        grandchild_count = c.named_child_count;
                        if include_anonymous {
                            grandchild_count = c.visible_child_count;
                        }
                    }
                    (
                        c.padding,
                        c.size,
                        c.flags & F_VISIBLE != 0,
                        c.flags & F_NAMED != 0,
                        c.flags & F_EXTRA != 0,
                        grandchild_count,
                    )
                };
                let mut alias_symbol: Symbol = 0;
                if !extra {
                    if let Some(start) = alias_sequence {
                        alias_symbol = lang
                            .tables
                            .alias_sequences
                            .get(start + structural_child_index);
                    }
                    structural_child_index += 1;
                }
                if i > 0 {
                    position = length_add(position, padding);
                }
                // ts_node__is_relevant
                let mut relevant = visible && named;
                if include_anonymous {
                    relevant = visible || alias_symbol != 0;
                } else if alias_symbol != 0 {
                    relevant = lang.symbol_metadata(alias_symbol).named;
                }
                if relevant {
                    if index == child_index {
                        return Some(Node::node_new(
                            tree,
                            h.children + i as u32,
                            position,
                            alias_symbol,
                        ));
                    }
                    index += 1;
                } else {
                    let grandchild_index = child_index.wrapping_sub(index);
                    if grandchild_index < grandchild_count {
                        did_descend = true;
                        result =
                            Node::node_new(tree, h.children + i as u32, position, alias_symbol);
                        child_index = grandchild_index;
                        break;
                    }
                    index += grandchild_count;
                }
                position = length_add(position, size);
            }
        }

        None
    }

    /// Go `appendChildren`: the enumeration ts_node__child walks, appended to `buf`.
    /// Go recurses into a child that is not relevant; here its iterator goes on a stack.
    fn append_children(self, buf: &mut Vec<Node<'t>>, include_anonymous: bool) {
        let mut stack = vec![self.iterate_children()];
        while let Some(iterator) = stack.last_mut() {
            let Some(child) = iterator.next() else {
                stack.pop();
                continue;
            };
            if child.is_relevant(include_anonymous) {
                buf.push(child);
            } else if child.relevant_child_count(include_anonymous) > 0 {
                stack.push(child.iterate_children());
            }
        }
    }

    /// Go `preorderChildren`: the loop of ts_node_child_iterator_next with
    /// ts_node__is_relevant and ts_node__relevant_child_count folded in, on an explicit
    /// stack. A mask of 0 walks every node.
    ///
    /// Each stack entry is a NodeChildIterator used as Go's frame (its children, alias
    /// sequence, position, structural index and loop index `i` = `child_index`); a node with
    /// no children gets an empty one, which is popped at once (Go's early returns).
    fn preorder_children<F: FnMut(Node<'t>) -> bool>(self, mask: u32, f: &mut F) {
        let tree = self.tree;
        let sp = &tree.pool;
        let lang = tree.language();
        let mut stack = vec![self.iterate_children()];
        while let Some(frame) = stack.last_mut() {
            if frame.child_index == frame.child_count {
                stack.pop();
                continue;
            }
            let i = frame.child_index;
            let child = sp.children[(frame.children + i) as usize];
            let (padding, size, visible, extra, mut symbol, mut descend, descend_hidden) =
                if child.is_inline() {
                    (
                        sp.padding(child),
                        sp.size(child),
                        child.0 & INL_VISIBLE != 0,
                        child.0 & INL_EXTRA != 0,
                        sp.symbol(child),
                        false,
                        false,
                    )
                } else {
                    let c = sp.ptr(child);
                    let descend = c.child_count > 0 && (mask == 0 || c.kinds() & mask != 0);
                    (
                        c.padding,
                        c.size,
                        c.flags & F_VISIBLE != 0,
                        c.flags & F_EXTRA != 0,
                        c.symbol,
                        descend,
                        descend && c.visible_child_count > 0,
                    )
                };
            let mut alias_symbol: Symbol = 0;
            if !extra {
                if let Some(start) = frame.alias_sequence {
                    alias_symbol = lang
                        .tables
                        .alias_sequences
                        .get(start + frame.structural_child_index as usize);
                }
                frame.structural_child_index += 1;
            }
            if i > 0 {
                frame.position = length_add(frame.position, padding);
            }
            let position = frame.position;
            // Go adds size after the recursive call; the child's walk never reads this frame.
            frame.position = length_add(frame.position, size);
            frame.child_index += 1;
            let slot = frame.children + i;
            if visible || alias_symbol != 0 {
                if alias_symbol != 0 {
                    symbol = alias_symbol;
                }
                let node = Node::node_new(tree, slot, position, alias_symbol);
                if mask == 0 || lang.kind_bit(symbol) & mask != 0 {
                    descend = f(node) && descend;
                }
                if descend {
                    stack.push(node.iterate_children());
                }
            } else if descend_hidden {
                stack.push(Node::node_new(tree, slot, position, alias_symbol).iterate_children());
            }
        }
    }

    /// ts_node__field_name_from_language (Go `fieldNameFromLanguage`). `None` for "".
    fn field_name_from_language(self, structural_child_index: u32) -> Option<&'static str> {
        let lang = self.language();
        let (start, len) =
            lang.field_map(u32::from(self.tree.pool.ptr(self.subtree()).production_id));
        for j in start..start + len {
            let m = lang.tables.field_map_entries.get(j);
            if !m.inherited && u32::from(m.child_index) == structural_child_index {
                return lang.field_name_for_id(m.field_id);
            }
        }
        None
    }

    /// ts_node_child_with_descendant (Go `childWithDescendant`)
    fn child_with_descendant(self, descendant: Node<'t>) -> Option<Node<'t>> {
        let mut this = self;
        let start_byte = descendant.start_byte();
        let end_byte = descendant.end_byte();
        let is_empty = start_byte == end_byte;

        'outer: loop {
            let mut iter = this.iterate_children();
            while let Some(c) = iter.next() {
                this = c;
                if this.start_byte() > start_byte {
                    return None;
                }
                if this.id == descendant.id {
                    return Some(this);
                }

                // If the descendant is empty, and the end byte is within `self`, we check
                // whether `self` contains it or not.
                if is_empty && iter.position.bytes >= end_byte && this.child_count() > 0 {
                    // If the child is not null, return self if it's relevant, else return
                    // the child.
                    if let Some(child) = this.child_with_descendant(descendant) {
                        if this.is_relevant(true) {
                            return Some(this);
                        }
                        return Some(child);
                    }
                }

                let more = if is_empty {
                    iter.position.bytes <= end_byte
                } else {
                    iter.position.bytes < end_byte
                };
                if !more && this.child_count() != 0 {
                    if this.is_relevant(true) {
                        break 'outer;
                    }
                    continue 'outer;
                }
            }
            return None;
        }

        Some(this)
    }
}

impl<'t> Node<'t> {
    /// ts_node_language
    pub fn language(self) -> &'static Language {
        self.tree.language()
    }

    /// ts_node_type
    pub fn kind(self) -> &'static str {
        let mut symbol = self.alias();
        if symbol == 0 {
            symbol = self.tree.pool.symbol(self.subtree());
        }
        self.language().symbol_name(symbol).unwrap_or("")
    }

    /// ts_node_symbol
    pub fn symbol(self) -> Symbol {
        let mut symbol = self.alias();
        if symbol == 0 {
            symbol = self.tree.pool.symbol(self.subtree());
        }
        self.language().public_symbol(symbol)
    }

    /// ts_node_is_named
    pub fn is_named(self) -> bool {
        let alias = self.alias();
        if alias != 0 {
            return self.language().symbol_metadata(alias).named;
        }
        self.tree.pool.named(self.subtree())
    }

    /// ts_node_is_error
    pub fn is_error(self) -> bool {
        self.symbol() == SYMBOL_ERROR
    }

    /// ts_node_is_missing
    pub fn is_missing(self) -> bool {
        self.tree.pool.missing(self.subtree())
    }

    /// ts_node_is_extra
    pub fn is_extra(self) -> bool {
        self.tree.pool.extra(self.subtree())
    }

    /// ts_node_has_error
    pub fn has_error(self) -> bool {
        self.tree.pool.error_cost(self.subtree()) > 0
    }

    /// ts_node_start_byte
    pub fn start_byte(self) -> u32 {
        self.x as u32
    }

    /// ts_node_end_byte
    pub fn end_byte(self) -> u32 {
        (self.x as u32).wrapping_add(self.tree.pool.size(self.subtree()).bytes)
    }

    /// ts_node_start_point
    pub fn start_position(self) -> Point {
        Point {
            row: self.y as u32,
            column: (self.y >> 32) as u32,
        }
    }

    /// ts_node_end_point
    pub fn end_position(self) -> Point {
        point_add(
            self.start_position(),
            self.tree.pool.size(self.subtree()).extent,
        )
    }

    /// ts_node_child_count
    pub fn child_count(self) -> u32 {
        self.relevant_child_count(true)
    }

    /// ts_node_child
    pub fn child(self, index: u32) -> Option<Node<'t>> {
        self.child_inner(index, true)
    }

    /// ts_node_named_child_count
    pub fn named_child_count(self) -> u32 {
        self.relevant_child_count(false)
    }

    /// ts_node_named_child
    pub fn named_child(self, index: u32) -> Option<Node<'t>> {
        self.child_inner(index, false)
    }

    /// Appends `child(0..child_count())` to `buf` in one pass. No C counterpart.
    pub fn children(self, buf: &mut Vec<Node<'t>>) {
        self.append_children(buf, true)
    }

    /// Appends `named_child(0..named_child_count())` to `buf` in one pass.
    pub fn named_children(self, buf: &mut Vec<Node<'t>>) {
        self.append_children(buf, false)
    }

    /// ts_node_child_by_field_name
    pub fn child_by_field_name(self, name: &str) -> Option<Node<'t>> {
        self.child_by_field_id(self.language().field_id_for_name(name))
    }

    /// ts_node_child_by_field_id
    pub fn child_by_field_id(self, field_id: FieldId) -> Option<Node<'t>> {
        let mut this = self;
        'recur: loop {
            if field_id == 0 || this.child_count() == 0 {
                return None;
            }

            let tree = this.tree;
            let sp = &tree.pool;
            let lang = tree.language();
            let entries = lang.tables.field_map_entries;
            let (start, len) = lang.field_map(u32::from(sp.ptr(this.subtree()).production_id));
            if len == 0 {
                return None;
            }
            // Go's fieldMap slice is entries[lo..hi].
            let (mut lo, mut hi) = (start, start + len);

            // The field mappings are sorted by their field id. Scan all the mappings to find
            // the ones for the given field id.
            while entries.get(lo).field_id < field_id {
                lo += 1;
                if lo == hi {
                    return None;
                }
            }
            while entries.get(hi - 1).field_id > field_id {
                hi -= 1;
                if lo == hi {
                    return None;
                }
            }

            // The loop of ts_node_child_iterator_next, as in preorder_children.
            let h = sp.ptr(this.subtree());
            let alias_sequence = lang.alias_sequence(u32::from(h.production_id));
            let mut position = this.position();
            let mut structural_child_index = 0u32;
            let children = &sp.children[h.children as usize..(h.children + h.child_count) as usize];
            for (i, &t) in children.iter().enumerate() {
                let (padding, size, visible, extra) = if t.is_inline() {
                    (
                        sp.padding(t),
                        sp.size(t),
                        t.0 & INL_VISIBLE != 0,
                        t.0 & INL_EXTRA != 0,
                    )
                } else {
                    let c = sp.ptr(t);
                    (
                        c.padding,
                        c.size,
                        c.flags & F_VISIBLE != 0,
                        c.flags & F_EXTRA != 0,
                    )
                };
                if i > 0 {
                    position = length_add(position, padding);
                }
                if !extra {
                    let mut alias_symbol: Symbol = 0;
                    if let Some(s) = alias_sequence {
                        alias_symbol = lang
                            .tables
                            .alias_sequences
                            .get(s + structural_child_index as usize);
                    }
                    let index = structural_child_index;
                    structural_child_index += 1;
                    if index >= u32::from(entries.get(lo).child_index) {
                        let child =
                            Node::node_new(tree, h.children + i as u32, position, alias_symbol);

                        // Hidden nodes' fields are "inherited" by their visible parent.
                        if entries.get(lo).inherited {
                            // If this is the *last* possible child node for this field, then
                            // perform a tail call to avoid recursion.
                            if hi - lo == 1 {
                                this = child;
                                continue 'recur;
                            }

                            // Otherwise, descend into this child, but if it doesn't contain
                            // the field, continue searching subsequent children.
                            let result = child.child_by_field_id(field_id);
                            if result.is_some() {
                                return result;
                            }
                            lo += 1;
                            if lo == hi {
                                return None;
                            }
                        } else if visible || alias_symbol != 0 {
                            return Some(child);
                        } else if child.child_count() > 0 {
                            // If the field refers to a hidden node with visible children,
                            // return the first visible child.
                            return child.child(0);
                        } else {
                            // Otherwise, continue searching subsequent children.
                            lo += 1;
                            if lo == hi {
                                return None;
                            }
                        }
                    }
                }
                position = length_add(position, size);
            }

            return None;
        }
    }

    /// ts_node_field_name_for_child. `None` where C returns NULL.
    pub fn field_name_for_child(self, mut child_index: u32) -> Option<&'static str> {
        let mut result = self;
        let mut did_descend = true;
        let mut inherited_field_name: Option<&'static str> = None;

        while did_descend {
            did_descend = false;

            let mut index = 0u32;
            let mut iterator = result.iterate_children();
            while let Some(child) = iterator.next() {
                if child.is_relevant(true) {
                    if index == child_index {
                        if child.is_extra() {
                            return None;
                        }
                        let field_name = result.field_name_from_language(
                            iterator.structural_child_index.wrapping_sub(1),
                        );
                        if field_name.is_some() {
                            return field_name;
                        }
                        return inherited_field_name;
                    }
                    index += 1;
                } else {
                    let grandchild_index = child_index.wrapping_sub(index);
                    let grandchild_count = child.relevant_child_count(true);
                    if grandchild_index < grandchild_count {
                        let field_name = result.field_name_from_language(
                            iterator.structural_child_index.wrapping_sub(1),
                        );
                        if field_name.is_some() {
                            inherited_field_name = field_name;
                        }

                        did_descend = true;
                        result = child;
                        child_index = grandchild_index;
                        break;
                    }
                    index += grandchild_count;
                }
            }
        }

        None
    }

    /// ts_node_parent: walks down from the root.
    pub fn parent(self) -> Option<Node<'t>> {
        let mut node = self.tree.root_node();
        if node.id == self.id {
            return None;
        }

        while let Some(next_node) = node.child_with_descendant(self) {
            if next_node.id == self.id {
                break;
            }
            node = next_node;
        }

        Some(node)
    }

    /// ts_node_eq: the same tree and the same subtree.
    pub fn equals(self, other: Node<'t>) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.id == other.id
    }

    /// `src[start_byte..end_byte]`, as go-tree-sitter's `Utf8Text`.
    pub fn utf8_text(self, src: &[u8]) -> &[u8] {
        &src[self.start_byte() as usize..self.end_byte() as usize]
    }

    /// ts_node_string, byte-identical.
    pub fn to_sexp(self) -> String {
        let alias_symbol = self.alias();
        let lang = self.language();
        self.tree.pool.string(
            self.subtree(),
            alias_symbol,
            lang.symbol_metadata(alias_symbol).visible,
            lang,
            false,
        )
    }

    /// Calls `f` for this node and, where `f` returns true, for everything below it, in
    /// document order. No C counterpart.
    pub fn preorder(self, mut f: impl FnMut(Node<'t>) -> bool) {
        if f(self) {
            self.preorder_children(0, &mut f);
        }
    }

    /// `preorder` with `f` called only for the kinds in `mask` (`Language::kind_mask`);
    /// subtrees without one of them are skipped.
    pub fn preorder_mask(self, mask: u32, mut f: impl FnMut(Node<'t>) -> bool) {
        if mask == 0 {
            self.preorder(f);
            return;
        }
        let sp = &self.tree.pool;
        let t = self.subtree();
        let mut symbol = self.alias();
        if symbol == 0 {
            symbol = sp.symbol(t);
        }
        if self.language().kind_bit(symbol) & mask != 0 && !f(self) {
            return;
        }
        if sp.child_count(t) > 0 && sp.ptr(t).kinds() & mask != 0 {
            self.preorder_children(mask, &mut f);
        }
    }

    /// ts_tree_cursor_new
    pub fn walk(self) -> TreeCursor<'t> {
        TreeCursor::new(self)
    }

    /// go-tree-sitter's `ChildrenByFieldName`; resets `cursor`.
    pub fn children_by_field_name(self, name: &str, cursor: &mut TreeCursor<'t>) -> Vec<Node<'t>> {
        let field_id = self.language().field_id_for_name(name);
        let mut done = field_id == 0;
        if !done {
            cursor.reset(self);
            cursor.goto_first_child();
        }
        let mut result = Vec::new();
        while !done {
            while cursor.field_id() != field_id {
                if !cursor.goto_next_sibling() {
                    return result;
                }
            }
            result.push(cursor.node());
            if !cursor.goto_next_sibling() {
                done = true;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests;
