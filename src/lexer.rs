//! lexer.c (gositter `lexer.go`). The input is one byte slice and the included ranges are the
//! single DEFAULT_RANGE {0, UINT32_MAX}, so included_range_count is 1 and
//! current_included_range_index is 0, or 1 at end of input. The chunk fields are kept because
//! ts_lexer__advance tests them: the "chunk" is `src[chunk_start..]`.

use crate::length::Length;
use crate::{Lexer, Logger};

pub(crate) const BYTE_ORDER_MARK: i32 = 0xFEFF;
/// TS_DECODE_ERROR
pub(crate) const DECODE_ERROR: i32 = -1;
pub(crate) const INCLUDED_RANGE_COUNT: u32 = 1;

/// ColumnData
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColumnData {
    pub(crate) value: u32,
    pub(crate) valid: bool,
}

/// Every Lexer field besides `lookahead`, `result_symbol` and `src`. The parser keeps it
/// between parses and moves it into the `Lexer` of each parse.
#[derive(Default)]
pub(crate) struct LexerState {
    pub(crate) current_position: Length,
    pub(crate) token_start_position: Length,
    pub(crate) token_end_position: Length,

    pub(crate) logger: Option<Logger>,

    pub(crate) current_included_range_index: u32,
    pub(crate) chunk_start: u32,
    pub(crate) chunk_size: u32,
    pub(crate) lookahead_size: u32,
    /// chunk != NULL
    pub(crate) has_chunk: bool,
    pub(crate) did_get_column: bool,
    pub(crate) column_data: ColumnData,

    /// debug_buffer, for log messages only.
    pub(crate) debug_buf: Vec<u8>,
}

impl<'s> Lexer<'s> {
    /// A lexer over `src` with the kept `state`: ts_lexer_set_input.
    pub(crate) fn new(_src: &'s [u8], _state: LexerState) -> Lexer<'s> {
        todo!("gositter-rs-2f0.6")
    }
}

impl Lexer<'_> {
    /// The state to keep until the next parse.
    pub(crate) fn into_state(self) -> LexerState {
        todo!("gositter-rs-2f0.6")
    }

    /// The C LOG macro of lexer.c.
    pub(crate) fn log_character(&mut self, _message: &str) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__set_column_data
    pub(crate) fn set_column_data(&mut self, _value: u32) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__increment_column_data
    pub(crate) fn increment_column_data(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__invalidate_column_data
    pub(crate) fn invalidate_column_data(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__eof
    pub fn eof(&self) -> bool {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__clear_chunk
    pub(crate) fn clear_chunk(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__get_chunk, with ts_string_input_read as the callback.
    pub(crate) fn get_chunk(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__get_lookahead, UTF-8 only.
    pub(crate) fn get_lookahead(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer_goto
    pub(crate) fn go_to(&mut self, _position: Length) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__advance, with ts_lexer__do_advance folded in.
    pub fn advance(&mut self, _skip: bool) {
        todo!("gositter-rs-2f0.6")
    }

    /// `advance(skip)` repeated while `lookahead` is an ASCII character in `set`
    /// (gositter CONTRACT.md section 6, self-loops). `set[0]` and `set[128..]` are false.
    pub fn advance_run(&mut self, _set: &'static [bool; 256], _skip: bool) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__mark_end
    pub fn mark_end(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__get_column: code points since the start of the line.
    pub fn get_column(&mut self) -> u32 {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer__is_at_included_range_start
    pub fn is_at_included_range_start(&self) -> bool {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer_reset
    pub(crate) fn reset(&mut self, _position: Length) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer_start
    pub(crate) fn start(&mut self) {
        todo!("gositter-rs-2f0.6")
    }

    /// ts_lexer_finish
    pub(crate) fn finish(&mut self, _lookahead_end_byte: &mut u32) {
        todo!("gositter-rs-2f0.6")
    }
}

/// gositter `lexer_test.go`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LogType, Point};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn new_test_lexer(src: &[u8]) -> Lexer<'_> {
        let mut l = Lexer::new(src, LexerState::default());
        l.start();
        l
    }

    fn len(bytes: u32, row: u32, column: u32) -> Length {
        Length {
            bytes,
            extent: Point { row, column },
        }
    }

    /// (name, source, (code point, width) of each character).
    type DecodeCase = (&'static str, &'static [u8], &'static [(i32, u32)]);

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_decode() {
        let bad = (DECODE_ERROR, 1);
        let cases: &[DecodeCase] = &[
            ("ascii", b"ab", &[('a' as i32, 1), ('b' as i32, 1)]),
            (
                "nul byte is not eof",
                b"a\x00b",
                &[('a' as i32, 1), (0, 1), ('b' as i32, 1)],
            ),
            ("two byte", "é".as_bytes(), &[(0xE9, 2)]),
            ("three byte", "€".as_bytes(), &[(0x20AC, 3)]),
            ("four byte", "\u{1F600}".as_bytes(), &[(0x1F600, 4)]),
            ("max", "\u{10FFFF}".as_bytes(), &[(0x10FFFF, 4)]),
            ("valid U+FFFD", b"\xef\xbf\xbd", &[(0xFFFD, 3)]),
            ("lone continuation", b"\x80", &[bad]),
            ("0xff", b"\xffa", &[bad, ('a' as i32, 1)]),
            ("overlong 2", b"\xc0\x80", &[bad, bad]),
            ("overlong 3", b"\xe0\x80\x80", &[bad, bad, bad]),
            ("surrogate", b"\xed\xa0\x80", &[bad, bad, bad]),
            ("above max", b"\xf4\x90\x80\x80", &[bad, bad, bad, bad]),
            ("0xf5 lead", b"\xf5\x80\x80\x80", &[bad, bad, bad, bad]),
            (
                "truncated at end",
                b"a\xe2\x82",
                &[('a' as i32, 1), bad, bad],
            ),
            (
                "truncated then ascii",
                b"\xe2\x82a",
                &[bad, bad, ('a' as i32, 1)],
            ),
        ];
        for &(name, src, want) in cases {
            let mut l = new_test_lexer(src);
            for (i, &w) in want.iter().enumerate() {
                assert!(!l.eof(), "{name}: eof before char {i}");
                assert_eq!((l.lookahead, l.state.lookahead_size), w, "{name}: char {i}");
                l.advance(false);
            }
            assert!(
                l.eof() && l.lookahead == 0,
                "{name}: at end eof={} lookahead={}",
                l.eof(),
                l.lookahead
            );
            assert_eq!(
                l.state.current_position.bytes,
                src.len() as u32,
                "{name}: end position"
            );
        }
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_advance_after_eof() {
        let logged = Arc::new(AtomicUsize::new(0));
        let mut l = new_test_lexer(b"x");
        let counter = logged.clone();
        l.state.logger = Some(Box::new(move |_: LogType, _: &[u8]| {
            counter.fetch_add(1, Ordering::Relaxed);
        }));
        l.advance(false);
        assert!(l.eof());
        assert_eq!(logged.load(Ordering::Relaxed), 1);
        let before = l.state.current_position;
        l.advance(false);
        l.advance(true);
        assert_eq!(l.state.current_position, before);
        assert_eq!(logged.load(Ordering::Relaxed), 1);
        assert_eq!(l.lookahead, 0);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_empty_input() {
        let l = new_test_lexer(b"");
        assert!(l.eof());
        assert_eq!(l.lookahead, 0);
        assert!(!l.is_at_included_range_start());
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_bom_and_positions() {
        let mut l = new_test_lexer("\u{feff}a\néb".as_bytes());
        // The BOM is skipped by start: token start moves past it.
        assert_eq!(l.lookahead, 'a' as i32);
        assert_eq!(l.state.token_start_position.bytes, 3);
        assert_eq!(l.state.current_position.extent.column, 3);
        assert_eq!(l.get_column(), 0, "column after BOM");
        l.advance(false); // a
        l.advance(false); // \n
        assert_eq!(l.state.current_position, len(5, 1, 0));
        l.advance(false); // é, two bytes, one column of get_column
        assert_eq!(l.state.current_position, len(7, 1, 2));
        assert_eq!(l.get_column(), 1);

        // With the cached column gone, get_column re-scans the line and comes back to the
        // same place.
        l.invalidate_column_data();
        assert_eq!(l.get_column(), 1);
        assert!(l.state.did_get_column);
        assert_eq!(l.state.current_position, len(7, 1, 2));
        assert_eq!(l.lookahead, 'b' as i32);
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_is_at_included_range_start() {
        let mut l = new_test_lexer(b"ab");
        assert!(l.is_at_included_range_start(), "want true at byte 0");
        l.advance(false);
        assert!(!l.is_at_included_range_start(), "want false at byte 1");
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_finish() {
        let mut l = new_test_lexer(b"ab");
        l.advance(false);
        let mut end = 0;
        l.finish(&mut end);
        // mark_end was never called, so the token ends at the current position.
        assert_eq!((l.state.token_end_position.bytes, end), (1, 2));

        let mut l = new_test_lexer(b"a\xff");
        l.advance(false);
        l.mark_end();
        let mut end = 0;
        l.finish(&mut end);
        assert_eq!(end, 1 + 1 + 4, "lookahead end on invalid UTF-8");
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn lexer_reset_backwards() {
        let mut l = new_test_lexer(b"abc");
        l.advance(false);
        l.advance(false);
        l.reset(len(1, 0, 1));
        l.start();
        assert_eq!(l.lookahead, 'b' as i32);
        assert_eq!(l.state.current_position.bytes, 1);
        // Reset to the end and back: EOF must clear.
        l.reset(len(3, 0, 3));
        l.start();
        assert!(l.eof(), "want eof at end");
        l.reset(len(2, 0, 2));
        l.start();
        assert!(!l.eof());
        assert_eq!(l.lookahead, 'c' as i32);
    }

    /// The definition of advance_run: advance, one character at a time.
    fn advance_run_ref(l: &mut Lexer<'_>, set: &[bool; 256], skip: bool) {
        while (l.lookahead as u32) < 128 && set[l.lookahead as usize] && l.state.has_chunk {
            l.advance(skip);
        }
    }

    /// Everything a lex function or the parser can see of a lexer.
    fn lexer_snapshot(l: &mut Lexer<'_>) -> [u32; 16] {
        let b = |v: bool| v as u32;
        let s = &l.state;
        let mut out = [
            l.lookahead as u32,
            s.lookahead_size,
            b(s.has_chunk),
            0,
            s.current_position.bytes,
            s.current_position.extent.row,
            s.current_position.extent.column,
            s.token_start_position.bytes,
            s.token_start_position.extent.row,
            s.token_start_position.extent.column,
            s.column_data.value,
            b(s.column_data.valid),
            s.chunk_start,
            s.chunk_size,
            0,
            0,
        ];
        out[3] = b(l.eof());
        out[14] = l.get_column();
        out
    }

    fn check_advance_run(
        src: &[u8],
        set: &'static [bool; 256],
        start: usize,
        skip: bool,
        column_valid: bool,
    ) {
        let make = || {
            let mut l = new_test_lexer(src);
            while l.state.current_position.bytes < start as u32 && l.state.has_chunk {
                l.advance(false);
            }
            if !column_valid {
                l.invalidate_column_data();
            }
            l
        };
        let (mut a, mut b) = (make(), make());
        advance_run_ref(&mut a, set, skip);
        b.advance_run(set, skip);
        assert_eq!(
            lexer_snapshot(&mut b),
            lexer_snapshot(&mut a),
            "src {src:?} start {start} skip {skip} column_valid {column_valid}"
        );
    }

    fn set_of(f: impl Fn(u8) -> bool) -> &'static [bool; 256] {
        let mut set = [false; 256];
        for c in 1..128u8 {
            set[c as usize] = f(c);
        }
        Box::leak(Box::new(set))
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn advance_run() {
        let ident = set_of(|c| c == b'_' || c.is_ascii_digit() || c.is_ascii_lowercase());
        let blank = set_of(|c| c == b' ' || (b'\t'..=b'\r').contains(&c));
        let not_quote = set_of(|c| c != b'"');
        let srcs: &[&[u8]] = &[
            b"",
            b"a",
            b"abc",
            b"abc def",
            b"  \n\t x",
            b"\n\n\n",
            b"ab\n",
            b"abc\xc3\xa9def",
            b"ab\xffcd",
            b"\xef\xbb\xbfabc",
            b"a\x00b",
            b"line one\nline two\r\n  three\"x",
            "é\nabc".as_bytes(),
            b"abc\n\xe2\x82",
        ];
        for src in srcs {
            for set in [ident, blank, not_quote] {
                for start in 0..=src.len() {
                    for skip in [false, true] {
                        check_advance_run(src, set, start, skip, true);
                        check_advance_run(src, set, start, skip, false);
                    }
                }
            }
        }
    }

    /// gositter `FuzzAdvanceRun` as a seeded loop (the fuzz target is plan Task 13).
    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn advance_run_random() {
        let sets = [
            set_of(|c| c == b' ' || (b'\t'..=b'\r').contains(&c)),
            set_of(|c| c != b'\n' && c != b'"'),
            set_of(|c| c & 1 == 0),
            set_of(|_| true),
        ];
        let alphabet = b"ab \n\t\"_9\xc3\xa9\xff\xe2\x82\x00";
        let mut x: u64 = 1;
        let mut next = move || {
            // xorshift64
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let seed: &[u8] = b"ab cd\n  ef\xc3\xa9 \"g\"\n";
        check_advance_run(seed, sets[0], 3, true, true);
        for _ in 0..10_000 {
            let n = (next() % 24) as usize;
            let src: Vec<u8> = (0..n)
                .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
                .collect();
            let which = next() as u8;
            let start = (next() as u8) as usize % (src.len() + 1);
            let skip = next() & 1 == 1;
            check_advance_run(
                &src,
                sets[(which % 4) as usize],
                start,
                skip,
                which & 4 == 0,
            );
        }
    }
}
