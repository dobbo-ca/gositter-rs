//! lexer.c (gositter `lexer.go`). The input is one byte slice and the included ranges are the
//! single DEFAULT_RANGE {0, UINT32_MAX}, so included_range_count is 1 and
//! current_included_range_index is 0, or 1 at end of input. The chunk fields are kept because
//! ts_lexer__advance tests them: the "chunk" is `src[chunk_start..]`.

use crate::length::{LENGTH_UNDEFINED, Length, length_is_undefined};
use crate::{Lexer, LogType, Logger};

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
    pub(crate) fn new(src: &'s [u8], state: LexerState) -> Lexer<'s> {
        let mut l = Lexer {
            lookahead: 0,
            result_symbol: 0,
            src,
            state,
        };
        l.clear_chunk();
        let position = l.state.current_position;
        l.go_to(position);
        l
    }
}

/// Go `utf8.DecodeRune` on the start of `bytes` (not empty, first byte 0x80 or more):
/// the code point and its width, or `(DECODE_ERROR, 1)` for an invalid or truncated
/// sequence. Rust's UTF-8 check rejects the same sequences as Go's: overlongs, surrogates,
/// values above U+10FFFF, bad continuation bytes.
fn decode_rune(bytes: &[u8]) -> (i32, u32) {
    let head = &bytes[..bytes.len().min(4)];
    let valid = match std::str::from_utf8(head) {
        Ok(s) => s,
        Err(e) => std::str::from_utf8(&head[..e.valid_up_to()]).unwrap_or_default(),
    };
    match valid.chars().next() {
        Some(c) => (c as i32, c.len_utf8() as u32),
        None => (DECODE_ERROR, 1),
    }
}

impl Lexer<'_> {
    /// The state to keep until the next parse.
    pub(crate) fn into_state(self) -> LexerState {
        self.state
    }

    /// The C LOG macro of lexer.c.
    pub(crate) fn log_character(&mut self, message: &str) {
        let st = &mut self.state;
        let buf = &mut st.debug_buf;
        buf.clear();
        buf.extend_from_slice(message.as_bytes());
        let c = self.lookahead;
        if (32..127).contains(&c) {
            buf.extend_from_slice(b" character:'");
            buf.push(c as u8);
            buf.push(b'\'');
        } else {
            buf.extend_from_slice(b" character:");
            buf.extend_from_slice(c.to_string().as_bytes());
        }
        if let Some(logger) = st.logger.as_mut() {
            logger(LogType::Lex, buf);
        }
    }

    /// ts_lexer__set_column_data
    pub(crate) fn set_column_data(&mut self, value: u32) {
        self.state.column_data = ColumnData { value, valid: true };
    }

    /// ts_lexer__increment_column_data
    pub(crate) fn increment_column_data(&mut self) {
        if self.state.column_data.valid {
            self.state.column_data.value += 1;
        }
    }

    /// ts_lexer__invalidate_column_data
    pub(crate) fn invalidate_column_data(&mut self) {
        self.state.column_data = ColumnData {
            value: 0,
            valid: false,
        };
    }

    /// ts_lexer__eof
    pub fn eof(&self) -> bool {
        self.state.current_included_range_index == INCLUDED_RANGE_COUNT
    }

    /// ts_lexer__clear_chunk
    pub(crate) fn clear_chunk(&mut self) {
        self.state.has_chunk = false;
        self.state.chunk_size = 0;
        self.state.chunk_start = 0;
    }

    /// ts_lexer__get_chunk, with ts_string_input_read as the callback.
    pub(crate) fn get_chunk(&mut self) {
        let st = &mut self.state;
        st.chunk_start = st.current_position.bytes;
        // Go compares with uint32(len(src)): a source of 4 GiB or more wraps, as there.
        let src_len = self.src.len() as u32;
        if st.current_position.bytes >= src_len {
            st.chunk_size = 0;
            st.current_included_range_index = INCLUDED_RANGE_COUNT;
            st.has_chunk = false;
        } else {
            st.chunk_size = src_len - st.current_position.bytes;
            st.has_chunk = true;
        }
    }

    /// ts_lexer__get_lookahead, UTF-8 only.
    pub(crate) fn get_lookahead(&mut self) {
        let position_in_chunk = self
            .state
            .current_position
            .bytes
            .wrapping_sub(self.state.chunk_start);
        let size = self.state.chunk_size.wrapping_sub(position_in_chunk);

        if size == 0 {
            self.state.lookahead_size = 1;
            self.lookahead = 0;
            return;
        }

        let chunk = &self.src[self.state.current_position.bytes as usize..];
        let c = chunk[0];
        if c < 0x80 {
            self.state.lookahead_size = 1;
            self.lookahead = c as i32;
            return;
        }

        let (r, n) = decode_rune(chunk);
        if r == DECODE_ERROR {
            // C fetches a fresh chunk here when fewer than 4 bytes remain and decodes
            // again. The input is contiguous, so only chunk_start moves.
            if size < 4 {
                self.get_chunk();
            }
            self.lookahead = DECODE_ERROR;
            self.state.lookahead_size = 1;
            return;
        }
        self.lookahead = r;
        self.state.lookahead_size = n;
    }

    /// ts_lexer_goto
    pub(crate) fn go_to(&mut self, position: Length) {
        if position.bytes != self.state.current_position.bytes {
            self.invalidate_column_data();
        }

        let st = &mut self.state;
        st.current_position = position;

        // The range loop: the default range ends at UINT32_MAX, so it is always found. Its
        // start is byte 0, point (0,0).
        if st.current_position.bytes == 0 {
            st.current_position = Length::default();
        }
        st.current_included_range_index = 0;

        if st.has_chunk
            && (st.current_position.bytes < st.chunk_start
                || st.current_position.bytes >= st.chunk_start + st.chunk_size)
        {
            self.clear_chunk();
        }

        self.state.lookahead_size = 0;
        self.lookahead = 0;
    }

    /// ts_lexer__advance, with ts_lexer__do_advance folded in: the lex functions call this
    /// once per character, and one call is cheaper than two.
    pub fn advance(&mut self, skip: bool) {
        if !self.state.has_chunk {
            return;
        }

        if self.state.logger.is_some() {
            self.log_character(if skip { "skip" } else { "consume" });
        }

        // ts_lexer__do_advance
        let st = &mut self.state;
        if st.lookahead_size != 0 {
            if self.lookahead == '\n' as i32 {
                st.current_position.extent.row += 1;
                st.current_position.extent.column = 0;
                st.column_data = ColumnData {
                    value: 0,
                    valid: true,
                };
            } else {
                let is_bom = st.current_position.bytes == 0 && self.lookahead == BYTE_ORDER_MARK;
                if !is_bom && st.column_data.valid {
                    st.column_data.value += 1;
                }
                st.current_position.extent.column += st.lookahead_size;
            }
            st.current_position.bytes += st.lookahead_size;
        }

        // The loop that steps to the next included range is dead: the position never
        // reaches the default range's end_byte of UINT32_MAX.

        if skip {
            st.token_start_position = st.current_position;
        }

        // One unsigned compare for "inside the chunk". There, an ASCII byte is
        // ts_lexer__get_lookahead's whole answer, so the call is skipped.
        let pos = st.current_position.bytes;
        if pos.wrapping_sub(st.chunk_start) < st.chunk_size {
            let c = self.src[pos as usize];
            if c < 0x80 {
                st.lookahead_size = 1;
                self.lookahead = c as i32;
                return;
            }
        } else {
            self.get_chunk();
        }
        self.get_lookahead();
    }

    /// `advance(skip)` repeated while `lookahead` is an ASCII character in `set`
    /// (gositter CONTRACT.md section 6, self-loops). `set[0]` and `set[128..]` are false.
    pub fn advance_run(&mut self, set: &'static [bool; 256], skip: bool) {
        let pos = self.state.current_position.bytes;
        if self.state.logger.is_some()
            || self.state.lookahead_size != 1
            || pos.wrapping_sub(self.state.chunk_start) >= self.state.chunk_size
        {
            // Logging wants one message per character.
            while (self.lookahead as u32) < 128
                && set[self.lookahead as usize]
                && self.state.has_chunk
            {
                self.advance(skip);
            }
            return;
        }

        // Inside the chunk with a one-byte lookahead, lookahead is src[pos] or a decode
        // error, and a byte of 0x80 or more ends the run.
        let src = self.src;
        let mut end = pos;
        let (mut rows, mut line_start) = (0u32, pos);
        while (end as usize) < src.len() && set[src[end as usize] as usize] {
            if src[end as usize] == b'\n' {
                rows += 1;
                line_start = end + 1;
            }
            end += 1;
        }
        if end == pos {
            return;
        }

        // What ts_lexer__do_advance adds up one character at a time.
        let st = &mut self.state;
        if rows > 0 {
            st.current_position.extent.row += rows;
            st.current_position.extent.column = end - line_start;
            st.column_data = ColumnData {
                value: end - line_start,
                valid: true,
            };
        } else {
            st.current_position.extent.column += end - pos;
            if st.column_data.valid {
                st.column_data.value += end - pos;
            }
        }
        st.current_position.bytes = end;

        if skip {
            st.token_start_position = st.current_position;
        }

        if end.wrapping_sub(st.chunk_start) < st.chunk_size {
            let c = src[end as usize];
            if c < 0x80 {
                st.lookahead_size = 1;
                self.lookahead = c as i32;
                return;
            }
        } else {
            self.get_chunk();
        }
        self.get_lookahead();
    }

    /// ts_lexer__mark_end
    pub fn mark_end(&mut self) {
        // The included range boundary case needs a range index above 0: dead here.
        self.state.token_end_position = self.state.current_position;
    }

    /// ts_lexer__get_column: code points since the start of the line.
    pub fn get_column(&mut self) -> u32 {
        self.state.did_get_column = true;

        if !self.state.column_data.valid {
            let goal_byte = self.state.current_position.bytes;

            // Back up to the beginning of the line.
            let cur = self.state.current_position;
            let start_of_col = Length {
                bytes: cur.bytes - cur.extent.column,
                extent: crate::Point {
                    row: cur.extent.row,
                    column: 0,
                },
            };
            self.go_to(start_of_col);
            self.set_column_data(0);
            self.get_chunk();

            if !self.eof() {
                self.get_lookahead();

                // C calls ts_lexer__do_advance, which does not log.
                let logger = self.state.logger.take();
                while self.state.current_position.bytes < goal_byte
                    && !self.eof()
                    && self.state.has_chunk
                {
                    self.advance(false);
                    if self.eof() {
                        break;
                    }
                }
                self.state.logger = logger;
            }
        }

        self.state.column_data.value
    }

    /// ts_lexer__is_at_included_range_start
    pub fn is_at_included_range_start(&self) -> bool {
        self.state.current_included_range_index < INCLUDED_RANGE_COUNT
            && self.state.current_position.bytes == 0
    }

    /// ts_lexer_reset
    pub(crate) fn reset(&mut self, position: Length) {
        if position.bytes != self.state.current_position.bytes {
            self.go_to(position);
        }
    }

    /// ts_lexer_start
    pub(crate) fn start(&mut self) {
        self.state.token_start_position = self.state.current_position;
        self.state.token_end_position = LENGTH_UNDEFINED;
        self.result_symbol = 0;
        self.state.did_get_column = false;
        if !self.eof() {
            if self.state.chunk_size == 0 {
                self.get_chunk();
            }
            if self.state.lookahead_size == 0 {
                self.get_lookahead();
            }
            if self.state.current_position.bytes == 0 {
                if self.lookahead == BYTE_ORDER_MARK {
                    self.advance(true);
                }
                self.set_column_data(0);
            }
        }
    }

    /// ts_lexer_finish
    pub(crate) fn finish(&mut self, lookahead_end_byte: &mut u32) {
        if length_is_undefined(self.state.token_end_position) {
            self.mark_end();
        }

        // Only true at an included range boundary; kept as written.
        if self.state.token_end_position.bytes < self.state.token_start_position.bytes {
            self.state.token_start_position = self.state.token_end_position;
        }

        let mut current_lookahead_end_byte = self.state.current_position.bytes + 1;

        // The decoder may have looked at up to 4 bytes to call a sequence invalid.
        if self.lookahead == DECODE_ERROR {
            current_lookahead_end_byte += 4;
        }

        if current_lookahead_end_byte > *lookahead_end_byte {
            *lookahead_end_byte = current_lookahead_end_byte;
        }
    }
}

/// gositter `lexer_test.go`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
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
    type DecodeCase<'a> = (&'a str, &'a [u8], &'a [(i32, u32)]);

    #[test]
    fn lexer_decode() {
        let bad = (DECODE_ERROR, 1);
        let cases: &[DecodeCase<'_>] = &[
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
    fn lexer_empty_input() {
        let l = new_test_lexer(b"");
        assert!(l.eof());
        assert_eq!(l.lookahead, 0);
        assert!(!l.is_at_included_range_start());
    }

    #[test]
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
    fn lexer_is_at_included_range_start() {
        let mut l = new_test_lexer(b"ab");
        assert!(l.is_at_included_range_start(), "want true at byte 0");
        l.advance(false);
        assert!(!l.is_at_included_range_start(), "want false at byte 1");
    }

    #[test]
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
