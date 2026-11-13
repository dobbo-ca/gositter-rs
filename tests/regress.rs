//! The regress corpus: 1,423 cases with the C runtime's dump of each (gositter
//! `regress_test.go`): the corpus that CI unzips, and the replay of each case against its
//! dump (ignored until the parser and the grammars land, plan Tasks 11 and 15).

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use gositter::{FieldId, Language, Node, Parser, SummaryCheck, TreeCursor};

/// Cases per directory, from gositter v0.2.0 `testdata/regress`.
const CASES: [(&str, usize); 21] = [
    ("bash", 65),
    ("c", 50),
    ("cpp", 85),
    ("c-sharp", 73),
    ("go", 51),
    ("hcl", 71),
    ("java", 53),
    ("javascript", 85),
    ("json", 50),
    ("julia", 68),
    ("kotlin", 79),
    ("lua", 80),
    ("php", 66),
    ("python", 63),
    ("ruby", 70),
    ("rust", 85),
    ("scala", 78),
    ("tsx", 78),
    ("typescript", 65),
    ("verilog", 61),
    ("zig", 47),
];

#[test]
fn corpus_is_complete_and_unzipped() {
    let mut total = 0;
    for (dir, want) in CASES {
        assert!(common::DIRS.contains(&dir), "{dir}");
        let goldens = common::goldens(dir);
        assert_eq!(goldens.len(), want, "{dir}: unzipped goldens");
        for golden in &goldens {
            let source = golden.with_extension("");
            assert!(source.is_file(), "{}: no source", golden.display());
            let text = std::fs::read(golden).unwrap();
            assert!(
                text.starts_with(b"0 \""),
                "{}: not a dump",
                golden.display()
            );
            assert!(
                text.ends_with(b"\n"),
                "{}: no final newline",
                golden.display()
            );
        }
        total += goldens.len();
    }
    assert_eq!(total, 1423);
    assert_eq!(CASES.len(), common::DIRS.len());
}

#[test]
fn goldens_are_sorted_by_path() {
    let g = common::goldens("json");
    let mut sorted = g.clone();
    sorted.sort();
    assert_eq!(g, sorted);
}

/// Go `regressKinds` (`init()`): ERROR and up to 31 symbol names spread over the table, named
/// to `kind_mask` before the language parses.
fn regress_kinds(l: &'static Language) -> HashMap<&'static str, u32> {
    let mut kinds = HashMap::new();
    kinds.insert("ERROR", l.kind_mask(&["ERROR"]));
    let names = l.tables.symbol_names;
    let mut i = 1;
    while i < names.len() && kinds.len() < 32 {
        kinds.insert(names[i], l.kind_mask(&[names[i]]));
        i += names.len() / 40 + 1;
    }
    kinds
}

/// The language of `dir`, with its regress kinds named first. Go names them in `init()`, before
/// any test parses; here the tests of this binary share each language across threads, and a kind
/// named during another test's parse changes the kind bits of that parse's summaries.
fn language(dir: &str) -> &'static Language {
    let l = common::language(dir).unwrap_or_else(|| panic!("{dir} grammar"));
    regress_kinds(l);
    l
}

/// Go `TestRegress` for one directory: replays each source against its C dump, twice on one
/// parser, the second time in reverse order, so state left over from another file shows up
/// as well.
fn regress(dir: &'static str) {
    common::run_on_big_stack(move || {
        let dumps = common::goldens(dir);
        assert!(!dumps.is_empty(), "no cases");
        let want_cases = CASES.iter().find(|(d, _)| *d == dir).unwrap().1;
        assert_eq!(dumps.len(), want_cases, "{dir}: cases");
        let l = language(dir);
        let kinds = regress_kinds(l);
        // verilog has no fields and an empty name table.
        let fields: &[&str] = l.tables.field_names.get(1..).unwrap_or_default();
        let mut p = Parser::new();
        p.set_language(l).unwrap();
        let mut errs = Vec::new();
        let (mut with_errors, mut one_child) = (0, 0);
        let mut summaries = SummaryCheck::default();
        for pass in 0..2 {
            let mut order: Vec<&PathBuf> = dumps.iter().collect();
            if pass == 1 {
                order.reverse();
            }
            for golden in order {
                let name = golden.display().to_string();
                let src = std::fs::read(golden.with_extension("")).unwrap();
                let want = std::fs::read(golden).unwrap();
                let root = p.parse(&src).expect("parse").root_node();
                if pass == 0 && root.has_error() {
                    with_errors += 1;
                }
                if pass == 0 {
                    check_children(&mut errs, &name, root);
                    check_preorder(&mut errs, &name, root);
                    check_cursor(&mut errs, &name, root);
                    check_preorder_mask(&mut errs, &name, root, &kinds);
                }
                let mut got = Vec::new();
                gositter_treedump::dump(&mut got, root, fields, &src);
                if got != want {
                    errs.push(format!("{name} (pass {pass}): {}", first_diff(&want, &got)));
                }
                // The hooks borrow the parser, so they run once the tree is no longer used
                // (Go runs them before the dump; neither changes the parser).
                if pass == 0 {
                    if !p.repeat_flags_ok() {
                        errs.push(format!("{name}: fHasRepeat misses a repeat node"));
                    }
                    let (n, ok) = p.one_child_summary_ok();
                    if !ok {
                        errs.push(format!(
                            "{name}: summarizeOneChild differs from summarizeChildren"
                        ));
                    }
                    one_child += n;
                    add_summaries(&mut errs, &name, &p, &mut summaries);
                }
            }
        }
        let (scans, ok) = p.error_scans_ok();
        if !ok {
            errs.push("a kept error scan differs from a fresh one".to_string());
        }
        println!(
            "{dir}: {} cases, {with_errors} with errors, {one_child} one-child nodes checked, {scans} error scans kept",
            dumps.len()
        );
        println!(
            "{dir}: {} summaries checked, {} of two heap children, {} that depend on the column, {} resumed",
            summaries.nodes, summaries.two_heap, summaries.column, summaries.resumed
        );
        assert!(errs.is_empty(), "{}", errs.join("\n"));
    });
}

#[test]
fn regress_json() {
    regress("json");
}

macro_rules! regress_tests {
    ($($name:ident => $dir:literal,)*) => {$(
        #[test]
        #[ignore = "gositter-rs-2f0.15"]
        fn $name() {
            regress($dir);
        }
    )*};
}

regress_tests! {
    regress_bash => "bash",
    regress_c => "c",
    regress_cpp => "cpp",
    regress_c_sharp => "c-sharp",
    regress_go => "go",
    regress_hcl => "hcl",
    regress_java => "java",
    regress_javascript => "javascript",
    regress_julia => "julia",
    regress_kotlin => "kotlin",
    regress_lua => "lua",
    regress_php => "php",
    regress_python => "python",
    regress_ruby => "ruby",
    regress_rust => "rust",
    regress_scala => "scala",
    regress_tsx => "tsx",
    regress_typescript => "typescript",
    regress_verilog => "verilog",
    regress_zig => "zig",
}

/// Checks the summaries of the last tree of `p` and adds the counts to `sum`.
fn add_summaries(errs: &mut Vec<String>, name: &str, p: &Parser, sum: &mut SummaryCheck) {
    let c = p.summaries_ok();
    if !c.bad.is_empty() {
        errs.push(format!("{name}: {}", c.bad));
    }
    sum.nodes += c.nodes;
    sum.two_heap += c.two_heap;
    sum.column += c.column;
    sum.resumed += c.resumed;
}

/// Go `TestSummaries` for some of `summaryCases`: shapes the summary paths split on, by
/// hand. Each summary is checked against the reference in `src/export.rs`.
fn summaries_of(cases: Vec<(&'static str, &'static str, String)>) {
    common::run_on_big_stack(move || {
        let mut errs = Vec::new();
        for (name, dir, src) in cases {
            let mut p = Parser::new();
            p.set_language(language(dir)).unwrap();
            let mut sum = SummaryCheck::default();
            p.parse(src.as_bytes());
            add_summaries(&mut errs, name, &p, &mut sum);
            println!(
                "{name}: {} summaries checked, {} of two heap children, {} that depend on the column, {} resumed",
                sum.nodes, sum.two_heap, sum.column, sum.resumed
            );
        }
        assert!(errs.is_empty(), "{}", errs.join("\n"));
    });
}

#[test]
fn summaries_json() {
    summaries_of(vec![
        (
            "json wide",
            "json",
            "[".to_string() + &"1,".repeat(6000) + "1]",
        ),
        (
            "json pairs",
            "json",
            "{".to_string() + &"\"a\":[{\"b\":null}],".repeat(2000) + "\"z\":0}",
        ),
        ("json errors", "json", "[1,,{]\"a\" 2}".repeat(300)),
    ]);
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn summaries() {
    summaries_of(vec![
        ("c define", "c", "#define A( ".repeat(400)),
        ("c extras", "c", "int a; /* c */ // d\n".repeat(2000)),
        (
            "c nested errors",
            "c",
            "int f( { if (x { #define B(\n ) y = ; } ".repeat(200),
        ),
        ("cpp define", "cpp", "#define A( ".repeat(400)),
        ("bash expansion", "bash", "${".repeat(400)),
        (
            "bash heredocs",
            "bash",
            "cat <<-EOF\n\tx $y\n\tEOF\n".repeat(300) + &"a=(1 2\n".repeat(50),
        ),
        (
            "hcl",
            "hcl",
            "a = 1 # c\nb \"x\" {\n  c = [1, 2]\n}\n".repeat(1000),
        ),
        ("hcl errors", "hcl", "a = { b = [ 1, \n".repeat(300)),
        (
            "scala indent",
            "scala",
            "def f(x: Int) =\n  if x > 0 then\n    x\n  else\n    -x\n".repeat(300),
        ),
        (
            "scala errors",
            "scala",
            "object A:\n  def f =\n    val x = (\n  ]\n".repeat(100),
        ),
    ]);
}

/// Go `firstDiff`.
fn first_diff(want: &[u8], got: &[u8]) -> String {
    let w: Vec<&[u8]> = want.split(|&b| b == b'\n').collect();
    let g: Vec<&[u8]> = got.split(|&b| b == b'\n').collect();
    for (i, (a, b)) in w.iter().zip(&g).enumerate() {
        if a != b {
            return format!("line {}\n want: {}\n  got: {}", i + 1, clip(a), clip(b));
        }
    }
    format!("line counts differ: want {}, got {}", w.len(), g.len())
}

fn clip(b: &[u8]) -> String {
    String::from_utf8_lossy(&b[..b.len().min(400)]).into_owned()
}

/// `children` and `named_children` must list exactly what `child(i)` and `named_child(i)`
/// return. Wide nodes are sampled, as in treedump.
fn check_children(errs: &mut Vec<String>, name: &str, root: Node<'_>) {
    let mut all = Vec::new();
    let mut named = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        all.clear();
        n.children(&mut all);
        named.clear();
        n.named_children(&mut named);
        if all.len() as u32 != n.child_count() || named.len() as u32 != n.named_child_count() {
            errs.push(format!(
                "{name}: {} at {}: Children {} of {}, NamedChildren {} of {}",
                n.kind(),
                n.start_byte(),
                all.len(),
                n.child_count(),
                named.len(),
                n.named_child_count()
            ));
            return;
        }
        for (i, &c) in all.iter().enumerate() {
            if (all.len() <= 256 || i.is_multiple_of(37)) && Some(c) != n.child(i as u32) {
                errs.push(format!(
                    "{name}: {} at {}: Children[{i}] differs from Child",
                    n.kind(),
                    n.start_byte()
                ));
                return;
            }
        }
        for (i, &c) in named.iter().enumerate() {
            if (named.len() <= 256 || i.is_multiple_of(37)) && Some(c) != n.named_child(i as u32) {
                errs.push(format!(
                    "{name}: {} at {}: NamedChildren[{i}] differs from NamedChild",
                    n.kind(),
                    n.start_byte()
                ));
                return;
            }
        }
        // Every index and two past the end, against the loop `child` had before the one for
        // a node with no alias sequence.
        for anonymous in [true, false] {
            let count = if anonymous {
                all.len() as u32
            } else {
                named.len() as u32
            };
            for i in 0..count + 2 {
                if count > 2000 && i < count && !i.is_multiple_of(37) {
                    continue;
                }
                let got = if anonymous {
                    n.child(i)
                } else {
                    n.named_child(i)
                };
                if got != n.child_ref(i, anonymous) {
                    errs.push(format!(
                        "{name}: {} at {}: child {i} of {count} (anonymous {anonymous}) differs from ChildRef",
                        n.kind(),
                        n.start_byte()
                    ));
                    return;
                }
            }
        }
        // The dump compares a field's child by index only; this pins its position and alias.
        for id in 1..=n.language().tables.field_count {
            let id = id as FieldId;
            if n.child_by_field_id(id) != n.child_by_field_id_ref(id) {
                errs.push(format!(
                    "{name}: {} at {}: ChildByFieldID({id}) differs from the iterator form",
                    n.kind(),
                    n.start_byte()
                ));
                return;
            }
        }
        stack.extend_from_slice(&all);
    }
}

/// Go `TestChildrenWide`: a UTF-16 copy of a source parses to an ERROR node with thousands of
/// children and no alias sequence, the node a `child(i)` walk spends its time in.
fn children_wide_of(dirs: &'static [&'static str]) {
    common::run_on_big_stack(move || {
        let mut errs = Vec::new();
        for &dir in dirs {
            let mut wide = vec![0xFF, 0xFE];
            for golden in common::goldens(dir) {
                let src = std::fs::read(golden.with_extension("")).unwrap();
                for b in src {
                    wide.push(b);
                    wide.push(0);
                }
                if wide.len() > 60000 {
                    break;
                }
            }
            let mut p = Parser::new();
            p.set_language(language(dir)).unwrap();
            let root = p.parse(&wide).expect("parse").root_node();
            let mut widest = 0;
            root.preorder(|n| {
                widest = widest.max(n.child_count());
                true
            });
            if widest < 5000 {
                errs.push(format!(
                    "{dir}: widest node has {widest} children, want 5000 or more"
                ));
            }
            check_children(&mut errs, &format!("{dir} as UTF-16"), root);
        }
        assert!(errs.is_empty(), "{}", errs.join("\n"));
    });
}

#[test]
fn children_wide_json() {
    children_wide_of(&["json"]);
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn children_wide() {
    children_wide_of(&["rust", "tsx", "typescript"]);
}

/// The recursive `children` walk `preorder` must match: `n`, then (unless `skip(n)`) its
/// children's walks.
fn children_walk<'t>(n: Node<'t>, skip: fn(Node<'t>) -> bool, want: &mut Vec<Node<'t>>) {
    want.push(n);
    if skip(n) {
        return;
    }
    let mut kids = Vec::new();
    n.children(&mut kids);
    for c in kids {
        children_walk(c, skip, want);
    }
}

/// `preorder` must visit what a recursive `children` loop visits, in that order, and skip
/// the nodes below one `f` refuses (here: every third named node).
fn check_preorder(errs: &mut Vec<String>, name: &str, root: Node<'_>) {
    fn skip_third(n: Node<'_>) -> bool {
        n.is_named() && n.start_byte().is_multiple_of(3)
    }
    let mut kids = Vec::new();
    root.children(&mut kids);
    let mut want = Vec::new();
    for &c in &kids {
        children_walk(c, skip_third, &mut want);
    }
    let (mut i, mut bad) = (0, false);
    for &c in &kids {
        c.preorder(|n| {
            bad = bad || i >= want.len() || want[i] != n;
            i += 1;
            !skip_third(n)
        });
    }
    if bad || i != want.len() {
        errs.push(format!(
            "{name}: Preorder visits {i} nodes, the Children walk {}, or the order differs",
            want.len()
        ));
    }
    i = 0;
    want.clear();
    children_walk(root, |_| false, &mut want);
    root.preorder(|n| {
        bad = bad || i >= want.len() || want[i] != n;
        i += 1;
        true
    });
    if bad || i != want.len() {
        errs.push(format!(
            "{name}: full Preorder visits {i} nodes, the Children walk {}, or the order differs",
            want.len()
        ));
    }
}

/// `preorder_mask` must call `f` for the nodes of its kinds that `preorder` visits when `f`
/// refuses some of them (here: every third), and for no other node.
fn check_preorder_mask(
    errs: &mut Vec<String>,
    name: &str,
    root: Node<'_>,
    kinds: &HashMap<&'static str, u32>,
) {
    fn skip(n: Node<'_>) -> bool {
        n.start_byte().is_multiple_of(3)
    }
    let mut walk = |mask: u32| {
        let (mut want, mut got) = (Vec::new(), Vec::new());
        root.preorder(|n| {
            if kinds.get(n.kind()).copied().unwrap_or(0) & mask == 0 {
                return true;
            }
            want.push(n);
            !skip(n)
        });
        root.preorder_mask(mask, |n| {
            got.push(n);
            !skip(n)
        });
        if got != want {
            errs.push(format!(
                "{name}: PreorderMask({mask:#x}) calls fn on {} nodes, Preorder visits {} of those kinds, or they differ",
                got.len(),
                want.len()
            ));
        }
    };
    let mut all = 0u32;
    for &bit in kinds.values() {
        walk(bit);
        all |= bit;
    }
    walk(all);
    walk(all & 0x00ff00ff);
}

/// One node of `check_cursor`'s walk: records it and compares the cursor's field name with
/// the parent's `field_name_for_child`.
fn cursor_visit<'t>(
    errs: &mut Vec<String>,
    name: &str,
    c: &TreeCursor<'t>,
    got: &mut Vec<Node<'t>>,
    parents: &[Node<'t>],
    index: &[u32],
) {
    let n = c.node();
    got.push(n);
    if let (Some(&parent), Some(&i)) = (parents.last(), index.last())
        && (i < 256 || i.is_multiple_of(37))
    {
        let (f, g) = (parent.field_name_for_child(i), c.field_name());
        if f != g {
            errs.push(format!(
                "{name}: {} at {}: FieldName {:?}, FieldNameForChild {:?}",
                n.kind(),
                n.start_byte(),
                g.unwrap_or(""),
                f.unwrap_or("")
            ));
        }
    }
}

/// A cursor must visit what `preorder` visits, in that order, from the root and from inner
/// nodes (one cursor, reused through `reset`), report the node's parent on `goto_parent`,
/// and name each field as the parent's `field_name_for_child` does.
fn check_cursor(errs: &mut Vec<String>, name: &str, root: Node<'_>) {
    let mut c = root.walk();
    let mut all = Vec::new();
    root.preorder(|n| {
        all.push(n);
        true
    });
    let mut starts = vec![root];
    let mut i = all.len() / 7;
    while i < all.len() && starts.len() < 9 {
        starts.push(all[i]);
        i += all.len() / 7 + 1;
    }
    if let Some(&n) = all.iter().find(|n| n.is_extra()) {
        starts.push(n);
    }
    for start in starts {
        let mut want = Vec::new();
        start.preorder(|n| {
            want.push(n);
            true
        });
        c.reset(start);
        let mut got = Vec::new();
        let (mut parents, mut index): (Vec<Node<'_>>, Vec<u32>) = (Vec::new(), Vec::new());
        cursor_visit(errs, name, &c, &mut got, &parents, &index);
        let mut done = false;
        while !done {
            if c.goto_first_child() {
                parents.push(*got.last().unwrap());
                index.push(0);
                cursor_visit(errs, name, &c, &mut got, &parents, &index);
                continue;
            }
            loop {
                if index.is_empty() {
                    done = true;
                    break;
                }
                if c.goto_next_sibling() {
                    *index.last_mut().unwrap() += 1;
                    cursor_visit(errs, name, &c, &mut got, &parents, &index);
                    break;
                }
                if !c.goto_parent() || c.node() != *parents.last().unwrap() {
                    errs.push(format!(
                        "{name}: GotoParent from {} does not reach the parent",
                        start.start_byte()
                    ));
                    return;
                }
                parents.pop();
                index.pop();
            }
        }
        if got.len() != want.len() {
            errs.push(format!(
                "{name}: cursor from {} at {} visits {} nodes, Preorder {}",
                start.kind(),
                start.start_byte(),
                got.len(),
                want.len()
            ));
            return;
        }
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            if g != w {
                errs.push(format!(
                    "{name}: cursor from {} at {} differs from Preorder at node {i}",
                    start.kind(),
                    start.start_byte()
                ));
                return;
            }
        }
    }
}
