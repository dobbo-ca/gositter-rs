//! treedump against a fake tree, with the lines gositter `internal/treedump/dump.go` writes
//! for it worked out by hand from `dump.go:80-273`. Plan Task 3.

use gositter_treedump::{DumpCursor, DumpNode, SEXP_SKIPPED, dump, go_quote, sampled};

/// One node of a fake tree. The fault fields make a node break one rule, so the dump
/// must print the check that catches it.
#[derive(Clone, Default)]
struct FakeNode {
    kind: &'static str,
    symbol: u16,
    named: bool,
    extra: bool,
    missing: bool,
    has_error: bool,
    start: u32,
    end: u32,
    start_pt: (u32, u32),
    end_pt: (u32, u32),
    parent: Option<usize>,
    children: Vec<usize>,
    /// The field of each child, by child index.
    fields: Vec<Option<&'static str>>,
    /// Fault: `child(child_count)` returns this node (the dump prints `OOB=`).
    oob_child: Option<usize>,
    /// Fault: `text` returns these bytes (the dump prints `T!`).
    wrong_text: Option<&'static [u8]>,
}

struct FakeTree {
    nodes: Vec<FakeNode>,
    sexp: String,
}

#[derive(Clone, Copy)]
struct F<'a> {
    tree: &'a FakeTree,
    i: usize,
}

impl<'a> F<'a> {
    fn node(self) -> &'a FakeNode {
        &self.tree.nodes[self.i]
    }
    fn at(self, i: usize) -> F<'a> {
        F { tree: self.tree, i }
    }
}

struct FakeCursor<'a> {
    tree: &'a FakeTree,
    /// (node, index in its parent's children) from the start node down.
    stack: Vec<(usize, usize)>,
}

impl<'a> DumpCursor for FakeCursor<'a> {
    type Node = F<'a>;
    fn node(&self) -> F<'a> {
        F {
            tree: self.tree,
            i: self.stack.last().unwrap().0,
        }
    }
    fn field(&self) -> Option<&'static str> {
        if self.stack.len() < 2 {
            return None;
        }
        let (_, index) = self.stack[self.stack.len() - 1];
        let (parent, _) = self.stack[self.stack.len() - 2];
        self.tree.nodes[parent].fields[index]
    }
    fn first_child(&mut self) -> bool {
        let (n, _) = *self.stack.last().unwrap();
        match self.tree.nodes[n].children.first() {
            Some(&c) => {
                self.stack.push((c, 0));
                true
            }
            None => false,
        }
    }
    fn next(&mut self) -> bool {
        if self.stack.len() < 2 {
            return false;
        }
        let (_, index) = self.stack[self.stack.len() - 1];
        let (parent, _) = self.stack[self.stack.len() - 2];
        match self.tree.nodes[parent].children.get(index + 1) {
            Some(&c) => {
                *self.stack.last_mut().unwrap() = (c, index + 1);
                true
            }
            None => false,
        }
    }
    fn up(&mut self) -> bool {
        if self.stack.len() < 2 {
            return false;
        }
        self.stack.pop();
        true
    }
}

impl<'a> DumpNode for F<'a> {
    type Cursor = FakeCursor<'a>;
    fn kind(self) -> &'static str {
        self.node().kind
    }
    fn symbol(self) -> u16 {
        self.node().symbol
    }
    fn named(self) -> bool {
        self.node().named
    }
    fn extra(self) -> bool {
        self.node().extra
    }
    fn missing(self) -> bool {
        self.node().missing
    }
    fn is_error(self) -> bool {
        self.node().kind == "ERROR"
    }
    fn has_error(self) -> bool {
        self.node().has_error
    }
    fn start(self) -> u32 {
        self.node().start
    }
    fn end(self) -> u32 {
        self.node().end
    }
    fn start_pt(self) -> (u32, u32) {
        self.node().start_pt
    }
    fn end_pt(self) -> (u32, u32) {
        self.node().end_pt
    }
    fn cc(self) -> u32 {
        self.node().children.len() as u32
    }
    fn ncc(self) -> u32 {
        self.node()
            .children
            .iter()
            .filter(|&&c| self.tree.nodes[c].named)
            .count() as u32
    }
    fn child(self, i: u32) -> Option<Self> {
        let n = self.node();
        match n.children.get(i as usize) {
            Some(&c) => Some(self.at(c)),
            None if i as usize == n.children.len() => n.oob_child.map(|c| self.at(c)),
            None => None,
        }
    }
    fn named_child(self, i: u32) -> Option<Self> {
        let named = self
            .node()
            .children
            .iter()
            .filter(|&&c| self.tree.nodes[c].named);
        named.copied().nth(i as usize).map(|c| self.at(c))
    }
    fn by_field(self, name: &str) -> Option<Self> {
        let n = self.node();
        let k = n.fields.iter().position(|f| *f == Some(name))?;
        Some(self.at(n.children[k]))
    }
    fn field_for_child(self, i: u32) -> Option<&'static str> {
        self.node().fields.get(i as usize).copied().flatten()
    }
    fn parent(self) -> Option<Self> {
        self.node().parent.map(|p| self.at(p))
    }
    fn same(self, other: Self) -> bool {
        self.i == other.i
    }
    fn text(self, src: &[u8]) -> &[u8] {
        let n = self.node();
        match n.wrong_text {
            Some(t) => t,
            None => &src[n.start as usize..n.end as usize],
        }
    }
    fn walk(self) -> FakeCursor<'a> {
        FakeCursor {
            tree: self.tree,
            stack: vec![(self.i, 0)],
        }
    }
    fn sexp(self) -> String {
        self.tree.sexp.clone()
    }
    fn children_by_field(self, name: &str, cursor: &mut FakeCursor<'a>) -> Vec<Self> {
        *cursor = self.walk();
        let n = self.node();
        n.fields
            .iter()
            .zip(&n.children)
            .filter(|(f, _)| **f == Some(name))
            .map(|(_, &c)| self.at(c))
            .collect()
    }
}

/// document over (pair with field key, MISSING number, extra comment), src "abcdefgh".
fn small_tree() -> FakeTree {
    let leaf = |kind, symbol, start, end| FakeNode {
        kind,
        symbol,
        named: true,
        start,
        end,
        start_pt: (0, start),
        end_pt: (0, end),
        parent: Some(0),
        ..Default::default()
    };
    let root = FakeNode {
        kind: "document",
        symbol: 15,
        named: true,
        has_error: true,
        start: 0,
        end: 8,
        start_pt: (0, 0),
        end_pt: (0, 8),
        children: vec![1, 2, 3],
        fields: vec![Some("key"), None, None],
        ..Default::default()
    };
    let pair = FakeNode {
        oob_child: Some(2),
        ..leaf("pair", 3, 0, 3)
    };
    let missing = FakeNode {
        missing: true,
        ..leaf("number", 4, 3, 3)
    };
    let comment = FakeNode {
        extra: true,
        wrong_text: Some(b"xxxx"),
        ..leaf("comment", 5, 4, 8)
    };
    FakeTree {
        nodes: vec![root, pair, missing, comment],
        sexp: "(document (pair) (MISSING number) (comment))".to_string(),
    }
}

#[test]
#[ignore = "gositter-rs-2f0.3"]
fn dump_fake_tree() {
    let tree = small_tree();
    let mut out = Vec::new();
    let count = dump(
        &mut out,
        F { tree: &tree, i: 0 },
        &["key", "value"],
        b"abcdefgh",
    );
    assert_eq!(count, 4);
    let want = concat!(
        "0 \"document\" 15 n---h - 0-8 0:0-0:8 3 3 key=0[0] F0=key N=0,1,2\n",
        "1 \"pair\" 3 n---- key 0-3 0:0-0:3 0 0 P=ok OOB=3:3:\"number\"/nil/\n",
        "1 \"number\" 4 n-m-- - 3-3 0:3-0:3 0 0 P=ok\n",
        "1 \"comment\" 5 nx--- - 4-8 0:4-0:8 0 0 P=ok T!\n",
        "SEXP (document (pair) (MISSING number) (comment))\n",
    );
    assert_eq!(String::from_utf8(out).unwrap(), want);
}

/// A chain of `n` anonymous nodes, each the only child of the one before.
fn chain(n: usize) -> FakeTree {
    let nodes = (0..n)
        .map(|i| FakeNode {
            kind: "a",
            symbol: 1,
            parent: i.checked_sub(1),
            children: if i + 1 < n { vec![i + 1] } else { vec![] },
            fields: if i + 1 < n { vec![None] } else { vec![] },
            ..Default::default()
        })
        .collect();
    FakeTree {
        nodes,
        sexp: "(a)".to_string(),
    }
}

#[test]
#[ignore = "gositter-rs-2f0.3"]
fn sexp_only_up_to_depth_100() {
    // dump.go:117-123: sexpMaxDepth = 100.
    let at_100 = chain(101);
    let mut out = Vec::new();
    assert_eq!(
        dump(
            &mut out,
            F {
                tree: &at_100,
                i: 0
            },
            &[],
            b""
        ),
        101
    );
    assert!(out.ends_with(b"\nSEXP (a)\n"));
    let at_101 = chain(102);
    let mut out = Vec::new();
    assert_eq!(
        dump(
            &mut out,
            F {
                tree: &at_101,
                i: 0
            },
            &[],
            b""
        ),
        102
    );
    assert!(out.ends_with(SEXP_SKIPPED.as_bytes()));
    // dump.go:161: the parent check runs to depth 64, then only at every 101st node.
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1], "1 \"a\" 1 ----- - 0-0 0:0-0:0 1 0 P=ok");
    assert_eq!(lines[65], "65 \"a\" 1 ----- - 0-0 0:0-0:0 1 0");
    assert_eq!(lines[101], "101 \"a\" 1 ----- - 0-0 0:0-0:0 0 0 P=ok");
}

#[test]
#[ignore = "gositter-rs-2f0.3"]
fn sampled_rule() {
    // dump.go:52-54.
    for n in [0u32, 1, 256, 257, 1000] {
        for i in 0..n {
            assert_eq!(
                sampled(i, n),
                n <= 256 || i < 128 || i >= n - 64 || i % 257 == 0,
                "{i} of {n}"
            );
        }
    }
}

#[test]
#[ignore = "gositter-rs-2f0.3"]
fn go_quote_cases() {
    // Go strconv.Quote. Kind names of the 21 grammars include all of these shapes.
    assert_eq!(go_quote("\""), r#""\"""#);
    assert_eq!(go_quote("\\"), r#""\\""#);
    assert_eq!(go_quote("\0"), r#""\x00""#);
    assert_eq!(go_quote("\n"), r#""\n""#);
    assert_eq!(go_quote("\t\r\x07\x08\x0b\x0c"), r#""\t\r\a\b\v\f""#);
    assert_eq!(go_quote("\x7f"), r#""\x7f""#);
    assert_eq!(go_quote("\x1b"), r#""\x1b""#);
    assert_eq!(go_quote("–>"), "\"–>\"");
    assert_eq!(go_quote("[–>"), "\"[–>\"");
    assert_eq!(go_quote("é"), "\"é\"");
    assert_eq!(go_quote("\u{a0}"), "\"\\u00a0\"");
    assert_eq!(go_quote("\u{200b}"), "\"\\u200b\"");
    assert_eq!(go_quote("\u{feff}"), "\"\\ufeff\"");
    assert_eq!(go_quote("\u{e000}"), "\"\\ue000\"");
    assert_eq!(go_quote("\u{10ffff}"), r#""\U0010ffff""#);
    assert_eq!(go_quote("a b"), "\"a b\"");
    assert_eq!(go_quote(""), "\"\"");
}
