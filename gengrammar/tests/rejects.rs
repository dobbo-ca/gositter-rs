use gengrammar::parse::parse_grammar;

/// The source must be refused with a `parser.c:<line>: ` message ending in `msg`.
fn rej(src: &str, msg: &str) {
    match parse_grammar(src) {
        Ok(_) => panic!("accepted: {src}"),
        Err(e) => {
            assert!(e.0.starts_with("parser.c:"), "{}", e.0);
            assert!(
                e.0.ends_with(msg),
                "got {:?}, want ...{:?}\n{src}",
                e.0,
                msg
            );
        }
    }
}

/// A lex function with one case 0 whose body is `body`, as in Go's `TestRejects`.
fn lex_fn(body: &str) -> String {
    format!(
        "enum e {{ sym_a = 1 }};\nstatic TSCharacterRange set_1[] = {{ {{'a', 'z'}} }};\n\
         static bool ts_lex(TSLexer *lexer, TSStateId state) {{\nSTART_LEXER();\neof = lexer->eof(lexer);\nswitch (state) {{\ncase 0:\n\
         {body}\ndefault:\nreturn false;\n}}\n}}\n"
    )
}

fn actions(body: &str) -> String {
    format!(
        "enum e {{ sym_a = 1 }};\nstatic const TSParseActionEntry ts_parse_actions[] = {{ {body} }};"
    )
}

const LANG: &str =
    "TS_PUBLIC const TSLanguage *tree_sitter_x(void) {\n  static const TSLanguage language = ";

// The 8 sources of Go's TestRejects.

#[test]
fn go_map_key_above_16_bits() {
    rej(
        &lex_fn("ADVANCE_MAP(0x10000, 1,);\nEND_STATE();"),
        "ADVANCE_MAP key 65536 out of range",
    );
}

#[test]
fn go_duplicate_map_key() {
    rej(
        &lex_fn("ADVANCE_MAP('a', 1, 0x61, 2,);\nEND_STATE();"),
        "duplicate ADVANCE_MAP key 97",
    );
}

#[test]
fn go_wrong_set_length() {
    rej(
        &lex_fn("if (set_contains(set_1, 2, lookahead)) ADVANCE(1);\nEND_STATE();"),
        "set_contains(set_1, 2): table has 1 ranges",
    );
}

#[test]
fn go_case_falls_through() {
    rej(
        &lex_fn("ACCEPT_TOKEN(sym_a);\ncase 1:\nEND_STATE();"),
        "case 0: END_STATE is not the last statement",
    );
}

#[test]
fn go_unknown_symbol() {
    rej(
        &lex_fn("ACCEPT_TOKEN(sym_zzz);\nEND_STATE();"),
        "unknown symbol sym_zzz",
    );
}

#[test]
fn go_unknown_operator() {
    rej(
        &lex_fn("if (lookahead & 1) ADVANCE(1);\nEND_STATE();"),
        "unexpected \"&\" in lex condition",
    );
}

#[test]
fn go_child_count_above_255() {
    rej(
        "enum e { sym_a = 1 };\nstatic const TSParseActionEntry ts_parse_actions[] = { REDUCE(sym_a, 256, 0, 0), };",
        "child_count 256 out of range",
    );
}

#[test]
fn go_index_past_dimension() {
    rej(
        "static const uint16_t t[2] = { [2] = 1, };",
        "t: index 2 out of range",
    );
}

// One test per reject of parse.go.

#[test]
fn unexpected_at_top_level() {
    rej("int x;", "unexpected \"int\" at top level");
    rej("\n\n\"x\"", "unexpected \"x\" at top level");
    rej(
        "static const uint16_t t[1] = {1}; 5",
        "unexpected \"5\" at top level",
    );
}

#[test]
fn expected_found() {
    rej("enum e x", "expected \"{\", found \"x\"");
    rej("enum e { a", "expected \"}\", found \"\"");
    rej("extern \"C\" x", "expected \"{\", found \"x\"");
    rej("static const char x", "expected \"*\", found \"x\"");
}

#[test]
fn expected_identifier() {
    rej("enum { a };", "expected identifier, found \"{\"");
}

#[test]
fn unknown_identifier() {
    rej(
        "static const uint16_t t[1] = { zzz };",
        "unknown identifier \"zzz\"",
    );
}

#[test]
fn unexpected_in_expression() {
    rej(
        "static const uint16_t t[1] = { \"s\" };",
        "unexpected \"s\" in expression",
    );
    rej(
        "static const uint16_t t[1] = { };\nenum e { a = ; };",
        "unexpected \";\" in expression",
    );
    rej(
        "static const uint16_t t[1] = {",
        "unexpected \"\" in expression",
    );
}

#[test]
fn missing_large_state_count() {
    rej(
        "static const uint16_t t[1] = { SMALL_STATE(1) };",
        "missing #define LARGE_STATE_COUNT",
    );
}

#[test]
fn duplicate_constant() {
    rej("enum e { a, a };", "duplicate constant a");
    rej("enum e { a };\nenum f { b, a };", "duplicate constant a");
    rej(
        "enum e { ts_builtin_sym_end };",
        "duplicate constant ts_builtin_sym_end",
    );
}

#[test]
fn unsupported_dimensions() {
    rej("static const uint16_t t = 1;", "t: unsupported dimensions");
    rej(
        "static const uint16_t t[2][] = {{1}};",
        "t: unsupported dimensions",
    );
}

#[test]
fn duplicate_table() {
    rej(
        "static const uint16_t t[1] = {1};\nstatic const uint16_t t[1] = {1};",
        "duplicate table t",
    );
}

#[test]
fn unknown_element_type() {
    rej(
        "static const foo_t t[1] = {1};",
        "t: unknown element type foo_t",
    );
}

#[test]
fn struct_has_no_field() {
    rej(
        "static const TSLexMode t[1] = { {.bogus = 1} };",
        "TSLexMode has no field bogus",
    );
}

#[test]
fn too_many_initialisers() {
    rej(
        "static const TSLexMode t[1] = { {1, 2, 3} };",
        "too many initialisers for TSLexMode",
    );
    rej(
        "static const TSLexMode t[1] = { {.external_lex_state = 1, 2} };",
        "too many initialisers for TSLexMode",
    );
}

#[test]
fn index_out_of_range() {
    rej(
        "static const uint16_t t[2][2] = { {1}, [2] = {1} };",
        "t: index 2 out of range",
    );
    rej(
        "static const uint16_t t[2][2] = { {1, 2, 3} };",
        "t: index 2 out of range",
    );
    rej(
        "static const uint16_t t[3] = { 1, 2, 3, 4 };",
        "t: index 3 out of range",
    );
    rej(
        "static const uint16_t t[3] = { [0 - 0] = 1 };",
        "expected \"]\", found \"-\"",
    );
}

#[test]
fn negative_index() {
    // Go indexes a slice with it and panics; here it is an error.
    rej(
        "static const uint16_t t[3] = { [-1] = 1 };",
        "t: index -1 out of range",
    );
    rej(
        "static const char * const t[] = { [-1] = \"a\" };",
        "t: index -1 out of range",
    );
}

#[test]
fn strings_for_dimension() {
    rej(
        "static const char * const n[3] = { \"a\" };",
        "n: 1 strings for dimension 3",
    );
    rej(
        "static const char * const n[1] = { \"a\", \"b\" };",
        "n: 2 strings for dimension 1",
    );
}

#[test]
fn expected_string() {
    rej("static const char * const n[] = { 5 };", "expected string");
}

#[test]
fn entry_bounds() {
    rej(
        &actions("[0] = {.entry = {.count = 256, .reusable = false}},"),
        "entry.count 256 out of range",
    );
    rej(
        &actions("[0] = {.entry = {.count = 0, .reusable = 2}},"),
        "entry.reusable 2 out of range",
    );
    rej(&actions("SHIFT(65536),"), "shift state 65536 out of range");
    rej(&actions("SHIFT_REPEAT(-1),"), "shift state -1 out of range");
    rej(
        &actions("REDUCE(65536, 1, 0, 0),"),
        "symbol 65536 out of range",
    );
    rej(
        &actions("REDUCE(sym_a, 1, -32769, 0),"),
        "dynamic_precedence -32769 out of range",
    );
    rej(
        &actions("REDUCE(sym_a, 1, 32768, 0),"),
        "dynamic_precedence 32768 out of range",
    );
    rej(
        &actions("REDUCE(sym_a, 1, 0, 65536),"),
        "production_id 65536 out of range",
    );
}

#[test]
fn lex_state_bounds() {
    rej(
        &lex_fn("if (lookahead == 'a') ADVANCE(65536);\nEND_STATE();"),
        "lex state 65536 out of range",
    );
    rej(
        &lex_fn("if (lookahead == 'a') SKIP(-1);\nEND_STATE();"),
        "lex state -1 out of range",
    );
    rej(
        &lex_fn("ADVANCE_MAP('a', 65536,);\nEND_STATE();"),
        "ADVANCE_MAP state 65536 out of range",
    );
    rej(
        &lex_fn("ADVANCE_MAP(-1, 1,);\nEND_STATE();"),
        "ADVANCE_MAP key -1 out of range",
    );
    rej(
        "static bool f(TSLexer *lexer, TSStateId state) {\nSTART_LEXER();\neof = lexer->eof(lexer);\nswitch (state) {\ncase 65536:\nEND_STATE();\ndefault:\nreturn false;\n}\n}\n",
        "lex state 65536 out of range",
    );
}

#[test]
fn action_argument_counts() {
    rej(&actions("SHIFT(1, 2),"), "SHIFT with 2 arguments");
    rej(&actions("SHIFT(),"), "SHIFT with 0 arguments");
    rej(
        &actions("SHIFT_REPEAT(1, 2),"),
        "SHIFT_REPEAT with 2 arguments",
    );
    rej(&actions("REDUCE(sym_a, 1, 0),"), "REDUCE with 3 arguments");
    rej(&actions("SHIFT_EXTRA(1),"), "SHIFT_EXTRA with 1 arguments");
}

#[test]
fn unknown_parse_action() {
    rej(&actions("FOO(),"), "unknown parse action FOO");
}

#[test]
fn negative_action_index() {
    rej(
        &actions("[-1] = SHIFT(1),"),
        "ts_parse_actions: index -1 out of range",
    );
}

#[test]
fn duplicate_case() {
    rej(
        &lex_fn("END_STATE();\ncase 0:\nEND_STATE();"),
        "duplicate case 0",
    );
}

#[test]
fn end_state_is_not_the_last_statement() {
    rej(
        &lex_fn("END_STATE();\nACCEPT_TOKEN(sym_a);"),
        "case 0: END_STATE is not the last statement",
    );
    // A case without END_STATE would fall into the next.
    rej(
        &lex_fn("ACCEPT_TOKEN(sym_a);"),
        "case 0: END_STATE is not the last statement",
    );
    rej(
        &lex_fn("if (eof) ADVANCE(1);"),
        "case 0: END_STATE is not the last statement",
    );
}

#[test]
fn unknown_symbol() {
    rej(
        &lex_fn("ACCEPT_TOKEN(ts_zzz);\nEND_STATE();"),
        "unknown symbol ts_zzz",
    );
}

#[test]
fn duplicate_advance_map_key() {
    rej(
        &lex_fn("ADVANCE_MAP(1, 1, 2, 2, 1, 3,);\nEND_STATE();"),
        "duplicate ADVANCE_MAP key 1",
    );
}

#[test]
fn expected_advance_or_skip() {
    rej(
        &lex_fn("if (lookahead == 'a') FOO(1);\nEND_STATE();"),
        "expected ADVANCE or SKIP, found \"FOO\"",
    );
}

#[test]
fn unknown_character_set() {
    rej(
        &lex_fn("if (set_contains(set_9, 1, lookahead)) ADVANCE(1);\nEND_STATE();"),
        "unknown character set set_9",
    );
    // A table of another type is not a character set.
    rej(
        &lex_fn("if (set_contains(t, 1, lookahead)) ADVANCE(1);\nEND_STATE();")
            .replace("enum e", "static const uint16_t t[1] = {1};\nenum e"),
        "unknown character set t",
    );
}

#[test]
fn set_contains_length() {
    rej(
        &lex_fn("if (set_contains(set_1, 0, lookahead)) ADVANCE(1);\nEND_STATE();"),
        "set_contains(set_1, 0): table has 1 ranges",
    );
}

#[test]
fn unexpected_in_lex_function() {
    rej(
        &lex_fn("foo();\nEND_STATE();"),
        "unexpected \"foo\" in lex function",
    );
    rej(
        &lex_fn("return true;"),
        "unexpected \"return\" in lex function",
    );
}

#[test]
fn unexpected_in_lex_condition() {
    rej(
        &lex_fn("if (lookahead == -1) ADVANCE(1);\nEND_STATE();"),
        "unexpected \"-\" in lex condition",
    );
    rej(
        &lex_fn("if (lookahead == \"a\") ADVANCE(1);\nEND_STATE();"),
        "unexpected \"a\" in lex condition",
    );
    rej(
        &lex_fn("if (lookahead == foo) ADVANCE(1);\nEND_STATE();"),
        "unexpected \"foo\" in lex condition",
    );
    rej(
        &lex_fn("if (lookahead = 1) ADVANCE(1);\nEND_STATE();"),
        "unexpected \"=\" in lex condition",
    );
    rej(
        &lex_fn("if (lookahead -> x) ADVANCE(1);\nEND_STATE();"),
        "unexpected \"->\" in lex condition",
    );
    // An unclosed condition runs into the next keyword.
    rej(&lex_fn("if ("), "unexpected \"default\" in lex condition");
}

#[test]
fn malformed_lex_conditions() {
    // Go re-parses the copied condition and fails later; the AST is built here, so these fail here.
    for cond in [
        "",
        "lookahead",
        "5",
        "eof == 1",
        "lookahead == eof",
        "!lookahead",
        "lookahead == 1 == 2",
        "lookahead ==",
        "== 1",
        "(lookahead == 1",
        "lookahead == 1 lookahead == 2",
        "&& eof",
        "eof eof",
        "lookahead < < 1",
    ] {
        let e = parse_grammar(&lex_fn(&format!("if ({cond}) ADVANCE(1);\nEND_STATE();")));
        match e {
            Ok(_) => panic!("accepted condition {cond:?}"),
            Err(e) => assert!(
                e.0.contains("lex condition") || e.0.contains("unexpected"),
                "{cond:?}: {}",
                e.0
            ),
        }
    }
}

#[test]
fn unexpected_function() {
    rej(
        "TS_PUBLIC const TSLanguage *foo(void) {}",
        "unexpected function foo",
    );
    rej(
        &format!(
            "{LANG}{{}};\nreturn &language;\n}}\nTS_PUBLIC const TSLanguage *tree_sitter_y(void) {{}}"
        ),
        "unexpected function tree_sitter_y",
    );
}

#[test]
fn unterminated_tslanguage_initialiser() {
    rej(
        &format!("{LANG}{{ .a = f(1"),
        "unterminated TSLanguage initialiser",
    );
    rej(
        &format!("{LANG}{{ .a = 1"),
        "unterminated TSLanguage initialiser",
    );
}

#[test]
fn duplicate_tslanguage_field() {
    rej(
        &format!("{LANG}{{ .a = 1, .a = 2 }};"),
        "duplicate TSLanguage field a",
    );
    rej(
        &format!("{LANG}{{ .s = 1, .t = 2, .s = 3 }};"),
        "duplicate TSLanguage field s",
    );
}

#[test]
fn scanner_prototype_cut_off_at_end_of_input() {
    // Go loops forever here.
    for src in ["void foo(", "unsigned", "bool x(void)", "void"] {
        rej(src, "unterminated scanner prototype");
    }
}

#[test]
fn tokenizer_errors_surface_through_the_parser() {
    rej("enum e { a = 10u };", "bad number \"10u\"");
    rej("/* x", "unterminated comment");
    rej("enum e { a };\n+", "unexpected byte '+'");
    rej(
        "static const char * const n[] = { \"a };",
        "unterminated string",
    );
}

#[test]
fn string_that_is_not_utf8() {
    // gengrammar keeps names as `String`; a name that is not UTF-8 is refused, not altered.
    rej(
        "static const char * const n[] = { \"\\xff\" };",
        "string is not valid UTF-8",
    );
}
