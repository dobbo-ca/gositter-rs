//! Port of `grammars/ruby/scanner.go` (tree-sitter-ruby v0.23.1 `src/scanner.c`). Statement
//! order and the advance/mark_end/eof call sequence follow C on every path.
//!
//! The wctype calls run in the "C" locale (nothing calls setlocale), where they are ASCII only.

use crate::SERIALIZATION_BUFFER_SIZE;
use crate::cchar;

// enum TokenType
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

/// `char::from_u32` of the lookahead, for `match` on a char literal. -1 (EOF) and anything that
/// is not a scalar value is `None`.
fn la(lexer: &gositter::Lexer<'_>) -> Option<char> {
    char::from_u32(lexer.lookahead as u32)
}

fn ch(c: char) -> i32 {
    c as i32
}

/// iswspace
fn iswspace(c: i32) -> bool {
    c == ch(' ') || (ch('\t')..=ch('\r')).contains(&c)
}

/// iswdigit
fn iswdigit(c: i32) -> bool {
    (ch('0')..=ch('9')).contains(&c)
}

/// iswupper
fn iswupper(c: i32) -> bool {
    (ch('A')..=ch('Z')).contains(&c)
}

/// iswlower
fn iswlower(c: i32) -> bool {
    (ch('a')..=ch('z')).contains(&c)
}

/// iswalpha
fn iswalpha(c: i32) -> bool {
    iswupper(c) || iswlower(c)
}

/// iswalnum
fn iswalnum(c: i32) -> bool {
    iswalpha(c) || iswdigit(c)
}

/// NON_IDENTIFIER_CHARS, indexed by byte.
fn non_identifier_char(b: u8) -> bool {
    b"\0\n\r\t :;`\"'@$#.,|^&<=>+-*/\\%?!~()[]{}".contains(&b)
}

/// `strchr("!@&`'+~=/\\,;.<>*$?:\"", c) != NULL`. strchr also finds the terminating NUL, so 0 is
/// in the set.
fn short_interpolation_global(b: u8) -> bool {
    b"!@&`'+~=/\\,;.<>*$?:\"\0".contains(&b)
}

/// is_iden_char. C passes (char)lookahead and memchr compares bytes, so only the low byte of the
/// code point counts (U+0140 is '@' here).
fn is_iden_char(c: i32) -> bool {
    !non_identifier_char(c as u8)
}

/// scan_operator
fn scan_operator(lexer: &mut gositter::Lexer<'_>) -> bool {
    match la(lexer) {
        // <, <=, <<, <=>
        Some('<') => {
            lexer.advance(false);
            if lexer.lookahead == ch('<') {
                lexer.advance(false);
            } else if lexer.lookahead == ch('=') {
                lexer.advance(false);
                if lexer.lookahead == ch('>') {
                    lexer.advance(false);
                }
            }
            true
        }

        // >, >=, >>
        Some('>') => {
            lexer.advance(false);
            if lexer.lookahead == ch('>') || lexer.lookahead == ch('=') {
                lexer.advance(false);
            }
            true
        }

        // ==, ===, =~
        Some('=') => {
            lexer.advance(false);
            if lexer.lookahead == ch('~') {
                lexer.advance(false);
                return true;
            }
            if lexer.lookahead == ch('=') {
                lexer.advance(false);
                if lexer.lookahead == ch('=') {
                    lexer.advance(false);
                }
                return true;
            }
            false
        }

        // +, -, ~, +@, -@, ~@
        Some('+' | '-' | '~') => {
            lexer.advance(false);
            if lexer.lookahead == ch('@') {
                lexer.advance(false);
            }
            true
        }

        // ..
        Some('.') => {
            lexer.advance(false);
            if lexer.lookahead == ch('.') {
                lexer.advance(false);
                return true;
            }
            false
        }

        // &, ^, |, /, %`
        Some('&' | '^' | '|' | '/' | '%' | '`') => {
            lexer.advance(false);
            true
        }

        // !, !=, !~
        Some('!') => {
            lexer.advance(false);
            if lexer.lookahead == ch('=') || lexer.lookahead == ch('~') {
                lexer.advance(false);
            }
            true
        }

        // *, **
        Some('*') => {
            lexer.advance(false);
            if lexer.lookahead == ch('*') {
                lexer.advance(false);
            }
            true
        }

        // [], []=
        Some('[') => {
            lexer.advance(false);
            if lexer.lookahead == ch(']') {
                lexer.advance(false);
            } else {
                return false;
            }
            if lexer.lookahead == ch('=') {
                lexer.advance(false);
            }
            true
        }

        _ => false,
    }
}

/// scan_symbol_identifier
fn scan_symbol_identifier(lexer: &mut gositter::Lexer<'_>) -> bool {
    if lexer.lookahead == ch('@') {
        lexer.advance(false);
        if lexer.lookahead == ch('@') {
            lexer.advance(false);
        }
    } else if lexer.lookahead == ch('$') {
        lexer.advance(false);
    }

    if is_iden_char(lexer.lookahead) {
        lexer.advance(false);
    } else if !scan_operator(lexer) {
        return false;
    }

    while is_iden_char(lexer.lookahead) {
        lexer.advance(false);
    }

    if lexer.lookahead == ch('?') || lexer.lookahead == ch('!') {
        lexer.advance(false);
    }

    if lexer.lookahead == ch('=') {
        lexer.mark_end();
        lexer.advance(false);
        if lexer.lookahead != ch('>') {
            lexer.mark_end();
        }
    }

    true
}

/// scan_heredoc_word. Returns C's `String word` in place of the Heredoc out-param.
fn scan_heredoc_word(lexer: &mut gositter::Lexer<'_>) -> (Vec<u8>, bool) {
    let mut word = Vec::new();
    let mut quote = 0;

    match la(lexer) {
        Some('\'' | '"' | '`') => {
            quote = lexer.lookahead;
            lexer.advance(false);
            while lexer.lookahead != quote && !lexer.eof() {
                word.push(lexer.lookahead as u8);
                lexer.advance(false);
            }
            lexer.advance(false);
        }

        _ => {
            if iswalnum(lexer.lookahead) || lexer.lookahead == ch('_') {
                word.push(lexer.lookahead as u8);
                lexer.advance(false);
                while iswalnum(lexer.lookahead) || lexer.lookahead == ch('_') {
                    word.push(lexer.lookahead as u8);
                    lexer.advance(false);
                }
            }
        }
    }

    (word, quote != ch('\''))
}

/// scan_short_interpolation. start is a C char: the '@' and '$' tests see only the low byte of
/// the code point, and so does strchr.
fn scan_short_interpolation(
    lexer: &mut gositter::Lexer<'_>,
    has_content: bool,
    content_symbol: u16,
) -> bool {
    let start = lexer.lookahead as u8;
    if start == b'@' || start == b'$' {
        if has_content {
            lexer.result_symbol = content_symbol;
            return true;
        }
        lexer.mark_end();
        lexer.advance(false);
        let mut is_short_interpolation = false;
        if start == b'$' {
            if short_interpolation_global(lexer.lookahead as u8) {
                is_short_interpolation = true;
            } else if lexer.lookahead == ch('-') {
                lexer.advance(false);
                is_short_interpolation = iswalpha(lexer.lookahead) || lexer.lookahead == ch('_');
            } else {
                is_short_interpolation = iswalnum(lexer.lookahead) || lexer.lookahead == ch('_');
            }
        }
        if start == b'@' {
            if lexer.lookahead == ch('@') {
                lexer.advance(false);
            }
            is_short_interpolation = is_iden_char(lexer.lookahead) && !iswdigit(lexer.lookahead);
        }

        if is_short_interpolation {
            lexer.result_symbol = tok::SHORT_INTERPOLATION;
            return true;
        }
    }
    false
}

impl Scanner {
    /// skip
    fn skip(&mut self, lexer: &mut gositter::Lexer<'_>) {
        self.has_leading_whitespace = true;
        lexer.advance(true);
    }

    /// scan_whitespace
    fn scan_whitespace(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let heredoc_body_start_is_valid = !self.open_heredocs.is_empty()
            && !self.open_heredocs[0].started
            && valid_symbols[tok::HEREDOC_BODY_START as usize];
        let mut crossed_newline = false;

        loop {
            if !valid_symbols[tok::NO_LINE_BREAK as usize]
                && valid_symbols[tok::LINE_BREAK as usize]
                && lexer.is_at_included_range_start()
            {
                lexer.mark_end();
                lexer.result_symbol = tok::LINE_BREAK;
                return true;
            }

            match la(lexer) {
                Some(' ' | '\t') => self.skip(lexer),
                Some('\r') => {
                    if heredoc_body_start_is_valid {
                        lexer.result_symbol = tok::HEREDOC_BODY_START;
                        self.open_heredocs[0].started = true;
                        return true;
                    }
                    self.skip(lexer);
                }
                Some('\n') => {
                    if heredoc_body_start_is_valid {
                        lexer.result_symbol = tok::HEREDOC_BODY_START;
                        self.open_heredocs[0].started = true;
                        return true;
                    } else if !valid_symbols[tok::NO_LINE_BREAK as usize]
                        && valid_symbols[tok::LINE_BREAK as usize]
                        && !crossed_newline
                    {
                        lexer.mark_end();
                        lexer.advance(false);
                        crossed_newline = true;
                    } else {
                        self.skip(lexer);
                    }
                }
                Some('\\') => {
                    lexer.advance(false);
                    if lexer.lookahead == ch('\r') {
                        self.skip(lexer);
                    }
                    if iswspace(lexer.lookahead) {
                        self.skip(lexer);
                    } else {
                        return false;
                    }
                }
                _ => {
                    if crossed_newline {
                        if lexer.lookahead != ch('.')
                            && lexer.lookahead != ch('&')
                            && lexer.lookahead != ch('#')
                        {
                            lexer.result_symbol = tok::LINE_BREAK;
                        } else if lexer.lookahead == ch('.') {
                            // Don't return LINE_BREAK for the call operator (`.`) but do return one for range
                            // operators (`..` and `...`)
                            lexer.advance(false);
                            if !lexer.eof() && lexer.lookahead == ch('.') {
                                lexer.result_symbol = tok::LINE_BREAK;
                            } else {
                                return false;
                            }
                        }
                    }
                    return true;
                }
            }
        }
    }

    /// scan_open_delimiter
    fn scan_open_delimiter(
        &mut self,
        lexer: &mut gositter::Lexer<'_>,
        literal: &mut Literal,
        valid_symbols: &[bool],
    ) -> bool {
        match la(lexer) {
            Some('"') => {
                literal.typ = tok::STRING_START;
                literal.open_delimiter = lexer.lookahead;
                literal.close_delimiter = lexer.lookahead;
                literal.allows_interpolation = true;
                lexer.advance(false);
                true
            }

            Some('\'') => {
                literal.typ = tok::STRING_START;
                literal.open_delimiter = lexer.lookahead;
                literal.close_delimiter = lexer.lookahead;
                literal.allows_interpolation = false;
                lexer.advance(false);
                true
            }

            Some('`') => {
                if !valid_symbols[tok::SUBSHELL_START as usize] {
                    return false;
                }
                literal.typ = tok::SUBSHELL_START;
                literal.open_delimiter = lexer.lookahead;
                literal.close_delimiter = lexer.lookahead;
                literal.allows_interpolation = true;
                lexer.advance(false);
                true
            }

            Some('/') => {
                if !valid_symbols[tok::REGEX_START as usize] {
                    return false;
                }
                literal.typ = tok::REGEX_START;
                literal.open_delimiter = lexer.lookahead;
                literal.close_delimiter = lexer.lookahead;
                literal.allows_interpolation = true;
                lexer.advance(false);
                if valid_symbols[tok::FORWARD_SLASH as usize] {
                    if !self.has_leading_whitespace {
                        return false;
                    }
                    if lexer.lookahead == ch(' ')
                        || lexer.lookahead == ch('\t')
                        || lexer.lookahead == ch('\n')
                        || lexer.lookahead == ch('\r')
                    {
                        return false;
                    }
                    if lexer.lookahead == ch('=') {
                        return false;
                    }
                }
                true
            }

            Some('%') => {
                lexer.advance(false);

                match la(lexer) {
                    Some('s') => {
                        if !valid_symbols[tok::SIMPLE_SYMBOL as usize] {
                            return false;
                        }
                        literal.typ = tok::SYMBOL_START;
                        literal.allows_interpolation = false;
                        lexer.advance(false);
                    }

                    Some('r') => {
                        if !valid_symbols[tok::REGEX_START as usize] {
                            return false;
                        }
                        literal.typ = tok::REGEX_START;
                        literal.allows_interpolation = true;
                        lexer.advance(false);
                    }

                    Some('x') => {
                        if !valid_symbols[tok::SUBSHELL_START as usize] {
                            return false;
                        }
                        literal.typ = tok::SUBSHELL_START;
                        literal.allows_interpolation = true;
                        lexer.advance(false);
                    }

                    Some('q') => {
                        if !valid_symbols[tok::STRING_START as usize] {
                            return false;
                        }
                        literal.typ = tok::STRING_START;
                        literal.allows_interpolation = false;
                        lexer.advance(false);
                    }

                    Some('Q') => {
                        if !valid_symbols[tok::STRING_START as usize] {
                            return false;
                        }
                        literal.typ = tok::STRING_START;
                        literal.allows_interpolation = true;
                        lexer.advance(false);
                    }

                    Some('w') => {
                        if !valid_symbols[tok::STRING_ARRAY_START as usize] {
                            return false;
                        }
                        literal.typ = tok::STRING_ARRAY_START;
                        literal.allows_interpolation = false;
                        lexer.advance(false);
                    }

                    Some('i') => {
                        if !valid_symbols[tok::SYMBOL_ARRAY_START as usize] {
                            return false;
                        }
                        literal.typ = tok::SYMBOL_ARRAY_START;
                        literal.allows_interpolation = false;
                        lexer.advance(false);
                    }

                    Some('W') => {
                        if !valid_symbols[tok::STRING_ARRAY_START as usize] {
                            return false;
                        }
                        literal.typ = tok::STRING_ARRAY_START;
                        literal.allows_interpolation = true;
                        lexer.advance(false);
                    }

                    Some('I') => {
                        if !valid_symbols[tok::SYMBOL_ARRAY_START as usize] {
                            return false;
                        }
                        literal.typ = tok::SYMBOL_ARRAY_START;
                        literal.allows_interpolation = true;
                        lexer.advance(false);
                    }

                    _ => {
                        if !valid_symbols[tok::STRING_START as usize] {
                            return false;
                        }
                        literal.typ = tok::STRING_START;
                        literal.allows_interpolation = true;
                    }
                }

                match la(lexer) {
                    Some('(') => {
                        literal.open_delimiter = ch('(');
                        literal.close_delimiter = ch(')');
                    }

                    Some('[') => {
                        literal.open_delimiter = ch('[');
                        literal.close_delimiter = ch(']');
                    }

                    Some('{') => {
                        literal.open_delimiter = ch('{');
                        literal.close_delimiter = ch('}');
                    }

                    Some('<') => {
                        literal.open_delimiter = ch('<');
                        literal.close_delimiter = ch('>');
                    }

                    Some('\r' | '\n' | ' ' | '\t') => {
                        // If the `/` operator is valid, then so is the `%` operator, which means
                        // that a `%` followed by whitespace should be considered an operator,
                        // not a percent string.
                        if valid_symbols[tok::FORWARD_SLASH as usize] {
                            return false;
                        }
                        // Both delimiters stay 0 here, as in C: the literal then ends
                        // at a NUL or at the end of input.
                    }

                    Some(
                        '|' | '!' | '#' | '/' | '\\' | '@' | '$' | '%' | '^' | '&' | '*' | ')'
                        | ']' | '}' | '>' | '+' | '-' | '~' | '`' | ',' | '.' | '?' | ':' | ';'
                        | '_' | '"' | '\'',
                    ) => {
                        literal.open_delimiter = lexer.lookahead;
                        literal.close_delimiter = lexer.lookahead;
                    }
                    _ => return false,
                }

                lexer.advance(false);
                true
            }

            _ => false,
        }
    }

    /// scan_heredoc_content
    fn scan_heredoc_content(&mut self, lexer: &mut gositter::Lexer<'_>) -> bool {
        let mut position_in_word = 0;
        let mut look_for_heredoc_end = true;
        let mut has_content = false;

        loop {
            if position_in_word == self.open_heredocs[0].word.len() {
                if !has_content {
                    lexer.mark_end();
                }
                while lexer.lookahead == ch(' ') || lexer.lookahead == ch('\t') {
                    lexer.advance(false);
                }
                if lexer.lookahead == ch('\n') || lexer.lookahead == ch('\r') {
                    if has_content {
                        lexer.result_symbol = tok::HEREDOC_CONTENT;
                    } else {
                        self.erase_first_heredoc();
                        lexer.result_symbol = tok::HEREDOC_BODY_END;
                    }
                    return true;
                }
                has_content = true;
                position_in_word = 0;
            }

            if lexer.eof() {
                lexer.mark_end();
                if has_content {
                    lexer.result_symbol = tok::HEREDOC_CONTENT;
                } else {
                    self.erase_first_heredoc();
                    lexer.result_symbol = tok::HEREDOC_BODY_END;
                }
                return true;
            }

            // The word is an Array(char). Where char is signed a byte >= 0x80 compares as
            // negative: 0xFF equals the -1 of an invalid UTF-8 byte and nothing else matches.
            // Where it is unsigned (linux/arm64) 0x80..0xFF equal U+0080..U+00FF.
            // A word is never empty (serialize and the scan of the word see to it), so the index
            // is in range as C asserts.
            if lexer.lookahead == cchar::value(self.open_heredocs[0].word[position_in_word])
                && look_for_heredoc_end
            {
                lexer.advance(false);
                position_in_word += 1;
            } else {
                position_in_word = 0;
                look_for_heredoc_end = false;

                if self.open_heredocs[0].allows_interpolation && lexer.lookahead == ch('\\') {
                    if has_content {
                        lexer.result_symbol = tok::HEREDOC_CONTENT;
                        return true;
                    }
                    return false;
                }

                if self.open_heredocs[0].allows_interpolation && lexer.lookahead == ch('#') {
                    lexer.mark_end();
                    lexer.advance(false);
                    if lexer.lookahead == ch('{') {
                        if has_content {
                            lexer.result_symbol = tok::HEREDOC_CONTENT;
                            return true;
                        }
                        return false;
                    }
                    if scan_short_interpolation(lexer, has_content, tok::HEREDOC_CONTENT) {
                        return true;
                    }
                } else if lexer.lookahead == ch('\r') || lexer.lookahead == ch('\n') {
                    if lexer.lookahead == ch('\r') {
                        lexer.advance(false);
                        if lexer.lookahead == ch('\n') {
                            lexer.advance(false);
                        }
                    } else {
                        lexer.advance(false);
                    }
                    has_content = true;
                    look_for_heredoc_end = true;
                    while lexer.lookahead == ch(' ') || lexer.lookahead == ch('\t') {
                        lexer.advance(false);
                        if !self.open_heredocs[0].end_word_indentation_allowed {
                            look_for_heredoc_end = false;
                        }
                    }
                    lexer.mark_end();
                } else {
                    has_content = true;
                    lexer.advance(false);
                    lexer.mark_end();
                }
            }
        }
    }

    /// scan_literal_content
    fn scan_literal_content(&mut self, lexer: &mut gositter::Lexer<'_>) -> bool {
        // `literal` is C's pointer to the top of the stack; only nesting_depth changes, and the
        // pop is followed by a return.
        let top = self.literal_stack.len() - 1;
        let typ = self.literal_stack[top].typ;
        let open_delimiter = self.literal_stack[top].open_delimiter;
        let close_delimiter = self.literal_stack[top].close_delimiter;
        let allows_interpolation = self.literal_stack[top].allows_interpolation;
        let mut has_content = false;
        let stop_on_space = typ == tok::SYMBOL_ARRAY_START || typ == tok::STRING_ARRAY_START;

        loop {
            if stop_on_space && iswspace(lexer.lookahead) {
                if has_content {
                    lexer.mark_end();
                    lexer.result_symbol = tok::STRING_CONTENT;
                    return true;
                }
                return false;
            }
            if lexer.lookahead == close_delimiter {
                lexer.mark_end();
                if self.literal_stack[top].nesting_depth == 1 {
                    if has_content {
                        lexer.result_symbol = tok::STRING_CONTENT;
                    } else {
                        lexer.advance(false);
                        if typ == tok::REGEX_START {
                            while iswlower(lexer.lookahead) {
                                lexer.advance(false);
                            }
                        }
                        self.literal_stack.pop();
                        lexer.result_symbol = tok::STRING_END;
                        lexer.mark_end();
                    }
                    return true;
                }
                self.literal_stack[top].nesting_depth =
                    self.literal_stack[top].nesting_depth.wrapping_sub(1);
                lexer.advance(false);
            } else if lexer.lookahead == open_delimiter {
                self.literal_stack[top].nesting_depth =
                    self.literal_stack[top].nesting_depth.wrapping_add(1);
                lexer.advance(false);
            } else if allows_interpolation && lexer.lookahead == ch('#') {
                lexer.mark_end();
                lexer.advance(false);
                if lexer.lookahead == ch('{') {
                    if has_content {
                        lexer.result_symbol = tok::STRING_CONTENT;
                        return true;
                    }
                    return false;
                }
                if scan_short_interpolation(lexer, has_content, tok::STRING_CONTENT) {
                    return true;
                }
            } else if lexer.lookahead == ch('\\') {
                if allows_interpolation {
                    if has_content {
                        lexer.mark_end();
                        lexer.result_symbol = tok::STRING_CONTENT;
                        return true;
                    }
                    return false;
                }
                lexer.advance(false);
                lexer.advance(false);
            } else if lexer.eof() {
                lexer.advance(false);
                lexer.mark_end();
                return false;
            } else {
                lexer.advance(false);
            }

            has_content = true;
        }
    }
}

impl gositter::ExternalScanner for Scanner {
    /// scan / tree_sitter_ruby_external_scanner_scan
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        self.has_leading_whitespace = false;

        // Contents of literals, which match any character except for some close delimiter
        if !valid_symbols[tok::STRING_START as usize] {
            if (valid_symbols[tok::STRING_CONTENT as usize]
                || valid_symbols[tok::STRING_END as usize])
                && !self.literal_stack.is_empty()
            {
                return self.scan_literal_content(lexer);
            }
            if (valid_symbols[tok::HEREDOC_CONTENT as usize]
                || valid_symbols[tok::HEREDOC_BODY_END as usize])
                && !self.open_heredocs.is_empty()
            {
                return self.scan_heredoc_content(lexer);
            }
        }

        // Whitespace
        lexer.result_symbol = tok::NONE;
        if !self.scan_whitespace(lexer, valid_symbols) {
            return false;
        }
        if lexer.result_symbol != tok::NONE {
            return true;
        }

        match la(lexer) {
            Some('&') if valid_symbols[tok::BLOCK_AMPERSAND as usize] => {
                lexer.advance(false);
                if lexer.lookahead != ch('&')
                    && lexer.lookahead != ch('.')
                    && lexer.lookahead != ch('=')
                    && !iswspace(lexer.lookahead)
                {
                    lexer.result_symbol = tok::BLOCK_AMPERSAND;
                    return true;
                }
                return false;
            }

            Some('<') if valid_symbols[tok::SINGLETON_CLASS_LEFT_ANGLE_LEFT_ANGLE as usize] => {
                lexer.advance(false);
                if lexer.lookahead == ch('<') {
                    lexer.advance(false);
                    lexer.result_symbol = tok::SINGLETON_CLASS_LEFT_ANGLE_LEFT_ANGLE;
                    return true;
                }
                return false;
            }

            Some('*')
                if valid_symbols[tok::SPLAT_STAR as usize]
                    || valid_symbols[tok::BINARY_STAR as usize]
                    || valid_symbols[tok::HASH_SPLAT_STAR_STAR as usize]
                    || valid_symbols[tok::BINARY_STAR_STAR as usize] =>
            {
                lexer.advance(false);
                if lexer.lookahead == ch('=') {
                    return false;
                }
                if lexer.lookahead == ch('*') {
                    if valid_symbols[tok::HASH_SPLAT_STAR_STAR as usize]
                        || valid_symbols[tok::BINARY_STAR_STAR as usize]
                    {
                        lexer.advance(false);
                        if lexer.lookahead == ch('=') {
                            return false;
                        }
                        if valid_symbols[tok::BINARY_STAR_STAR as usize]
                            && !self.has_leading_whitespace
                        {
                            lexer.result_symbol = tok::BINARY_STAR_STAR;
                            return true;
                        }
                        if valid_symbols[tok::HASH_SPLAT_STAR_STAR as usize]
                            && !iswspace(lexer.lookahead)
                        {
                            lexer.result_symbol = tok::HASH_SPLAT_STAR_STAR;
                            return true;
                        }
                        if valid_symbols[tok::BINARY_STAR_STAR as usize] {
                            lexer.result_symbol = tok::BINARY_STAR_STAR;
                            return true;
                        }
                        if valid_symbols[tok::HASH_SPLAT_STAR_STAR as usize] {
                            lexer.result_symbol = tok::HASH_SPLAT_STAR_STAR;
                            return true;
                        }
                        return false;
                    }
                    return false;
                }
                if valid_symbols[tok::BINARY_STAR as usize] && !self.has_leading_whitespace {
                    lexer.result_symbol = tok::BINARY_STAR;
                    return true;
                }
                if valid_symbols[tok::SPLAT_STAR as usize] && !iswspace(lexer.lookahead) {
                    lexer.result_symbol = tok::SPLAT_STAR;
                    return true;
                }
                if valid_symbols[tok::BINARY_STAR as usize] {
                    lexer.result_symbol = tok::BINARY_STAR;
                    return true;
                }
                if valid_symbols[tok::SPLAT_STAR as usize] {
                    lexer.result_symbol = tok::SPLAT_STAR;
                    return true;
                }
                return false;
            }

            Some('-')
                if valid_symbols[tok::UNARY_MINUS as usize]
                    || valid_symbols[tok::UNARY_MINUS_NUM as usize]
                    || valid_symbols[tok::BINARY_MINUS as usize] =>
            {
                lexer.advance(false);
                if lexer.lookahead != ch('=') && lexer.lookahead != ch('>') {
                    if valid_symbols[tok::UNARY_MINUS_NUM as usize]
                        && (!valid_symbols[tok::BINARY_STAR as usize]
                            || self.has_leading_whitespace)
                        && iswdigit(lexer.lookahead)
                    {
                        lexer.result_symbol = tok::UNARY_MINUS_NUM;
                        return true;
                    }
                    if valid_symbols[tok::UNARY_MINUS as usize]
                        && self.has_leading_whitespace
                        && !iswspace(lexer.lookahead)
                    {
                        lexer.result_symbol = tok::UNARY_MINUS;
                    } else if valid_symbols[tok::BINARY_MINUS as usize] {
                        lexer.result_symbol = tok::BINARY_MINUS;
                    } else {
                        lexer.result_symbol = tok::UNARY_MINUS;
                    }
                    return true;
                }
                return false;
            }

            Some(':') if valid_symbols[tok::SYMBOL_START as usize] => {
                let mut literal = Literal {
                    typ: tok::SYMBOL_START,
                    nesting_depth: 1,
                    ..Literal::default()
                };
                lexer.advance(false);

                match la(lexer) {
                    Some('"') => {
                        lexer.advance(false);
                        literal.open_delimiter = ch('"');
                        literal.close_delimiter = ch('"');
                        literal.allows_interpolation = true;
                        self.literal_stack.push(literal);
                        lexer.result_symbol = tok::SYMBOL_START;
                        return true;
                    }

                    Some('\'') => {
                        lexer.advance(false);
                        literal.open_delimiter = ch('\'');
                        literal.close_delimiter = ch('\'');
                        literal.allows_interpolation = false;
                        self.literal_stack.push(literal);
                        lexer.result_symbol = tok::SYMBOL_START;
                        return true;
                    }

                    _ => {
                        if scan_symbol_identifier(lexer) {
                            lexer.result_symbol = tok::SIMPLE_SYMBOL;
                            return true;
                        }
                    }
                }

                return false;
            }

            // Treat a square bracket as an element reference if either:
            // * the bracket is not preceded by any whitespace
            // * an arbitrary expression is not valid at the current position.
            Some('[')
                if valid_symbols[tok::ELEMENT_REFERENCE_BRACKET as usize]
                    && (!self.has_leading_whitespace
                        || !valid_symbols[tok::STRING_START as usize]) =>
            {
                lexer.advance(false);
                lexer.result_symbol = tok::ELEMENT_REFERENCE_BRACKET;
                return true;
            }

            _ => {}
        }

        // Open delimiters for literals
        if ((valid_symbols[tok::HASH_KEY_SYMBOL as usize]
            || valid_symbols[tok::IDENTIFIER_SUFFIX as usize])
            && (iswalpha(lexer.lookahead) || lexer.lookahead == ch('_')))
            || (valid_symbols[tok::CONSTANT_SUFFIX as usize] && iswupper(lexer.lookahead))
        {
            let mut valid_identifier_symbol = tok::IDENTIFIER_SUFFIX;
            if iswupper(lexer.lookahead) {
                valid_identifier_symbol = tok::CONSTANT_SUFFIX;
            }
            while iswalnum(lexer.lookahead) || lexer.lookahead == ch('_') {
                lexer.advance(false);
            }

            if valid_symbols[tok::HASH_KEY_SYMBOL as usize] && lexer.lookahead == ch(':') {
                lexer.mark_end();
                lexer.advance(false);
                if lexer.lookahead != ch(':') {
                    lexer.result_symbol = tok::HASH_KEY_SYMBOL;
                    return true;
                }
            } else if valid_symbols[valid_identifier_symbol as usize] && lexer.lookahead == ch('!')
            {
                lexer.advance(false);
                if lexer.lookahead != ch('=') {
                    lexer.result_symbol = valid_identifier_symbol;
                    return true;
                }
            }

            return false;
        }

        // Open delimiters for literals
        if valid_symbols[tok::STRING_START as usize] {
            let mut literal = Literal {
                nesting_depth: 1,
                ..Literal::default()
            };

            if lexer.lookahead == ch('<') {
                lexer.advance(false);
                if lexer.lookahead != ch('<') {
                    return false;
                }
                lexer.advance(false);

                let mut hd = Heredoc::default();
                if lexer.lookahead == ch('-') || lexer.lookahead == ch('~') {
                    lexer.advance(false);
                    hd.end_word_indentation_allowed = true;
                }

                (hd.word, hd.allows_interpolation) = scan_heredoc_word(lexer);
                if hd.word.is_empty() {
                    return false;
                }
                self.open_heredocs.push(hd);
                lexer.result_symbol = tok::HEREDOC_START;
                return true;
            }

            if self.scan_open_delimiter(lexer, &mut literal, valid_symbols) {
                let typ = literal.typ;
                self.literal_stack.push(literal);
                lexer.result_symbol = typ;
                return true;
            }
            return false;
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
