
#define LANGUAGE_VERSION 14
#define STATE_COUNT 4
#define LARGE_STATE_COUNT 2
#define SYMBOL_COUNT 3
#define ALIAS_COUNT 0
#define TOKEN_COUNT 2
#define EXTERNAL_TOKEN_COUNT 1
#define FIELD_COUNT 1
#define MAX_ALIAS_SEQUENCE_LENGTH 2
#define PRODUCTION_ID_COUNT 2
enum ts_symbol_identifiers { sym_a = 1, sym_b = 2, };
enum ts_external_scanner_symbol_identifiers { ts_external_token_x = 0, };
enum ts_field_identifiers { field_f = 1, };
static const char * const ts_symbol_names[] = { [ts_builtin_sym_end] = "end", [sym_a] = "\"a\?", [sym_b] = "\0", };
static TSCharacterRange set_1[] = { {'a', 'z'}, {0xaa, 0xaa}, };
static const uint16_t ts_small_parse_table[] = { [0] = 1, ACTIONS(3), 1, sym_b, [7] = 2, };
static const uint32_t ts_small_parse_table_map[] = { [SMALL_STATE(2)] = 0, [SMALL_STATE(3)] = 7, };
static const TSLexMode ts_lex_modes[4] = { [1] = {.lex_state = 5, .external_lex_state = 2}, [3] = {(TSStateId)(-1),}, };
static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 2, .reusable = true}}, REDUCE(sym_b, 2, -1, 7), SHIFT_REPEAT(9),
  [4] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
};
static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(1);
      ADVANCE_MAP('"', 2, '\\', 3, 0xa0, 4, );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      if ((!eof && set_contains(set_1, 2, lookahead))) ADVANCE(5);
      if (lookahead != 0 &&
          (lookahead < '(' || ')' < lookahead)) ADVANCE(6);
      END_STATE();
    case 1:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    default:
      return false;
  }
}
static const char * const ts_field_names[] = { [0] = NULL, [field_f] = "f", };
static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {.visible = false, .named = true,},
  [sym_a] = {.visible = true, .named = true,},
  [sym_b] = {.visible = true, .named = false, .supertype = true,},
};
static const TSSymbol ts_symbol_map[] = { [ts_builtin_sym_end] = ts_builtin_sym_end, [sym_a] = sym_a, [sym_b] = sym_b, };
static const uint16_t ts_non_terminal_alias_map[] = { 0, };
static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = { [0] = {0}, [1] = {[1] = sym_b}, };
static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = { [0] = {[sym_a] = STATE(1), [sym_b] = ACTIONS(1),}, [1] = {[ts_builtin_sym_end] = ACTIONS(4),}, };
static const TSFieldMapSlice ts_field_map_slices[PRODUCTION_ID_COUNT] = { [1] = {.index = 0, .length = 1}, };
static const TSFieldMapEntry ts_field_map_entries[] = { [0] = {.field_id = field_f, .child_index = 0}, };
static const TSStateId ts_primary_state_ids[STATE_COUNT] = { [0] = 0, [1] = 1, [2] = 2, [3] = 3, };
static const bool ts_external_scanner_states[3][EXTERNAL_TOKEN_COUNT] = { [1] = {[ts_external_token_x] = true,}, [2] = {[ts_external_token_x] = true,}, };
static const TSSymbol ts_external_scanner_symbol_map[EXTERNAL_TOKEN_COUNT] = { [ts_external_token_x] = sym_a, };
void *tree_sitter_mini_external_scanner_create(void);
void tree_sitter_mini_external_scanner_destroy(void *);
bool tree_sitter_mini_external_scanner_scan(void *, TSLexer *, const bool *);
unsigned tree_sitter_mini_external_scanner_serialize(void *, char *);
void tree_sitter_mini_external_scanner_deserialize(void *, const char *, unsigned);
TS_PUBLIC const TSLanguage *tree_sitter_mini(void) {
  static const TSLanguage language = {
    .abi_version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .field_names = ts_field_names,
    .field_map_slices = ts_field_map_slices,
    .field_map_entries = ts_field_map_entries,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = (const void*)ts_lex_modes,
    .lex_fn = ts_lex,
    .external_scanner = {
      &ts_external_scanner_states[0][0],
      ts_external_scanner_symbol_map,
      tree_sitter_mini_external_scanner_create,
      tree_sitter_mini_external_scanner_destroy,
      tree_sitter_mini_external_scanner_scan,
      tree_sitter_mini_external_scanner_serialize,
      tree_sitter_mini_external_scanner_deserialize,
    },
    .primary_state_ids = ts_primary_state_ids,
    .name = "mini",
    .metadata = {
      .major_version = 0,
      .minor_version = 1,
      .patch_version = 2,
    },
  };
  return &language;
}
