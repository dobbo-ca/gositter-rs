use gengrammar::cond::{CmpOp, Cond, Operand};
use gengrammar::ctok::Tok;
use gengrammar::parse::{Grammar, LexCase, Stmt, parse_grammar};

const MINI: &str = include_str!("../../testdata/gengrammar/mini.c");

fn parse(src: &str) -> Grammar {
    parse_grammar(src).unwrap_or_else(|e| panic!("{e}\n{src}"))
}

#[test]
fn mini_tables_and_actions() {
    let g = parse(MINI);
    assert_eq!(g.tables["ts_symbol_names"].strs, ["end", "\"a?", ""]); // "\0" is the C string ""
    assert_eq!(
        g.tables["ts_small_parse_table"].vals,
        [1, 3, 1, 2, 0, 0, 0, 2]
    );
    assert_eq!(g.tables["ts_small_parse_table_map"].vals, [0, 7]);
    assert_eq!(g.tables["ts_lex_modes"].vals, [0, 0, 5, 2, 0, 0, 65535, 0]);
    assert_eq!(
        g.actions,
        [
            0, 0, 0, 0, 0, 2, 1, 0, 0, 0, 1, 2, 2, -1, 7, 0, 0, 9, 2, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1,
            0
        ]
    );
    assert_eq!(g.tables["set_1"].vals, [97, 122, 0xaa, 0xaa]);
    assert_eq!(g.defines["LANGUAGE_VERSION"], 14);
    assert_eq!(g.defines["LARGE_STATE_COUNT"], 2);
    assert_eq!(g.name, "");
}

#[test]
fn mini_constants_order_and_widths() {
    let g = parse(MINI);
    assert_eq!(g.consts["sym_a"], 1);
    assert_eq!(g.consts["sym_b"], 2);
    assert_eq!(g.consts["ts_builtin_sym_end"], 0);
    assert_eq!(g.consts["ts_builtin_sym_error"], 0xFFFF);
    assert_eq!(g.consts["ts_builtin_sym_error_repeat"], 0xFFFE);
    assert_eq!(g.enums.len(), 1);
    assert_eq!(g.enums[0].name, "ts_symbol_identifiers");
    assert_eq!(g.enums[0].names, ["sym_a", "sym_b"]);
    assert_eq!(g.enums[0].vals, [1, 2]);
    assert_eq!(
        g.order,
        [
            "ts_symbol_names",
            "set_1",
            "ts_small_parse_table",
            "ts_small_parse_table_map",
            "ts_lex_modes",
            "ts_parse_actions"
        ]
    );
    let modes = &g.tables["ts_lex_modes"];
    assert_eq!((modes.width(), modes.len()), (2, 4));
    assert_eq!(modes.fields, Some(&["lex_state", "external_lex_state"][..]));
    assert_eq!(modes.dims, [4]);
    let small = &g.tables["ts_small_parse_table"];
    assert_eq!((small.width(), small.len(), small.fields), (1, 8, None));
    assert_eq!(small.dims, [0]);
    let names = &g.tables["ts_symbol_names"];
    assert_eq!((names.ctype.as_str(), names.len()), ("char*", 3));
    let set = &g.tables["set_1"];
    assert_eq!((set.width(), set.len()), (2, 2));
}

fn cmp(op: CmpOp, a: Operand, b: Operand) -> Cond {
    Cond::Cmp(op, a, b)
}

fn bx(c: Cond) -> Box<Cond> {
    Box::new(c)
}

#[test]
fn mini_lex_function_is_an_ast() {
    use Operand::{Lit, Lookahead};
    let g = parse(MINI);
    assert_eq!(g.lex_fns.len(), 1);
    let f = &g.lex_fns[0];
    assert_eq!(f.cname, "ts_lex");
    let blanks = Cond::Or(
        bx(Cond::And(
            bx(cmp(CmpOp::Le, Lit(9), Lookahead)),
            bx(cmp(CmpOp::Le, Lookahead, Lit(13))),
        )),
        bx(cmp(CmpOp::Eq, Lookahead, Lit(32))),
    );
    let ident = Cond::And(
        bx(cmp(CmpOp::Ne, Lookahead, Lit(0))),
        bx(Cond::Or(
            bx(cmp(CmpOp::Lt, Lookahead, Lit(40))),
            bx(cmp(CmpOp::Lt, Lit(41), Lookahead)),
        )),
    );
    let want = vec![
        LexCase {
            state: 0,
            stmts: vec![
                Stmt::If {
                    cond: Cond::Eof,
                    skip: false,
                    target: 1,
                },
                Stmt::Map(vec![(34, 2), (92, 3), (0xa0, 4)]),
                Stmt::If {
                    cond: blanks,
                    skip: true,
                    target: 0,
                },
                Stmt::If {
                    cond: Cond::And(
                        bx(Cond::Not(bx(Cond::Eof))),
                        bx(Cond::SetContains {
                            set: "set_1".into(),
                            n: 2,
                        }),
                    ),
                    skip: false,
                    target: 5,
                },
                Stmt::If {
                    cond: ident,
                    skip: false,
                    target: 6,
                },
                Stmt::End,
            ],
        },
        LexCase {
            state: 1,
            stmts: vec![Stmt::Accept("ts_builtin_sym_end".into()), Stmt::End],
        },
    ];
    assert_eq!(f.cases, want);
}

#[test]
fn enum_counter_starts_at_zero_in_each_enum() {
    let g = parse("enum a { x, y = 5, z };\nenum b { p, q, r = 2, s };\n");
    assert_eq!((g.consts["x"], g.consts["y"], g.consts["z"]), (0, 5, 6));
    assert_eq!(
        (g.consts["p"], g.consts["q"], g.consts["r"], g.consts["s"]),
        (0, 1, 2, 3)
    );
    assert_eq!(g.enums.len(), 2);
    assert_eq!(g.enums[1].vals, [0, 1, 2, 3]);
    assert_eq!(g.enums[1].names, ["p", "q", "r", "s"]);
}

#[test]
fn enum_values_are_expressions() {
    let g = parse(
        "#define N 7\nenum e { a = N, b = 'x', c = -3, d = (uint16_t)(-1), e1 = (TSStateId)65537, f = (4), g = true, h = ts_builtin_sym_error };",
    );
    let v = |n: &str| g.consts[n];
    assert_eq!(
        (
            v("a"),
            v("b"),
            v("c"),
            v("d"),
            v("e1"),
            v("f"),
            v("g"),
            v("h")
        ),
        (7, 120, -3, 65535, 1, 4, 1, 0xFFFF)
    );
}

#[test]
fn array_designators_move_the_cursor() {
    let g = parse(
        "static const uint16_t t[4] = { [2] = 1, 9 };\nstatic const uint16_t u[4] = { [2] = 1, [0] = 7, 8 };",
    );
    assert_eq!(g.tables["t"].vals, [0, 0, 1, 9]);
    assert_eq!(g.tables["u"].vals, [7, 8, 1, 0]);
}

#[test]
fn open_dimension_grows_with_the_highest_index() {
    let g = parse(
        "static const uint16_t t[][2] = { {1, 2}, [3] = {7, 8}, };\nstatic const uint16_t e[] = {};",
    );
    let t = &g.tables["t"];
    assert_eq!(t.vals, [1, 2, 0, 0, 0, 0, 7, 8]);
    assert_eq!((t.width(), t.len()), (2, 4));
    assert_eq!(g.tables["e"].vals, [] as [i64; 0]);
    assert_eq!(g.tables["e"].len(), 0);
}

#[test]
fn nested_dimensions_are_row_major() {
    let g = parse(
        "static const uint16_t t[2][3] = { {1, 2, 3}, {4, 5, 6} };\nstatic const bool b[2][2] = { [1] = {[1] = true} };",
    );
    let t = &g.tables["t"];
    assert_eq!(t.vals, [1, 2, 3, 4, 5, 6]);
    assert_eq!((t.width(), t.len()), (3, 2));
    assert_eq!(g.tables["b"].vals, [0, 0, 0, 1]);
}

#[test]
fn struct_designators_move_the_field_cursor() {
    let g = parse(
        "static const TSLexMode m[3] = { [1] = {.lex_state = 7, 8}, [2] = {.external_lex_state = 3}, };\n\
         static const TSFieldMapEntry f[] = { {.field_id = 1, .child_index = 2, .inherited = true}, {4, 5} };",
    );
    assert_eq!(g.tables["m"].vals, [0, 0, 7, 8, 0, 3]);
    assert_eq!(g.tables["f"].vals, [1, 2, 1, 4, 5, 0]);
    assert_eq!(g.tables["f"].len(), 2);
    assert_eq!(
        g.tables["f"].fields,
        Some(&["field_id", "child_index", "inherited"][..])
    );
}

#[test]
fn every_struct_layout_is_known() {
    let src = "static const TSSymbolMetadata a[1] = { {.visible = true, .named = true, .supertype = true} };\n\
         static const TSFieldMapSlice b[1] = { {.index = 1, .length = 2} };\n\
         static const TSMapSlice c[1] = { {.index = 1, .length = 2} };\n\
         static const TSLexerMode d[1] = { {.lex_state = 1, .external_lex_state = 2, .reserved_word_set_id = 3} };\n\
         static const TSCharacterRange e[1] = { {.start = 1, .end = 2} };\n\
         static const uint32_t f[1] = { 1 };\n\
         static const TSSymbol h[1] = { 1 };\n\
         static const TSStateId i[1] = { 1 };\n";
    let g = parse(src);
    assert_eq!(g.tables["a"].vals, [1, 1, 1]);
    assert_eq!(g.tables["d"].vals, [1, 2, 3]);
    assert_eq!(
        g.tables["d"].fields,
        Some(&["lex_state", "external_lex_state", "reserved_word_set_id"][..])
    );
    assert_eq!(g.tables["e"].vals, [1, 2]);
    assert_eq!(g.order, ["a", "b", "c", "d", "e", "f", "h", "i"]);
}

#[test]
fn strings_with_designators_and_null() {
    let g =
        parse("static const char * const n[4] = { [1] = \"a\\x00\\x62\", NULL, [3] = \"c\", };");
    assert_eq!(g.tables["n"].strs, ["", "a", "", "c"]);
    let g = parse("static const char * const n[] = { \"x\", \"y\" };");
    assert_eq!(g.tables["n"].strs, ["x", "y"]);
}

#[test]
fn small_state_and_identity_macros() {
    let g = parse(
        "#define LARGE_STATE_COUNT 10\nstatic const uint16_t t[] = { [SMALL_STATE(12)] = STATE(5), ACTIONS(6) };",
    );
    assert_eq!(g.tables["t"].vals, [0, 0, 5, 6]);
}

#[test]
fn parse_actions_of_every_kind() {
    let g = parse(
        "enum e { sym_a = 3 };\nstatic const TSParseActionEntry ts_parse_actions[] = {\n\
         [0] = {.entry = {.count = 0, .reusable = false}},\n\
         SHIFT(7), SHIFT(STATE(8)), SHIFT_REPEAT(9), SHIFT_EXTRA(), ACCEPT_INPUT(), RECOVER(),\n\
         REDUCE(sym_a, 255, 32767, 65535), REDUCE(0, 0, -32768, 0),\n\
         };",
    );
    assert_eq!(
        g.actions,
        [
            0, 0, 0, 0, 0, //
            0, 0, 7, 0, 0, //
            0, 0, 8, 0, 0, //
            0, 0, 9, 2, 0, //
            0, 0, 0, 1, 0, //
            2, 0, 0, 0, 0, //
            3, 0, 0, 0, 0, //
            1, 255, 3, 32767, 65535, //
            1, 0, 0, -32768, 0
        ]
    );
}

#[test]
fn lex_function_statements_and_empty_case() {
    // An empty case is accepted, as in Go: it has no statements and so no END_STATE to demand.
    let g = parse(
        "enum e { sym_a = 1 };\nstatic bool f(TSLexer *lexer, TSStateId state) {\nSTART_LEXER();\neof = lexer->eof(lexer);\nswitch (state) {\n\
         case 0:\ncase 1:\nif (eof) SKIP(1);\nADVANCE_MAP();\nEND_STATE();\ndefault:\nreturn false;\n}\n}\n",
    );
    assert_eq!(
        g.lex_fns[0].cases[0],
        LexCase {
            state: 0,
            stmts: vec![]
        }
    );
    assert_eq!(
        g.lex_fns[0].cases[1].stmts,
        [
            Stmt::If {
                cond: Cond::Eof,
                skip: true,
                target: 1
            },
            Stmt::Map(vec![]),
            Stmt::End
        ]
    );
}

fn lex_cond(cond: &str) -> Cond {
    let src = format!(
        "static bool f(TSLexer *lexer, TSStateId state) {{\nSTART_LEXER();\neof = lexer->eof(lexer);\nswitch (state) {{\ncase 0:\nif ({cond}) ADVANCE(1);\nEND_STATE();\ndefault:\nreturn false;\n}}\n}}\n"
    );
    match &parse(&src).lex_fns[0].cases[0].stmts[0] {
        Stmt::If { cond, .. } => cond.clone(),
        s => panic!("{s:?}"),
    }
}

#[test]
fn conditions_follow_go_precedence() {
    use CmpOp::*;
    use Operand::{Lit, Lookahead};
    // || binds looser than &&, which binds looser than comparisons; ! binds tightest.
    assert_eq!(
        lex_cond("lookahead == 'a' || lookahead == 0x62 && !eof"),
        Cond::Or(
            bx(cmp(Eq, Lookahead, Lit(97))),
            bx(Cond::And(
                bx(cmp(Eq, Lookahead, Lit(0x62))),
                bx(Cond::Not(bx(Cond::Eof)))
            ))
        )
    );
    assert_eq!(
        lex_cond("(lookahead == 1 || lookahead == 2) && lookahead != 3"),
        Cond::And(
            bx(Cond::Or(
                bx(cmp(Eq, Lookahead, Lit(1))),
                bx(cmp(Eq, Lookahead, Lit(2)))
            )),
            bx(cmp(Ne, Lookahead, Lit(3)))
        )
    );
    assert_eq!(lex_cond("((eof))"), Cond::Eof);
    assert_eq!(
        lex_cond("!(lookahead >= 5)"),
        Cond::Not(bx(cmp(Ge, Lookahead, Lit(5))))
    );
    assert_eq!(lex_cond("(lookahead) > (7)"), cmp(Gt, Lookahead, Lit(7)));
    assert_eq!(lex_cond("lookahead <= 'a'"), cmp(Le, Lookahead, Lit(97)));
    assert_eq!(lex_cond("'\\n' < lookahead"), cmp(Lt, Lit(10), Lookahead));
    // A char literal above 0x7F is its value (Go writes it as a hex literal).
    assert_eq!(
        lex_cond("lookahead == '\\xe9'"),
        cmp(Eq, Lookahead, Lit(0xe9))
    );
    assert_eq!(
        lex_cond("lookahead == 65536"),
        cmp(Eq, Lookahead, Lit(65536))
    );
    assert_eq!(
        lex_cond("!eof && lookahead == 0"),
        Cond::And(bx(Cond::Not(bx(Cond::Eof))), bx(cmp(Eq, Lookahead, Lit(0))))
    );
}

#[test]
fn skip_and_advance_and_set_contains() {
    let g = parse(
        "static TSCharacterRange set_1[] = { {'a', 'z'}, {0xaa, 0xaa}, };\n\
         static bool f(TSLexer *lexer, TSStateId state) {\nSTART_LEXER();\neof = lexer->eof(lexer);\nswitch (state) {\ncase 4:\n\
         if (!set_contains(set_1, 2, lookahead)) SKIP(4);\nEND_STATE();\ndefault:\nreturn false;\n}\n}\n",
    );
    assert_eq!(
        g.lex_fns[0].cases[0],
        LexCase {
            state: 4,
            stmts: vec![
                Stmt::If {
                    cond: Cond::Not(bx(Cond::SetContains {
                        set: "set_1".into(),
                        n: 2
                    })),
                    skip: true,
                    target: 4
                },
                Stmt::End
            ]
        }
    );
}

#[test]
fn tslanguage_initialiser_is_flattened() {
    let g = parse(
        "extern \"C\" {\nvoid *tree_sitter_x_external_scanner_create(void);\nunsigned tree_sitter_x_external_scanner_serialize(void *, char *);\nbool y(void);\n}\n\
         TS_PUBLIC const TSLanguage *tree_sitter_x(void) {\n  static const TSLanguage language = {\n    .abi_version = LANGUAGE_VERSION,\n    .sub = { 1, f(2, 3), },\n    .arr = ts_arr[1],\n    .ext = {\n      .states = &ts_ext[0][0],\n    },\n  };\n  return &language;\n}\n",
    );
    assert_eq!(g.name, "x");
    let keys: Vec<&str> = g.lang.keys().map(String::as_str).collect();
    assert_eq!(keys, ["abi_version", "arr", "ext.states", "sub.0", "sub.1"]);
    let kinds = |k: &str| -> Vec<Tok> { g.lang[k].iter().map(|t| t.tok.clone()).collect() };
    assert_eq!(
        kinds("abi_version"),
        [Tok::Ident("LANGUAGE_VERSION".into())]
    );
    assert_eq!(kinds("sub.0"), [Tok::Num(1)]);
    assert_eq!(
        kinds("sub.1"),
        [
            Tok::Ident("f".into()),
            Tok::Punct("("),
            Tok::Num(2),
            Tok::Punct(","),
            Tok::Num(3),
            Tok::Punct(")")
        ]
    );
    assert_eq!(kinds("arr").len(), 4);
}

#[test]
fn scanner_prototypes_are_skipped() {
    let g =
        parse("void *a(void);\nbool b(void *p, const char *q);\nunsigned c(void);\nenum e { z };");
    assert_eq!(g.consts["z"], 0);
}

/// Go's `int` arithmetic wraps (parse.go:26-31, 301, 340-355, 455-458), so Rust must not panic.
#[test]
fn table_arithmetic_wraps_like_go() {
    let t = |src: &str| parse(src).tables["x"].vals.clone();
    // width: 1 * 2^32 * 2^32 wraps to 0, so make(0) and `{}` returns.
    assert!(t("static uint16_t x[1][0x100000000][0x100000000] = {};").is_empty());
    // dims[0] * width = 2^62 * 4 wraps to 0.
    assert!(t("static uint16_t x[0x4000000000000000][4] = {};").is_empty());
    // stride = 3 * 0x5555555555555556 = 2^64 + 2 wraps to 2, so vals = make(1*2).
    assert_eq!(
        t("static uint16_t x[1][3][0x5555555555555556] = { {} };"),
        [0, 0]
    );
    // (idx+1)*4 = 4 and idx*4 = 0 after wrapping: vals grows to 4 and is written at 0..4.
    assert_eq!(
        t("static uint16_t x[][4] = { [0x4000000000000000] = {1,2,3,4} };"),
        [1, 2, 3, 4]
    );
    // (idx+1)*2 = 2^64 wraps to 0 (no append); init(off=-2) returns on `{}`.
    assert!(t("static uint16_t x[][2] = { [0x7FFFFFFFFFFFFFFF] = {} };").is_empty());
    // idx++ wraps to MinInt64 and the `idx < 0` check fires.
    let e =
        parse_grammar("static uint16_t x[][2] = { [0x7FFFFFFFFFFFFFFF] = {}, {} };").unwrap_err();
    assert!(
        e.0.ends_with("x: index -9223372036854775808 out of range"),
        "{}",
        e.0
    );
}

/// idx*5 = 2^64+4 wraps to 4; (idx+1)*5 = 9, so actions grows to 9 and 4..9 is written.
#[test]
fn parse_action_index_wraps_like_go() {
    let g = parse(
        "enum e { sym_a = 1 };\nstatic const TSParseActionEntry ts_parse_actions[] = { [0x3333333333333334] = SHIFT(1) };",
    );
    assert_eq!(g.actions.len(), 9);
    assert_eq!(g.actions[..4], [0, 0, 0, 0]);
}
