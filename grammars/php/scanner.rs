//! Port of gositter `grammars/php/scanner.go` (tree-sitter-php v0.24.2 `common/scanner.h`):
//! state, serialize and deserialize. `scan` waits for the runtime Lexer (Task 12).

use crate::SERIALIZATION_BUFFER_SIZE;

// enum TokenType. Only `scan` reads these.
#[allow(dead_code)]
mod tok {
    pub const AUTOMATIC_SEMICOLON: u16 = 0;
    pub const ENCAPSED_STRING_CHARS: u16 = 1;
    pub const ENCAPSED_STRING_CHARS_AFTER_VARIABLE: u16 = 2;
    pub const EXECUTION_STRING_CHARS: u16 = 3;
    pub const EXECUTION_STRING_CHARS_AFTER_VARIABLE: u16 = 4;
    pub const ENCAPSED_STRING_CHARS_HEREDOC: u16 = 5;
    pub const ENCAPSED_STRING_CHARS_AFTER_VARIABLE_HEREDOC: u16 = 6;
    pub const EOF_TOKEN: u16 = 7;
    pub const HEREDOC_START: u16 = 8;
    pub const HEREDOC_END: u16 = 9;
    pub const NOWDOC_STRING: u16 = 10;
    pub const SENTINEL_ERROR: u16 = 11; // only valid during error recovery
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct Heredoc {
    end_word_indentation_allowed: bool,
    word: Vec<i32>,
}

/// `has_leading_whitespace` is written but never read or serialized.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    has_leading_whitespace: bool,
    heredocs: Vec<Heredoc>,
}

// iswspace, iswalnum, iswdigit, iswxdigit in the "C" locale (the scanner never calls setlocale):
// ASCII only, false for the -1 of an invalid byte.

fn iswspace(c: i32) -> bool {
    let _ = c;
    todo!()
}

fn iswdigit(c: i32) -> bool {
    let _ = c;
    todo!()
}

fn iswxdigit(c: i32) -> bool {
    let _ = c;
    todo!()
}

fn iswalnum(c: i32) -> bool {
    let _ = c;
    todo!()
}

impl Scanner {
    /// `newScanner`
    pub fn new() -> Self {
        todo!()
    }

    /// `Serialize`
    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        let _ = (self, buf);
        todo!()
    }

    /// `Deserialize`
    pub fn deserialize(&mut self, buf: &[u8]) {
        let _ = (self, buf);
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ser(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    fn word(s: &str) -> Vec<i32> {
        s.chars().map(|c| c as i32).collect()
    }

    fn hd(end_word_indentation_allowed: bool, word: Vec<i32>) -> Heredoc {
        Heredoc {
            end_word_indentation_allowed,
            word,
        }
    }

    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(ser(&s), [0], "fresh scanner");

        s.heredocs = vec![
            hd(false, word("EOT")),
            hd(true, word("é")),
            Heredoc::default(),
        ];
        // count, then per heredoc: flag, uint32 length, int32 code points, all little-endian.
        #[rustfmt::skip]
        let want = [
            3,
            0, 3, 0, 0, 0, b'E', 0, 0, 0, b'O', 0, 0, 0, b'T', 0, 0, 0,
            1, 1, 0, 0, 0, 0xE9, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];
        let b = ser(&s);
        assert_eq!(b, want);

        // Dirty target with one heredoc: all of it is replaced.
        let mut got = Scanner {
            has_leading_whitespace: true,
            heredocs: vec![hd(true, word("ZZZZ"))],
        };
        got.deserialize(&b);
        assert!(
            !got.has_leading_whitespace && got.heredocs.len() == 3,
            "state = {got:?}"
        );
        for (i, (g, w)) in got.heredocs.iter().zip(&s.heredocs).enumerate() {
            assert!(
                g.end_word_indentation_allowed == w.end_word_indentation_allowed
                    && g.word == w.word,
                "heredoc {i} = {g:?}, want {w:?}"
            );
        }
        assert_eq!(b, ser(&got), "serialize after deserialize differs");
    }

    // C's deserialize empties the heredocs but never lowers heredocs.size.
    #[test]
    fn deserialize_keeps_count() {
        let mut s = Scanner {
            has_leading_whitespace: false,
            heredocs: vec![hd(false, word("A")), hd(true, word("B"))],
        };
        s.deserialize(&[]);
        assert_eq!(
            ser(&s),
            [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "after empty state"
        );

        s.heredocs[1].word = word("B");
        s.deserialize(&[0]);
        // Go checks word == nil; an empty Vec is the Rust nil.
        assert!(
            s.heredocs.len() == 2 && s.heredocs[1].word.is_empty(),
            "after [0] = {:?}",
            s.heredocs
        );

        s.deserialize(&[1, 0, 1, 0, 0, 0, b'C', 0, 0, 0]);
        assert_eq!(
            ser(&s),
            [2, 0, 1, 0, 0, 0, b'C', 0, 0, 0, 0, 0, 0, 0, 0],
            "after one heredoc"
        );
    }

    #[test]
    fn serialize_limits() {
        // size + 5 + word_size >= 1024 gives 0. One heredoc: 1 + 5 + 4n.
        let mut s = Scanner {
            has_leading_whitespace: false,
            heredocs: vec![hd(false, vec![0; 254])],
        };
        assert_eq!(ser(&s).len(), 1 + 5 + 254 * 4, "254 code points");
        s.heredocs[0].word = vec![0; 255];
        assert_eq!(ser(&s).len(), 0, "255 code points");

        // Empty heredocs: 1 + 5n, the check fails for the 205th.
        s.heredocs = vec![Heredoc::default(); 204];
        let b = ser(&s);
        assert_eq!(b.len(), 1 + 5 * 204, "204 heredocs");
        let mut got = Scanner::default();
        got.deserialize(&b);
        assert!(
            got.heredocs.len() == 204 && ser(&got) == b,
            "204 heredocs do not round trip"
        );
        s.heredocs = vec![Heredoc::default(); 205];
        assert_eq!(ser(&s).len(), 0, "205 heredocs");
    }

    #[test]
    fn wctype() {
        for c in -2..0x3100i32 {
            let sp = matches!(c, 32 | 9..=13);
            assert_eq!(iswspace(c), sp, "iswspace({c:#x}) = {}", !sp);
            assert!(
                !(c >= 0x80 && (iswalnum(c) || iswdigit(c) || iswxdigit(c))),
                "non-ASCII {c:#x} classified"
            );
        }
    }
}
