//! parser.c (gositter `parser.go`).
//!
//! Not ported, because old_tree is always NULL: ts_parser__reuse_node,
//! ts_parser__breakdown_lookahead (a lookahead is always a leaf here), the ReusableNode,
//! included range differences. Also not ported: wasm, cancellation, timeouts and the
//! progress callback (parse_budget stops a parse that makes no progress instead), LOG_STACK
//! and LOG_TREE (dot graphs).

use crate::length::{Length, length_sub};
use crate::lexer::LexerState;
use crate::stack::{STACK_VERSION_NONE, Stack, StackVersion};
use crate::subtree::{
    ERR_SLAB_FULL, ERROR_COST_PER_SKIPPED_CHAR, ERROR_COST_PER_SKIPPED_LINE,
    ERROR_COST_PER_SKIPPED_TREE, ERROR_STATE, F_EXTRA, F_FRAGILE_LEFT, F_FRAGILE_RIGHT,
    F_HAS_EXTERNAL_SCANNER_STATE_CHANGE, F_HAS_REPEAT, NULL_SUBTREE, Subtree, SubtreeArray,
    SubtreeHeap, TREE_STATE_NONE,
};
use crate::{
    ExternalScanner, Language, Lexer, LogType, Logger, MIN_COMPATIBLE_LANGUAGE_VERSION,
    PARSE_ACTION_TYPE_ACCEPT, PARSE_ACTION_TYPE_RECOVER, PARSE_ACTION_TYPE_REDUCE,
    PARSE_ACTION_TYPE_SHIFT, SERIALIZATION_BUFFER_SIZE, SYMBOL_END, SYMBOL_ERROR,
    SYMBOL_ERROR_REPEAT, StateId, Symbol, TREE_SITTER_LANGUAGE_VERSION, Tree,
};

/// MAX_VERSION_COUNT
const MAX_VERSION_COUNT: u32 = 6;
/// MAX_VERSION_COUNT_OVERFLOW
const MAX_VERSION_COUNT_OVERFLOW: u32 = 4;
/// MAX_SUMMARY_DEPTH
const MAX_SUMMARY_DEPTH: u32 = 16;
/// MAX_COST_DIFFERENCE
const MAX_COST_DIFFERENCE: u32 = 18 * ERROR_COST_PER_SKIPPED_TREE;
/// sizeof(debug_buffer)
const DEBUG_BUFFER_SIZE: usize = SERIALIZATION_BUFFER_SIZE;

/// TokenCache
#[derive(Clone, Copy, Debug, Default)]
struct TokenCache {
    token: Subtree,
    last_external_token: Subtree,
    byte_index: u32,
}

/// ReduceAction (reduce_action.h)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ReduceAction {
    count: u32,
    symbol: Symbol,
    dynamic_precedence: i32,
    production_id: u16,
}

/// What ts_parser__do_all_potential_reductions finds over every token of a state. No C
/// counterpart: C scans the table again at each error.
#[derive(Clone, Debug, Default)]
struct ReduceScan {
    reduce_actions: Vec<ReduceAction>,
    has_shift_action: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MissingToken {
    symbol: Symbol,
    state_after: StateId,
}

/// TSParser. One per thread at a time; it can move between threads.
pub struct Parser {
    /// The lexer fields between parses; each parse moves them into a `Lexer` and back.
    lexer_state: LexerState,
    pub(crate) stack: Stack,
    /// The result of the last parse. It owns the subtree pool (Go `p.pool`).
    pub(crate) tree: Tree,
    pub(crate) language: Option<&'static Language>,
    reduce_actions: Vec<ReduceAction>,
    /// Go `reduceScans` keyed by (language, state): indexed by state, cleared by
    /// set_language.
    reduce_scans: Vec<Option<ReduceScan>>,
    /// Go `missingScans`, as reduce_scans.
    missing_scans: Vec<Option<Vec<MissingToken>>>,
    finished_tree: Subtree,
    trailing_extras: Vec<Subtree>,
    trailing_extras2: Vec<Subtree>,
    token_cache: TokenCache,
    /// external_scanner_payload
    external_scanner: Option<Box<dyn ExternalScanner>>,
    accept_count: u32,
    /// Written by recover and reset, read nowhere, as in Go (ts_parser_has_error is not
    /// ported).
    #[allow(dead_code)]
    has_error: bool,
    /// C serializes scanner state into lexer.debug_buffer. Log messages use that buffer
    /// too, so the state gets its own.
    scanner_buf: [u8; SERIALIZATION_BUFFER_SIZE],
    /// valid_external_tokens as `&[bool]`: the `Bools` table row copied out, because a
    /// view of 0/1 bytes cannot be borrowed as bools without unsafe.
    valid_external_tokens: Vec<bool>,
    /// The scanner of the last parse. It never goes to another language.
    pub(crate) kept_scanner: Option<Box<dyn ExternalScanner>>,
    kept_language: Option<&'static Language>,
    /// Bumped on every scanner construction; kept_scanner_id while a scanner is kept.
    pub(crate) scanner_generation: u64,
}

/// ErrorStatus
#[derive(Clone, Copy, Debug, Default)]
struct ErrorStatus {
    cost: u32,
    node_count: u32,
    dynamic_precedence: i32,
    is_in_error: bool,
}

// ErrorComparison
const ERROR_COMPARISON_TAKE_LEFT: u32 = 0;
const ERROR_COMPARISON_PREFER_LEFT: u32 = 1;
const ERROR_COMPARISON_NONE: u32 = 2;
const ERROR_COMPARISON_PREFER_RIGHT: u32 = 3;
const ERROR_COMPARISON_TAKE_RIGHT: u32 = 4;

// Logging. Every call site is behind `if lexer.state.logger.is_some()`.

/// logBuf builds one message without format!, in the lexer's reused debug buffer.
struct LogBuf(Vec<u8>);

impl LogBuf {
    fn s(mut self, v: &str) -> LogBuf {
        self.0.extend_from_slice(v.as_bytes());
        self
    }

    fn u(mut self, v: u32) -> LogBuf {
        let mut digits = [0u8; 10];
        let mut i = digits.len();
        let mut v = v;
        loop {
            i -= 1;
            digits[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.0.extend_from_slice(&digits[i..]);
        self
    }

    fn d(mut self, v: i32) -> LogBuf {
        if v < 0 {
            self.0.push(b'-');
        }
        self.u(v.unsigned_abs())
    }

    /// The LOG macro plus ts_parser__log. snprintf cuts at sizeof(buffer) - 1.
    fn log(self, lexer: &mut Lexer<'_>) {
        let mut b = self.0;
        let n = b.len().min(DEBUG_BUFFER_SIZE - 1);
        if let Some(logger) = lexer.state.logger.as_mut() {
            logger(LogType::Parse, &b[..n]);
        }
        b.clear();
        lexer.state.debug_buf = b;
    }
}

/// lb: an empty message in the lexer's debug buffer.
fn lb(lexer: &mut Lexer<'_>) -> LogBuf {
    let mut b = std::mem::take(&mut lexer.state.debug_buf);
    b.clear();
    LogBuf(b)
}

/// LOG_LOOKAHEAD
fn log_lookahead(lexer: &mut Lexer<'_>, symbol_name: &str, size: u32) {
    let mut b = lb(lexer).s("lexed_lookahead sym:");
    for &c in symbol_name.as_bytes() {
        if b.0.len() >= DEBUG_BUFFER_SIZE {
            break;
        }
        match c {
            b'\t' => b.0.extend_from_slice(b"\\t"),
            b'\n' => b.0.extend_from_slice(b"\\n"),
            0x0B => b.0.extend_from_slice(b"\\v"),
            0x0C => b.0.extend_from_slice(b"\\f"),
            b'\r' => b.0.extend_from_slice(b"\\r"),
            b'\\' => b.0.extend_from_slice(b"\\\\"),
            _ => b.0.push(c),
        }
    }
    b.s(", size:").u(size).log(lexer);
}

// ts_parser__compare_versions
fn compare_versions(a: ErrorStatus, b: ErrorStatus) -> u32 {
    if !a.is_in_error && b.is_in_error {
        if a.cost < b.cost {
            return ERROR_COMPARISON_TAKE_LEFT;
        }
        return ERROR_COMPARISON_PREFER_LEFT;
    }

    if a.is_in_error && !b.is_in_error {
        if b.cost < a.cost {
            return ERROR_COMPARISON_TAKE_RIGHT;
        }
        return ERROR_COMPARISON_PREFER_RIGHT;
    }

    if a.cost < b.cost {
        if (b.cost - a.cost).wrapping_mul(a.node_count.wrapping_add(1)) > MAX_COST_DIFFERENCE {
            return ERROR_COMPARISON_TAKE_LEFT;
        }
        return ERROR_COMPARISON_PREFER_LEFT;
    }

    if b.cost < a.cost {
        if (a.cost - b.cost).wrapping_mul(b.node_count.wrapping_add(1)) > MAX_COST_DIFFERENCE {
            return ERROR_COMPARISON_TAKE_RIGHT;
        }
        return ERROR_COMPARISON_PREFER_RIGHT;
    }

    if a.dynamic_precedence > b.dynamic_precedence {
        return ERROR_COMPARISON_PREFER_LEFT;
    }
    if b.dynamic_precedence > a.dynamic_precedence {
        return ERROR_COMPARISON_PREFER_RIGHT;
    }
    ERROR_COMPARISON_NONE
}

// Whether each of the actions `[start, end)` of the parse action table is a SHIFT_REPEAT,
// which ts_parser__advance skips: the action before them is then the last one that does
// anything.
fn only_repetitions(lang: &Language, start: usize, end: usize) -> bool {
    for i in start..end {
        let a = lang.tables.parse_actions.get(i);
        if a.action_type != PARSE_ACTION_TYPE_SHIFT || !a.repetition() {
            return false;
        }
    }
    true
}

// The symbol loop of ts_parser__do_all_potential_reductions: fills `actions` and reports
// has_shift_action. A free function so that error_scans_ok can scan again without a
// mutable parser.
fn scan_reductions(
    lang: &Language,
    state: StateId,
    first_symbol: u32,
    end_symbol: u32,
    actions: &mut Vec<ReduceAction>,
) -> bool {
    let mut has_shift_action = false;
    actions.clear();
    for symbol in first_symbol..end_symbol {
        let (first, count, _) = lang.table_entry(state, symbol as Symbol);
        for k in first..first + count as usize {
            let action = lang.tables.parse_actions.get(k);
            match action.action_type {
                PARSE_ACTION_TYPE_SHIFT | PARSE_ACTION_TYPE_RECOVER => {
                    if !action.extra() && !action.repetition() {
                        has_shift_action = true;
                    }
                }
                PARSE_ACTION_TYPE_REDUCE if action.child_count > 0 => {
                    reduce_action_set_add(
                        actions,
                        ReduceAction {
                            symbol: action.symbol,
                            count: u32::from(action.child_count),
                            dynamic_precedence: i32::from(action.dynamic_precedence),
                            production_id: action.production_id,
                        },
                    );
                }
                _ => {}
            }
        }
    }
    has_shift_action
}

// ts_reduce_action_set_add (reduce_action.h)
fn reduce_action_set_add(actions: &mut Vec<ReduceAction>, new_action: ReduceAction) {
    for action in actions.iter() {
        if action.symbol == new_action.symbol && action.count == new_action.count {
            return;
        }
    }
    actions.push(new_action);
}

// The tokens ts_parser__handle_error tries as a missing token in a state, each with the
// state it leads to.
fn scan_missing_tokens(lang: &Language, state: StateId) -> Vec<MissingToken> {
    let mut tokens = Vec::new();
    let token_count = u32::from(lang.tables.token_count as u16);
    for missing_symbol in 1..token_count {
        let state_after_missing_symbol = lang.next_state(state, missing_symbol as Symbol);
        if state_after_missing_symbol == 0 || state_after_missing_symbol == state {
            continue;
        }
        tokens.push(MissingToken {
            symbol: missing_symbol as Symbol,
            state_after: state_after_missing_symbol,
        });
    }
    tokens
}

impl Parser {
    /// The language of the parse. Only called while one is set.
    fn lang(&self) -> &'static Language {
        self.language.expect("gositter: no language")
    }

    // SYM_NAME
    fn sym_name(&self, symbol: Symbol) -> &'static str {
        self.lang().symbol_name(symbol).unwrap_or("")
    }

    // TREE_NAME
    fn tree_name(&self, tree: Subtree) -> &'static str {
        self.sym_name(self.tree.pool.symbol(tree))
    }

    // ts_parser__breakdown_top_of_stack. Without reused nodes no link is ever pending, so
    // the pop comes back empty; ported for the stack walk it does.
    fn breakdown_top_of_stack(&mut self, lexer: &mut Lexer<'_>, version: StackVersion) -> bool {
        let lang = self.lang();
        let mut did_break_down = false;
        let mut pending;

        loop {
            self.stack.pop_pending(&mut self.tree.pool, version);
            // The slices are taken out of the stack while the loop pushes; nothing below
            // walks the stack.
            let pop = std::mem::take(&mut self.stack.slices);
            if pop.is_empty() {
                self.stack.slices = pop;
                break;
            }

            did_break_down = true;
            pending = false;
            for mut slice in pop.iter().copied() {
                let mut state = self.stack.state(slice.version);
                let parent = self.tree.pool.array(slice.subtrees)[0];

                let child_count = self.tree.pool.child_count(parent) as usize;
                for k in 0..child_count {
                    let sp = &mut self.tree.pool;
                    let child = sp.children_of(parent)[k];
                    pending = sp.child_count(child) > 0;

                    if sp.is_error(child) {
                        state = ERROR_STATE;
                    } else if !sp.extra(child) {
                        state = lang.next_state(state, sp.symbol(child));
                    }

                    sp.retain(child);
                    self.stack.push(sp, slice.version, child, pending, state);
                }

                for j in 1..slice.subtrees.size as usize {
                    let t = self.tree.pool.array(slice.subtrees)[j];
                    self.stack
                        .push(&self.tree.pool, slice.version, t, false, state);
                }

                self.tree.pool.release(parent);
                self.tree.pool.array_free(&mut slice.subtrees);

                if lexer.state.logger.is_some() {
                    lb(lexer)
                        .s("breakdown_top_of_stack tree:")
                        .s(self.tree_name(parent))
                        .log(lexer);
                }
            }
            self.stack.slices = pop;
            if !pending {
                break;
            }
        }

        did_break_down
    }

    // ts_parser__version_status
    fn version_status(&mut self, version: StackVersion) -> ErrorStatus {
        let mut cost = self.stack.error_cost(version);
        let is_paused = self.stack.is_paused(version);
        if is_paused {
            cost = cost.wrapping_add(ERROR_COST_PER_SKIPPED_TREE);
        }
        ErrorStatus {
            cost,
            node_count: self.stack.node_count_since_error(version),
            dynamic_precedence: self.stack.dynamic_precedence(version),
            is_in_error: is_paused || self.stack.state(version) == ERROR_STATE,
        }
    }

    // ts_parser__better_version_exists
    fn better_version_exists(
        &mut self,
        version: StackVersion,
        is_in_error: bool,
        cost: u32,
    ) -> bool {
        if self.finished_tree != NULL_SUBTREE
            && self.tree.pool.error_cost(self.finished_tree) <= cost
        {
            return true;
        }

        let position = self.stack.position(version);
        let status = ErrorStatus {
            cost,
            is_in_error,
            dynamic_precedence: self.stack.dynamic_precedence(version),
            node_count: self.stack.node_count_since_error(version),
        };

        let n = self.stack.version_count();
        for i in 0..n {
            if i == version
                || !self.stack.is_active(i)
                || self.stack.position(i).bytes < position.bytes
            {
                continue;
            }
            let status_i = self.version_status(i);
            match compare_versions(status, status_i) {
                ERROR_COMPARISON_TAKE_RIGHT => return true,
                ERROR_COMPARISON_PREFER_RIGHT
                    if self.stack.can_merge(&self.tree.pool, i, version) =>
                {
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    // ts_parser__external_scanner_create. Go keeps a scanner that has Reset; here the
    // scanner of the last parse is kept (external_scanner_destroy) and reset() says whether
    // it can serve this one.
    fn external_scanner_create(&mut self) {
        if let Some(lang) = self.language
            && !lang.tables.external_scanner.states.is_empty()
            && let Some(create) = lang.tables.external_scanner.create
        {
            if self.kept_language.is_some_and(|k| std::ptr::eq(k, lang))
                && let Some(mut kept) = self.kept_scanner.take()
                && kept.reset()
            {
                self.external_scanner = Some(kept);
                return;
            }
            self.kept_scanner = None;
            self.kept_language = None;
            self.external_scanner = Some(create());
            self.scanner_generation += 1;
        }
    }

    // ts_parser__external_scanner_destroy. The scanner is kept for the next parse in the
    // same language.
    fn external_scanner_destroy(&mut self) {
        if let Some(scanner) = self.external_scanner.take() {
            self.kept_scanner = Some(scanner);
            self.kept_language = self.language;
        }
    }

    // ts_parser__external_scanner_serialize
    fn external_scanner_serialize(&mut self) -> u32 {
        let scanner = self
            .external_scanner
            .as_ref()
            .expect("gositter: no scanner");
        scanner.serialize(&mut self.scanner_buf) as u32
    }

    // ts_parser__external_scanner_deserialize
    fn external_scanner_deserialize(&mut self, external_token: Subtree) {
        let pool = &self.tree.pool;
        let mut data: &[u8] = &[];
        if external_token != NULL_SUBTREE {
            let (off, n) = pool.ptr(external_token).ext();
            data = &pool.ext[off as usize..(off + n) as usize];
        }
        self.external_scanner
            .as_mut()
            .expect("gositter: no scanner")
            .deserialize(data);
    }

    // ts_parser__external_scanner_scan
    fn external_scanner_scan(&mut self, lexer: &mut Lexer<'_>, external_lex_state: u16) -> bool {
        let lang = self.lang();
        let (start, len) = lang.enabled_external_tokens(u32::from(external_lex_state));
        let states = lang.tables.external_scanner.states;
        self.valid_external_tokens.clear();
        self.valid_external_tokens
            .extend((start..start + len).map(|i| states.get(i)));
        self.external_scanner
            .as_mut()
            .expect("gositter: no scanner")
            .scan(lexer, &self.valid_external_tokens)
    }

    // ts_parser__can_reuse_first_leaf
    fn can_reuse_first_leaf(
        &self,
        state: StateId,
        tree: Subtree,
        action_count: u32,
        is_reusable: bool,
    ) -> bool {
        let sp = &self.tree.pool;
        let lang = self.lang();
        let leaf_symbol = sp.leaf_symbol(tree);
        let leaf_state = sp.leaf_parse_state(tree);
        let current_lex_mode = lang.lex_mode_for_state(state);
        let leaf_lex_mode = lang.lex_mode_for_state(leaf_state);

        // At the end of a non-terminal extra node the lexer returns NULL. Don't reuse
        // tokens in this situation.
        if current_lex_mode.lex_state == 0xFFFF {
            return false;
        }

        // If the token was created in a state with the same set of lookaheads, it is
        // reusable.
        if action_count > 0
            && leaf_lex_mode == current_lex_mode
            && (leaf_symbol != lang.tables.keyword_capture_token
                || (!sp.is_keyword(tree) && sp.parse_state(tree) == state))
        {
            return true;
        }

        // Empty tokens are not reusable in states with different lookaheads.
        if sp.size(tree).bytes == 0 && leaf_symbol != SYMBOL_END {
            return false;
        }

        // If the current state allows external tokens or other tokens that conflict with
        // this token, this token is not reusable.
        current_lex_mode.external_lex_state == 0 && is_reusable
    }

    // ts_parser__lex
    fn lex(
        &mut self,
        lexer: &mut Lexer<'_>,
        version: StackVersion,
        parse_state: StateId,
    ) -> Subtree {
        let lang = self.lang();

        let mut lex_mode = lang.lex_mode_for_state(parse_state);
        if lex_mode.lex_state == 0xFFFF {
            if lexer.state.logger.is_some() {
                lb(lexer)
                    .s("no_lookahead_after_non_terminal_extra")
                    .log(lexer);
            }
            return NULL_SUBTREE;
        }

        let start_position = self.stack.position(version);
        let external_token = self.stack.last_external_token(version);

        let mut found_external_token = false;
        let mut error_mode = parse_state == ERROR_STATE;
        let mut skipped_error = false;
        let mut called_get_column = false;
        let mut first_error_character = 0i32;
        let mut error_start_position = Length::default();
        let mut error_end_position = Length::default();
        let mut lookahead_end_byte = 0u32;
        let mut external_scanner_state_len = 0u32;
        let mut external_scanner_state_changed = false;
        lexer.reset(start_position);

        loop {
            let current_position = lexer.state.current_position;
            let column_data = lexer.state.column_data;

            if lex_mode.external_lex_state != 0 {
                if lexer.state.logger.is_some() {
                    lb(lexer)
                        .s("lex_external state:")
                        .u(u32::from(lex_mode.external_lex_state))
                        .s(", row:")
                        .u(current_position.extent.row)
                        .s(", column:")
                        .u(current_position.extent.column)
                        .log(lexer);
                }
                lexer.start();
                self.external_scanner_deserialize(external_token);
                let mut found_token =
                    self.external_scanner_scan(lexer, lex_mode.external_lex_state);
                lexer.finish(&mut lookahead_end_byte);

                if found_token {
                    external_scanner_state_len = self.external_scanner_serialize();
                    external_scanner_state_changed =
                        *self.tree.pool.external_scanner_state(external_token)
                            != self.scanner_buf[..external_scanner_state_len as usize];

                    // When recovering from an error, ignore any zero-length external tokens
                    // unless they have changed the external scanner's state. This avoids
                    // infinite loops, because the lexer is looking for any possible token
                    // here.
                    if lexer.state.token_end_position.bytes <= current_position.bytes
                        && (error_mode
                            || !self
                                .stack
                                .has_advanced_since_error(&self.tree.pool, version))
                        && !external_scanner_state_changed
                    {
                        if lexer.state.logger.is_some() {
                            let symbol = lang
                                .tables
                                .external_scanner
                                .symbol_map
                                .get(lexer.result_symbol as usize);
                            lb(lexer)
                                .s("ignore_empty_external_token symbol:")
                                .s(self.sym_name(symbol))
                                .log(lexer);
                        }
                        found_token = false;
                    }
                }

                if found_token {
                    found_external_token = true;
                    called_get_column = lexer.state.did_get_column;
                    break;
                }

                lexer.reset(current_position);
                lexer.state.column_data = column_data;
            }

            if lexer.state.logger.is_some() {
                lb(lexer)
                    .s("lex_internal state:")
                    .u(u32::from(lex_mode.lex_state))
                    .s(", row:")
                    .u(current_position.extent.row)
                    .s(", column:")
                    .u(current_position.extent.column)
                    .log(lexer);
            }
            lexer.start();
            let found_token = (lang.tables.lex_fn)(lexer, lex_mode.lex_state);
            lexer.finish(&mut lookahead_end_byte);
            if found_token {
                break;
            }

            if !error_mode {
                error_mode = true;
                lex_mode = lang.lex_mode_for_state(ERROR_STATE);
                lexer.reset(start_position);
                continue;
            }

            if !skipped_error {
                if lexer.state.logger.is_some() {
                    lb(lexer).s("skip_unrecognized_character").log(lexer);
                }
                skipped_error = true;
                error_start_position = lexer.state.token_start_position;
                error_end_position = lexer.state.token_start_position;
                first_error_character = lexer.lookahead;
            }

            if lexer.state.current_position.bytes == error_end_position.bytes {
                if lexer.eof() {
                    lexer.result_symbol = SYMBOL_ERROR;
                    break;
                }
                lexer.advance(false);
            }

            error_end_position = lexer.state.current_position;
        }

        let result = if skipped_error {
            let padding = length_sub(error_start_position, start_position);
            let size = length_sub(error_end_position, error_start_position);
            let lookahead_bytes = lookahead_end_byte.wrapping_sub(error_end_position.bytes);
            self.tree.pool.new_error(
                first_error_character,
                padding,
                size,
                lookahead_bytes,
                parse_state,
                lang,
            )
        } else {
            let mut is_keyword = false;
            let mut symbol = lexer.result_symbol;
            let padding = length_sub(lexer.state.token_start_position, start_position);
            let size = length_sub(
                lexer.state.token_end_position,
                lexer.state.token_start_position,
            );
            let lookahead_bytes =
                lookahead_end_byte.wrapping_sub(lexer.state.token_end_position.bytes);

            if found_external_token {
                symbol = lang.tables.external_scanner.symbol_map.get(symbol as usize);
            } else if symbol == lang.tables.keyword_capture_token && symbol != 0 {
                let end_byte = lexer.state.token_end_position.bytes;
                let token_start = lexer.state.token_start_position;
                lexer.reset(token_start);
                lexer.start();

                is_keyword = lang.tables.keyword_lex_fn.is_some_and(|f| f(lexer, 0));

                if is_keyword
                    && lexer.state.token_end_position.bytes == end_byte
                    && (lang.has_actions(parse_state, lexer.result_symbol)
                        || lang.is_reserved_word(parse_state, lexer.result_symbol))
                {
                    symbol = lexer.result_symbol;
                }
            }

            let result = self.tree.pool.new_leaf(
                symbol,
                padding,
                size,
                lookahead_bytes,
                parse_state,
                found_external_token,
                called_get_column,
                is_keyword,
                lang,
            );

            if found_external_token {
                // ts_external_scanner_state_init
                let n = external_scanner_state_len as usize;
                let state = &self.scanner_buf[..n];
                let sp = &mut self.tree.pool;
                if !external_scanner_state_changed
                    && !sp.external_scanner_state(external_token).is_empty()
                {
                    // Same bytes as the previous token's state: share them.
                    let (off, len) = sp.ptr(external_token).ext();
                    sp.ptr_mut(result).set_ext(off, len);
                } else {
                    if sp.ext.len() + n > u32::MAX as usize {
                        panic!("{ERR_SLAB_FULL}");
                    }
                    let off = sp.ext.len() as u32;
                    sp.ptr_mut(result).set_ext(off, n as u32);
                    sp.ext.extend_from_slice(state);
                }
                // In C the state shares a union with the node fields, and
                // ts_subtree_repeat_depth reads repeat_depth from it unchecked: bytes 16
                // and 17 of a state that is stored inline (at most 24 bytes). A longer state
                // is a pointer and leaves them zero.
                let h = sp.ptr_mut(result);
                if (17..=24).contains(&n) {
                    h.repeat_depth = u16::from(state[16]);
                    if n > 17 {
                        h.repeat_depth |= u16::from(state[17]) << 8;
                    }
                }
                h.set_flag(
                    F_HAS_EXTERNAL_SCANNER_STATE_CHANGE,
                    external_scanner_state_changed,
                );
            }
            result
        };

        if lexer.state.logger.is_some() {
            let name = self.tree_name(result);
            let size = self.tree.pool.total_size(result).bytes;
            log_lookahead(lexer, name, size);
        }
        result
    }

    // ts_parser__get_cached_token: (token, index of the first action, action count,
    // is_reusable), or NULL_SUBTREE.
    fn get_cached_token(
        &mut self,
        state: StateId,
        position: u32,
        last_external_token: Subtree,
    ) -> (Subtree, usize, u8, bool) {
        let cache = self.token_cache;
        if cache.token != NULL_SUBTREE
            && cache.byte_index == position
            && self
                .tree
                .pool
                .external_scanner_state_eq(cache.last_external_token, last_external_token)
        {
            let (first, count, is_reusable) = self
                .lang()
                .table_entry(state, self.tree.pool.symbol(cache.token));
            if self.can_reuse_first_leaf(state, cache.token, u32::from(count), is_reusable) {
                self.tree.pool.retain(cache.token);
                return (cache.token, first, count, is_reusable);
            }
            // C leaves the table entry filled in here. Every path that follows a miss
            // overwrites it before use.
        }
        (NULL_SUBTREE, 0, 0, false)
    }

    // ts_parser__set_cached_token
    fn set_cached_token(&mut self, byte_index: u32, last_external_token: Subtree, token: Subtree) {
        let sp = &mut self.tree.pool;
        let cache = &mut self.token_cache;
        if token != NULL_SUBTREE {
            sp.retain(token);
        }
        if last_external_token != NULL_SUBTREE {
            sp.retain(last_external_token);
        }
        if cache.token != NULL_SUBTREE {
            sp.release(cache.token);
        }
        if cache.last_external_token != NULL_SUBTREE {
            sp.release(cache.last_external_token);
        }
        cache.token = token;
        cache.byte_index = byte_index;
        cache.last_external_token = last_external_token;
    }

    // ts_parser__select_tree. Reports whether right should replace left.
    fn select_tree(&mut self, lexer: &mut Lexer<'_>, left: Subtree, right: Subtree) -> bool {
        if left == NULL_SUBTREE {
            return true;
        }
        if right == NULL_SUBTREE {
            return false;
        }

        let sp = &self.tree.pool;
        if sp.error_cost(right) < sp.error_cost(left) {
            if lexer.state.logger.is_some() {
                lb(lexer)
                    .s("select_smaller_error symbol:")
                    .s(self.tree_name(right))
                    .s(", over_symbol:")
                    .s(self.tree_name(left))
                    .log(lexer);
            }
            return true;
        }

        if sp.error_cost(left) < sp.error_cost(right) {
            if lexer.state.logger.is_some() {
                lb(lexer)
                    .s("select_smaller_error symbol:")
                    .s(self.tree_name(left))
                    .s(", over_symbol:")
                    .s(self.tree_name(right))
                    .log(lexer);
            }
            return false;
        }

        if sp.dynamic_precedence(right) > sp.dynamic_precedence(left) {
            if lexer.state.logger.is_some() {
                lb(lexer)
                    .s("select_higher_precedence symbol:")
                    .s(self.tree_name(right))
                    .s(", prec:")
                    .d(sp.dynamic_precedence(right))
                    .s(", over_symbol:")
                    .s(self.tree_name(left))
                    .s(", other_prec:")
                    .d(sp.dynamic_precedence(left))
                    .log(lexer);
            }
            return true;
        }

        if sp.dynamic_precedence(left) > sp.dynamic_precedence(right) {
            if lexer.state.logger.is_some() {
                lb(lexer)
                    .s("select_higher_precedence symbol:")
                    .s(self.tree_name(left))
                    .s(", prec:")
                    .d(sp.dynamic_precedence(left))
                    .s(", over_symbol:")
                    .s(self.tree_name(right))
                    .s(", other_prec:")
                    .d(sp.dynamic_precedence(right))
                    .log(lexer);
            }
            return false;
        }

        if sp.error_cost(left) > 0 {
            return true;
        }

        match self.tree.pool.compare(left, right) {
            -1 => {
                if lexer.state.logger.is_some() {
                    lb(lexer)
                        .s("select_earlier symbol:")
                        .s(self.tree_name(left))
                        .s(", over_symbol:")
                        .s(self.tree_name(right))
                        .log(lexer);
                }
                false
            }
            1 => {
                if lexer.state.logger.is_some() {
                    lb(lexer)
                        .s("select_earlier symbol:")
                        .s(self.tree_name(right))
                        .s(", over_symbol:")
                        .s(self.tree_name(left))
                        .log(lexer);
                }
                true
            }
            _ => {
                if lexer.state.logger.is_some() {
                    lb(lexer)
                        .s("select_existing symbol:")
                        .s(self.tree_name(left))
                        .s(", over_symbol:")
                        .s(self.tree_name(right))
                        .log(lexer);
                }
                false
            }
        }
    }

    // ts_parser__select_children. C builds the temporary node in the reused scratch_trees
    // array; here it sits at the end of the slabs and is dropped again. It owns no
    // reference to its children.
    fn select_children(
        &mut self,
        lexer: &mut Lexer<'_>,
        left: Subtree,
        children: SubtreeArray,
    ) -> bool {
        let lang = self.lang();
        let sp = &mut self.tree.pool;
        let mark = sp.children.len();
        let mut scratch = SubtreeArray {
            off: 0,
            size: children.size,
            cap: children.size,
        };
        let room = sp.room(children.size);
        scratch.off = sp.bump_children(room, children.size);
        let from = children.off as usize;
        sp.children
            .copy_within(from..from + children.size as usize, scratch.off as usize);

        let symbol = sp.symbol(left);
        let scratch_tree = sp.new_node(symbol, &mut scratch, 0, lang);
        let result = self.select_tree(lexer, left, scratch_tree);

        let sp = &mut self.tree.pool;
        sp.free((scratch_tree.0 >> 1) as u32);
        sp.children.truncate(mark);
        result
    }

    // ts_parser__shift
    fn shift(&mut self, version: StackVersion, state: StateId, lookahead: Subtree, extra: bool) {
        let sp = &mut self.tree.pool;
        let is_leaf = sp.child_count(lookahead) == 0;
        let mut subtree_to_push = lookahead;
        if extra != sp.extra(lookahead) && is_leaf {
            let mut result = sp.make_mut(lookahead);
            sp.set_extra(&mut result, extra);
            subtree_to_push = result;
        }

        self.stack
            .push(sp, version, subtree_to_push, !is_leaf, state);
        if sp.has_external_tokens(subtree_to_push) {
            let token = sp.last_external_token(subtree_to_push);
            self.stack.set_last_external_token(sp, version, token);
        }
    }

    // ts_parser__reduce
    #[allow(clippy::too_many_arguments)]
    fn reduce(
        &mut self,
        lexer: &mut Lexer<'_>,
        version: StackVersion,
        symbol: Symbol,
        count: u32,
        dynamic_precedence: i32,
        production_id: u16,
        is_fragile: bool,
        end_of_non_terminal_extra: bool,
    ) -> StackVersion {
        let lang = self.lang();
        let initial_version_count = self.stack.version_count();

        // Pop the given number of nodes from the given version of the parse stack. If
        // stack versions have previously merged, there may be more than one path back
        // through the stack. For each path, create a new parent node and push it in place
        // of the children. The slices are taken out of the stack until the end: nothing
        // below walks the stack.
        self.stack.pop_count(&mut self.tree.pool, version, count);
        let pop = std::mem::take(&mut self.stack.slices);
        let mut removed_version_count = 0u32;
        let mut i = 0usize;
        while i < pop.len() {
            let mut slice = pop[i];
            let slice_version = slice.version.wrapping_sub(removed_version_count);

            // The version count may be exceeded temporarily, but only by a limited
            // threshold.
            if slice_version > MAX_VERSION_COUNT + MAX_VERSION_COUNT_OVERFLOW {
                self.stack
                    .remove_version(&mut self.tree.pool, slice_version);
                self.tree.pool.array_delete(&mut slice.subtrees);
                removed_version_count += 1;
                while i + 1 < pop.len() {
                    let mut next_slice = pop[i + 1];
                    if next_slice.version != slice.version {
                        break;
                    }
                    self.tree.pool.array_delete(&mut next_slice.subtrees);
                    i += 1;
                }
                i += 1;
                continue;
            }

            // Extra tokens on top of the stack are not included in the new parent. They are
            // re-pushed after it.
            let mut children = slice.subtrees;
            self.tree
                .pool
                .array_remove_trailing_extras(&mut children, &mut self.trailing_extras);

            let mut parent =
                self.tree
                    .pool
                    .new_node(symbol, &mut children, u32::from(production_id), lang);

            // This pop may have caused multiple stack versions to collapse into one. Choose
            // one of the arrays of trees to be the parent node's children, and delete the
            // rest.
            while i + 1 < pop.len() {
                let mut next_slice = pop[i + 1];
                if next_slice.version != slice.version {
                    break;
                }
                i += 1;

                let mut next_slice_children = next_slice.subtrees;
                self.tree.pool.array_remove_trailing_extras(
                    &mut next_slice_children,
                    &mut self.trailing_extras2,
                );

                if self.select_children(lexer, parent, next_slice_children) {
                    for &t in &self.trailing_extras {
                        self.tree.pool.release(t);
                    }
                    self.trailing_extras.clear();
                    self.tree.pool.release(parent);
                    std::mem::swap(&mut self.trailing_extras, &mut self.trailing_extras2);
                    parent = self.tree.pool.new_node(
                        symbol,
                        &mut next_slice_children,
                        u32::from(production_id),
                        lang,
                    );
                } else {
                    self.trailing_extras2.clear();
                    self.tree.pool.array_delete(&mut next_slice.subtrees);
                }
            }

            let state = self.stack.state(slice_version);
            let next_state = lang.next_state(state, symbol);
            let h = self.tree.pool.ptr_mut(parent);
            if end_of_non_terminal_extra && next_state == state {
                h.flags |= F_EXTRA;
            }
            if is_fragile || pop.len() > 1 || initial_version_count > 1 {
                h.flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
                h.parse_state = TREE_STATE_NONE;
            } else {
                h.parse_state = state;
            }
            h.dynamic_precedence = h.dynamic_precedence.wrapping_add(dynamic_precedence);

            // Push the parent node onto the stack, along with any extra tokens that were
            // previously on top of the stack.
            self.stack
                .push(&self.tree.pool, slice_version, parent, false, next_state);
            for &t in &self.trailing_extras {
                self.stack
                    .push(&self.tree.pool, slice_version, t, false, next_state);
            }

            for j in 0..slice_version {
                if j == version {
                    continue;
                }
                if self.stack.merge(&mut self.tree.pool, j, slice_version) {
                    removed_version_count += 1;
                    break;
                }
            }
            i += 1;
        }
        self.stack.slices = pop;

        // Return the first new stack version that was created.
        if self.stack.version_count() > initial_version_count {
            return initial_version_count;
        }
        STACK_VERSION_NONE
    }

    // ts_parser__reduce followed by the ts_stack_renumber_version of ts_parser__advance, for
    // the last action of a table entry that does anything (see only_repetitions) when this
    // version owns an unbranched chain and the reduced version could not merge with another
    // one. The general path would pop one slice into a new version, fail every
    // ts_stack_merge, and move that version back over this one. Reports false and changes
    // nothing if the stack is not of that shape.
    #[allow(clippy::too_many_arguments)]
    fn reduce_in_place(
        &mut self,
        version: StackVersion,
        symbol: Symbol,
        count: u32,
        dynamic_precedence: i32,
        production_id: u16,
        is_fragile: bool,
        end_of_non_terminal_extra: bool,
    ) -> bool {
        let lang = self.lang();
        let version_count = self.stack.version_count();
        if version_count > MAX_VERSION_COUNT + MAX_VERSION_COUNT_OVERFLOW
            || self.stack.heads[version as usize].lookahead_when_paused != NULL_SUBTREE
        {
            return false;
        }
        let Some((mut children, trailing, below)) =
            self.stack.pop_linear(&mut self.tree.pool, version, count)
        else {
            return false;
        };
        let state = self.stack.nodes[below as usize].state;
        let next_state = lang.next_state(state, symbol);

        // ts_stack_can_merge needs the state and byte position of the new head. Both are
        // known now: popping and pushing the same subtrees leaves the position where it is.
        if version_count > 1 {
            let s = &self.stack;
            let position = s.nodes[s.heads[version as usize].node as usize]
                .position
                .bytes;
            let can_merge = s.heads.iter().enumerate().any(|(j, head)| {
                let node = &s.nodes[head.node as usize];
                j as u32 != version && node.state == next_state && node.position.bytes == position
            });
            if can_merge {
                self.stack.undo_pop_linear(&mut self.tree.pool, children);
                return false;
            }
        }
        self.stack.heads[version as usize].node = below;

        // ts_subtree_array_remove_trailing_extras: they are the last trailing subtrees of
        // the slice.
        children.size -= trailing;
        let start = (children.off + children.size) as usize;
        self.trailing_extras.clear();
        self.trailing_extras
            .extend_from_slice(&self.tree.pool.children[start..start + trailing as usize]);

        let sp = &mut self.tree.pool;
        if sp.quick_free.wrapping_add(sp.merged_free) > (sp.children.len() as u32) / 4
            && children.size > 0
        {
            sp.settle_children(&mut children);
        }
        let parent = sp.new_node(symbol, &mut children, u32::from(production_id), lang);

        let h = sp.ptr_mut(parent);
        if end_of_non_terminal_extra && next_state == state {
            h.flags |= F_EXTRA;
        }
        if is_fragile || version_count > 1 {
            h.flags |= F_FRAGILE_LEFT | F_FRAGILE_RIGHT;
            h.parse_state = TREE_STATE_NONE;
        } else {
            h.parse_state = state;
        }
        h.dynamic_precedence = h.dynamic_precedence.wrapping_add(dynamic_precedence);

        self.stack.push(sp, version, parent, false, next_state);
        for &t in &self.trailing_extras {
            self.stack.push(sp, version, t, false, next_state);
        }
        true
    }

    // ts_parser__accept
    fn accept(&mut self, lexer: &mut Lexer<'_>, version: StackVersion, lookahead: Subtree) {
        let lang = self.lang();
        self.stack
            .push(&self.tree.pool, version, lookahead, false, 1);

        self.stack.pop_all(&mut self.tree.pool, version);
        // pop_all_subtrees reads the forks of the walk, not the slices.
        let pop = std::mem::take(&mut self.stack.slices);
        for &slice in &pop {
            let mut trees = self
                .stack
                .pop_all_subtrees(&mut self.tree.pool, version, slice);

            let mut root = NULL_SUBTREE;
            for j in (0..trees.size).rev() {
                let sp = &mut self.tree.pool;
                let tree = sp.children[(trees.off + j) as usize];
                if !sp.extra(tree) {
                    let (off, n) = if tree.is_inline() {
                        (0, 0)
                    } else {
                        let h = sp.ptr(tree);
                        (h.children, h.child_count)
                    };
                    for k in off..off + n {
                        let child = sp.children[k as usize];
                        sp.retain(child);
                    }
                    sp.array_splice(&mut trees, j, 1, off, n);
                    let symbol = sp.symbol(tree);
                    let production_id = u32::from(sp.ptr(tree).production_id);
                    root = sp.new_node(symbol, &mut trees, production_id, lang);
                    sp.release(tree);
                    break;
                }
            }

            self.accept_count += 1;

            if self.finished_tree != NULL_SUBTREE {
                if self.select_tree(lexer, self.finished_tree, root) {
                    self.tree.pool.release(self.finished_tree);
                    self.finished_tree = root;
                } else {
                    self.tree.pool.release(root);
                }
            } else {
                self.finished_tree = root;
            }
        }

        let first = pop[0].version;
        self.stack.slices = pop;
        self.stack.remove_version(&mut self.tree.pool, first);
        self.stack.halt(version);
    }

    // ts_parser__do_all_potential_reductions
    fn do_all_potential_reductions(
        &mut self,
        lexer: &mut Lexer<'_>,
        starting_version: StackVersion,
        lookahead_symbol: Symbol,
    ) -> bool {
        let initial_version_count = self.stack.version_count();

        let mut can_shift_lookahead_symbol = false;
        let mut version = starting_version;
        for i in 0u32.. {
            let version_count = self.stack.version_count();
            if version >= version_count {
                break;
            }

            let mut merged = false;
            for j in initial_version_count..version {
                if self.stack.merge(&mut self.tree.pool, j, version) {
                    merged = true;
                    break;
                }
            }
            if merged {
                continue;
            }

            let state = self.stack.state(version);
            let has_shift_action = if lookahead_symbol != 0 {
                // TSSymbol arithmetic: wraps at 0xFFFF
                self.potential_reductions(
                    state,
                    u32::from(lookahead_symbol),
                    u32::from(lookahead_symbol.wrapping_add(1)),
                )
            } else {
                // Every token of the state: the answer depends on the state alone, and
                // error handling asks again at each error.
                let s = state as usize;
                if self.reduce_scans.len() <= s {
                    self.reduce_scans.resize_with(s + 1, || None);
                }
                if self.reduce_scans[s].is_none() {
                    let token_count = u32::from(self.lang().tables.token_count as u16);
                    let has = self.potential_reductions(state, 1, token_count);
                    self.reduce_scans[s] = Some(ReduceScan {
                        reduce_actions: self.reduce_actions.clone(),
                        has_shift_action: has,
                    });
                }
                self.reduce_scans[s].as_ref().unwrap().has_shift_action
            };

            // The actions are read by index: reduce changes neither reduce_actions nor
            // reduce_scans.
            let action_count = if lookahead_symbol != 0 {
                self.reduce_actions.len()
            } else {
                self.reduce_scans[state as usize]
                    .as_ref()
                    .unwrap()
                    .reduce_actions
                    .len()
            };
            let mut reduction_version = STACK_VERSION_NONE;
            let mut j = 0;
            while j < action_count {
                let action = if lookahead_symbol != 0 {
                    self.reduce_actions[j]
                } else {
                    self.reduce_scans[state as usize]
                        .as_ref()
                        .unwrap()
                        .reduce_actions[j]
                };

                reduction_version = self.reduce(
                    lexer,
                    version,
                    action.symbol,
                    action.count,
                    action.dynamic_precedence,
                    action.production_id,
                    true,
                    false,
                );
                j += 1;
            }

            if has_shift_action {
                can_shift_lookahead_symbol = true;
            } else if reduction_version != STACK_VERSION_NONE && i < MAX_VERSION_COUNT {
                self.stack
                    .renumber_version(&mut self.tree.pool, reduction_version, version);
                continue;
            } else if lookahead_symbol != 0 {
                self.stack.remove_version(&mut self.tree.pool, version);
            }

            if version == starting_version {
                version = version_count;
            } else {
                version += 1;
            }
        }

        can_shift_lookahead_symbol
    }

    // The symbol loop of ts_parser__do_all_potential_reductions: fills reduce_actions and
    // reports has_shift_action.
    fn potential_reductions(&mut self, state: StateId, first_symbol: u32, end_symbol: u32) -> bool {
        scan_reductions(
            self.lang(),
            state,
            first_symbol,
            end_symbol,
            &mut self.reduce_actions,
        )
    }

    // The tokens ts_parser__handle_error tries as a missing token in a state. Kept per
    // state, as reduce_scans is.
    fn missing_tokens(&mut self, state: StateId) -> &[MissingToken] {
        let lang = self.lang();
        let s = state as usize;
        if self.missing_scans.len() <= s {
            self.missing_scans.resize_with(s + 1, || None);
        }
        self.missing_scans[s].get_or_insert_with(|| scan_missing_tokens(lang, state))
    }

    /// ErrorScansOK: whether every scan kept for error handling equals a fresh scan of the
    /// table, and how many are kept. Only the current language's: set_language clears them.
    pub(crate) fn check_error_scans(&self) -> (usize, bool) {
        let Some(lang) = self.language else {
            return (0, true);
        };
        let token_count = u32::from(lang.tables.token_count as u16);
        let mut states = 0;
        let mut actions = Vec::new();
        for (state, scan) in self.reduce_scans.iter().enumerate() {
            let Some(scan) = scan else {
                continue;
            };
            let has = scan_reductions(lang, state as StateId, 1, token_count, &mut actions);
            if has != scan.has_shift_action || actions != scan.reduce_actions {
                return (states, false);
            }
            states += 1;
        }
        for (state, tokens) in self.missing_scans.iter().enumerate() {
            let Some(tokens) = tokens else {
                continue;
            };
            if scan_missing_tokens(lang, state as StateId) != *tokens {
                return (states, false);
            }
            states += 1;
        }
        (states, true)
    }

    // ts_parser__recover_to_state
    fn recover_to_state(&mut self, version: StackVersion, depth: u32, goal_state: StateId) -> bool {
        let lang = self.lang();
        // Copied out: pop_error below runs another stack walk into stack.slices. Go's pop
        // aliases it, and the walk writes slot 0 after it has been read.
        let pop = self
            .stack
            .pop_count(&mut self.tree.pool, version, depth)
            .to_vec();
        let mut previous_version = STACK_VERSION_NONE;

        for mut slice in pop {
            if slice.version == previous_version {
                self.tree.pool.array_delete(&mut slice.subtrees);
                continue;
            }

            if self.stack.state(slice.version) != goal_state {
                self.stack.halt(slice.version);
                self.tree.pool.array_delete(&mut slice.subtrees);
                continue;
            }

            // Not in ts_parser__recover_to_state: the summary of the ERROR node this one
            // replaces. summarize goes on from it, where C sums every child again at each
            // recovery.
            let mut resume = SubtreeHeap::default();
            let mut error_trees = self.stack.pop_error(&mut self.tree.pool, slice.version);
            if error_trees.size > 0 {
                let sp = &mut self.tree.pool;
                let error_tree = sp.children[error_trees.off as usize];
                let error_child_count = sp.child_count(error_tree);
                if error_child_count > 0 {
                    resume = *sp.ptr(error_tree);
                    let children = sp.ptr(error_tree).children;
                    sp.array_splice(&mut slice.subtrees, 0, 0, children, error_child_count);
                    for k in 0..error_child_count {
                        let t = sp.children[(slice.subtrees.off + k) as usize];
                        sp.retain(t);
                    }
                }
                sp.array_delete(&mut error_trees);
            }

            self.tree
                .pool
                .array_remove_trailing_extras(&mut slice.subtrees, &mut self.trailing_extras);

            if slice.subtrees.size > 0 {
                self.tree.pool.resume = resume;
                let error_node = self
                    .tree
                    .pool
                    .new_error_node(&mut slice.subtrees, true, lang);
                self.stack.push(
                    &self.tree.pool,
                    slice.version,
                    error_node,
                    false,
                    goal_state,
                );
            } else {
                self.tree.pool.array_free(&mut slice.subtrees);
            }

            for &tree in &self.trailing_extras {
                self.stack
                    .push(&self.tree.pool, slice.version, tree, false, goal_state);
            }

            previous_version = slice.version;
        }

        previous_version != STACK_VERSION_NONE
    }

    // ts_parser__recover
    fn recover(&mut self, lexer: &mut Lexer<'_>, version: StackVersion, lookahead: Subtree) {
        let lang = self.lang();
        let mut lookahead = lookahead;
        let mut did_recover = false;
        let previous_version_count = self.stack.version_count();
        let position = self.stack.position(version);
        // Copied out: recover_to_state changes the stack.
        let summary = self.stack.get_summary(version).map(|s| s.to_vec());
        let node_count_since_error = self.stack.node_count_since_error(version);
        let current_error_cost = self.stack.error_cost(version);

        // In the error state there are two strategies for recovering with a given lookahead
        // token:
        // 1. Find a previous state on the stack in which that lookahead token would be
        //    valid, pop back to it and wrap the popped subtrees in an ERROR node.
        // 2. Wrap the lookahead token in an ERROR node, push it and stay in the error
        //    state.
        //
        // First strategy 1, using the summary recorded on entering the error state.
        if let Some(summary) = &summary
            && !self.tree.pool.is_error(lookahead)
        {
            for entry in summary {
                if entry.state == ERROR_STATE {
                    continue;
                }
                if entry.position.bytes == position.bytes {
                    continue;
                }
                let mut depth = entry.depth;
                if node_count_since_error > 0 {
                    depth = depth.wrapping_add(1);
                }

                // Do not recover in ways that create redundant stack versions.
                let would_merge = (0..previous_version_count).any(|j| {
                    self.stack.state(j) == entry.state
                        && self.stack.position(j).bytes == position.bytes
                });
                if would_merge {
                    continue;
                }

                // Do not recover if the result would clearly be worse than some existing
                // stack version.
                let new_cost = current_error_cost
                    .wrapping_add(entry.depth.wrapping_mul(ERROR_COST_PER_SKIPPED_TREE))
                    .wrapping_add(
                        position
                            .bytes
                            .wrapping_sub(entry.position.bytes)
                            .wrapping_mul(ERROR_COST_PER_SKIPPED_CHAR),
                    )
                    .wrapping_add(
                        position
                            .extent
                            .row
                            .wrapping_sub(entry.position.extent.row)
                            .wrapping_mul(ERROR_COST_PER_SKIPPED_LINE),
                    );
                if self.better_version_exists(version, false, new_cost) {
                    break;
                }

                // If the current lookahead token is valid in some previous state, recover to
                // that state. Then stop looking.
                if lang.has_actions(entry.state, self.tree.pool.symbol(lookahead))
                    && self.recover_to_state(version, depth, entry.state)
                {
                    did_recover = true;
                    if lexer.state.logger.is_some() {
                        lb(lexer)
                            .s("recover_to_previous state:")
                            .u(u32::from(entry.state))
                            .s(", depth:")
                            .u(depth)
                            .log(lexer);
                    }
                    break;
                }
            }
        }

        // Some stack versions may have been created and then halted during the attempt.
        // Remove those versions.
        let mut i = previous_version_count;
        while i < self.stack.version_count() {
            if !self.stack.is_active(i) {
                self.stack.remove_version(&mut self.tree.pool, i);
            } else {
                i += 1;
            }
        }

        // If strategy 1 succeeded, a new stack version can handle the current lookahead
        // token. In addition, try strategy 2, unless there are already too many stack
        // versions.
        if did_recover && self.stack.version_count() > MAX_VERSION_COUNT {
            self.stack.halt(version);
            self.tree.pool.release(lookahead);
            return;
        }

        if did_recover && self.tree.pool.has_external_scanner_state_change(lookahead) {
            self.stack.halt(version);
            self.tree.pool.release(lookahead);
            return;
        }

        // If the parser is still in the error state at the end of the file, just wrap
        // everything in an ERROR node and terminate.
        if self.tree.pool.is_eof(lookahead) {
            if lexer.state.logger.is_some() {
                lb(lexer).s("recover_eof").log(lexer);
            }
            let mut children = SubtreeArray::default();
            let parent = self.tree.pool.new_error_node(&mut children, false, lang);
            self.stack.push(&self.tree.pool, version, parent, false, 1);
            self.accept(lexer, version, lookahead);
            return;
        }

        // Do not recover if the result would clearly be worse than some existing stack
        // version.
        let new_cost = current_error_cost
            .wrapping_add(ERROR_COST_PER_SKIPPED_TREE)
            .wrapping_add(
                self.tree
                    .pool
                    .total_bytes(lookahead)
                    .wrapping_mul(ERROR_COST_PER_SKIPPED_CHAR),
            )
            .wrapping_add(
                self.tree
                    .pool
                    .total_size(lookahead)
                    .extent
                    .row
                    .wrapping_mul(ERROR_COST_PER_SKIPPED_LINE),
            );
        if self.better_version_exists(version, false, new_cost) {
            self.stack.halt(version);
            self.tree.pool.release(lookahead);
            return;
        }

        // If the current lookahead token is an extra token, mark it as extra, so it is not
        // counted in error cost calculations.
        let (first, count, _) = lang.table_entry(1, self.tree.pool.symbol(lookahead));
        if count > 0 {
            let action = lang.tables.parse_actions.get(first + count as usize - 1);
            if action.action_type == PARSE_ACTION_TYPE_SHIFT && action.extra() {
                let sp = &mut self.tree.pool;
                let mut mutable_lookahead = sp.make_mut(lookahead);
                sp.set_extra(&mut mutable_lookahead, true);
                lookahead = mutable_lookahead;
            }
        }

        // Wrap the lookahead token in an ERROR.
        if lexer.state.logger.is_some() {
            lb(lexer)
                .s("skip_token symbol:")
                .s(self.tree_name(lookahead))
                .log(lexer);
        }
        let sp = &mut self.tree.pool;
        let mut children = SubtreeArray::default();
        sp.array_reserve(&mut children, 1);
        sp.array_push(&mut children, lookahead);
        let mut error_repeat = sp.new_node(SYMBOL_ERROR_REPEAT, &mut children, 0, lang);

        // If other tokens have already been skipped, there is already an ERROR at the top
        // of the stack. Pop it and wrap the two together.
        if node_count_since_error > 0 {
            self.stack.pop_count(&mut self.tree.pool, version, 1);
            // Taken out of the stack until the end: nothing below walks it.
            let mut pop = std::mem::take(&mut self.stack.slices);

            // If multiple stack versions have merged at this point, just pick one of the
            // errors arbitrarily and discard the rest.
            if pop.len() > 1 {
                for slice in &mut pop[1..] {
                    self.tree.pool.array_delete(&mut slice.subtrees);
                }
                while self.stack.version_count() > pop[0].version + 1 {
                    self.stack
                        .remove_version(&mut self.tree.pool, pop[0].version + 1);
                }
            }

            self.stack
                .renumber_version(&mut self.tree.pool, pop[0].version, version);
            let sp = &mut self.tree.pool;
            sp.array_push(&mut pop[0].subtrees, error_repeat);
            error_repeat = sp.new_node(SYMBOL_ERROR_REPEAT, &mut pop[0].subtrees, 0, lang);
            self.stack.slices = pop;
        }

        // Push the new ERROR onto the stack.
        self.stack
            .push(&self.tree.pool, version, error_repeat, false, ERROR_STATE);
        if self.tree.pool.has_external_tokens(lookahead) {
            let token = self.tree.pool.last_external_token(lookahead);
            self.stack
                .set_last_external_token(&mut self.tree.pool, version, token);
        }

        let mut has_error = true;
        for i in 0..self.stack.version_count() {
            let status = self.version_status(i);
            if !status.is_in_error {
                has_error = false;
                break;
            }
        }
        self.has_error = has_error;
    }

    // ts_parser__handle_error
    fn handle_error(&mut self, lexer: &mut Lexer<'_>, version: StackVersion, lookahead: Subtree) {
        let lang = self.lang();
        let previous_version_count = self.stack.version_count();

        // Perform any reductions that can happen in this state, regardless of the
        // lookahead.
        self.do_all_potential_reductions(lexer, version, 0);
        let version_count = self.stack.version_count();
        let position = self.stack.position(version);

        // Push a discontinuity onto the stack. Merge all of the stack versions that were
        // created in the previous step.
        let mut did_insert_missing_token = false;
        let mut v = version;
        while v < version_count {
            if !did_insert_missing_token {
                let state = self.stack.state(v);
                self.missing_tokens(state);
                // Taken out while the loop below mutates the parser: nothing in it reads
                // missing_scans.
                let tokens = self.missing_scans[state as usize]
                    .take()
                    .unwrap_or_default();
                for &missing in &tokens {
                    let missing_symbol = missing.symbol;
                    let state_after_missing_symbol = missing.state_after;
                    let leaf_symbol = self.tree.pool.leaf_symbol(lookahead);

                    if lang.has_reduce_action(state_after_missing_symbol, leaf_symbol) {
                        // The lexer snaps to the next included range, and the missing token's
                        // padding places it there. With one range the padding is zero.
                        lexer.reset(position);
                        lexer.mark_end();
                        let padding = length_sub(lexer.state.token_end_position, position);
                        let sp = &mut self.tree.pool;
                        let lookahead_bytes = sp
                            .total_bytes(lookahead)
                            .wrapping_add(sp.lookahead_bytes(lookahead));

                        let version_with_missing_tree = self.stack.copy_version(sp, v);
                        let missing_tree =
                            sp.new_missing_leaf(missing_symbol, padding, lookahead_bytes, lang);
                        self.stack.push(
                            sp,
                            version_with_missing_tree,
                            missing_tree,
                            false,
                            state_after_missing_symbol,
                        );

                        if self.do_all_potential_reductions(
                            lexer,
                            version_with_missing_tree,
                            leaf_symbol,
                        ) {
                            if lexer.state.logger.is_some() {
                                lb(lexer)
                                    .s("recover_with_missing symbol:")
                                    .s(self.sym_name(missing_symbol))
                                    .s(", state:")
                                    .u(u32::from(self.stack.state(version_with_missing_tree)))
                                    .log(lexer);
                            }
                            did_insert_missing_token = true;
                            break;
                        }
                    }
                }
                self.missing_scans[state as usize] = Some(tokens);
            }

            self.stack
                .push(&self.tree.pool, v, NULL_SUBTREE, false, ERROR_STATE);
            if v == version {
                v = previous_version_count;
            } else {
                v += 1;
            }
        }

        for _ in previous_version_count..version_count {
            self.stack
                .merge(&mut self.tree.pool, version, previous_version_count);
        }

        self.stack
            .record_summary(&mut self.tree.pool, version, MAX_SUMMARY_DEPTH);

        // Begin recovery with the current lookahead node, rather than waiting for the next
        // turn of the parse loop.
        self.recover(lexer, version, lookahead);
    }

    // ts_parser__advance
    fn advance(&mut self, lexer: &mut Lexer<'_>, version: StackVersion) -> bool {
        let lang = self.lang();
        let mut state = self.stack.state(version);
        let position = self.stack.position(version).bytes;
        let last_external_token = self.stack.last_external_token(version);

        // No old tree, so nothing to reuse but the token last returned by the lexer.
        let (mut lookahead, mut first, mut count, _) =
            self.get_cached_token(state, position, last_external_token);

        let mut needs_lex = lookahead == NULL_SUBTREE;
        loop {
            // Otherwise, re-run the lexer.
            if needs_lex {
                needs_lex = false;
                lookahead = self.lex(lexer, version, state);

                if lookahead != NULL_SUBTREE {
                    self.set_cached_token(position, last_external_token, lookahead);
                    (first, count, _) = lang.table_entry(state, self.tree.pool.symbol(lookahead));
                } else {
                    // When parsing a non-terminal extra, a null lookahead indicates the end
                    // of the rule. The reduction is stored in the EOF table entry.
                    (first, count, _) = lang.table_entry(state, SYMBOL_END);
                }
            }

            // Process each parse action for the current lookahead token in the current
            // state. REDUCE actions always create a new stack version, whereas SHIFT
            // actions update the existing stack version and terminate this loop.
            let n = count as usize;
            let mut last_reduction_version = STACK_VERSION_NONE;
            for i in 0..n {
                let action = lang.tables.parse_actions.get(first + i);

                match action.action_type {
                    PARSE_ACTION_TYPE_SHIFT => {
                        if action.repetition() {
                            continue;
                        }
                        let next_state = if action.extra() {
                            if lexer.state.logger.is_some() {
                                lb(lexer).s("shift_extra").log(lexer);
                            }
                            state
                        } else {
                            let next_state = action.state();
                            if lexer.state.logger.is_some() {
                                lb(lexer)
                                    .s("shift state:")
                                    .u(u32::from(next_state))
                                    .log(lexer);
                            }
                            next_state
                        };

                        self.shift(version, next_state, lookahead, action.extra());
                        return true;
                    }

                    PARSE_ACTION_TYPE_REDUCE => {
                        let is_fragile = n > 1;
                        let end_of_non_terminal_extra = lookahead == NULL_SUBTREE;
                        if lexer.state.logger.is_some() {
                            lb(lexer)
                                .s("reduce sym:")
                                .s(self.sym_name(action.symbol))
                                .s(", child_count:")
                                .u(u32::from(action.child_count))
                                .log(lexer);
                        }
                        if (i == n - 1 || only_repetitions(lang, first + i + 1, first + n))
                            && self.reduce_in_place(
                                version,
                                action.symbol,
                                u32::from(action.child_count),
                                i32::from(action.dynamic_precedence),
                                action.production_id,
                                is_fragile,
                                end_of_non_terminal_extra,
                            )
                        {
                            last_reduction_version = version;
                            continue;
                        }
                        let reduction_version = self.reduce(
                            lexer,
                            version,
                            action.symbol,
                            u32::from(action.child_count),
                            i32::from(action.dynamic_precedence),
                            action.production_id,
                            is_fragile,
                            end_of_non_terminal_extra,
                        );
                        if reduction_version != STACK_VERSION_NONE {
                            last_reduction_version = reduction_version;
                        }
                    }

                    PARSE_ACTION_TYPE_ACCEPT => {
                        if lexer.state.logger.is_some() {
                            lb(lexer).s("accept").log(lexer);
                        }
                        self.accept(lexer, version, lookahead);
                        return true;
                    }

                    PARSE_ACTION_TYPE_RECOVER => {
                        self.recover(lexer, version, lookahead);
                        return true;
                    }

                    _ => {}
                }
            }

            // If a reduction was performed, then replace the current stack version with one
            // of the stack versions created by a reduction, and continue processing this
            // version of the stack with the same lookahead symbol.
            if last_reduction_version != STACK_VERSION_NONE {
                self.stack
                    .renumber_version(&mut self.tree.pool, last_reduction_version, version);
                state = self.stack.state(version);

                // At the end of a non-terminal extra rule, the lexer returns a null subtree.
                // After the reduction, run the lexer again based on the current parse state.
                if lookahead == NULL_SUBTREE {
                    needs_lex = true;
                } else {
                    (first, count, _) =
                        lang.table_entry(state, self.tree.pool.leaf_symbol(lookahead));
                }

                continue;
            }

            // A non-terminal extra rule was reduced and merged into an existing stack
            // version. This version can be discarded.
            if lookahead == NULL_SUBTREE {
                self.stack.halt(version);
                return true;
            }

            // If the current lookahead token is a keyword that is not valid, but the default
            // word token *is* valid, then treat the lookahead token as the word token
            // instead.
            let keyword_capture_token = lang.tables.keyword_capture_token;
            let sp = &self.tree.pool;
            if sp.is_keyword(lookahead)
                && sp.symbol(lookahead) != keyword_capture_token
                && !lang.is_reserved_word(state, sp.symbol(lookahead))
            {
                (first, count, _) = lang.table_entry(state, keyword_capture_token);
                if count > 0 {
                    if lexer.state.logger.is_some() {
                        lb(lexer)
                            .s("switch from_keyword:")
                            .s(self.tree_name(lookahead))
                            .s(", to_word_token:")
                            .s(self.sym_name(keyword_capture_token))
                            .log(lexer);
                    }

                    let sp = &mut self.tree.pool;
                    let mut mutable_lookahead = sp.make_mut(lookahead);
                    sp.set_symbol(&mut mutable_lookahead, keyword_capture_token, lang);
                    lookahead = mutable_lookahead;
                    continue;
                }
            }

            // If the previous subtree on the stack was reused from an old tree, break it
            // down and try again. Never the case here; see breakdown_top_of_stack.
            if self.breakdown_top_of_stack(lexer, version) {
                state = self.stack.state(version);
                self.tree.pool.release(lookahead);
                needs_lex = true;
                continue;
            }

            // Otherwise, there is definitely an error in this version of the parse stack.
            // Mark this version as paused and continue processing any other stack versions
            // that exist. If all versions end up paused, then error recovery is needed.
            if lexer.state.logger.is_some() {
                lb(lexer).s("detect_error").log(lexer);
            }
            self.stack.pause(version, lookahead);
            return true;
        }
    }

    // ts_parser__condense_stack
    fn condense_stack(&mut self, lexer: &mut Lexer<'_>) -> u32 {
        let mut made_changes = false;
        let mut min_error_cost = u32::MAX;

        // One active version: nothing below prunes, compares, merges or resumes it.
        // ts_parser__version_status is still called, because
        // ts_stack_node_count_since_error can lower node_count_at_last_error.
        if self.stack.version_count() == 1 && self.stack.is_active(0) {
            let status = self.version_status(0);
            if !status.is_in_error {
                min_error_cost = status.cost;
            }
            return min_error_cost;
        }

        let mut i = 0u32;
        while i < self.stack.version_count() {
            // Prune any versions that have been marked for removal.
            if self.stack.is_halted(i) {
                self.stack.remove_version(&mut self.tree.pool, i);
                // Go: i--, then i++.
                continue;
            }

            // Keep track of the minimum error cost of any stack version so that it can be
            // returned.
            let status_i = self.version_status(i);
            if !status_i.is_in_error && status_i.cost < min_error_cost {
                min_error_cost = status_i.cost;
            }

            // Examine each pair of stack versions, removing any versions that are clearly
            // worse than another version. Ensure that the versions are ordered from most
            // promising to least promising. i and j wrap below zero and back, as the
            // unsigned C counters do.
            let mut j = 0u32;
            while j < i {
                let status_j = self.version_status(j);

                match compare_versions(status_j, status_i) {
                    ERROR_COMPARISON_TAKE_LEFT => {
                        made_changes = true;
                        self.stack.remove_version(&mut self.tree.pool, i);
                        i = i.wrapping_sub(1);
                        j = i;
                    }

                    ERROR_COMPARISON_PREFER_LEFT | ERROR_COMPARISON_NONE => {
                        if self.stack.merge(&mut self.tree.pool, j, i) {
                            made_changes = true;
                            i = i.wrapping_sub(1);
                            j = i;
                        }
                    }

                    ERROR_COMPARISON_PREFER_RIGHT => {
                        made_changes = true;
                        if self.stack.merge(&mut self.tree.pool, j, i) {
                            i = i.wrapping_sub(1);
                            j = i;
                        } else {
                            self.stack.swap_versions(i, j);
                        }
                    }

                    ERROR_COMPARISON_TAKE_RIGHT => {
                        made_changes = true;
                        self.stack.remove_version(&mut self.tree.pool, j);
                        i = i.wrapping_sub(1);
                        j = j.wrapping_sub(1);
                    }

                    _ => {}
                }
                j = j.wrapping_add(1);
            }
            i = i.wrapping_add(1);
        }

        // Enforce a hard upper bound on the number of stack versions by discarding the
        // least promising versions.
        while self.stack.version_count() > MAX_VERSION_COUNT {
            self.stack
                .remove_version(&mut self.tree.pool, MAX_VERSION_COUNT);
            made_changes = true;
        }

        // If the best-performing stack version is currently paused, or all versions are
        // paused, then resume the best paused version and begin the error recovery
        // process. Otherwise, remove the paused versions.
        if self.stack.version_count() > 0 {
            let mut has_unpaused_version = false;
            let mut i = 0u32;
            let mut n = self.stack.version_count();
            while i < n {
                if self.stack.is_paused(i) {
                    if !has_unpaused_version && self.accept_count < MAX_VERSION_COUNT {
                        if lexer.state.logger.is_some() {
                            lb(lexer).s("resume version:").u(i).log(lexer);
                        }
                        min_error_cost = self.stack.error_cost(i);
                        let lookahead = self.stack.resume(i);
                        self.handle_error(lexer, i, lookahead);
                        has_unpaused_version = true;
                    } else {
                        self.stack.remove_version(&mut self.tree.pool, i);
                        n -= 1;
                        // Go: i--, then i++.
                        continue;
                    }
                } else {
                    has_unpaused_version = true;
                }
                i += 1;
            }
        }

        if made_changes && lexer.state.logger.is_some() {
            lb(lexer).s("condense").log(lexer);
        }

        min_error_cost
    }

    // ts_parser__balance_subtree. C walks every node; only nodes with repeat_depth > 0
    // change, so subtrees without F_HAS_REPEAT are left out.
    fn balance_subtree(&mut self) {
        let lang = self.lang();
        let finished_tree = self.finished_tree;
        let sp = &mut self.tree.pool;

        sp.tree_stack.clear();
        if sp.child_count(finished_tree) > 0 && sp.ptr(finished_tree).ref_count == 1 {
            sp.tree_stack.push(finished_tree);
        }

        while let Some(&tree) = sp.tree_stack.last() {
            let h = *sp.ptr(tree);
            if h.repeat_depth > 0 {
                let child1 = sp.children[h.children as usize];
                let child2 = sp.children[(h.children + h.child_count - 1) as usize];
                let repeat_delta =
                    i64::from(sp.repeat_depth(child1)) - i64::from(sp.repeat_depth(child2));
                if repeat_delta > 0 {
                    // Go also counts n down by i; nothing reads it.
                    let mut i = repeat_delta as u32 / 2;
                    while i > 0 {
                        sp.compress(tree, i, lang);
                        i /= 2;
                    }
                }
            }

            sp.tree_stack.pop();

            let h = *sp.ptr(tree);
            for k in h.children..h.children + h.child_count {
                let child = sp.children[k as usize];
                if child.is_inline() {
                    continue;
                }
                let ch = sp.ptr(child);
                if ch.flags & F_HAS_REPEAT != 0 && ch.ref_count == 1 {
                    sp.tree_stack.push(child);
                }
            }
        }
    }
}

/// `set_language` was given a grammar of an ABI this runtime does not accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageError {
    pub abi_version: u32,
}

impl std::fmt::Display for LanguageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "gositter: incompatible language ABI version {}",
            self.abi_version
        )
    }
}

impl std::error::Error for LanguageError {}

impl Default for Parser {
    fn default() -> Self {
        Parser::new()
    }
}

impl Parser {
    /// ts_parser_new
    pub fn new() -> Parser {
        let mut p = Parser {
            lexer_state: LexerState {
                debug_buf: Vec::with_capacity(DEBUG_BUFFER_SIZE),
                ..LexerState::default()
            },
            stack: Stack::default(),
            tree: Tree::empty(),
            language: None,
            reduce_actions: Vec::with_capacity(4),
            reduce_scans: Vec::new(),
            missing_scans: Vec::new(),
            finished_tree: NULL_SUBTREE,
            trailing_extras: Vec::new(),
            trailing_extras2: Vec::new(),
            token_cache: TokenCache::default(),
            external_scanner: None,
            accept_count: 0,
            has_error: false,
            scanner_buf: [0; SERIALIZATION_BUFFER_SIZE],
            valid_external_tokens: Vec::new(),
            kept_scanner: None,
            kept_language: None,
            scanner_generation: 0,
        };
        p.reset();
        p
    }

    /// ts_parser_set_language. Errors on an incompatible ABI version.
    pub fn set_language(&mut self, language: &'static Language) -> Result<(), LanguageError> {
        self.reset();
        self.language = None;
        // Never keep another language's scanner and its buffers.
        self.kept_scanner = None;
        self.kept_language = None;
        // Go keys these by language; here they hold one language's.
        self.reduce_scans.clear();
        self.missing_scans.clear();

        let abi_version = language.tables.abi_version;
        if !(MIN_COMPATIBLE_LANGUAGE_VERSION..=TREE_SITTER_LANGUAGE_VERSION).contains(&abi_version)
        {
            return Err(LanguageError { abi_version });
        }

        self.language = Some(language);
        Ok(())
    }

    /// ts_parser_set_logger. `None` turns logging off.
    pub fn set_logger(&mut self, logger: Option<Logger>) {
        self.lexer_state.logger = logger;
    }

    /// ts_parser_reset. C releases the stack, the token cache and the finished tree one
    /// reference at a time. Here everything lives in the parser's slabs and nothing outlives
    /// the reset, so they are truncated instead. The tree of the last parse dies with them.
    pub fn reset(&mut self) {
        self.external_scanner_destroy();
        let mut lexer = Lexer::new(&[], std::mem::take(&mut self.lexer_state));
        lexer.reset(Length::default());
        self.lexer_state = lexer.into_state();
        self.tree.pool.reset();
        self.stack.reset(&mut self.tree.pool);
        self.token_cache = TokenCache::default();
        self.finished_tree = NULL_SUBTREE;
        self.trailing_extras.clear();
        self.trailing_extras2.clear();
        self.accept_count = 0;
        self.has_error = false;
        self.tree.language = None;
        self.tree.root = 0;
    }

    /// ts_parser_parse with no old tree and UTF-8 input. `None` if no language is set.
    ///
    /// The tree and its nodes borrow the parser until its next parse, so a node held across
    /// a second parse does not compile (`tree_does_not_outlive_parse`):
    ///
    /// ```compile_fail,E0499
    /// let mut p = gositter::Parser::new();
    /// let root = p.parse(b"x").unwrap().root_node();
    /// p.parse(b"y");
    /// let _ = root.kind();
    /// ```
    pub fn parse(&mut self, src: &[u8]) -> Option<&Tree> {
        self.parse_budget(src, 0)
    }

    /// `parse`, but `None` once advance ran more than `max_stall` times in a row without
    /// passing the furthest byte reached; the parser is then reset. 0 is no limit. The
    /// count depends only on the language and `src`.
    pub fn parse_budget(&mut self, src: &[u8], max_stall: u32) -> Option<&Tree> {
        self.language?;
        let max_stall = if max_stall == 0 { u32::MAX } else { max_stall };

        // C resets at the end of a parse. The tree borrows the slabs, so the reset happens
        // here, before the next one.
        self.reset();

        let mut lexer = Lexer::new(src, std::mem::take(&mut self.lexer_state));

        self.external_scanner_create();
        if lexer.state.logger.is_some() {
            lb(&mut lexer).s("new_parse").log(&mut lexer);
        }

        let mut last_position = 0u32;
        let mut stalled = 0u32;
        let mut version_count;
        loop {
            let mut version = 0u32;
            loop {
                version_count = self.stack.version_count();
                if version >= version_count {
                    break;
                }
                while self.stack.is_active(version) {
                    if lexer.state.logger.is_some() {
                        let pos = self.stack.position(version);
                        lb(&mut lexer)
                            .s("process version:")
                            .u(version)
                            .s(", version_count:")
                            .u(self.stack.version_count())
                            .s(", state:")
                            .u(u32::from(self.stack.state(version)))
                            .s(", row:")
                            .u(pos.extent.row)
                            .s(", col:")
                            .u(pos.extent.column)
                            .log(&mut lexer);
                    }

                    self.advance(&mut lexer, version);

                    let position = self.stack.position(version).bytes;
                    if position > last_position {
                        last_position = position;
                        stalled = 0;
                        break;
                    }
                    stalled = stalled.wrapping_add(1);
                    if stalled > max_stall {
                        self.lexer_state = lexer.into_state();
                        self.reset();
                        return None;
                    }
                    if version > 0 && position == last_position {
                        break;
                    }
                }
                version += 1;
            }

            // After advancing each version of the stack, re-sort the versions by their cost,
            // removing any versions that are no longer worth pursuing.
            let min_error_cost = self.condense_stack(&mut lexer);

            // If there's already a finished parse tree that's better than any in-progress
            // version, then terminate parsing. Clear the parse stack to remove any extra
            // references to subtrees within the finished tree, ensuring that these subtrees
            // can be safely mutated in-place for rebalancing.
            if self.finished_tree != NULL_SUBTREE
                && self.tree.pool.error_cost(self.finished_tree) < min_error_cost
            {
                self.stack.clear(&mut self.tree.pool);
                break;
            }

            if version_count == 0 {
                break;
            }
        }

        self.balance_subtree();
        if lexer.state.logger.is_some() {
            lb(&mut lexer).s("done").log(&mut lexer);
        }

        // ts_tree_new. The root gets a slot in the children slab so that a Node can address
        // it like any other subtree.
        let sp = &mut self.tree.pool;
        let off = sp.room(1);
        let root = sp.bump_children(off, 1);
        sp.children[root as usize] = self.finished_tree;
        self.tree.language = self.language;
        self.tree.root = root;
        self.finished_tree = NULL_SUBTREE;

        self.external_scanner_destroy();
        self.lexer_state = lexer.into_state();
        Some(&self.tree)
    }
}
