//! What a generated `lex.rs` calls. `LexOps` is the lexer interface of `lex` and `lex_keywords`,
//! so the runtime `Lexer` and the oracle's `MockLexer` both drive the same generated code.

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

/// C `set_contains`: binary search over `(start, end)` ranges, as `gengrammar/src/cond.rs`.
pub fn set_contains(ranges: &[(i32, i32)], lookahead: i32) -> bool {
    let (mut index, mut size) = (0, ranges.len());
    if size == 0 {
        return false;
    }
    while size > 1 {
        let half = size / 2;
        let mid = index + half;
        let (start, end) = ranges[mid];
        if (start..=end).contains(&lookahead) {
            return true;
        } else if lookahead > end {
            index = mid;
        }
        size -= half;
    }
    (ranges[index].0..=ranges[index].1).contains(&lookahead)
}

/// `set_contains` with the answers for code points below 128 precomputed:
/// `ascii[c] == set_contains(ranges, c)`.
#[inline]
pub fn set_contains_ascii(ascii: &[bool; 128], ranges: &[(i32, i32)], lookahead: i32) -> bool {
    if (0..128).contains(&lookahead) {
        ascii[lookahead as usize]
    } else {
        set_contains(ranges, lookahead)
    }
}
