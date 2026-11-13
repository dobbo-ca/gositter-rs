//! What a generated `lex.rs` calls. `LexOps` is the lexer interface of `lex` and `lex_keywords`,
//! so the runtime `Lexer` and the oracle's `MockLexer` both drive the same generated code.
//! `run_table` interprets the table form (`lex.bin`, format in `gengrammar/src/lexgen.rs`).

/// The lexer methods a generated lex function uses (gositter `lexer.go`).
pub trait LexOps {
    fn lookahead(&self) -> i32;
    fn eof(&self) -> bool;
    fn advance(&mut self, skip: bool);
    /// `advance(skip)` while the lookahead is an ASCII character in `set`.
    fn advance_run(&mut self, set: &'static [bool; 256], skip: bool);
    fn mark_end(&mut self);
    fn set_result_symbol(&mut self, sym: u16);
}

impl LexOps for gositter::Lexer<'_> {
    #[inline]
    fn lookahead(&self) -> i32 {
        self.lookahead
    }
    #[inline]
    fn eof(&self) -> bool {
        gositter::Lexer::eof(self)
    }
    #[inline]
    fn advance(&mut self, skip: bool) {
        gositter::Lexer::advance(self, skip)
    }
    #[inline]
    fn advance_run(&mut self, set: &'static [bool; 256], skip: bool) {
        gositter::Lexer::advance_run(self, set, skip)
    }
    #[inline]
    fn mark_end(&mut self) {
        gositter::Lexer::mark_end(self)
    }
    #[inline]
    fn set_result_symbol(&mut self, sym: u16) {
        self.result_symbol = sym;
    }
}

/// C `set_contains`: binary search over `(start, end)` ranges.
#[allow(dead_code)] // tests-first stub
pub fn set_contains(ranges: &[(i32, i32)], lookahead: i32) -> bool {
    let _ = (ranges, lookahead);
    todo!()
}

/// `set_contains` with the answers for code points below 128 precomputed:
/// `ascii[c] == set_contains(ranges, c)`.
#[allow(dead_code)] // tests-first stub
pub fn set_contains_ascii(ascii: &[bool; 128], ranges: &[(i32, i32)], lookahead: i32) -> bool {
    let _ = (ascii, ranges, lookahead);
    todo!()
}

/// The table-form lexer: runs the state machine of `data` (one function's region of `lex.bin`)
/// with the observable calls of the code form. `runs` are the run sets the records index.
#[allow(dead_code)] // tests-first stub
pub fn run_table<L: LexOps>(
    data: &'static [u8],
    runs: &'static [&'static [bool; 256]],
    l: &mut L,
    state: u16,
) -> bool {
    let _ = (data, runs, l, state);
    todo!()
}
