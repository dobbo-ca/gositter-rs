use gengrammar::blob::{check_language, write_blob};
use gengrammar::parse::parse_grammar;
use gengrammar::rust::mod_rs;

const LANG: &str = include_str!("../../testdata/gengrammar/mini_lang.c");

fn mini() -> String {
    let mut g = parse_grammar(LANG).unwrap();
    let info = check_language(&mut g).unwrap();
    let blob = write_blob(&g, &info);
    mod_rs("", &g, &info, &blob)
}

fn has_line(t: &str, line: &str) {
    assert!(t.lines().any(|l| l == line), "missing {line:?} in\n{t}");
}

#[test]
fn language_emitted() {
    let t = mini();
    for pair in [
        "#[rustfmt::skip]\nmod lex;",
        "#[rustfmt::skip]\nmod scanner;",
        "fn lex_main(l: &mut gositter::Lexer<'_>, state: u16) -> bool {\n    lex::lex(l, state)\n}",
    ] {
        assert!(t.contains(pair), "missing {pair:?} in\n{t}");
    }
    for line in [
        "const TABLES: &[u8] = include_bytes!(\"tables.bin\");",
        "static LANGUAGE: gositter::Language = gositter::Language::new(gositter::LanguageTables {",
        "    abi_version: COUNTS.abi_version,",
        "    state_count: COUNTS.state_count,",
        "    max_alias_sequence_length: COUNTS.max_alias_sequence_length,",
        "    parse_table: gositter::U16s::at(TABLES, TS_PARSE_TABLE.0, TS_PARSE_TABLE.1),",
        "    small_parse_table_map: gositter::U32s::at(TABLES, TS_SMALL_PARSE_TABLE_MAP.0, TS_SMALL_PARSE_TABLE_MAP.1),",
        "    parse_actions: gositter::ParseActions::at(TABLES, TS_PARSE_ACTIONS.0, TS_PARSE_ACTIONS.1),",
        "    symbol_names: SYMBOL_NAMES,",
        "    field_names: FIELD_NAMES,",
        "    supertype_symbols: gositter::U16s::EMPTY,",
        "    lex_fn: lex_main,",
        "    keyword_lex_fn: None,",
        "        states: gositter::Bools::at(TABLES, TS_EXTERNAL_SCANNER_STATES.0, TS_EXTERNAL_SCANNER_STATES.1),",
        "        create: Some(scanner::new),",
        "    name: \"mini\",",
        "    metadata: gositter::LanguageMetadata { major_version: 0, minor_version: 1, patch_version: 2 },",
        "});",
        "pub fn language() -> &'static gositter::Language {",
        "    &LANGUAGE",
    ] {
        has_line(&t, line);
    }
    assert!(!t.contains("lex_keywords_main"), "{t}");
    assert!(!t.contains("allow(dead_code)"), "{t}");
}
