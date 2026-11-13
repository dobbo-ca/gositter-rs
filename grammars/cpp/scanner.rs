//! Port of gositter `grammars/cpp/scanner.go` (tree-sitter-cpp v0.23.4 `src/scanner.c`): the
//! scanner state, `serialize`, `deserialize`, `reset` and `scan`.

use gositter::{Lexer, Symbol};

/// enum TokenType
#[derive(Clone, Copy)]
enum Tok {
    RawStringDelimiter,
    RawStringContent,
}

/// iswspace in the "C" locale, which is what the C build runs in: checked on darwin for every code
/// point and -1, only these six are space.
fn iswspace(c: i32) -> bool {
    c == b' ' as i32 || (b'\t' as i32..=b'\r' as i32).contains(&c)
}

/// `MAX_DELIMITER_LENGTH`.
const MAX_DELIMITER_LENGTH: usize = 16;

/// `sizeof(wchar_t)`: 4 on every target of the C build. The serialized state is `delimiter_length`
/// of them, little-endian.
const SIZEOF_WCHAR: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    delimiter_length: u8,
    /// `wchar_t` is a signed 32-bit int.
    delimiter: [i32; MAX_DELIMITER_LENGTH],
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// The state `new` returns.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        let mut size = 0;
        for c in &self.delimiter[..usize::from(self.delimiter_length)] {
            buf[size..size + SIZEOF_WCHAR].copy_from_slice(&c.to_le_bytes());
            size += SIZEOF_WCHAR;
        }
        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        // C asserts length % sizeof(wchar_t) == 0. The runtime only passes back what `serialize`
        // wrote, so at most 64 bytes in whole wchar_t units.
        self.delimiter_length = (buf.len() / SIZEOF_WCHAR) as u8;
        // Entries past the new length keep their old value, as with C's memcpy. Nothing reads them.
        for (d, c) in self
            .delimiter
            .iter_mut()
            .zip(buf.as_chunks::<SIZEOF_WCHAR>().0)
        {
            *d = i32::from_le_bytes(*c);
        }
    }

    /// scan_raw_string_delimiter
    fn scan_raw_string_delimiter(&mut self, l: &mut Lexer<'_>) -> bool {
        if self.delimiter_length > 0 {
            // Closing delimiter: must match the opening one exactly.
            for c in &self.delimiter[..usize::from(self.delimiter_length)] {
                if l.lookahead != *c {
                    return false;
                }
                l.advance(false);
            }
            self.reset();
            return true;
        }

        // Opening delimiter. A failed scan leaves delimiter_length dirty, as C does; the runtime
        // deserializes before the next scan.
        loop {
            if usize::from(self.delimiter_length) >= MAX_DELIMITER_LENGTH
                || l.eof()
                || l.lookahead == b'\\' as i32
                || iswspace(l.lookahead)
            {
                return false;
            }
            if l.lookahead == b'(' as i32 {
                // Empty delimiter: fail, the grammar has a rule without one.
                return self.delimiter_length > 0;
            }
            self.delimiter[usize::from(self.delimiter_length)] = l.lookahead;
            self.delimiter_length += 1;
            l.advance(false);
        }
    }

    /// scan_raw_string_content
    fn scan_raw_string_content(&mut self, l: &mut Lexer<'_>) -> bool {
        // Progress through the delimiter since the last ')'.
        let mut delimiter_index: i32 = -1;
        loop {
            // At EOF the content ends there: an incomplete raw_string_literal.
            if l.eof() {
                l.mark_end();
                return true;
            }

            if delimiter_index >= 0 {
                if delimiter_index == i32::from(self.delimiter_length) {
                    if l.lookahead == b'"' as i32 {
                        return true;
                    }
                    delimiter_index = -1;
                } else if l.lookahead == self.delimiter[delimiter_index as usize] {
                    delimiter_index += 1;
                } else {
                    delimiter_index = -1;
                }
            }

            if delimiter_index == -1 && l.lookahead == b')' as i32 {
                // The token excludes the )delimiter" part.
                l.mark_end();
                delimiter_index = 0;
            }

            l.advance(false);
        }
    }

    /// tree_sitter_cpp_external_scanner_scan
    fn scan(&mut self, l: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool {
        if valid_symbols[Tok::RawStringDelimiter as usize]
            && valid_symbols[Tok::RawStringContent as usize]
        {
            // error recovery
            return false;
        }

        // No whitespace skipping: the raw-string grammar is space-sensitive.
        if valid_symbols[Tok::RawStringDelimiter as usize] {
            l.result_symbol = Tok::RawStringDelimiter as Symbol;
            return self.scan_raw_string_delimiter(l);
        }

        if valid_symbols[Tok::RawStringContent as usize] {
            l.result_symbol = Tok::RawStringContent as Symbol;
            return self.scan_raw_string_content(l);
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

    // TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert!(serialize(&s).is_empty(), "fresh scanner is not empty");

        // 'a', U+00E9, U+1F600, a real NUL and TS_DECODE_ERROR (-1), as 4-byte little-endian
        // wchar_t.
        s.delimiter_length = 5;
        s.delimiter[..5].copy_from_slice(&[b'a' as i32, 0xE9, 0x1F600, 0, -1]);
        let want: Vec<u8> = vec![
            b'a', 0, 0, 0, //
            0xE9, 0, 0, 0, //
            0x00, 0xF6, 0x01, 0, //
            0, 0, 0, 0, //
            0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let b = serialize(&s);
        assert_eq!(b, want);

        // Dirty target: only the first delimiter_length entries count.
        let mut got = Scanner {
            delimiter_length: 16,
            delimiter: [b'z' as i32; MAX_DELIMITER_LENGTH],
        };
        got.deserialize(&b);
        assert!(
            got.delimiter_length == 5 && got.delimiter[4] == -1 && got.delimiter[2] == 0x1F600,
            "state = {got:?}"
        );
        assert_eq!(b, serialize(&got), "serialize after deserialize differs");
    }

    // TestDeserializeEmpty
    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            delimiter_length: 2,
            ..Scanner::new()
        };
        (s.delimiter[0], s.delimiter[1]) = (b'x' as i32, b'y' as i32);
        s.deserialize(&[]);
        assert!(
            s.delimiter_length == 0 && serialize(&s).is_empty(),
            "state = {s:?}"
        );
        // C does not memcpy when length is 0, so the old chars stay.
        assert_eq!(s.delimiter[0], b'x' as i32, "delimiter was cleared");
    }

    // TestSerializeMax
    #[test]
    fn serialize_max() {
        let mut s = Scanner::new();
        s.delimiter_length = MAX_DELIMITER_LENGTH as u8;
        for (i, d) in s.delimiter.iter_mut().enumerate() {
            *d = b'A' as i32 + i as i32;
        }
        let b = serialize(&s);
        assert_eq!(b.len(), 64);
        let mut got = Scanner::new();
        got.deserialize(&b);
        assert_eq!(got, s);
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
    #[test]
    fn reset_hand_made() {
        let mut s = Scanner::new();
        s.delimiter_length = 3;
        (s.delimiter[0], s.delimiter[15]) = (b'a' as i32, b'z' as i32);
        check_reset(&mut s, "hand-made");
    }
}

#[cfg(test)]
mod parser_tests {
    use super::Scanner;
    use super::tests::check_reset;

    // Raw strings through the parser: the scanner is wired in and its tokens land where the C
    // grammar puts them. TestRawStrings (scanner_test.go)
    #[test]
    fn raw_strings() {
        for (src, want) in [
            (
                r#"auto s = R"x(a)y)x";"#,
                "(raw_string_literal delimiter: (raw_string_delimiter) (raw_string_content) (raw_string_delimiter))",
            ),
            (
                r#"auto s = R"(a"b)";"#,
                "(raw_string_literal (raw_string_content))",
            ),
            // 17 chars: over MAX_DELIMITER_LENGTH, no delimiter token.
            (
                r#"auto s = R"abcdefghijklmnopq(a)abcdefghijklmnopq";"#,
                "ERROR",
            ),
            // Unterminated: content runs to EOF.
            (r#"auto s = R"x(abc"#, "(raw_string_content)"),
        ] {
            let mut p = gositter::Parser::new();
            p.set_language(super::super::language()).unwrap();
            let tree = p.parse(src.as_bytes()).unwrap();
            let got = tree.root_node().to_sexp();
            assert!(got.contains(want), "{src}\n got {got}\nwant {want}");
        }
    }

    pub(super) const RESET_INPUTS: [&str; 9] = [
        "",
        "int a;\n",
        "auto s = R\"x(body)x\";\n",
        "auto s = R\"(body)\";\n",
        "auto s = R\"abc(body without an end",
        "auto s = R\"abcdefghijklmnopq(x)\";\n",
        "auto s = R\"ab cd(x)\";\n",
        "auto s = R\"x(a)y\" R\"y(b",
        "auto s = LR\"d(\n\n",
    ];

    // B parsed after A on one parser has the tree of B on a fresh parser.
    // TestReusedScannerSameTree (reset_test.go)
    #[test]
    fn reused_scanner_same_tree() {
        let mut reused = gositter::Parser::new();
        reused.set_language(super::super::language()).unwrap();
        for a in RESET_INPUTS {
            for b in RESET_INPUTS {
                reused.parse(a.as_bytes());
                let got = reused.parse(b.as_bytes()).unwrap().root_node().to_sexp();
                let mut fresh = gositter::Parser::new();
                fresh.set_language(super::super::language()).unwrap();
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
    fn reset_is_new() {
        let mut dirty = 0;
        let mut p = gositter::Parser::new();
        p.set_language(super::super::language()).unwrap();
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
