//! Port of the scanner state of `grammars/ruby/scanner.go` (tree-sitter-ruby v0.23.1
//! `src/scanner.c`): `new`, `serialize` and `deserialize`. `scan` and the helpers only it uses
//! (`isw*`, `scan_whitespace`, ...) wait for the runtime `Lexer` (Task 12).

use crate::SERIALIZATION_BUFFER_SIZE;

// enum TokenType. The tokens `serialize` and the tests do not touch are read by `scan` (Task 12).
#[allow(dead_code)]
mod tok {
    pub const LINE_BREAK: u16 = 0;
    pub const NO_LINE_BREAK: u16 = 1;

    // Delimited literals
    pub const SIMPLE_SYMBOL: u16 = 2;
    pub const STRING_START: u16 = 3;
    pub const SYMBOL_START: u16 = 4;
    pub const SUBSHELL_START: u16 = 5;
    pub const REGEX_START: u16 = 6;
    pub const STRING_ARRAY_START: u16 = 7;
    pub const SYMBOL_ARRAY_START: u16 = 8;
    pub const HEREDOC_BODY_START: u16 = 9;
    pub const STRING_CONTENT: u16 = 10;
    pub const HEREDOC_CONTENT: u16 = 11;
    pub const STRING_END: u16 = 12;
    pub const HEREDOC_BODY_END: u16 = 13;
    pub const HEREDOC_START: u16 = 14;

    // Whitespace-sensitive tokens
    pub const FORWARD_SLASH: u16 = 15;
    pub const BLOCK_AMPERSAND: u16 = 16;
    pub const SPLAT_STAR: u16 = 17;
    pub const UNARY_MINUS: u16 = 18;
    pub const UNARY_MINUS_NUM: u16 = 19;
    pub const BINARY_MINUS: u16 = 20;
    pub const BINARY_STAR: u16 = 21;
    pub const SINGLETON_CLASS_LEFT_ANGLE_LEFT_ANGLE: u16 = 22;
    pub const HASH_KEY_SYMBOL: u16 = 23;
    pub const IDENTIFIER_SUFFIX: u16 = 24;
    pub const CONSTANT_SUFFIX: u16 = 25;
    pub const HASH_SPLAT_STAR_STAR: u16 = 26;
    pub const BINARY_STAR_STAR: u16 = 27;
    pub const ELEMENT_REFERENCE_BRACKET: u16 = 28;
    pub const SHORT_INTERPOLATION: u16 = 29;

    pub const NONE: u16 = 30;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Literal {
    pub typ: u16,
    pub open_delimiter: i32,
    pub close_delimiter: i32,
    pub nesting_depth: i32,
    pub allows_interpolation: bool,
}

/// Heredoc. `word` is C's `Array(char)`: each element is the low byte of a code point and is
/// read back as a char (`cchar::value`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heredoc {
    pub word: Vec<u8>,
    pub end_word_indentation_allowed: bool,
    pub allows_interpolation: bool,
    pub started: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scanner {
    pub has_leading_whitespace: bool,
    pub literal_stack: Vec<Literal>,
    pub open_heredocs: Vec<Heredoc>,
}

impl Scanner {
    /// `tree_sitter_ruby_external_scanner_create`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `reset`.
    fn reset(&mut self) {
        self.literal_stack.clear();
        self.open_heredocs.clear();
    }

    /// `array_delete(&heredoc->word); array_erase(&scanner->open_heredocs, 0)`.
    pub fn erase_first_heredoc(&mut self) {
        self.open_heredocs.remove(0);
    }

    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        if self.literal_stack.len() * 5 + 2 >= SERIALIZATION_BUFFER_SIZE {
            return 0;
        }

        buf[0] = self.literal_stack.len() as u8;
        let mut size = 1;
        for l in &self.literal_stack {
            buf[size] = l.typ as u8;
            buf[size + 1] = l.open_delimiter as u8;
            buf[size + 2] = l.close_delimiter as u8;
            buf[size + 3] = l.nesting_depth as u8;
            buf[size + 4] = u8::from(l.allows_interpolation);
            size += 5;
        }

        buf[size] = self.open_heredocs.len() as u8;
        size += 1;
        for h in &self.open_heredocs {
            if size + 2 + h.word.len() >= SERIALIZATION_BUFFER_SIZE {
                return 0;
            }
            // Not in C. C tests size+2 but writes 4 bytes plus the word, so it can write 1 byte
            // past the buffer, and it stores the word length in one byte, so a word over 255
            // bytes reads back short. Both end in an assert abort in the C build. Drop the
            // state the way C does for a state that is too big.
            if size + 4 + h.word.len() > SERIALIZATION_BUFFER_SIZE || h.word.len() > 255 {
                return 0;
            }
            buf[size] = u8::from(h.end_word_indentation_allowed);
            buf[size + 1] = u8::from(h.allows_interpolation);
            buf[size + 2] = u8::from(h.started);
            buf[size + 3] = h.word.len() as u8;
            size += 4;
            buf[size..size + h.word.len()].copy_from_slice(&h.word);
            size += h.word.len();
        }

        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        self.has_leading_whitespace = false;
        self.reset();

        if buf.is_empty() {
            return;
        }

        let literal_depth = usize::from(buf[0]);
        let mut size = 1;
        for _ in 0..literal_depth {
            self.literal_stack.push(Literal {
                typ: u16::from(buf[size]),
                open_delimiter: i32::from(buf[size + 1]),
                close_delimiter: i32::from(buf[size + 2]),
                nesting_depth: i32::from(buf[size + 3]),
                allows_interpolation: buf[size + 4] != 0,
            });
            size += 5;
        }

        let open_heredoc_count = usize::from(buf[size]);
        size += 1;
        for _ in 0..open_heredoc_count {
            let word_length = usize::from(buf[size + 3]);
            self.open_heredocs.push(Heredoc {
                end_word_indentation_allowed: buf[size] != 0,
                allows_interpolation: buf[size + 1] != 0,
                started: buf[size + 2] != 0,
                word: buf[size + 4..size + 4 + word_length].to_vec(),
            });
            size += 4 + word_length;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    // Deserialize into a dirty scanner, then compare state and bytes.
    fn round_trip(s: &Scanner) -> Vec<u8> {
        let b = serialize(s);
        let mut got = Scanner {
            has_leading_whitespace: true,
            literal_stack: vec![Literal {
                typ: tok::REGEX_START,
                nesting_depth: 9,
                ..Literal::default()
            }],
            open_heredocs: vec![
                Heredoc {
                    word: b"OLD".to_vec(),
                    started: true,
                    ..Heredoc::default()
                },
                Heredoc {
                    word: b"OLDER".to_vec(),
                    ..Heredoc::default()
                },
            ],
        };
        got.deserialize(&b);
        assert!(
            !got.has_leading_whitespace,
            "has_leading_whitespace not reset"
        );
        assert_eq!(b, serialize(&got), "serialize after deserialize differs");
        assert!(
            got.literal_stack.len() == s.literal_stack.len()
                && got.open_heredocs.len() == s.open_heredocs.len(),
            "state = {got:?}, want {s:?}"
        );
        for (i, (w, g)) in s.open_heredocs.iter().zip(&got.open_heredocs).enumerate() {
            assert!(
                w.word == g.word
                    && w.started == g.started
                    && w.allows_interpolation == g.allows_interpolation
                    && w.end_word_indentation_allowed == g.end_word_indentation_allowed,
                "heredoc {i} = {g:?}, want {w:?}"
            );
        }
        b
    }

    // scanner_test.go TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(serialize(&s), [0, 0], "fresh scanner");

        s.has_leading_whitespace = true; // not serialized
        s.literal_stack = vec![
            Literal {
                typ: tok::STRING_START,
                open_delimiter: b'"' as i32,
                close_delimiter: b'"' as i32,
                nesting_depth: 1,
                allows_interpolation: true,
            },
            Literal {
                typ: tok::STRING_ARRAY_START,
                open_delimiter: b'(' as i32,
                close_delimiter: b')' as i32,
                nesting_depth: 3,
                allows_interpolation: false,
            },
        ];
        s.open_heredocs = vec![
            Heredoc {
                word: b"EOS".to_vec(),
                end_word_indentation_allowed: true,
                allows_interpolation: true,
                started: true,
            },
            Heredoc {
                word: vec![0xFF, b'x'],
                ..Heredoc::default()
            },
        ];
        let want = [
            2,
            tok::STRING_START as u8,
            b'"',
            b'"',
            1,
            1,
            tok::STRING_ARRAY_START as u8,
            b'(',
            b')',
            3,
            0,
            2,
            1,
            1,
            1,
            3,
            b'E',
            b'O',
            b'S',
            0,
            0,
            0,
            2,
            0xFF,
            b'x',
        ];
        assert_eq!(round_trip(&s), want, "bytes");
        let mut got = Scanner::default();
        got.deserialize(&want);
        assert_eq!(got.literal_stack, s.literal_stack, "literals");
    }

    // C writes (char)nesting_depth and reads it back as unsigned char.
    // scanner_test.go TestNestingDepthWraps
    #[test]
    fn nesting_depth_wraps() {
        let s = Scanner {
            literal_stack: vec![
                Literal {
                    typ: tok::STRING_START,
                    open_delimiter: b'(' as i32,
                    close_delimiter: b')' as i32,
                    nesting_depth: 300,
                    ..Literal::default()
                },
                Literal {
                    typ: tok::STRING_START,
                    open_delimiter: b'(' as i32,
                    close_delimiter: b')' as i32,
                    nesting_depth: -1,
                    ..Literal::default()
                },
            ],
            ..Scanner::default()
        };
        let mut got = Scanner::default();
        got.deserialize(&serialize(&s));
        assert!(
            got.literal_stack[0].nesting_depth == 44 && got.literal_stack[1].nesting_depth == 255,
            "depths = {}, {}, want 44, 255",
            got.literal_stack[0].nesting_depth,
            got.literal_stack[1].nesting_depth
        );
    }

    // scanner_test.go TestDeserializeEmpty
    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            has_leading_whitespace: true,
            literal_stack: vec![Literal {
                typ: tok::STRING_START,
                ..Literal::default()
            }],
            open_heredocs: vec![Heredoc {
                word: b"A".to_vec(),
                ..Heredoc::default()
            }],
        };
        s.deserialize(&[]);
        assert!(
            !s.has_leading_whitespace && s.literal_stack.is_empty() && s.open_heredocs.is_empty(),
            "state = {s:?}"
        );
    }

    // scanner_test.go TestSerializeLimits
    #[test]
    fn serialize_limits() {
        // literal_stack.size * 5 + 2 >= 1024 gives 0: 204 fit, 205 do not.
        let mut s = Scanner {
            literal_stack: vec![Literal::default(); 204],
            ..Scanner::default()
        };
        assert_eq!(round_trip(&s).len(), 204 * 5 + 2, "204 literals");
        s.literal_stack = vec![Literal::default(); 205];
        assert_eq!(serialize(&s).len(), 0, "205 literals");

        // size + 2 + word.size >= 1024 gives 0. size is 2 at the first heredoc.
        let word = |n: usize| vec![b'W'; n];
        let heredoc = |n: usize| Heredoc {
            word: word(n),
            ..Heredoc::default()
        };
        let four = |n: usize| Scanner {
            open_heredocs: vec![heredoc(250), heredoc(250), heredoc(250), heredoc(n)],
            ..Scanner::default()
        };
        // Three heredocs take 2 + 3*254 = 764 bytes; the fourth has 260 left.
        assert_eq!(round_trip(&four(255)).len(), 764 + 4 + 255, "fits");
        // Exactly full: 764 + 4 + 256 would be 1024, but a 256 byte word does not fit the length
        // byte. C aborts there; the port drops the state.
        assert_eq!(serialize(&four(256)).len(), 0, "256 byte word");
        // C's own test: 764+2+258 >= 1024
        assert_eq!(serialize(&four(258)).len(), 0, "too long");
        // C passes its test (size+2+len == 1023) and writes one byte past the buffer. The port
        // drops the state and must not panic.
        // 2 + 5 + 762 = 769; 769 + 2 + 252 = 1023 passes C's test; 769 + 4 + 252 = 1025.
        let s = Scanner {
            open_heredocs: vec![
                heredoc(1),
                heredoc(250),
                heredoc(250),
                heredoc(250),
                heredoc(252),
            ],
            ..Scanner::default()
        };
        assert_eq!(serialize(&s).len(), 0, "overflowing state");
    }

    // The word buffers are recycled between Deserialize calls; no two heredocs may end up
    // sharing one.
    // scanner_test.go TestWordBuffersNotShared
    #[test]
    fn word_buffers_not_shared() {
        let heredoc = |w: &[u8]| Heredoc {
            word: w.to_vec(),
            ..Heredoc::default()
        };
        let a = Scanner {
            open_heredocs: vec![heredoc(b"AAAA"), heredoc(b"BBBB"), heredoc(b"CCCC")],
            ..Scanner::default()
        };
        let b = serialize(&a);
        let mut s = Scanner::default();
        for _ in 0..3 {
            s.deserialize(&b);
            s.erase_first_heredoc();
            s.erase_first_heredoc();
            assert_eq!(s.open_heredocs[0].word, b"CCCC", "after erase");
            s.deserialize(&b);
            assert_eq!(serialize(&s), b, "bytes");
        }
    }
}
