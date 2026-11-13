
#define LANGUAGE_VERSION 14
#define LARGE_STATE_COUNT 2
enum ts_symbol_identifiers { sym_a = 1, sym_b = 2, };
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
