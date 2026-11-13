use gengrammar::blob::check_language;
use gengrammar::parse::parse_grammar;

const LANG: &str = include_str!("../../testdata/gengrammar/mini_lang.c");

fn check(src: &str) -> Result<gengrammar::blob::LangInfo, String> {
    let mut g = parse_grammar(src).map_err(|e| e.0)?;
    check_language(&mut g).map_err(|e| e.0)
}

/// `mini_lang.c` with the first `from` replaced by `to` must be refused with exactly `msg`.
fn rej(from: &str, to: &str, msg: &str) {
    let src = LANG.replacen(from, to, 1);
    assert_ne!(src, LANG, "{from:?} is not in mini_lang.c");
    assert_eq!(check(&src).unwrap_err(), msg);
}

#[test]
fn mini_lang_is_accepted() {
    let info = check(LANG).unwrap();
    assert!(info.counts.contains(&("abi_version", 14)));
    assert!(info.counts.contains(&("symbol_count", 3)));
    assert!(!info.counts.iter().any(|c| c.0 == "keyword_capture_token"));
    assert_eq!(info.symbol_names, "ts_symbol_names");
    assert_eq!(info.field_names, "ts_field_names");
    assert!(info.used.contains(&"ts_parse_actions".to_string()));
    assert!(!info.used.contains(&"set_1".to_string()));
}

#[test]
fn field_set_twice() {
    rej(
        ".abi_version = LANGUAGE_VERSION,",
        ".version = LANGUAGE_VERSION,\n    .abi_version = LANGUAGE_VERSION,",
        "TSLanguage.abi_version set twice",
    );
}

#[test]
fn abi_version_must_match_define() {
    rej(
        ".abi_version = LANGUAGE_VERSION,",
        ".abi_version = 15,",
        "ABI version 15 does not match LANGUAGE_VERSION",
    );
    rej(
        ".abi_version = LANGUAGE_VERSION,",
        "",
        "ABI version 0 does not match LANGUAGE_VERSION",
    );
}

#[test]
fn no_such_table() {
    rej(
        ".parse_actions = ts_parse_actions,",
        ".parse_actions = ts_nope,",
        "TSLanguage.parse_actions: no table ts_nope",
    );
}

// One test per field of Go's `want` map. Each edit makes that table disagree with the counts
// and no earlier one.

#[test]
fn count_parse_table() {
    rej(
        ".large_state_count = LARGE_STATE_COUNT,",
        ".large_state_count = 3,",
        "ts_parse_table has 6 entries, the counts say 9",
    );
}

#[test]
fn count_small_parse_table_map() {
    rej(
        ".state_count = STATE_COUNT,",
        ".state_count = 5,",
        "ts_small_parse_table_map has 2 entries, the counts say 3",
    );
}

#[test]
fn count_symbol_names() {
    rej(
        ".alias_count = ALIAS_COUNT,",
        ".alias_count = 1,",
        "ts_symbol_names has 3 entries, the counts say 4",
    );
}

#[test]
fn count_symbol_metadata() {
    rej(
        "[sym_b] = {.visible = true, .named = false, .supertype = true,},",
        "[sym_b] = {.visible = true, .named = false, .supertype = true,}, [3] = {.visible = true,},",
        "ts_symbol_metadata has 4 entries, the counts say 3",
    );
}

#[test]
fn count_public_symbol_map() {
    rej(
        "[sym_b] = sym_b, };",
        "[sym_b] = sym_b, [3] = 0, };",
        "ts_symbol_map has 4 entries, the counts say 3",
    );
}

#[test]
fn count_field_names() {
    rej(
        "[field_f] = \"f\", };",
        "[field_f] = \"f\", [2] = \"g\", };",
        "ts_field_names has 3 entries, the counts say 2",
    );
}

#[test]
fn count_field_map_slices() {
    rej(
        ".production_id_count = PRODUCTION_ID_COUNT,",
        ".production_id_count = 3,",
        "ts_field_map_slices has 2 entries, the counts say 3",
    );
}

#[test]
fn count_alias_sequences() {
    rej(
        ".max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,",
        ".max_alias_sequence_length = 3,",
        "ts_alias_sequences has 4 entries, the counts say 6",
    );
}

#[test]
fn count_lex_modes() {
    rej(
        "ts_lex_modes[4]",
        "ts_lex_modes[5]",
        "ts_lex_modes has 5 entries, the counts say 4",
    );
}

#[test]
fn count_primary_state_ids() {
    rej(
        "ts_primary_state_ids[STATE_COUNT]",
        "ts_primary_state_ids[5]",
        "ts_primary_state_ids has 5 entries, the counts say 4",
    );
}

#[test]
fn count_supertype_symbols() {
    let src = LANG
        .replacen(
            "void *tree_sitter_mini_external_scanner_create(void);",
            "static const TSSymbol ts_supertype_symbols[] = { sym_b, };\n\
             void *tree_sitter_mini_external_scanner_create(void);",
            1,
        )
        .replacen(
            ".primary_state_ids = ts_primary_state_ids,",
            ".primary_state_ids = ts_primary_state_ids,\n    .supertype_symbols = ts_supertype_symbols,",
            1,
        );
    assert_eq!(
        check(&src).unwrap_err(),
        "ts_supertype_symbols has 1 entries, the counts say 0"
    );
}

#[test]
fn reserved_words_row_width() {
    let src = LANG
        .replacen(
            "void *tree_sitter_mini_external_scanner_create(void);",
            "static const TSSymbol ts_reserved_words[1][2] = { {sym_a, 0}, };\n\
             void *tree_sitter_mini_external_scanner_create(void);",
            1,
        )
        .replacen(
            ".primary_state_ids = ts_primary_state_ids,",
            ".primary_state_ids = ts_primary_state_ids,\n    .reserved_words = &ts_reserved_words[0][0],",
            1,
        );
    assert_eq!(
        check(&src).unwrap_err(),
        "ts_reserved_words rows are 2 wide, max_reserved_word_set_size is 0"
    );
}

#[test]
fn lex_fn_must_be_ts_lex() {
    rej(
        ".lex_fn = ts_lex,",
        ".lex_fn = ts_lex_x,",
        "lex_fn is \"ts_lex_x\"",
    );
}

#[test]
fn keyword_lex_fn_must_be_ts_lex_keywords() {
    rej(
        ".lex_fn = ts_lex,",
        ".lex_fn = ts_lex,\n    .keyword_lex_fn = ts_lex_x,",
        "keyword_lex_fn is \"ts_lex_x\"",
    );
}

#[test]
fn lex_function_count() {
    rej(
        ".lex_fn = ts_lex,",
        ".lex_fn = ts_lex,\n    .keyword_capture_token = sym_a,",
        "1 lex functions, TSLanguage uses 2",
    );
}

#[test]
fn unexpected_lex_function() {
    rej(
        "static bool ts_lex(TSLexer",
        "static bool ts_lex_other(TSLexer",
        "unexpected lex function ts_lex_other",
    );
}

#[test]
fn name_must_match() {
    rej(
        ".name = \"mini\",",
        ".name = \"other\",",
        "TSLanguage.name does not match tree_sitter_mini",
    );
}

#[test]
fn metadata_out_of_range() {
    rej(
        ".minor_version = 1,",
        ".minor_version = 256,",
        "metadata.minor_version out of range",
    );
}

#[test]
fn external_scanner_state_rows() {
    rej(
        "ts_external_scanner_states[3][EXTERNAL_TOKEN_COUNT]",
        "ts_external_scanner_states[3][2]",
        "ts_external_scanner_states rows are 2 wide, external_token_count is 1",
    );
}

#[test]
fn external_scanner_symbol_map_length() {
    rej(
        "ts_external_scanner_symbol_map[EXTERNAL_TOKEN_COUNT]",
        "ts_external_scanner_symbol_map[2]",
        "ts_external_scanner_symbol_map has 2 entries, external_token_count is 1",
    );
}

#[test]
fn external_scanner_function_names() {
    rej(
        "      tree_sitter_mini_external_scanner_scan,",
        "      tree_sitter_other_external_scanner_scan,",
        "external_scanner.scan is \"tree_sitter_other_external_scanner_scan\"",
    );
}

#[test]
fn external_tokens_need_a_scanner() {
    let from = LANG.find("    .external_scanner = {").unwrap();
    let to = from + LANG[from..].find("    },\n").unwrap() + "    },\n".len();
    let src = format!("{}{}", &LANG[..from], &LANG[to..]);
    assert_eq!(
        check(&src).unwrap_err(),
        "external tokens without external_scanner"
    );
}

#[test]
fn field_is_not_handled() {
    rej(
        ".name = \"mini\",",
        ".name = \"mini\",\n    .bogus = 1,",
        "TSLanguage.bogus is not handled",
    );
}

#[test]
fn table_is_not_referenced() {
    rej(
        "void *tree_sitter_mini_external_scanner_create(void);",
        "static const uint16_t ts_extra[] = { 1, };\n\
         void *tree_sitter_mini_external_scanner_create(void);",
        "table ts_extra is not referenced by TSLanguage",
    );
}

#[test]
fn value_does_not_fit_a_scalar() {
    rej(
        "[3] = 3, };",
        "[3] = 70000, };",
        "ts_primary_state_ids: value 70000 does not fit TSStateId",
    );
}

#[test]
fn value_does_not_fit_a_struct_field() {
    rej(
        ".lex_state = 5,",
        ".lex_state = 65536,",
        "ts_lex_modes: value 65536 does not fit LexState",
    );
    rej(
        "[sym_a] = {.visible = true, .named = true,},",
        "[sym_a] = {.visible = 2, .named = true,},",
        "ts_symbol_metadata: value 2 does not fit Visible",
    );
}

#[test]
fn value_does_not_fit_a_count() {
    rej(
        ".max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,",
        ".max_alias_sequence_length = 70000,",
        "TSLanguage.max_alias_sequence_length: value 70000 does not fit u16",
    );
}

#[test]
fn value_does_not_fit_a_symbol() {
    rej(
        "enum ts_field_identifiers",
        "enum ts_big { big_x = 70000, };\nenum ts_field_identifiers",
        "big_x: value 70000 does not fit u16",
    );
}

#[test]
fn cannot_evaluate() {
    rej(
        ".alias_count = ALIAS_COUNT,",
        ".alias_count = unknown_name,",
        "TSLanguage.alias_count: cannot evaluate",
    );
}

#[test]
fn two_names() {
    rej(
        ".symbol_names = ts_symbol_names,",
        ".symbol_names = ts_symbol_names ts_field_names,",
        "TSLanguage.symbol_names: two names",
    );
}

#[test]
fn no_table_name() {
    rej(
        ".symbol_names = ts_symbol_names,",
        ".symbol_names = 0,",
        "TSLanguage.symbol_names: no table name",
    );
}

#[test]
fn parse_action_entry_runs_past_the_end() {
    rej(
        "[4] = {.entry = {.count = 1, .reusable = false}}",
        "[4] = {.entry = {.count = 2, .reusable = false}}",
        "ts_parse_actions: entry 4 runs past the end",
    );
}
