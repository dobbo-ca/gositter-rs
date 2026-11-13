//! Port of gositter `grammars/python/scanner.go` (tree-sitter-python v0.25.0 `src/scanner.c`): the
//! scanner state, `serialize`, `deserialize`, `reset` and `scan`.

use gositter::{Lexer, Symbol};

/// enum TokenType. The scanner's own order; it is not the order of the grammar's externals for the
/// three closers, and C only ORs those together.
#[allow(dead_code)] // Comment is in C's enum; nothing reads it
#[derive(Clone, Copy)]
enum Tok {
    Newline,
    Indent,
    Dedent,
    StringStart,
    StringContent,
    EscapeInterpolation,
    StringEnd,
    Comment,
    CloseParen,
    CloseBracket,
    CloseBrace,
    Except,
}

// enum Flags. A delimiter is one byte of these.
const FLAG_SINGLE_QUOTE: u8 = 1 << 0;
const FLAG_DOUBLE_QUOTE: u8 = 1 << 1;
const FLAG_BACK_QUOTE: u8 = 1 << 2;
const FLAG_RAW: u8 = 1 << 3;
const FLAG_FORMAT: u8 = 1 << 4;
const FLAG_TRIPLE: u8 = 1 << 5;
const FLAG_BYTES: u8 = 1 << 6;

/// end_character
fn end_character(delimiter: u8) -> i32 {
    if delimiter & FLAG_SINGLE_QUOTE != 0 {
        return b'\'' as i32;
    }
    if delimiter & FLAG_DOUBLE_QUOTE != 0 {
        return b'"' as i32;
    }
    if delimiter & FLAG_BACK_QUOTE != 0 {
        return b'`' as i32;
    }
    0
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanner {
    indents: Vec<u16>,
    delimiters: Vec<u8>,
    inside_interpolated_string: bool,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        let mut s = Scanner {
            indents: Vec::new(),
            delimiters: Vec::new(),
            inside_interpolated_string: false,
        };
        s.deserialize(&[]);
        s
    }

    /// The state `new` returns, with the buffers kept.
    pub fn reset(&mut self) {
        self.inside_interpolated_string = false;
        self.deserialize(&[]);
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = u8::from(self.inside_interpolated_string);
        let delimiter_count = self.delimiters.len().min(255);
        buf[1] = delimiter_count as u8;
        let mut size = 2;

        buf[size..size + delimiter_count].copy_from_slice(&self.delimiters[..delimiter_count]);
        size += delimiter_count;

        // C tests only size < 1024 and then writes two bytes. With an odd delimiter count and 511+
        // indents (511 nested blocks, then a string start) it writes one byte past the buffer and
        // returns 1025; the runtime's ts_assert(length <= 1024) then aborts the process. There is
        // no C tree to match, so stop one pair earlier.
        for indent in self.indents.iter().skip(1) {
            if size + 1 >= buf.len() {
                break;
            }
            buf[size..size + 2].copy_from_slice(&indent.to_le_bytes());
            size += 2;
        }

        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        self.delimiters.clear();
        self.indents.clear();
        self.indents.push(0);
        // inside_interpolated_string is not reset for an empty buffer; C leaves it.

        if !buf.is_empty() {
            self.inside_interpolated_string = buf[0] != 0;

            let delimiter_count = usize::from(buf[1]);
            let mut size = 2;
            self.delimiters
                .extend_from_slice(&buf[size..size + delimiter_count]);
            size += delimiter_count;

            self.indents.extend(
                buf[size..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c)),
            );
        }
    }

    /// tree_sitter_python_external_scanner_scan
    fn scan(&mut self, l: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let valid = |t: Tok| valid_symbols[t as usize];
        let error_recovery_mode = valid(Tok::StringContent) && valid(Tok::Indent);
        let within_brackets =
            valid(Tok::CloseBrace) || valid(Tok::CloseParen) || valid(Tok::CloseBracket);

        // C and gositter set advanced_once = true only right before two returns, so it is
        // always false where it is read; the store is left out (clippy: unused_assignments).
        let advanced_once = false;
        if valid(Tok::EscapeInterpolation)
            && !self.delimiters.is_empty()
            && (l.lookahead == b'{' as i32 || l.lookahead == b'}' as i32)
            && !error_recovery_mode
        {
            let delimiter = self.delimiters[self.delimiters.len() - 1];
            if delimiter & FLAG_FORMAT != 0 {
                l.mark_end();
                let is_left_brace = l.lookahead == b'{' as i32;
                l.advance(false);
                if (l.lookahead == b'{' as i32 && is_left_brace)
                    || (l.lookahead == b'}' as i32 && !is_left_brace)
                {
                    l.advance(false);
                    l.mark_end();
                    l.result_symbol = Tok::EscapeInterpolation as Symbol;
                    return true;
                }
                return false;
            }
        }

        if valid(Tok::StringContent) && !self.delimiters.is_empty() && !error_recovery_mode {
            let delimiter = self.delimiters[self.delimiters.len() - 1];
            let end_char = end_character(delimiter);
            let mut has_content = advanced_once;
            while l.lookahead != 0 {
                if (advanced_once || l.lookahead == b'{' as i32 || l.lookahead == b'}' as i32)
                    && delimiter & FLAG_FORMAT != 0
                {
                    l.mark_end();
                    l.result_symbol = Tok::StringContent as Symbol;
                    return has_content;
                }
                if l.lookahead == b'\\' as i32 {
                    if delimiter & FLAG_RAW != 0 {
                        // Step over the backslash.
                        l.advance(false);
                        // Step over any escaped quotes.
                        if l.lookahead == end_character(delimiter) || l.lookahead == b'\\' as i32 {
                            l.advance(false);
                        }
                        // Step over newlines
                        if l.lookahead == b'\r' as i32 {
                            l.advance(false);
                            if l.lookahead == b'\n' as i32 {
                                l.advance(false);
                            }
                        } else if l.lookahead == b'\n' as i32 {
                            l.advance(false);
                        }
                        continue;
                    }
                    if delimiter & FLAG_BYTES != 0 {
                        l.mark_end();
                        l.advance(false);
                        if l.lookahead == b'N' as i32
                            || l.lookahead == b'u' as i32
                            || l.lookahead == b'U' as i32
                        {
                            // In a bytes string \N{...}, \uXXXX and \UXXXXXXXX
                            // are not escape sequences.
                            l.advance(false);
                        } else {
                            l.result_symbol = Tok::StringContent as Symbol;
                            return has_content;
                        }
                    } else {
                        l.mark_end();
                        l.result_symbol = Tok::StringContent as Symbol;
                        return has_content;
                    }
                } else if l.lookahead == end_char {
                    if delimiter & FLAG_TRIPLE != 0 {
                        l.mark_end();
                        l.advance(false);
                        if l.lookahead == end_char {
                            l.advance(false);
                            if l.lookahead == end_char {
                                if has_content {
                                    l.result_symbol = Tok::StringContent as Symbol;
                                } else {
                                    l.advance(false);
                                    l.mark_end();
                                    self.delimiters.pop();
                                    l.result_symbol = Tok::StringEnd as Symbol;
                                    self.inside_interpolated_string = false;
                                }
                                return true;
                            }
                            l.mark_end();
                            l.result_symbol = Tok::StringContent as Symbol;
                            return true;
                        }
                        l.mark_end();
                        l.result_symbol = Tok::StringContent as Symbol;
                        return true;
                    }
                    if has_content {
                        l.result_symbol = Tok::StringContent as Symbol;
                    } else {
                        l.advance(false);
                        self.delimiters.pop();
                        l.result_symbol = Tok::StringEnd as Symbol;
                        self.inside_interpolated_string = false;
                    }
                    l.mark_end();
                    return true;
                } else if l.lookahead == b'\n' as i32 && has_content && delimiter & FLAG_TRIPLE == 0
                {
                    return false;
                }
                l.advance(false);
                has_content = true;
            }
        }

        l.mark_end();

        let mut found_end_of_line = false;
        let mut indent_length: u16 = 0; // wraps at 65536 as in C
        let mut first_comment_indent_length: i32 = -1;
        loop {
            if l.lookahead == b'\n' as i32 {
                found_end_of_line = true;
                indent_length = 0;
                l.advance(true);
            } else if l.lookahead == b' ' as i32 {
                indent_length = indent_length.wrapping_add(1);
                l.advance(true);
            } else if l.lookahead == b'\r' as i32 || l.lookahead == 0x0C {
                indent_length = 0;
                l.advance(true);
            } else if l.lookahead == b'\t' as i32 {
                indent_length = indent_length.wrapping_add(8);
                l.advance(true);
            } else if l.lookahead == b'#' as i32
                && (valid(Tok::Indent)
                    || valid(Tok::Dedent)
                    || valid(Tok::Newline)
                    || valid(Tok::Except))
            {
                // No EOL yet: a comment after an expression. No indent or
                // dedent token for it.
                if !found_end_of_line {
                    return false;
                }
                if first_comment_indent_length == -1 {
                    first_comment_indent_length = i32::from(indent_length);
                }
                while l.lookahead != 0 && l.lookahead != b'\n' as i32 {
                    l.advance(true);
                }
                l.advance(true);
                indent_length = 0;
            } else if l.lookahead == b'\\' as i32 {
                l.advance(true);
                if l.lookahead == b'\r' as i32 {
                    l.advance(true);
                }
                if l.lookahead == b'\n' as i32 || l.eof() {
                    l.advance(true);
                } else {
                    return false;
                }
            } else if l.eof() {
                indent_length = 0;
                found_end_of_line = true;
                break;
            } else {
                break;
            }
        }

        if found_end_of_line {
            if let Some(&current_indent_length) = self.indents.last() {
                if valid(Tok::Indent) && indent_length > current_indent_length {
                    self.indents.push(indent_length);
                    l.result_symbol = Tok::Indent as Symbol;
                    return true;
                }

                let next_tok_is_string_start = l.lookahead == b'"' as i32
                    || l.lookahead == b'\'' as i32
                    || l.lookahead == b'`' as i32;

                if (valid(Tok::Dedent)
                    || (!valid(Tok::Newline)
                        && !(valid(Tok::StringStart) && next_tok_is_string_start)
                        && !within_brackets))
                    && indent_length < current_indent_length
                    && !self.inside_interpolated_string
                    // Hold the dedent until comments at the block's own
                    // indentation are consumed.
                    && first_comment_indent_length < i32::from(current_indent_length)
                {
                    self.indents.pop();
                    l.result_symbol = Tok::Dedent as Symbol;
                    return true;
                }
            }

            if valid(Tok::Newline) && !error_recovery_mode {
                l.result_symbol = Tok::Newline as Symbol;
                return true;
            }
        }

        if first_comment_indent_length == -1 && valid(Tok::StringStart) {
            let mut delimiter: u8 = 0;

            let mut has_flags = false;
            while l.lookahead != 0 {
                if l.lookahead == b'f' as i32
                    || l.lookahead == b'F' as i32
                    || l.lookahead == b't' as i32
                    || l.lookahead == b'T' as i32
                {
                    delimiter |= FLAG_FORMAT;
                } else if l.lookahead == b'r' as i32 || l.lookahead == b'R' as i32 {
                    delimiter |= FLAG_RAW;
                } else if l.lookahead == b'b' as i32 || l.lookahead == b'B' as i32 {
                    delimiter |= FLAG_BYTES;
                } else if l.lookahead != b'u' as i32 && l.lookahead != b'U' as i32 {
                    break;
                }
                has_flags = true;
                l.advance(false);
            }

            if l.lookahead == b'`' as i32 {
                delimiter |= FLAG_BACK_QUOTE;
                l.advance(false);
                l.mark_end();
            } else if l.lookahead == b'\'' as i32 {
                delimiter |= FLAG_SINGLE_QUOTE;
                l.advance(false);
                l.mark_end();
                if l.lookahead == b'\'' as i32 {
                    l.advance(false);
                    if l.lookahead == b'\'' as i32 {
                        l.advance(false);
                        l.mark_end();
                        delimiter |= FLAG_TRIPLE;
                    }
                }
            } else if l.lookahead == b'"' as i32 {
                delimiter |= FLAG_DOUBLE_QUOTE;
                l.advance(false);
                l.mark_end();
                if l.lookahead == b'"' as i32 {
                    l.advance(false);
                    if l.lookahead == b'"' as i32 {
                        l.advance(false);
                        l.mark_end();
                        delimiter |= FLAG_TRIPLE;
                    }
                }
            }

            if end_character(delimiter) != 0 {
                self.delimiters.push(delimiter);
                l.result_symbol = Tok::StringStart as Symbol;
                self.inside_interpolated_string = delimiter & FLAG_FORMAT != 0;
                return true;
            }
            if has_flags {
                return false;
            }
        }

        false
    }
}

impl gositter::ExternalScanner for Scanner {
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        Scanner::scan(self, lexer, valid_symbols)
    }

    fn serialize(&self, buf: &mut [u8; gositter::SERIALIZATION_BUFFER_SIZE]) -> usize {
        Scanner::serialize(self, buf)
    }

    fn deserialize(&mut self, buf: &[u8]) {
        Scanner::deserialize(self, buf)
    }

    fn reset(&mut self) -> bool {
        Scanner::reset(self);
        true
    }
}

#[allow(dead_code)] // used by the generated language() (Task 10)
pub(crate) fn new() -> Box<dyn gositter::ExternalScanner> {
    Box::new(Scanner::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; crate::SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    fn round_trip(s: &Scanner) -> (Scanner, Vec<u8>) {
        let b = serialize(s);
        // Dirty target: deserialize must replace all of it.
        let mut got = Scanner {
            indents: vec![9, 9],
            delimiters: vec![0xFF],
            inside_interpolated_string: true,
        };
        got.deserialize(&b);
        let b2 = serialize(&got);
        assert_eq!(b, b2, "serialize after deserialize differs");
        (got, b)
    }

    // TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(serialize(&s), [0, 0], "fresh scanner");

        s.indents = vec![0, 4, 8, 0x1234];
        s.delimiters = vec![
            FLAG_DOUBLE_QUOTE | FLAG_FORMAT,
            FLAG_SINGLE_QUOTE | FLAG_TRIPLE | FLAG_RAW,
        ];
        s.inside_interpolated_string = true;
        let (got, b) = round_trip(&s);
        // inside, count, delimiters, then indents[1:] little-endian.
        let want = [1, 2, 0x12, 0x29, 4, 0, 8, 0, 0x34, 0x12];
        assert_eq!(b, want);
        assert!(
            got.indents == s.indents
                && got.delimiters == s.delimiters
                && got.inside_interpolated_string,
            "state = {got:?}, want {s:?}"
        );
    }

    // TestDeserializeEmpty
    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            indents: vec![0, 4],
            delimiters: vec![1],
            inside_interpolated_string: true,
        };
        s.deserialize(&[]);
        assert!(s.indents == [0] && s.delimiters.is_empty(), "state = {s:?}");
        // C does not touch inside_interpolated_string when length is 0.
        assert!(
            s.inside_interpolated_string,
            "inside_interpolated_string was reset"
        );
    }

    // TestSerializeLimits
    #[test]
    fn serialize_limits() {
        // 255 delimiters at most; the rest is dropped.
        let mut s = Scanner::new();
        s.delimiters = vec![FLAG_DOUBLE_QUOTE; 300];
        let (got, b) = round_trip(&s);
        assert!(
            b[1] == 255 && b.len() == 2 + 255 && got.delimiters.len() == 255,
            "len = {}, count byte = {}",
            b.len(),
            b[1]
        );

        // Even header: C fills the buffer exactly, 511 indents.
        let mut s = Scanner::new();
        s.indents.extend(1..=2000u16);
        let (got, b) = round_trip(&s);
        assert!(
            b.len() == 1024 && got.indents.len() == 512 && got.indents[511] == 511,
            "len = {}, indents = {}",
            b.len(),
            got.indents.len()
        );

        // Odd header: C overruns by one byte here. The port stops at 1023.
        s.delimiters = vec![FLAG_DOUBLE_QUOTE];
        let (_, b) = round_trip(&s);
        assert_eq!(b.len(), 1023);
    }

    // Reset: the state `new` returns.
    pub(super) fn check_reset(s: &mut Scanner, what: &str) {
        s.reset();
        assert_eq!(*s, Scanner::new(), "{what}: after reset");
        assert_eq!(
            serialize(s),
            serialize(&Scanner::new()),
            "{what}: reset and new serialize differently"
        );
    }

    // The hand-made half of TestResetIsNew (reset_test.go); the parser half is in `parser_tests`.
    // inside_interpolated_string is a field deserialize(&[]) leaves alone.
    #[test]
    fn reset_hand_made() {
        let mut s = Scanner {
            indents: vec![0, 4, 8],
            delimiters: vec![FLAG_DOUBLE_QUOTE | FLAG_FORMAT, FLAG_SINGLE_QUOTE],
            inside_interpolated_string: true,
        };
        check_reset(&mut s, "hand-made");
    }
}

#[cfg(test)]
mod parser_tests {
    use super::Scanner;
    use super::tests::check_reset;

    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    pub(super) const RESET_INPUTS: [&str; 12] = [
        "",
        "x = 1\n",
        "if a:\n    if b:\n        if c:\n            pass\n",
        "def f():\n    return f\"a{b}c{{d}}\"\n",
        "x = f\"a{f'b{c}'}d\"\n",
        "if a:\n    if b:\n        x = f\"a{b",
        "def f():\n    s = '''text\n  more",
        "class A:\n    def f(self):\n        if x:\n            y = rb\"raw",
        "x = (1,\n   2,\n",
        "if a:\n  if b:\n    f(\"\"\"doc {x}\n",
        "try:\n    pass\nexcept* E:\n    pass\n",
        "x = f\"{a!r:>{w}}\" + (\n",
    ];

    // B parsed after A on one parser has the tree of B on a fresh parser.
    // TestReusedScannerSameTree (reset_test.go)
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reused_scanner_same_tree() {
        let mut reused = gositter::Parser::new();
        reused.set_language(language()).unwrap();
        for a in RESET_INPUTS {
            for b in RESET_INPUTS {
                reused.parse(a.as_bytes());
                let got = reused.parse(b.as_bytes()).unwrap().root_node().to_sexp();
                let mut fresh = gositter::Parser::new();
                fresh.set_language(language()).unwrap();
                let want = fresh.parse(b.as_bytes()).unwrap().root_node().to_sexp();
                assert_eq!(got, want, "{b:?} after {a:?}");
            }
        }
    }

    /// The scanner the parser keeps for its next parse (gositter `KeptScanner`).
    fn kept(p: &gositter::Parser) -> &Scanner {
        let s: &dyn std::any::Any = p.kept_scanner().expect("kept scanner");
        s.downcast_ref::<Scanner>().expect("this grammar's scanner")
    }

    // TestResetIsNew (reset_test.go). Go's `kept` is the scanner the parser made; here it is
    // read back with kept_scanner, and a clone is reset since the borrow is shared.
    // Vec equality ignores capacity, so Go's `norm` is not needed.
    #[test]
    #[ignore = "gositter-rs-2f0.11"]
    fn reset_is_new() {
        let mut dirty = 0;
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        for input in RESET_INPUTS {
            for budget in [0u32, 1] {
                p.reset();
                p.parse_budget(input.as_bytes(), budget);
                let mut s = kept(&p).clone();
                if s != Scanner::new() {
                    dirty += 1;
                }
                check_reset(&mut s, input.trim());
                // Reset must hold after the scanner is reused too.
                p.reset();
                p.parse_budget(input.as_bytes(), budget);
            }
        }
        assert_ne!(
            dirty, 0,
            "no input left the scanner dirty, the test checks nothing"
        );
    }
}
