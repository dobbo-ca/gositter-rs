//! Port of gositter `grammars/lua/scanner.go` (tree-sitter-lua v0.5.0 `src/scanner.c`): the scanner
//! state, `serialize`, `deserialize` and `scan`.

use gositter::{Lexer, Symbol};

/// enum TokenType (the `Block` prefix on every name follows C's TokenType)
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy)]
enum Tok {
    BlockCommentStart,
    BlockCommentContent,
    BlockCommentEnd,
    BlockStringStart,
    BlockStringContent,
    BlockStringEnd,
}

/// iswspace in the "C" locale, which is what a process that never calls setlocale gets: the six
/// ASCII spaces, nothing above 0x7F, not -1.
fn iswspace(c: i32) -> bool {
    c == b' ' as i32 || (b'\t' as i32..=b'\r' as i32).contains(&c)
}

/// consume_char
fn consume_char(c: u8, l: &mut Lexer<'_>) -> bool {
    if l.lookahead != i32::from(c) {
        return false;
    }
    l.advance(false);
    true
}

/// consume_and_count_char. u8 on purpose: 256 '=' wrap to level 0.
fn consume_and_count_char(c: u8, l: &mut Lexer<'_>) -> u8 {
    let mut count: u8 = 0;
    while l.lookahead == i32::from(c) {
        count = count.wrapping_add(1);
        l.advance(false);
    }
    count
}

/// C never assigns `ending_char` a non-zero value; it only comes back from `deserialize`, so it is
/// 0 on every reachable path. Kept for the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    /// `char`, signed on the oracle's targets.
    ending_char: i8,
    level_count: u8,
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.ending_char as u8;
        buf[1] = self.level_count;
        2
    }

    /// Length 0 leaves the state as it is, as C does.
    pub fn deserialize(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            return;
        }
        self.ending_char = buf[0] as i8;
        if buf.len() == 1 {
            return;
        }
        self.level_count = buf[1];
    }

    /// reset_state
    fn reset_state(&mut self) {
        self.ending_char = 0;
        self.level_count = 0;
    }

    /// scan_block_start
    fn scan_block_start(&mut self, l: &mut Lexer<'_>) -> bool {
        if consume_char(b'[', l) {
            let level = consume_and_count_char(b'=', l);

            if consume_char(b'[', l) {
                self.level_count = level;
                return true;
            }
        }
        false
    }

    /// scan_block_end
    fn scan_block_end(&mut self, l: &mut Lexer<'_>) -> bool {
        if consume_char(b']', l) {
            let level = consume_and_count_char(b'=', l);

            if self.level_count == level && consume_char(b']', l) {
                return true;
            }
        }
        false
    }

    /// scan_block_content
    fn scan_block_content(&mut self, l: &mut Lexer<'_>) -> bool {
        while l.lookahead != 0 {
            if l.lookahead == b']' as i32 {
                l.mark_end();

                if self.scan_block_end(l) {
                    return true;
                }
            } else {
                l.advance(false);
            }
        }
        false
    }

    /// scan_comment_start
    fn scan_comment_start(&mut self, l: &mut Lexer<'_>) -> bool {
        if consume_char(b'-', l) && consume_char(b'-', l) {
            l.mark_end();

            if self.scan_block_start(l) {
                l.mark_end();
                l.result_symbol = Tok::BlockCommentStart as Symbol;
                return true;
            }
        }
        false
    }

    /// scan_comment_content
    fn scan_comment_content(&mut self, l: &mut Lexer<'_>) -> bool {
        if self.ending_char == 0 {
            // block comment
            if self.scan_block_content(l) {
                l.result_symbol = Tok::BlockCommentContent as Symbol;
                return true;
            }
            return false;
        }

        while l.lookahead != 0 {
            if l.lookahead == i32::from(self.ending_char) {
                self.reset_state();
                l.result_symbol = Tok::BlockCommentContent as Symbol;
                return true;
            }
            l.advance(false);
        }
        false
    }

    /// tree_sitter_lua_external_scanner_scan
    fn scan(&mut self, l: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let valid = |t: Tok| valid_symbols[t as usize];
        if valid(Tok::BlockStringEnd) && self.scan_block_end(l) {
            self.reset_state();
            l.result_symbol = Tok::BlockStringEnd as Symbol;
            return true;
        }

        if valid(Tok::BlockStringContent) && self.scan_block_content(l) {
            l.result_symbol = Tok::BlockStringContent as Symbol;
            return true;
        }

        if valid(Tok::BlockCommentEnd) && self.ending_char == 0 && self.scan_block_end(l) {
            self.reset_state();
            l.result_symbol = Tok::BlockCommentEnd as Symbol;
            return true;
        }

        if valid(Tok::BlockCommentContent) && self.scan_comment_content(l) {
            return true;
        }

        // skip_whitespaces
        while iswspace(l.lookahead) {
            l.advance(true);
        }

        if valid(Tok::BlockStringStart) && self.scan_block_start(l) {
            l.result_symbol = Tok::BlockStringStart as Symbol;
            return true;
        }

        if valid(Tok::BlockCommentStart) && self.scan_comment_start(l) {
            return true;
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
}

#[allow(dead_code)] // used by the generated language() (Task 10)
pub(crate) fn new() -> Box<dyn gositter::ExternalScanner> {
    Box::new(Scanner::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TestSerializeRoundTrip
    #[test]
    fn serialize_round_trip() {
        let mut buf = [0u8; crate::SERIALIZATION_BUFFER_SIZE];
        let mut s = Scanner::new();
        let n = s.serialize(&mut buf);
        assert_eq!(&buf[..n], [0, 0], "fresh scanner");

        let cases: [[u8; 2]; 6] = [[0, 0], [0, 3], [0, 255], [b'"', 7], [0x80, 1], [0xFF, 0xFF]];
        for want in cases {
            // Dirty target.
            let mut got = Scanner {
                ending_char: 5,
                level_count: 9,
            };
            got.deserialize(&want);
            let n = got.serialize(&mut buf);
            assert_eq!(&buf[..n], want, "round trip of {want:?}");
        }
        // char is signed: 0x80 must not compare equal to code point U+0080.
        s.deserialize(&[0x80, 0]);
        assert_eq!(s.ending_char, -128);
    }

    // TestDeserializeShort
    #[test]
    fn deserialize_short() {
        // C returns early: length 0 touches nothing, length 1 only ending_char.
        let mut s = Scanner {
            ending_char: 5,
            level_count: 9,
        };
        s.deserialize(&[]);
        assert!(
            s.ending_char == 5 && s.level_count == 9,
            "length 0 changed the state: {s:?}"
        );
        s.deserialize(&[7]);
        assert!(s.ending_char == 7 && s.level_count == 9, "length 1: {s:?}");
    }
}

#[cfg(test)]
mod parser_tests {
    // Trees for long brackets, incl. the uint8 wrap at 256 '='. TestLongBrackets (scanner_test.go)
    #[test]
    fn long_brackets() {
        let eq = |n: usize| "=".repeat(n);
        for (src, want) in [
            (
                "x = [==[a]]b]=]c]==]".to_string(),
                "(chunk (assignment_statement (variable_list name: (identifier)) (expression_list value: (string content: (string_content)))))",
            ),
            (
                "--[[ c ]] x()".to_string(),
                "(chunk (comment content: (comment_content)) (function_call name: (identifier) arguments: (arguments)))",
            ),
            // level 256 wraps to 0, so a plain ]] closes it
            (
                format!("x = [{}[a]]", eq(256)),
                "(chunk (assignment_statement (variable_list name: (identifier)) (expression_list value: (string content: (string_content)))))",
            ),
        ] {
            let mut p = gositter::Parser::new();
            p.set_language(super::super::language()).unwrap();
            let tree = p.parse(src.as_bytes()).unwrap();
            let got = tree.root_node().to_sexp();
            assert_eq!(got, want, "{src:?}");
        }
    }
}
