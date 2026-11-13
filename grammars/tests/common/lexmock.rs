//! Mock lexer for the lex oracle: code points, not bytes, and an FNV-1a 64 digest of every
//! call a lex function makes. Port of gositter v0.2.0 `cmd/gengrammar/check/mock.go.txt` (MIT);
//! `advance_run` is the loop of gositter `lexer.go:218-222`, so it hashes per character exactly as
//! single advances do.
#![allow(dead_code)] // each test crate uses part of it

use gositter_grammars::lex::LexOps;

const PRIME: u64 = 1099511628211;

pub struct MockLexer {
    input: Vec<i32>,
    pos: u32,
    mark: u32,
    /// `advance` calls at end of input.
    idle: u32,
    h: u64,
    lookahead: i32,
    result_symbol: u16,
}

impl MockLexer {
    fn mix(&mut self, v: u64) {
        self.h = (self.h ^ v).wrapping_mul(PRIME);
    }

    fn load(&mut self) {
        self.lookahead = self.input.get(self.pos as usize).copied().unwrap_or(0);
    }

    /// Calls `f` on `input` and returns the digest of what it did (`MockRun`).
    pub fn run(f: impl FnOnce(&mut MockLexer, u16) -> bool, state: u16, input: &[i32]) -> u64 {
        let mut l = MockLexer {
            input: input.to_vec(),
            pos: 0,
            mark: 0,
            idle: 0,
            h: 14695981039346656037,
            lookahead: 0,
            result_symbol: 0,
        };
        l.load();
        let ret = f(&mut l, state);
        l.mix(u64::from(l.result_symbol));
        l.mix(u64::from(ret));
        l.mix(u64::from(l.pos));
        l.mix(u64::from(l.mark));
        l.h
    }
}

impl LexOps for MockLexer {
    fn lookahead(&self) -> i32 {
        self.lookahead
    }

    fn eof(&self) -> bool {
        self.pos as usize >= self.input.len()
    }

    fn advance(&mut self, skip: bool) {
        if self.eof() {
            // the real lexer does nothing at end of input
            self.idle += 1;
            assert!(self.idle <= 1 << 20, "lex function loops at end of input");
            return;
        }
        self.mix(if skip { 2 } else { 1 });
        self.mix(u64::from(self.pos));
        self.pos += 1;
        self.load();
    }

    fn advance_run(&mut self, set: &'static [bool; 256], skip: bool) {
        while (0..128).contains(&self.lookahead) && set[self.lookahead as usize] && !self.eof() {
            self.advance(skip);
        }
    }

    fn mark_end(&mut self) {
        self.mix(3);
        self.mix(u64::from(self.pos));
        self.mark = self.pos;
    }

    fn set_result_symbol(&mut self, sym: u16) {
        self.result_symbol = sym;
    }
}
