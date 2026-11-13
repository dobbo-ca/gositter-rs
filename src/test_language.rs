//! gositter `testLanguage()` (`subtree_test.go:19-35`) as static tables, plus a small parse
//! table of three small states for the lookup tests and the fields `key` and `value`.
// The subtree, stack and node tests (plan Tasks 8 to 10) use the rest.
#![allow(dead_code)]

use crate::*;

fn no_lex(_: &mut Lexer<'_>, _: StateId) -> bool {
    false
}

pub(crate) const TS_END: Symbol = 0;
pub(crate) const TS_ANON: Symbol = 1;
pub(crate) const TS_IDENT: Symbol = 2;
pub(crate) const TS_HIDDEN: Symbol = 3;
pub(crate) const TS_RULE: Symbol = 4;
pub(crate) const TS_REPEAT: Symbol = 5;

/// State 0, the one large state.
const PARSE_TABLE: [u8; 12] = le_bytes::<6, 12>(&[0, 1, 0, 0, 2, 0]);
/// States 1 to 3 as `[group count, (value, symbol count, symbols...)...]`. State 3 names
/// symbol 2 twice: the scan returns its first group, value 0.
const SMALL_PARSE_TABLE: [u8; 44] = le_bytes::<22, 44>(&[
    2, 5, 2, 1, 2, 7, 1, 4, // state 1
    1, 3, 3, 0, 1, 5, // state 2
    2, 0, 1, 2, 9, 2, 2, 3, // state 3
]);
const SMALL_PARSE_TABLE_MAP: [u8; 12] = [0, 0, 0, 0, 8, 0, 0, 0, 14, 0, 0, 0];
/// end {}, a visible, ident visible named, _hidden {}, rule visible named, rule_repeat1 {}.
const SYMBOL_METADATA: [u8; 6] = [0, 1, 3, 0, 3, 0];
const PUBLIC_SYMBOL_MAP: [u8; 12] = le_bytes::<6, 12>(&[0, 1, 2, 3, 4, 5]);

pub(crate) const TABLES: LanguageTables = LanguageTables {
    abi_version: 14,
    symbol_count: 6,
    alias_count: 0,
    token_count: 4,
    external_token_count: 0,
    state_count: 4,
    large_state_count: 1,
    production_id_count: 0,
    field_count: 2,
    max_alias_sequence_length: 0,
    parse_table: U16s::new(&PARSE_TABLE),
    small_parse_table: U16s::new(&SMALL_PARSE_TABLE),
    small_parse_table_map: U32s::new(&SMALL_PARSE_TABLE_MAP),
    parse_actions: ParseActions::EMPTY,
    symbol_names: &["end", "a", "ident", "_hidden", "rule", "rule_repeat1"],
    field_names: &["", "key", "value"],
    field_map_slices: MapSlices::EMPTY,
    field_map_entries: FieldMapEntries::EMPTY,
    symbol_metadata: SymbolMetadatas::new(&SYMBOL_METADATA),
    public_symbol_map: U16s::new(&PUBLIC_SYMBOL_MAP),
    alias_map: U16s::EMPTY,
    alias_sequences: U16s::EMPTY,
    lex_modes: LexModes::EMPTY,
    lex_fn: no_lex,
    keyword_lex_fn: None,
    keyword_capture_token: 0,
    external_scanner: ExternalScannerInfo {
        states: Bools::EMPTY,
        symbol_map: U16s::EMPTY,
        create: None,
    },
    primary_state_ids: U16s::EMPTY,
    name: "test",
    reserved_words: U16s::EMPTY,
    max_reserved_word_set_size: 0,
    supertype_count: 0,
    supertype_symbols: U16s::EMPTY,
    supertype_map_slices: MapSlices::EMPTY,
    supertype_map_entries: U16s::EMPTY,
    metadata: LanguageMetadata {
        major_version: 0,
        minor_version: 0,
        patch_version: 0,
    },
};

/// A fresh test language, so that no test sees another test's KindMask names.
pub(crate) fn test_language() -> &'static Language {
    Box::leak(Box::new(Language::new(TABLES)))
}
