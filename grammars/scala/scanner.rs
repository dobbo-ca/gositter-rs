//! Port of gositter `grammars/scala/scanner.go` (tree-sitter-scala v0.26.0 `src/scanner.c`):
//! state, serialize and deserialize. `scan` waits for the runtime Lexer (Task 12).

use crate::SERIALIZATION_BUFFER_SIZE;

// enum TokenType. Only `scan` reads these.
#[allow(dead_code)]
mod tok {
    pub const AUTOMATIC_SEMICOLON: u16 = 0;
    pub const INDENT: u16 = 1;
    pub const OUTDENT: u16 = 2;
    pub const COMMA_OUTDENT: u16 = 3;
    pub const SIMPLE_STRING_START: u16 = 4;
    pub const SIMPLE_STRING_MIDDLE: u16 = 5;
    pub const SIMPLE_MULTILINE_STRING_START: u16 = 6;
    pub const INTERPOLATED_STRING_MIDDLE: u16 = 7;
    pub const INTERPOLATED_MULTILINE_STRING_MIDDLE: u16 = 8;
    pub const RAW_STRING_START: u16 = 9;
    pub const RAW_STRING_MIDDLE: u16 = 10;
    pub const RAW_STRING_MULTILINE_MIDDLE: u16 = 11;
    pub const SINGLE_LINE_STRING_END: u16 = 12;
    pub const MULTILINE_STRING_END: u16 = 13;
    pub const ELSE: u16 = 14;
    pub const CATCH: u16 = 15;
    pub const FINALLY: u16 = 16;
    pub const EXTENDS: u16 = 17;
    pub const DERIVES: u16 = 18;
    pub const WITH: u16 = 19;
    pub const ERROR_SENTINEL: u16 = 20;
}

/// All four are `int16_t` in C and wrap the same way here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanner {
    indents: Vec<i16>,
    last_indentation_size: i16,
    last_newline_count: i16,
    last_column: i16,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

/// `iswspace` in the "C" locale: true only for 9..13 and 32.
fn iswspace(c: i32) -> bool {
    let _ = c;
    todo!()
}

/// `iswalnum`, same measurement: ASCII letters and digits only.
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

    fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    fn roundtrip(s: &Scanner) -> (Scanner, Vec<u8>) {
        let b = serialize(s);
        // Dirty target: deserialize must replace all of it.
        let mut got = Scanner {
            indents: vec![9, 9],
            last_indentation_size: 7,
            last_newline_count: 7,
            last_column: 7,
        };
        got.deserialize(&b);
        assert_eq!(b, serialize(&got), "serialize after deserialize differs");
        (got, b)
    }

    #[test]
    fn serialize_bytes() {
        // new() leaves -1, 0, -1 and no indents.
        assert_eq!(
            serialize(&Scanner::new()),
            [0xFF, 0xFF, 0, 0, 0xFF, 0xFF],
            "fresh scanner"
        );

        let s = Scanner {
            indents: vec![0, 2, 0x1234, -2],
            last_indentation_size: 4,
            last_newline_count: -32768,
            last_column: 0x0102,
        };
        let (got, b) = roundtrip(&s);
        // Three int16 then the indents, all little-endian.
        let want = [4, 0, 0, 0x80, 2, 1, 0, 0, 2, 0, 0x34, 0x12, 0xFE, 0xFF];
        assert_eq!(b, want);
        assert!(
            got.indents == s.indents
                && got.last_indentation_size == 4
                && got.last_newline_count == -32768
                && got.last_column == 0x0102,
            "state = {got:?}, want {s:?}"
        );
    }

    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            indents: vec![0, 4],
            last_indentation_size: 3,
            last_newline_count: 2,
            last_column: 1,
        };
        s.deserialize(&[]);
        assert!(
            s.indents.is_empty()
                && s.last_indentation_size == -1
                && s.last_newline_count == 0
                && s.last_column == -1,
            "state = {s:?}"
        );
    }

    #[test]
    fn serialize_limits() {
        // 509 indents fill the buffer exactly.
        let mut s = Scanner::new();
        s.indents.extend((0..509).map(|i| i as i16));
        let (mut got, b) = roundtrip(&s);
        assert!(
            b.len() == 1024 && got.indents.len() == 509 && got.indents[508] == 508,
            "len = {}, indents = {}",
            b.len(),
            got.indents.len()
        );

        // One more and C writes nothing, so the state comes back reset.
        s.indents.push(509);
        s.last_newline_count = 5;
        let b = serialize(&s);
        got.deserialize(&b);
        assert!(
            b.is_empty()
                && got.indents.is_empty()
                && got.last_newline_count == 0
                && got.last_column == -1,
            "len = {}, state = {got:?}",
            b.len()
        );
    }

    // The libc predicates in the "C" locale are ASCII only. U+00A0, U+0085, U+2028, U+3000 and -1
    // are the usual traps.
    #[test]
    fn wctype() {
        for c in -2..0x3100i32 {
            let space = matches!(c, 32 | 9..=13);
            let alnum =
                (0..128).contains(&c) && (matches!(c, 48..=57) || matches!(c | 0x20, 97..=122));
            assert!(
                iswspace(c) == space && iswalnum(c) == alnum,
                "c = {c}: iswspace {}, iswalnum {}",
                iswspace(c),
                iswalnum(c)
            );
        }
    }
}
