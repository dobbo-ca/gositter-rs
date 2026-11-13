//! Port of gositter `grammars/scala/scanner.go` (tree-sitter-scala v0.26.0 `src/scanner.c`):
//! state, serialize, deserialize and scan.

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
    // ' ', then \t \n \v \f \r.
    c == 32 || (9..=13).contains(&c)
}

/// `iswalnum`, same measurement: ASCII letters and digits only.
fn iswalnum(c: i32) -> bool {
    (48..=57).contains(&c) || (65..=90).contains(&c) || (97..=122).contains(&c)
}

impl Scanner {
    /// `newScanner`
    pub fn new() -> Self {
        Self {
            indents: Vec::new(),
            last_indentation_size: -1,
            last_newline_count: 0,
            last_column: -1,
        }
    }

    /// `Serialize`
    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        // Too deep to fit: C stores nothing, so the state reads back as reset.
        if (self.indents.len() + 3) * 2 > SERIALIZATION_BUFFER_SIZE {
            return 0;
        }

        let mut size = 0;
        let mut put = |v: i16| {
            // memcpy of an int16_t, little-endian
            buf[size..size + 2].copy_from_slice(&v.to_le_bytes());
            size += 2;
        };
        put(self.last_indentation_size);
        put(self.last_newline_count);
        put(self.last_column);
        for &v in &self.indents {
            put(v);
        }
        size
    }

    /// `Deserialize`
    pub fn deserialize(&mut self, buf: &[u8]) {
        self.indents.clear();
        self.last_indentation_size = -1;
        self.last_column = -1;
        self.last_newline_count = 0;

        if buf.is_empty() {
            return;
        }

        // The length is always even and at least 6: it comes from serialize.
        let get = |i: usize| i16::from_le_bytes([buf[i], buf[i + 1]]);
        self.last_indentation_size = get(0);
        self.last_newline_count = get(2);
        self.last_column = get(4);
        for size in (6..buf.len()).step_by(2) {
            self.indents.push(get(size));
        }
    }
}

/// `is_op_char`
fn is_op_char(c: i32) -> bool {
    matches!(
        u8::try_from(c),
        Ok(b'!'
            | b'#'
            | b'%'
            | b'&'
            | b'*'
            | b'+'
            | b'-'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'\\'
            | b'^'
            | b'|'
            | b'~'
            | b':')
    )
}

// enum StringMode
const STRING_MODE_SIMPLE: u8 = 0;
const STRING_MODE_INTERPOLATED: u8 = 1;
const STRING_MODE_RAW: u8 = 2;

/// `scan_string_content`
fn scan_string_content(
    lexer: &mut gositter::Lexer<'_>,
    is_multiline: bool,
    string_mode: u8,
) -> bool {
    let mut closing_quote_count = 0u32;
    loop {
        if lexer.lookahead == '"' as i32 {
            lexer.advance(false);
            closing_quote_count = closing_quote_count.wrapping_add(1);
            if !is_multiline {
                lexer.result_symbol = tok::SINGLE_LINE_STRING_END;
                lexer.mark_end();
                return true;
            }
            if closing_quote_count >= 3 && lexer.lookahead != '"' as i32 {
                lexer.result_symbol = tok::MULTILINE_STRING_END;
                lexer.mark_end();
                return true;
            }
        } else if lexer.lookahead == '$' as i32 && string_mode != STRING_MODE_SIMPLE {
            match string_mode {
                STRING_MODE_INTERPOLATED => {
                    lexer.result_symbol = tok::INTERPOLATED_STRING_MIDDLE;
                    if is_multiline {
                        lexer.result_symbol = tok::INTERPOLATED_MULTILINE_STRING_MIDDLE;
                    }
                }
                STRING_MODE_RAW => {
                    lexer.result_symbol = tok::RAW_STRING_MIDDLE;
                    if is_multiline {
                        lexer.result_symbol = tok::RAW_STRING_MULTILINE_MIDDLE;
                    }
                }
                _ => {}
            }
            lexer.mark_end();
            return true;
        } else {
            closing_quote_count = 0;
            if lexer.lookahead == '\\' as i32 {
                // Multiline strings ignore escape sequences.
                if is_multiline || string_mode == STRING_MODE_RAW {
                    lexer.advance(false);
                    // In a single-line raw string \" does not close the
                    // string and \\ keeps the next quote able to.
                    if !is_multiline
                        && string_mode == STRING_MODE_RAW
                        && (lexer.lookahead == '"' as i32 || lexer.lookahead == '\\' as i32)
                    {
                        lexer.advance(false);
                    }
                } else {
                    lexer.result_symbol = tok::INTERPOLATED_STRING_MIDDLE;
                    if string_mode == STRING_MODE_SIMPLE {
                        lexer.result_symbol = tok::SIMPLE_STRING_MIDDLE;
                    }
                    lexer.mark_end();
                    return true;
                }
                // With every symbol valid (error recovery) this runs outside
                // a string; a newline or the end of input stops it.
            } else if (lexer.lookahead == '\n' as i32 && !is_multiline) || lexer.eof() {
                return false;
            } else {
                lexer.advance(false);
            }
        }
    }
}

/// `detect_comment_start`
fn detect_comment_start(lexer: &mut gositter::Lexer<'_>) -> bool {
    lexer.mark_end();
    // Comments should not affect indentation.
    if lexer.lookahead == '/' as i32 {
        lexer.advance(false);
        if lexer.lookahead == '/' as i32 || lexer.lookahead == '*' as i32 {
            return true;
        }
    }
    false
}

/// `scan_word`
fn scan_word(lexer: &mut gositter::Lexer<'_>, word: &str) -> bool {
    for &b in word.as_bytes() {
        if lexer.lookahead != b as i32 {
            return false;
        }
        lexer.advance(false);
    }
    !iswalnum(lexer.lookahead)
}

impl gositter::ExternalScanner for Scanner {
    /// `Scan`
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let v = |t: u16| valid_symbols[t as usize];
        let prev = self.indents.last().copied().unwrap_or(-1);
        let mut newline_count = 0i16;
        let mut indentation_size = 0i16;

        while iswspace(lexer.lookahead) {
            if lexer.lookahead == '\n' as i32 {
                newline_count = newline_count.wrapping_add(1);
                indentation_size = 0;
            } else {
                indentation_size = indentation_size.wrapping_add(1);
            }
            lexer.advance(true);
        }

        // A comma that ends an indented block; its own token so the grammar
        // decides where that is allowed.
        if v(tok::COMMA_OUTDENT) && lexer.lookahead == ',' as i32 && prev != -1 {
            self.indents.pop();
            lexer.mark_end();
            lexer.result_symbol = tok::COMMA_OUTDENT;
            return true;
        }

        // Before advancing the lexer, check if we can double outdent.
        // lookahead == 0 is also true for a NUL byte, as in C.
        if v(tok::OUTDENT)
            && (lexer.lookahead == 0
                || (prev != -1
                    && (lexer.lookahead == ')' as i32
                        || lexer.lookahead == ']' as i32
                        || lexer.lookahead == '}' as i32))
                || (self.last_indentation_size != -1
                    && prev != -1
                    && self.last_indentation_size < prev))
        {
            self.indents.pop();
            lexer.result_symbol = tok::OUTDENT;
            return true;
        }
        self.last_indentation_size = -1;

        if v(tok::INDENT)
            && newline_count > 0
            && (self.indents.is_empty() || indentation_size > *self.indents.last().unwrap())
        {
            if detect_comment_start(lexer) {
                return false;
            }
            self.indents.push(indentation_size);
            lexer.result_symbol = tok::INDENT;
            return true;
        }

        // Saves indentation_size and newline_count for the next calls: multiple
        // outdent or automatic semicolon.
        if v(tok::OUTDENT)
            && (lexer.lookahead == 0
                || (newline_count > 0 && prev != -1 && indentation_size < prev))
        {
            self.indents.pop();
            lexer.result_symbol = tok::OUTDENT;
            lexer.mark_end();
            if detect_comment_start(lexer) {
                return false;
            }
            self.last_indentation_size = indentation_size;
            self.last_newline_count = newline_count;
            if lexer.eof() {
                self.last_column = -1;
            } else {
                self.last_column = lexer.get_column() as i16; // truncates as (int16_t)
            }
            return true;
        }

        // Recover newline_count from the outdent reset. get_column is only
        // called when the first operand is false, as C short-circuits.
        let is_eof = lexer.eof();
        if (self.last_newline_count > 0 && (is_eof && self.last_column == -1))
            // (uint32_t)int16_t sign-extends; so does `as u32`.
            || (!is_eof && lexer.get_column() == self.last_column as u32)
        {
            newline_count = newline_count.wrapping_add(self.last_newline_count);
        }
        self.last_newline_count = 0;

        if v(tok::AUTOMATIC_SEMICOLON) && newline_count > 0 {
            // No semicolon in the middle of an expression: comments,
            // else/catch clauses and so on leave this branch.

            lexer.mark_end();
            lexer.result_symbol = tok::AUTOMATIC_SEMICOLON;

            // Probably a multi-line field expression.
            if lexer.lookahead == '.' as i32 {
                return false;
            }

            // Single-line and multi-line comments
            if lexer.lookahead == '/' as i32 {
                lexer.advance(false);
                if lexer.lookahead == '/' as i32 {
                    return false;
                }
                if lexer.lookahead == '*' as i32 {
                    lexer.advance(false);
                    while !lexer.eof() {
                        if lexer.lookahead == '*' as i32 {
                            lexer.advance(false);
                            if lexer.lookahead == '/' as i32 {
                                lexer.advance(false);
                                break;
                            }
                        } else {
                            lexer.advance(false);
                        }
                    }
                    while iswspace(lexer.lookahead) {
                        if lexer.lookahead == '\n' as i32 || lexer.lookahead == '\r' as i32 {
                            return false;
                        }
                        lexer.advance(true);
                    }
                    // Code on the same line after the comment still gets a
                    // semicolon before it.
                    return true;
                }
            }

            if v(tok::ELSE) {
                return !scan_word(lexer, "else");
            }

            if v(tok::CATCH) && scan_word(lexer, "catch") {
                return false;
            }

            if v(tok::FINALLY) && scan_word(lexer, "finally") {
                return false;
            }

            if v(tok::EXTENDS) && scan_word(lexer, "extends") {
                return false;
            }

            if v(tok::WITH) && scan_word(lexer, "with") {
                return false;
            }

            if v(tok::DERIVES) && scan_word(lexer, "derives") {
                return false;
            }

            if newline_count > 1 {
                return true;
            }

            // No semicolon before a leading infix operator (symbolic or
            // back-ticked) that has an operand on the same line.
            if is_op_char(lexer.lookahead) || lexer.lookahead == '`' as i32 {
                if is_op_char(lexer.lookahead) {
                    lexer.advance(false);
                    while is_op_char(lexer.lookahead) {
                        lexer.advance(false);
                    }
                    let mut found_space = false;
                    while lexer.lookahead == ' ' as i32 || lexer.lookahead == '\t' as i32 {
                        lexer.advance(false);
                        found_space = true;
                    }
                    if found_space && !iswspace(lexer.lookahead) && !lexer.eof() {
                        return false;
                    }
                } else if lexer.lookahead == '`' as i32 {
                    lexer.advance(false);
                    while lexer.lookahead != '`' as i32 && !lexer.eof() {
                        lexer.advance(false);
                    }
                    if lexer.lookahead == '`' as i32 {
                        lexer.advance(false);
                        let mut found_space = false;
                        while lexer.lookahead == ' ' as i32 || lexer.lookahead == '\t' as i32 {
                            lexer.advance(false);
                            found_space = true;
                        }
                        if found_space && !iswspace(lexer.lookahead) && !lexer.eof() {
                            return false;
                        }
                    }
                }
            }

            return true;
        }

        while iswspace(lexer.lookahead) {
            if lexer.lookahead == '\n' as i32 {
                newline_count = newline_count.wrapping_add(1);
            }
            lexer.advance(true);
        }

        if v(tok::SIMPLE_STRING_START) && lexer.lookahead == '"' as i32 {
            lexer.advance(false);
            lexer.mark_end();

            if lexer.lookahead == '"' as i32 {
                lexer.advance(false);
                if lexer.lookahead == '"' as i32 {
                    lexer.advance(false);
                    lexer.result_symbol = tok::SIMPLE_MULTILINE_STRING_START;
                    lexer.mark_end();
                    return true;
                }
            }

            lexer.result_symbol = tok::SIMPLE_STRING_START;
            return true;
        }

        // raw" needs two tokens of lookahead, hence the scanner.
        if v(tok::RAW_STRING_START) && lexer.lookahead == 'r' as i32 {
            lexer.advance(false);
            if lexer.lookahead == 'a' as i32 {
                lexer.advance(false);
                if lexer.lookahead == 'w' as i32 {
                    lexer.advance(false);
                    if lexer.lookahead == '"' as i32 {
                        lexer.mark_end();
                        lexer.result_symbol = tok::RAW_STRING_START;
                        return true;
                    }
                }
            }
        }

        if v(tok::SIMPLE_STRING_MIDDLE) {
            return scan_string_content(lexer, false, STRING_MODE_SIMPLE);
        }

        if v(tok::INTERPOLATED_STRING_MIDDLE) {
            return scan_string_content(lexer, false, STRING_MODE_INTERPOLATED);
        }

        if v(tok::RAW_STRING_MIDDLE) {
            return scan_string_content(lexer, false, STRING_MODE_RAW);
        }

        if v(tok::RAW_STRING_MULTILINE_MIDDLE) {
            return scan_string_content(lexer, true, STRING_MODE_RAW);
        }

        if v(tok::INTERPOLATED_MULTILINE_STRING_MIDDLE) {
            return scan_string_content(lexer, true, STRING_MODE_INTERPOLATED);
        }

        // The simple multiline case has no MIDDLE token and all three kinds
        // share MULTILINE_STRING_END, so this comes after the two checks above.
        if v(tok::MULTILINE_STRING_END) {
            return scan_string_content(lexer, true, STRING_MODE_SIMPLE);
        }

        false
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
