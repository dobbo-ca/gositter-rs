//! Port of gositter v0.2.0 `internal/treedump` (`dump.go`, `gs.go`): one dump routine for
//! both runtimes, so that a format difference cannot hide or fake a tree difference. The
//! C goldens in `testdata/regress` are in this format.

/// A non-null node of a tree to dump: gositter's `treedump.Node` interface, method for
/// method. A null node is `None`.
pub trait DumpNode: Copy {
    type Cursor: DumpCursor<Node = Self>;
    fn kind(self) -> &'static str;
    fn symbol(self) -> u16;
    fn named(self) -> bool;
    fn extra(self) -> bool;
    fn missing(self) -> bool;
    fn is_error(self) -> bool;
    fn has_error(self) -> bool;
    fn start(self) -> u32;
    fn end(self) -> u32;
    fn start_pt(self) -> (u32, u32);
    fn end_pt(self) -> (u32, u32);
    fn cc(self) -> u32;
    fn ncc(self) -> u32;
    fn child(self, i: u32) -> Option<Self>;
    fn named_child(self, i: u32) -> Option<Self>;
    fn by_field(self, name: &str) -> Option<Self>;
    /// `None` where Go returns "".
    fn field_for_child(self, i: u32) -> Option<&'static str>;
    fn parent(self) -> Option<Self>;
    /// Go `Eq`: ts_node_eq.
    fn same(self, other: Self) -> bool;
    fn text(self, src: &[u8]) -> &[u8];
    fn walk(self) -> Self::Cursor;
    fn sexp(self) -> String;
    fn children_by_field(self, name: &str, cursor: &mut Self::Cursor) -> Vec<Self>;
}

/// gositter's `treedump.Cursor`. Go's `Close` is `Drop`.
pub trait DumpCursor {
    type Node;
    fn node(&self) -> Self::Node;
    /// `None` where Go returns "".
    fn field(&self) -> Option<&'static str>;
    fn first_child(&mut self) -> bool;
    fn next(&mut self) -> bool;
    fn up(&mut self) -> bool;
}

/// The last line of a dump whose tree is deeper than 100 levels.
pub const SEXP_SKIPPED: &str = "SEXP skipped\n";

/// Writes one line per node, pre-order, then the `SEXP` line, as gositter `treedump.Dump`;
/// returns the node count. `fields` are the language's field names in id order from 1.
pub fn dump<N: DumpNode>(_out: &mut Vec<u8>, _root: N, _fields: &[&str], _src: &[u8]) -> usize {
    todo!("gositter-rs-2f0.3")
}

/// Go `strconv.Quote`, as `%q` writes it.
pub fn go_quote(_s: &str) -> String {
    todo!("gositter-rs-2f0.3")
}

/// The fixed sampling rule for wide nodes (`dump.go` `sampled`).
#[doc(hidden)]
pub fn sampled(_i: u32, _n: u32) -> bool {
    todo!("gositter-rs-2f0.3")
}

/// gositter `gs.go`.
impl<'t> DumpNode for gositter::Node<'t> {
    type Cursor = gositter::TreeCursor<'t>;

    fn kind(self) -> &'static str {
        gositter::Node::kind(self)
    }
    fn symbol(self) -> u16 {
        gositter::Node::symbol(self)
    }
    fn named(self) -> bool {
        self.is_named()
    }
    fn extra(self) -> bool {
        self.is_extra()
    }
    fn missing(self) -> bool {
        self.is_missing()
    }
    fn is_error(self) -> bool {
        gositter::Node::is_error(self)
    }
    fn has_error(self) -> bool {
        gositter::Node::has_error(self)
    }
    fn start(self) -> u32 {
        self.start_byte()
    }
    fn end(self) -> u32 {
        self.end_byte()
    }
    fn start_pt(self) -> (u32, u32) {
        let p = self.start_position();
        (p.row, p.column)
    }
    fn end_pt(self) -> (u32, u32) {
        let p = self.end_position();
        (p.row, p.column)
    }
    fn cc(self) -> u32 {
        self.child_count()
    }
    fn ncc(self) -> u32 {
        self.named_child_count()
    }
    fn child(self, i: u32) -> Option<Self> {
        gositter::Node::child(self, i)
    }
    fn named_child(self, i: u32) -> Option<Self> {
        gositter::Node::named_child(self, i)
    }
    fn by_field(self, name: &str) -> Option<Self> {
        self.child_by_field_name(name)
    }
    fn field_for_child(self, i: u32) -> Option<&'static str> {
        self.field_name_for_child(i)
    }
    fn parent(self) -> Option<Self> {
        gositter::Node::parent(self)
    }
    fn same(self, other: Self) -> bool {
        self.equals(other)
    }
    fn text(self, src: &[u8]) -> &[u8] {
        self.utf8_text(src)
    }
    fn walk(self) -> Self::Cursor {
        gositter::Node::walk(self)
    }
    fn sexp(self) -> String {
        self.to_sexp()
    }
    fn children_by_field(self, name: &str, cursor: &mut Self::Cursor) -> Vec<Self> {
        self.children_by_field_name(name, cursor)
    }
}

impl<'t> DumpCursor for gositter::TreeCursor<'t> {
    type Node = gositter::Node<'t>;

    fn node(&self) -> Self::Node {
        gositter::TreeCursor::node(self)
    }
    fn field(&self) -> Option<&'static str> {
        self.field_name()
    }
    fn first_child(&mut self) -> bool {
        self.goto_first_child()
    }
    fn next(&mut self) -> bool {
        self.goto_next_sibling()
    }
    fn up(&mut self) -> bool {
        self.goto_parent()
    }
}
