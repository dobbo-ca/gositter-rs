//! gositter `parser_test.go`, plus the json twins and new tests of plan Task 11 Step 1.
//!
//! Not ported (N/A-Go: `testing.AllocsPerRun`, finalizers, weak pointers, the Go GC):
//! `TestParseAllocs`, `TestHugeSlabsFreed`, `TestKeptLinkSlabOutlivesGC`,
//! `TestHugeLinkSlabFreedByLinklessParses`. `BenchmarkParseJSON` is the `gsdump bench`
//! input `json_2000` (plan Task 14). The `AllocsPerRun` halves of `TestErrorRecoveryMemory`,
//! `TestChildrenSlabDropped` and `TestStackSlabsDropped` are dropped for the same reason.

mod common;

use std::sync::{Arc, Mutex};

use gositter::{
    KEPT_HEAP_CHUNKS, Language, LanguageTables, LogType, Parser, Point, SHRINK_CHILDREN,
    SHRINK_NODES, Tree,
};

fn grammar(dir: &str) -> &'static Language {
    common::language(dir).unwrap_or_else(|| panic!("{dir} grammar"))
}

fn parser(lang: &'static Language) -> Parser {
    let mut p = Parser::new();
    p.set_language(lang).unwrap();
    p
}

fn sexp_of(p: &mut Parser, src: &[u8]) -> String {
    p.parse(src).unwrap().root_node().to_sexp()
}

fn sexp(tree: &Tree, _src: &[u8]) -> Vec<u8> {
    tree.root_node().to_sexp().into_bytes()
}

fn dump(tree: &Tree, src: &[u8]) -> Vec<u8> {
    let root = tree.root_node();
    let fields = root
        .language()
        .tables
        .field_names
        .get(1..)
        .unwrap_or_default();
    let mut out = Vec::new();
    gositter_treedump::dump(&mut out, root, fields, src);
    out
}

/// The messages a test logger collects: (type, bytes) in order.
type Messages = Arc<Mutex<Vec<(LogType, Vec<u8>)>>>;

/// A logger that collects every message.
fn collect(p: &mut Parser) -> Messages {
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    p.set_logger(Some(Box::new(move |t: LogType, m: &[u8]| {
        sink.lock().unwrap().push((t, m.to_vec()))
    })));
    log
}

// The trees for these inputs were compared node for node with the C runtime
// (oracle cmp); the strings pin them.
#[test]
fn parse_json_string() {
    let cases: [(&[u8], &str); 7] = [
        (
            br#"{"a": [1, true, null]}"#,
            "(document (object (pair key: (string (string_content)) value: (array (number) (true) (null)))))",
        ),
        (b"\xef\xbb\xbf[]\n", "(document (array))"),
        (b"", "(document)"),
        (b"[1,", "(document (ERROR (number)))"),
        (
            br#"{"a": }"#,
            "(document (object (pair key: (string (string_content)) value: (MISSING number))))",
        ),
        (b"[1 2]", "(document (array (ERROR (number)) (number)))"),
        (b"[\x00]", r"(document (array (ERROR (UNEXPECTED '\0'))))"),
    ];
    for (src, want) in cases {
        let mut p = parser(grammar("json"));
        assert_eq!(sexp_of(&mut p, src), want, "{}", src.escape_ascii());
    }
}

#[test]
fn node_api() {
    let src = br#"{"k": [10, "s"]}"#;
    let mut p = parser(grammar("json"));
    let root = p.parse(src).unwrap().root_node();
    assert!(
        root.kind() == "document"
            && root.is_named()
            && !root.has_error()
            && root.start_byte() == 0
            && root.end_byte() == src.len() as u32,
        "root: {} {}-{}",
        root.kind(),
        root.start_byte(),
        root.end_byte()
    );
    assert!(root.parent().is_none(), "root has a parent");
    let obj = root.child(0).unwrap();
    let pair = obj.named_child(0).unwrap();
    assert!(
        obj.child_count() == 3 && obj.named_child_count() == 1 && pair.kind() == "pair",
        "object children: {} {} {}",
        obj.child_count(),
        obj.named_child_count(),
        pair.kind()
    );
    assert!(
        pair.parent().is_some_and(|n| n.equals(obj))
            && obj.parent().is_some_and(|n| n.equals(root)),
        "Parent"
    );

    let key = pair.child_by_field_name("key").unwrap();
    let value = pair.child_by_field_name("value").unwrap();
    assert!(
        key.utf8_text(src) == br#""k""# && value.kind() == "array",
        "fields: {} {}",
        key.utf8_text(src).escape_ascii(),
        value.kind()
    );
    let lang = root.language();
    let id = lang.field_id_for_name("value");
    assert!(
        id != 0
            && pair.child_by_field_id(id).is_some_and(|n| n.equals(value))
            && lang.field_name_for_id(id) == Some("value"),
        "ChildByFieldID"
    );
    assert!(
        pair.child_by_field_name("nope").is_none()
            && lang.field_id_for_name("") == 0
            && lang.field_id_for_name("valu") == 0,
        "unknown field"
    );
    assert!(
        pair.field_name_for_child(0) == Some("key")
            && pair.field_name_for_child(1).is_none()
            && pair.field_name_for_child(2) == Some("value"),
        "FieldNameForChild"
    );
    assert_eq!(value.start_position(), Point { row: 0, column: 6 });
    assert!(
        obj.child(99).is_none() && obj.named_child(1).is_none(),
        "out of range child must be null"
    );

    let mut c = pair.walk();
    let got = pair.children_by_field_name("value", &mut c);
    assert!(
        got.len() == 1 && got[0].equals(value),
        "ChildrenByFieldName = {} nodes",
        got.len()
    );
    assert!(
        pair.children_by_field_name("nope", &mut c).is_empty(),
        "ChildrenByFieldName on an unknown field"
    );

    // A cursor walk visits the same nodes as Child.
    c.reset(value);
    let mut kinds = Vec::new();
    let mut ok = c.goto_first_child();
    while ok {
        kinds.push(c.node().kind());
        ok = c.goto_next_sibling();
    }
    assert_eq!(kinds.join(" "), "[ number , string ]", "cursor walk");
    assert!(
        c.goto_parent() && c.node().equals(value) && !c.goto_parent(),
        "GotoParent"
    );
}

#[test]
fn parser_reuse_and_no_language() {
    assert!(
        Parser::new().parse(b"1").is_none(),
        "Parse without a language must return None"
    );
    let json = grammar("json");
    let mut p = Parser::new();
    let abi12: &'static Language = Box::leak(Box::new(Language::new(LanguageTables {
        abi_version: 12,
        ..json.tables
    })));
    let abi16: &'static Language = Box::leak(Box::new(Language::new(LanguageTables {
        abi_version: 16,
        ..json.tables
    })));
    assert!(p.set_language(abi12).is_err(), "ABI 12 accepted");
    assert!(p.set_language(abi16).is_err(), "ABI 16 accepted");
    p.set_language(json).unwrap();
    let inputs: [&[u8]; 5] = [b"[1, 2]", br#"{"a": "#, b"", b"[[[[]]]]", b"[1, 2]"];
    let mut first = String::new();
    for (i, src) in inputs.iter().enumerate() {
        let s = sexp_of(&mut p, src);
        if i == 0 {
            first = s.clone();
        }
        if i == inputs.len() - 1 {
            assert_eq!(s, first, "same input, different tree after reuse");
        }
    }
}

#[test]
fn logger_messages() {
    let mut p = parser(grammar("json"));
    let log = collect(&mut p);
    p.parse(b"[1]\t");
    let (mut parse, mut lex) = (Vec::new(), Vec::new());
    for (t, m) in log.lock().unwrap().iter() {
        let m = String::from_utf8_lossy(m).into_owned();
        if *t == LogType::Lex {
            lex.push(m);
        } else {
            parse.push(m);
        }
    }
    assert!(
        parse.first().map(String::as_str) == Some("new_parse")
            && parse.last().map(String::as_str) == Some("done"),
        "first/last: {:?} {:?}",
        parse.first(),
        parse.last()
    );
    let all = parse.join("\n");
    for want in [
        "process version:0, version_count:1, state:1, row:0, col:0",
        "lexed_lookahead sym:[, size:1",
        "shift state:",
        "reduce sym:array, child_count:3",
        "accept",
    ] {
        assert!(all.contains(want), "parse log lacks {want:?}");
    }
    let lex_all = lex.join("\n");
    for want in ["consume character:'['", "skip character:9"] {
        assert!(lex_all.contains(want), "lex log lacks {want:?}");
    }
    p.set_logger(None);
    p.parse(b"[1]");
}

// Inputs that never leave error recovery. Every array the recovery drops has
// to be reusable, also by an array of another size: these took 100 to 3000
// children slab entries per input byte when it was not, and at 100,000 bytes
// the 32-bit slab offset wrapped. limit is in slab entries per input byte.
fn error_recovery_case(name: &str, dir: &str, src: &[u8], limit: usize) {
    let mut p = parser(grammar(dir));
    let root = p.parse(src).unwrap().root_node();
    assert!(
        root.has_error() && root.end_byte() == src.len() as u32,
        "{name}: root: error={} end={}",
        root.has_error(),
        root.end_byte()
    );
    let (got, limit) = (p.children_slab_cap(), limit * src.len());
    assert!(
        got <= limit,
        "{name}: children slab: {got} entries, limit {limit}"
    );
}

#[test]
fn error_recovery_memory_json() {
    common::run_on_big_stack(|| {
        error_recovery_case(
            "json_open",
            "json",
            r#"{"a": "b"#.repeat(2500).as_bytes(),
            128,
        )
    });
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn error_recovery_memory() {
    common::run_on_big_stack(|| {
        let cases: [(&str, &str, String, usize); 21] = [
            ("kotlin_lt", "kotlin", "<".repeat(5000), 128),
            ("kotlin_rbracket", "kotlin", "]".repeat(5000), 128),
            // An ERROR node that grows by a few children per token.
            ("c_define", "c", "#define A( ".repeat(1000), 16),
            ("cpp_define", "cpp", "#define A( ".repeat(1000), 16),
            ("c_comment_end", "c", "*/\n".repeat(1000), 64),
            ("cpp_comment_end", "cpp", "*/\n".repeat(1000), 64),
            ("csharp_comment_end", "c-sharp", "*/\n".repeat(1000), 64),
            ("go_comment_end", "go", "*/\n".repeat(1000), 64),
            ("c_comment_start", "c", "/*\n".repeat(1000), 64),
            ("cpp_comment_start", "cpp", "/*\n".repeat(1000), 64),
            ("csharp_comment_start", "c-sharp", "/*\n".repeat(1000), 64),
            ("go_comment_start", "go", "/*\n".repeat(1000), 64),
            ("csharp_string", "c-sharp", r#""a "#.repeat(3000), 16),
            (
                "csharp_verbatim_string",
                "c-sharp",
                r#"@"a "#.repeat(3000),
                16,
            ),
            ("csharp_raw_string", "c-sharp", "\"\"\"\n".repeat(3000), 16),
            ("ruby_symbol", "ruby", r#":"a "#.repeat(3000), 16),
            ("verilog_module", "verilog", "module\n".repeat(3000), 16),
            ("rust_raw_string", "rust", r##"r#"a "##.repeat(3000), 16),
            // Reduces between the frees of error recovery.
            (
                "go_comment_end_string",
                "go",
                ("*/\n".repeat(3) + &r#""a "#.repeat(5)).repeat(1500),
                16,
            ),
            // 64 paths at the end of input: accept holds one array at a time.
            (
                "cpp_template",
                "cpp",
                "std::vector<std::map<int, x = 1;\n".repeat(1000),
                4,
            ),
            ("rust_match_arm", "rust", "x => {\n".repeat(3000), 128),
        ];
        for (name, dir, src, limit) in cases {
            error_recovery_case(name, dir, src.as_bytes(), limit);
        }
    });
}

// A reset hands the heap chunks of a large parse to the pool all parsers
// share. A parser that takes them over as they are must build the same tree.
#[test]
#[ignore = "gositter-rs-2f0.15"]
fn heap_chunks_reused() {
    common::run_on_big_stack(|| {
        let src = "func f() { g(1, 2, 3) }\n".repeat(5000);
        let other = "var x = []int{1, 2, 3}\n".repeat(5000);
        let go = grammar("go");
        let (mut a, mut b) = (parser(go), parser(go));
        let want = sexp_of(&mut a, src.as_bytes());
        let n = a.heap_chunks();
        assert!(n > 2 * KEPT_HEAP_CHUNKS, "input too small: {n} chunks");
        a.reset();
        assert_eq!(a.heap_chunks(), KEPT_HEAP_CHUNKS, "after reset");
        b.parse(other.as_bytes());
        b.reset();
        assert!(
            sexp_of(&mut b, src.as_bytes()) == want,
            "tree differs on chunks another parse has used"
        );
        assert!(
            sexp_of(&mut a, src.as_bytes()) == want,
            "tree differs after the chunks went through the pool"
        );
    });
}

// A reset keeps the children slab of a large parse for the next one, and
// drops it once a parse has used a small part of it.
#[test]
#[ignore = "gositter-rs-2f0.15"]
fn children_slab_dropped() {
    common::run_on_big_stack(|| {
        let large = "func f() { g(1, 2, 3) }\n".repeat(20000);
        let small = b"func f() { g(1, 2, 3) }\n";
        let go = grammar("go");
        let mut p = parser(go);
        p.parse(large.as_bytes());
        let n = p.children_slab_cap();
        assert!(n > SHRINK_CHILDREN, "input too small: slab of {n} entries");
        p.reset();
        p.reset();
        assert_eq!(p.children_slab_cap(), n, "after the large parse");
        let want = sexp_of(&mut p, small);
        p.reset();
        let got = p.children_slab_cap();
        assert!(
            got < SHRINK_CHILDREN,
            "after the small parse: slab of {got} entries, want it dropped"
        );
        assert_eq!(sexp_of(&mut p, small), want, "tree differs on the new slab");
        let want_large = sexp_of(&mut parser(go), large.as_bytes());
        assert!(
            sexp_of(&mut p, large.as_bytes()) == want_large,
            "large tree differs on the regrown slab"
        );
    });
}

/// Go `stallOf`: the smallest limit that lets src parse, by bisection.
fn stall_of(lang: &'static Language, src: &[u8]) -> u32 {
    let mut p = parser(lang);
    let (mut lo, mut hi) = (1u32, 1u32 << 20);
    assert!(p.parse_budget(src, hi).is_some(), "stops at {hi}");
    while lo < hi {
        let mid = (lo + hi) / 2;
        if p.parse_budget(src, mid).is_some() {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    assert!(lo >= 3, "input too simple: stall {lo}");
    lo
}

// An unclosed julia macro block retries the same byte without end: 14 s and
// 1.4 GB with no limit. The stop must leave nothing behind, in this parser
// or in the heap chunks it hands to the others.
#[test]
#[ignore = "gositter-rs-2f0.15"]
fn parse_budget_stops() {
    common::run_on_big_stack(|| {
        let stuck = b"@testset \"t0\" begin\n    @testset \"t1\" begin\n        l2";
        let src = "func f() { g(1, 2, 3) }\n".repeat(5000);
        let src = src.as_bytes();
        let go = grammar("go");
        let (mut a, mut b) = (Parser::new(), parser(go));
        let want = sexp_of(&mut b, src);
        // progress resets the count
        assert!(
            b.parse_budget(src, 1).is_some(),
            "clean input stopped at limit 1"
        );
        b.reset();

        a.set_language(grammar("julia")).unwrap();
        assert!(
            a.parse_budget(stuck, 65536).is_none(),
            "parsed, want a stop"
        );
        assert_eq!(a.heap_chunks(), KEPT_HEAP_CHUNKS, "after the stop");
        assert!(
            sexp_of(&mut b, src) == want,
            "tree differs on another parser after the stop"
        );
        a.set_language(go).unwrap();
        assert!(
            sexp_of(&mut a, src) == want,
            "tree differs on the stopped parser"
        );
    });
}

/// The json twin of `parse_budget_stops` (plan Task 11 Step 1).
#[test]
fn parse_budget_stops_json() {
    let json = grammar("json");
    let stuck = r#"[1, 2, {"a": [3, 4}"#.repeat(200);
    let other = "[".to_string()
        + &r#"{"a": [1, 2.5, true, null, "x\n"], "b": {"c": []}},"#.repeat(500)
        + "0]";
    let mut a = parser(json);
    assert!(
        a.parse_budget(stuck.as_bytes(), 1).is_none(),
        "parsed, want a stop"
    );
    let n = a.heap_chunks();
    assert!(
        n <= KEPT_HEAP_CHUNKS,
        "after the stop: {n} chunks, want at most {KEPT_HEAP_CHUNKS}"
    );
    let want = sexp_of(&mut parser(json), other.as_bytes());
    assert!(
        sexp_of(&mut a, other.as_bytes()) == want,
        "tree differs on the stopped parser"
    );
}

/// Go `TestParseBudgetBoundary`'s body: the limit is exact, and at the limit the tree is
/// the one `parse` builds, as `render` prints it.
fn budget_boundary(lang: &'static Language, src: &[u8], render: fn(&Tree, &[u8]) -> Vec<u8>) {
    let m = stall_of(lang, src);
    let mut p = parser(lang);
    let want = render(p.parse(src).unwrap(), src);
    assert!(
        p.parse_budget(src, m - 1).is_none(),
        "limit {}: parsed, want a stop",
        m - 1
    );
    let tree = p
        .parse_budget(src, m)
        .unwrap_or_else(|| panic!("limit {m}: stopped"));
    assert!(
        render(tree, src) == want,
        "limit {m}: tree differs from Parse"
    );
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn parse_budget_boundary() {
    common::run_on_big_stack(|| {
        let src = "func f() { g(1, 2, 3 }\n".repeat(100);
        budget_boundary(grammar("go"), src.as_bytes(), sexp);
    });
}

#[test]
fn parse_budget_boundary_json() {
    // treedump recurses once per level (Review Focus 2).
    common::run_on_big_stack(|| {
        let src = r#"[1, 2, {"a": [3, 4}"#.repeat(200);
        budget_boundary(grammar("json"), src.as_bytes(), dump);
    });
}

/// Go `TestParseBudgetCountStable`'s body: the count depends on the language and the
/// input only, not on what the parser did before.
fn budget_count_stable(name: &str, lang: &'static Language, src: &[u8]) {
    let large = src.repeat(2);
    let m = stall_of(lang, src);
    for prep in ["fresh", "reused", "stopped"] {
        for limit in [m - 1, m] {
            let mut p = parser(lang);
            match prep {
                "reused" => {
                    p.parse(&large);
                }
                "stopped" => {
                    p.parse_budget(&large, 1);
                }
                _ => {}
            }
            let got = p.parse_budget(src, limit).is_some();
            assert_eq!(
                got,
                limit == m,
                "{name}, {prep} parser, limit {limit} of {m}: parsed={got}"
            );
        }
    }
}

#[test]
#[ignore = "gositter-rs-2f0.15"]
fn parse_budget_count_stable() {
    common::run_on_big_stack(|| {
        // 17924: the count grows with the input, 8.5 per byte from here.
        budget_count_stable("kotlin_lt", grammar("kotlin"), "<".repeat(2500).as_bytes());
        budget_count_stable(
            "rust_match_arm",
            grammar("rust"),
            "x => {\n".repeat(3000).as_bytes(),
        );
    });
}

#[test]
fn parse_budget_count_stable_json() {
    let src = r#"[1, 2, {"a": [3, 4}"#.repeat(200);
    budget_count_stable("json", grammar("json"), src.as_bytes());
}

// A deep stack's slabs go to the free lists at reset, as children slabs, and
// a warm parser takes them back.
#[test]
#[ignore = "gositter-rs-2f0.15"]
fn stack_slabs_dropped() {
    common::run_on_big_stack(|| {
        let deep = "func f() {".to_string() + &"g(".repeat(10000);
        let small = b"func f() { g(1, 2, 3) }\n";
        let go = grammar("go");
        let want = sexp_of(&mut parser(go), deep.as_bytes());
        let mut p = parser(go);
        p.parse(deep.as_bytes());
        let n = p.stack_slab_cap();
        assert!(
            n > SHRINK_NODES,
            "input too shallow: stack slab of {n} entries"
        );
        p.parse(small);
        p.reset();
        let n = p.stack_slab_cap();
        assert!(
            n <= SHRINK_NODES,
            "after the small parse: stack slab of {n} entries, want it dropped"
        );
        assert!(
            sexp_of(&mut p, deep.as_bytes()) == want,
            "deep tree differs on the pooled slabs"
        );
    });
}

// After a huge input gives up its slabs, a small parse takes slabs of
// at most 4 times its need.
#[test]
#[ignore = "gositter-rs-2f0.15"]
fn small_parse_takes_small_slabs() {
    common::run_on_big_stack(|| {
        let huge = "func f() {".to_string()
            + &"g(".repeat(10000)
            + &"func f() { g(1, 2, 3) }\n".repeat(20000);
        let small = b"func f() { g(1, 2, 3) }\n";
        let mut p = parser(grammar("go"));
        p.parse(huge.as_bytes());
        assert!(
            p.children_slab_cap() > SHRINK_CHILDREN && p.stack_slab_cap() > SHRINK_NODES,
            "input too small: slabs of {} and {} entries",
            p.children_slab_cap(),
            p.stack_slab_cap()
        );
        p.parse(small);
        p.reset(); // the huge slabs are given up here
        p.parse(small);
        let n = p.children_slab_cap();
        assert!(
            n <= 4 * 4096,
            "children slab of {n} entries for a small parse"
        );
        let n = p.stack_slab_cap();
        assert!(n <= 4 * 64, "stack slab of {n} entries for a small parse");
    });
}

// Parsers on 8 threads pass slabs through the shared free lists. Each tree
// must be the one a lone parser builds.
#[test]
#[ignore = "gositter-rs-2f0.15"]
fn slabs_across_parsers() {
    common::run_on_big_stack(|| {
        let line = "func f() { g(1, 2, 3) }\n";
        let inputs: [String; 5] = [
            "func f() {".to_string() + &"g(".repeat(3000),
            line.repeat(8000),
            line.to_string(),
            "package p\nvar x = [...]int{1, 2, 3}\n".to_string(),
            "func f() {".to_string() + &"g(".repeat(9000) + &line.repeat(3000),
        ];
        let go = grammar("go");
        let mut lone = parser(go);
        let want: Vec<String> = inputs
            .iter()
            .map(|src| sexp_of(&mut lone, src.as_bytes()))
            .collect();
        std::thread::scope(|s| {
            for g in 0..8 {
                let (inputs, want) = (&inputs, &want);
                std::thread::Builder::new()
                    .stack_size(256 << 20)
                    .spawn_scoped(s, move || {
                        let mut p = parser(go);
                        for i in 0..3 * inputs.len() {
                            let k = (i + g) % inputs.len();
                            assert!(
                                sexp_of(&mut p, inputs[k].as_bytes()) == want[k],
                                "thread {g}: input {k} differs"
                            );
                            std::thread::yield_now();
                        }
                    })
                    .expect("spawn");
            }
        });
    });
}

/// Plan Task 11 Step 1: C cuts log messages at 1,023 bytes, which can split a character.
#[test]
fn log_lines_cut_at_1023() {
    let json = grammar("json");
    let long = "\u{e9}".repeat(1000);
    let mut names = json.tables.symbol_names.to_vec();
    let i = names
        .iter()
        .position(|n| *n == "[")
        .expect("json has a [ token");
    names[i] = Box::leak(long.clone().into_boxed_str());
    let lang: &'static Language = Box::leak(Box::new(Language::new(LanguageTables {
        symbol_names: Box::leak(names.into_boxed_slice()),
        ..json.tables
    })));
    let mut p = parser(lang);
    let log = collect(&mut p);
    p.parse(b"[1]");
    let full = format!("lexed_lookahead sym:{long}, size:1").into_bytes();
    let log = log.lock().unwrap();
    for (_, m) in log.iter() {
        assert!(m.len() <= 1023, "message of {} bytes", m.len());
    }
    assert!(
        log.iter()
            .any(|(t, m)| *t == LogType::Parse && m[..] == full[..1023]),
        "no lexed_lookahead message cut at 1,023 bytes"
    );
}

/// Review Focus 1: a kind named after a parse is not in that tree's mask bits; the next
/// parse has it.
#[test]
fn kind_mask_after_parse() {
    // A fresh copy: its kind bits are this test's alone.
    let lang: &'static Language = Box::leak(Box::new(Language::new(grammar("json").tables)));
    let mut p = parser(lang);
    let src = br#"{"a":1}"#;
    let tree = p.parse(src).unwrap();
    let mask = lang.kind_mask(&["pair"]);
    let mut n = 0;
    tree.root_node().preorder_mask(mask, |_| {
        n += 1;
        true
    });
    assert_eq!(n, 0, "the tree built before kind_mask");
    let tree = p.parse(src).unwrap();
    tree.root_node().preorder_mask(mask, |_| {
        n += 1;
        true
    });
    assert_eq!(n, 1, "the tree built after kind_mask");
}
