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

/// `sexpMaxDepth`: `ts_node_string` recurses once per level, so only shallow trees get it.
const SEXP_MAX_DEPTH: usize = 100;

/// Writes one line per node, pre-order, then the `SEXP` line, as gositter `treedump.Dump`;
/// returns the node count. `fields` are the language's field names in id order from 1.
///
/// Each line is `depth "kind" symbol flags cursorField start-end row:col-row:col childCount
/// namedChildCount [checks]`; the flags are n(amed) x(extra) m(issing) e(rror) h(as_error),
/// `-` when unset. The checks (`P=`, `<field>=i[i,j]`, `F<i>=`, `C<i>=`, `N=`, `T!`, `OOB=`)
/// are printed only when they have something to say; see gositter `dump.go:80-101`.
pub fn dump<N: DumpNode>(out: &mut Vec<u8>, root: N, fields: &[&str], src: &[u8]) -> usize {
    // Own copy: the probe name must not land in the caller's slice.
    let mut all_fields: Vec<&str> = fields.to_vec();
    all_fields.push("nonexistent_zz");
    let mut d: Dumper<'_, N> = Dumper {
        out: &mut *out,
        src,
        fields: all_fields,
        tc: root.walk(),
        scratch: root.walk(),
        ordinal: 0,
        max_depth: 0,
        kids: Vec::new(),
        kfield: Vec::new(),
    };
    d.visit(0, None, 0);
    let (count, max_depth) = (d.ordinal, d.max_depth);
    if max_depth <= SEXP_MAX_DEPTH {
        out.extend_from_slice(b"SEXP ");
        out.extend_from_slice(root.sexp().as_bytes());
        out.push(b'\n');
    } else {
        out.extend_from_slice(SEXP_SKIPPED.as_bytes());
    }
    count
}

struct Dumper<'a, N: DumpNode> {
    out: &'a mut Vec<u8>,
    src: &'a [u8],
    fields: Vec<&'a str>,
    /// The traversal.
    tc: N::Cursor,
    /// For `children_by_field`, which resets it.
    scratch: N::Cursor,
    ordinal: usize,
    max_depth: usize,
    /// Per-depth scratch rows, reused.
    kids: Vec<Vec<N>>,
    kfield: Vec<Vec<Option<&'a str>>>,
}

/// `start:end:"kind"`, or `nil`.
fn ident<N: DumpNode>(n: Option<N>) -> String {
    match n {
        None => "nil".to_string(),
        Some(n) => format!("{}:{}:{}", n.start(), n.end(), go_quote(n.kind())),
    }
}

/// The index of `n` among `kids` at or after `from`, by node equality.
fn index_from<N: DumpNode>(kids: &[N], n: N, from: usize) -> Option<usize> {
    (from..kids.len()).find(|&i| kids[i].same(n))
}

impl<N: DumpNode> Dumper<'_, N> {
    fn visit(&mut self, depth: usize, parent: Option<N>, index_in_parent: usize) {
        use std::fmt::Write;

        self.max_depth = self.max_depth.max(depth);
        let n = self.tc.node();
        let ordinal = self.ordinal;
        self.ordinal += 1;

        // Pass 1 over the children with the cursor: linear, and the reference list the
        // index-based accessors are checked against.
        if depth == self.kids.len() {
            self.kids.push(Vec::new());
            self.kfield.push(Vec::new());
        }
        let mut kids = std::mem::take(&mut self.kids[depth]);
        let mut kfield = std::mem::take(&mut self.kfield[depth]);
        kids.clear();
        kfield.clear();
        let cursor_field = self.tc.field();
        if self.tc.first_child() {
            loop {
                kids.push(self.tc.node());
                kfield.push(self.tc.field());
                if !self.tc.next() {
                    break;
                }
            }
            self.tc.up();
        }

        let mut flags = *b"-----";
        for (i, on) in [
            n.named(),
            n.extra(),
            n.missing(),
            n.is_error(),
            n.has_error(),
        ]
        .into_iter()
        .enumerate()
        {
            if on {
                flags[i] = b"nxmeh"[i];
            }
        }
        let flags = std::str::from_utf8(&flags).unwrap();
        let (sr, sc) = n.start_pt();
        let (er, ec) = n.end_pt();
        let (cc, ncc) = (n.cc(), n.ncc());
        let mut w = String::new();
        let _ = write!(
            w,
            "{depth} {} {} {flags} {} {}-{} {sr}:{sc}-{er}:{ec} {cc} {ncc}",
            go_quote(n.kind()),
            n.symbol(),
            cursor_field.unwrap_or("-"),
            n.start(),
            n.end(),
        );
        if cc as usize != kids.len() {
            let _ = write!(w, " CURSORKIDS={}", kids.len());
        }

        if (depth <= 64 && index_in_parent <= 256) || ordinal.is_multiple_of(101) {
            match (n.parent(), parent) {
                (None, None) => {}
                (Some(p), Some(q)) if p.same(q) => w.push_str(" P=ok"),
                (p, _) => {
                    w.push_str(" P=");
                    w.push_str(&ident(p));
                }
            }
        }

        for &f in &self.fields {
            let c = n.by_field(f);
            let in_cursor = kfield.contains(&Some(f));
            if c.is_none() && !in_cursor {
                continue;
            }
            let _ = write!(w, " {f}=");
            match c {
                None => w.push_str("nil"),
                Some(c) => match index_from(&kids, c, 0) {
                    Some(i) => {
                        let _ = write!(w, "{i}");
                    }
                    None => w.push_str(&ident(Some(c))),
                },
            }
            w.push('[');
            let mut from = 0;
            for (k, c) in n
                .children_by_field(f, &mut self.scratch)
                .into_iter()
                .enumerate()
            {
                if k > 0 {
                    w.push(',');
                }
                match index_from(&kids, c, from) {
                    Some(i) => {
                        let _ = write!(w, "{i}");
                        from = i + 1;
                    }
                    None => w.push_str(&ident(Some(c))),
                }
            }
            w.push(']');
        }

        for i in 0..cc {
            if !sampled(i, cc) {
                continue;
            }
            if let Some(f) = n.field_for_child(i) {
                let _ = write!(w, " F{i}={f}");
            }
            let c = n.child(i);
            let same = match c {
                Some(c) => (i as usize) < kids.len() && c.same(kids[i as usize]),
                None => false,
            };
            if !same {
                let _ = write!(w, " C{i}={}", ident(c));
            }
        }
        if ncc > 0 {
            w.push_str(" N=");
            let mut from = 0;
            for k in 0..ncc {
                if !sampled(k, ncc) {
                    from = 0; // a gap: the next hit can be anywhere
                    continue;
                }
                if k > 0 {
                    w.push(',');
                }
                match n.named_child(k) {
                    None => w.push_str("nil"),
                    Some(c) => match index_from(&kids, c, from) {
                        Some(i) => {
                            let _ = write!(w, "{i}");
                            from = i + 1;
                        }
                        None => w.push_str(&ident(Some(c))),
                    },
                }
            }
        }
        let (s, e) = (n.start(), n.end());
        if s > e
            || e as usize > self.src.len()
            || n.text(self.src) != &self.src[s as usize..e as usize]
        {
            w.push_str(" T!");
        }
        let (a, b, f) = (n.child(cc), n.named_child(ncc), n.field_for_child(cc));
        if a.is_some() || b.is_some() || f.is_some() {
            let _ = write!(w, " OOB={}/{}/{}", ident(a), ident(b), f.unwrap_or(""));
        }
        w.push('\n');
        self.out.extend_from_slice(w.as_bytes());

        // Pass 2: descend. kids is not read after this point, so the recursion may reuse
        // the deeper scratch rows only.
        self.kids[depth] = kids;
        self.kfield[depth] = kfield;
        if self.tc.first_child() {
            let mut i = 0;
            loop {
                self.visit(depth + 1, Some(n), i);
                i += 1;
                if !self.tc.next() {
                    break;
                }
            }
            self.tc.up();
        }
    }
}

/// Go `strconv.Quote`, as `%q` writes it.
pub fn go_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            c if go_is_print(c) => out.push(c),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    out.push('"');
    out
}

/// Go `unicode.IsPrint`: letters, marks, numbers, punctuation, symbols and U+0020.
///
/// ponytail: this lists only the code points that are not printable among the assigned ones
/// (Cc, Zs but U+0020, Zl, Zp, Cf, Co) and the noncharacters, from Unicode 17 as Go 1.27 has
/// it; any other unassigned code point counts as printable, where Go escapes it. Kind names
/// hold no unassigned code point; `gsdump kinds` (plan Task 12) diffs every kind name of the
/// 21 grammars against Go. Port Go's full range tables if a name ever needs them.
fn go_is_print(c: char) -> bool {
    let c = c as u32;
    if c < 0x80 {
        return (0x20..0x7f).contains(&c);
    }
    if c & 0xFFFE == 0xFFFE {
        return false; // noncharacters U+xxFFFE and U+xxFFFF
    }
    NOT_PRINT
        .binary_search_by(|&(lo, hi)| {
            if hi < c {
                std::cmp::Ordering::Less
            } else if lo > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_err()
}

/// Assigned code points from U+0080 up that Go's `unicode.IsPrint` rejects (categories Cc,
/// Zs but U+0020, Zl, Zp, Cf, Co), plus U+FDD0..U+FDEF; sorted, inclusive.
const NOT_PRINT: &[(u32, u32)] = &[
    (0x0080, 0x00A0),
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x1680, 0x1680),
    (0x180E, 0x180E),
    (0x2000, 0x200F),
    (0x2028, 0x202F),
    (0x205F, 0x2064),
    (0x2066, 0x206F),
    (0x3000, 0x3000),
    (0xE000, 0xF8FF),
    (0xFDD0, 0xFDEF),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
    (0xF0000, 0x10FFFF),
];

/// The fixed sampling rule for wide nodes (`dump.go` `sampled`): `child`, `named_child`,
/// `field_name_for_child` and `parent` are linear, so a dump of every node of a wide or deep
/// tree would be quadratic. Both sides sample the same nodes.
#[doc(hidden)]
pub fn sampled(i: u32, n: u32) -> bool {
    n <= 256 || i < 128 || i >= n - 64 || i.is_multiple_of(257)
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
