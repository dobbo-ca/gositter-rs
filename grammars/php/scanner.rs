//! Port of gositter `grammars/php/scanner.go` (tree-sitter-php v0.24.2 `common/scanner.h`):
//! state, serialize, deserialize and scan.

use crate::SERIALIZATION_BUFFER_SIZE;

// enum TokenType.
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
    // ' ', then \t \n \v \f \r.
    c == 32 || (9..=13).contains(&c)
}

fn iswdigit(c: i32) -> bool {
    (48..=57).contains(&c)
}

fn iswxdigit(c: i32) -> bool {
    iswdigit(c) || (97..=102).contains(&c) || (65..=70).contains(&c)
}

fn iswalnum(c: i32) -> bool {
    iswdigit(c) || (97..=122).contains(&c) || (65..=90).contains(&c)
}

impl Scanner {
    /// `newScanner`
    pub fn new() -> Self {
        Self::default()
    }

    /// `Serialize`
    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        let mut size = 0;

        // (char) cast: the count wraps at 256. It cannot get that far, 205 empty heredocs already
        // hit the size check below.
        buf[size] = self.heredocs.len() as u8;
        size += 1;
        for h in &self.heredocs {
            let word_size = h.word.len() * 4;
            if size + 5 + word_size >= SERIALIZATION_BUFFER_SIZE {
                return 0;
            }
            buf[size] = u8::from(h.end_word_indentation_allowed);
            size += 1;
            buf[size..size + 4].copy_from_slice(&(h.word.len() as u32).to_le_bytes());
            size += 4;
            for c in &h.word {
                buf[size..size + 4].copy_from_slice(&c.to_le_bytes());
                size += 4;
            }
        }

        size
    }

    /// `Deserialize`
    pub fn deserialize(&mut self, buf: &[u8]) {
        let mut size = 0;
        self.has_leading_whitespace = false;

        // C empties every heredoc but keeps heredocs.size. The vector never shrinks here, so a
        // state with fewer heredocs (or the empty state) leaves the extra ones in place with an
        // empty word. Kept: the runtime compares the serialized bytes, and scan reads
        // heredocs.size.
        for h in &mut self.heredocs {
            *h = Heredoc::default();
        }

        if buf.is_empty() {
            return;
        }

        let open_heredoc_count = buf[size] as usize;
        size += 1;
        if self.heredocs.len() < open_heredoc_count {
            self.heredocs.resize(open_heredoc_count, Heredoc::default());
        }
        for h in self.heredocs.iter_mut().take(open_heredoc_count) {
            h.end_word_indentation_allowed = buf[size] != 0;
            size += 1;
            let n = u32::from_le_bytes([buf[size], buf[size + 1], buf[size + 2], buf[size + 3]])
                as usize;
            size += 4;
            if n > 0 {
                h.word = buf[size..size + 4 * n]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| i32::from_le_bytes(*c))
                    .collect();
                size += 4 * n;
            }
        }
        // C: assert(size == length). Holds for anything serialize wrote.
    }
}

/// `scan_whitespace`
fn scan_whitespace(lexer: &mut gositter::Lexer<'_>) -> bool {
    loop {
        while iswspace(lexer.lookahead) {
            lexer.advance(false);
        }

        if lexer.lookahead == '/' as i32 {
            lexer.advance(false);

            if lexer.lookahead == '/' as i32 {
                lexer.advance(false);
                while lexer.lookahead != 0 && lexer.lookahead != '\n' as i32 {
                    lexer.advance(false);
                }
            } else {
                return false;
            }
        } else {
            return true;
        }
    }
}

/// `is_valid_name_char`
fn is_valid_name_char(lexer: &gositter::Lexer<'_>) -> bool {
    iswalnum(lexer.lookahead) || lexer.lookahead == '_' as i32 || lexer.lookahead >= 0x80
}

/// `is_escapable_sequence`
fn is_escapable_sequence(lexer: &mut gositter::Lexer<'_>) -> bool {
    let letter = lexer.lookahead;

    if matches!(
        u8::try_from(letter),
        Ok(b'n' | b'r' | b't' | b'v' | b'e' | b'f' | b'\\' | b'$' | b'"')
    ) {
        return true;
    }

    // Hex
    if letter == 'x' as i32 {
        lexer.advance(false);
        return iswxdigit(lexer.lookahead);
    }

    // Unicode. grammar.js sorts out "\u{$a}".
    if letter == 'u' as i32 {
        return true;
    }

    // Octal
    iswdigit(lexer.lookahead) && (48..=55).contains(&lexer.lookahead)
}

/// `scan_heredoc_word`
fn scan_heredoc_word(lexer: &mut gositter::Lexer<'_>) -> Vec<i32> {
    let mut result = Vec::new();

    while is_valid_name_char(lexer) {
        result.push(lexer.lookahead);
        lexer.advance(false);
    }

    result
}

/// The end tag test shared by scan_nowdoc_string and scan_encapsed_part_string.
fn is_tag_terminator(c: i32) -> bool {
    iswspace(c) || c == ';' as i32 || c == ',' as i32 || c == ')' as i32
}

impl Scanner {
    /// `scan_nowdoc_string`
    fn scan_nowdoc_string(&self, lexer: &mut gositter::Lexer<'_>) -> bool {
        let mut has_consumed_content = false;
        if self.heredocs.is_empty() {
            return false;
        }

        // Any whitespace may come before the closing tag.
        while iswspace(lexer.lookahead) {
            lexer.advance(false);
            has_consumed_content = true;
        }

        let mut end_tag_matched = false;
        let heredoc_tag = &self.heredocs[self.heredocs.len() - 1].word;

        for (i, &c) in heredoc_tag.iter().enumerate() {
            if lexer.lookahead != c {
                break;
            }
            lexer.advance(false);
            has_consumed_content = true;

            end_tag_matched = i == heredoc_tag.len() - 1 && is_tag_terminator(lexer.lookahead);
        }

        if end_tag_matched {
            while iswspace(lexer.lookahead)
                && lexer.lookahead != '\r' as i32
                && lexer.lookahead != '\n' as i32
            {
                lexer.advance(false);
                has_consumed_content = true;
            }

            // Leave the end tag to HEREDOC_END.
            if lexer.lookahead == ';' as i32
                || lexer.lookahead == ',' as i32
                || lexer.lookahead == ')' as i32
                || lexer.lookahead == '\n' as i32
                || lexer.lookahead == '\r' as i32
            {
                return false;
            }
        }

        let mut has_content = has_consumed_content;
        loop {
            lexer.mark_end();

            match lexer.lookahead {
                0x0A | 0x0D => return has_content,
                _ => {
                    if lexer.eof() {
                        return false;
                    }
                    lexer.advance(false);
                }
            }

            has_content = true;
        }
    }

    /// `scan_encapsed_part_string`
    fn scan_encapsed_part_string(
        &self,
        lexer: &mut gositter::Lexer<'_>,
        mut is_after_variable: bool,
        is_heredoc: bool,
        is_execution_string: bool,
    ) -> bool {
        const DQUOTE: i32 = '"' as i32;
        const BACKTICK: i32 = '`' as i32;
        const NL: i32 = '\n' as i32;
        const CR: i32 = '\r' as i32;
        const BACKSLASH: i32 = '\\' as i32;
        const DOLLAR: i32 = '$' as i32;
        const DASH: i32 = '-' as i32;
        const LBRACKET: i32 = '[' as i32;
        const LBRACE: i32 = '{' as i32;

        let mut has_consumed_content = false;

        if is_heredoc && !self.heredocs.is_empty() {
            // Whitespace before the closing tag, but not \r or \n.
            while iswspace(lexer.lookahead) && lexer.lookahead != CR && lexer.lookahead != NL {
                lexer.advance(false);
                has_consumed_content = true;
            }

            let heredoc_tag = &self.heredocs[self.heredocs.len() - 1].word;

            let mut end_tag_matched = false;

            for (i, &c) in heredoc_tag.iter().enumerate() {
                if lexer.lookahead != c {
                    break;
                }
                has_consumed_content = true;
                lexer.advance(false);

                end_tag_matched = i == heredoc_tag.len() - 1 && is_tag_terminator(lexer.lookahead);
            }

            if end_tag_matched {
                while iswspace(lexer.lookahead) && lexer.lookahead != CR && lexer.lookahead != NL {
                    lexer.advance(false);
                    has_consumed_content = true;
                }

                // Leave the end tag to HEREDOC_END.
                if lexer.lookahead == ';' as i32
                    || lexer.lookahead == ',' as i32
                    || lexer.lookahead == ')' as i32
                    || lexer.lookahead == NL
                    || lexer.lookahead == CR
                {
                    return false;
                }
            }
        }

        let mut has_content = has_consumed_content;
        loop {
            lexer.mark_end();

            // A Go `break` inside the switch leaves only the switch.
            'sw: {
                match lexer.lookahead {
                    DQUOTE => {
                        if !is_heredoc && !is_execution_string {
                            return has_content;
                        }
                        lexer.advance(false);
                    }
                    BACKTICK => {
                        if is_execution_string {
                            return has_content;
                        }
                        lexer.advance(false);
                    }
                    NL | CR => {
                        if is_heredoc {
                            return has_content;
                        }
                        lexer.advance(false);
                    }
                    BACKSLASH => {
                        lexer.advance(false);

                        // \{ is not an escape; both are plain characters.
                        if lexer.lookahead == LBRACE {
                            lexer.advance(false);
                            break 'sw;
                        }

                        if is_execution_string && lexer.lookahead == BACKTICK {
                            return has_content;
                        }

                        if is_heredoc && lexer.lookahead == BACKSLASH {
                            lexer.advance(false);
                            break 'sw;
                        }

                        if is_escapable_sequence(lexer) {
                            return has_content;
                        }
                    }
                    DOLLAR => {
                        lexer.advance(false);

                        if (is_valid_name_char(lexer) && !iswdigit(lexer.lookahead))
                            || lexer.lookahead == LBRACE
                        {
                            return has_content;
                        }
                    }
                    DASH | LBRACKET => {
                        if lexer.lookahead == DASH && is_after_variable {
                            lexer.advance(false);
                            if lexer.lookahead == '>' as i32 {
                                lexer.advance(false);
                                if is_valid_name_char(lexer) {
                                    return has_content;
                                }
                            }
                            break 'sw;
                        }
                        // C falls through from '-' into '[' when not after a variable.
                        if is_after_variable {
                            return has_content;
                        }
                        lexer.advance(false);
                    }
                    LBRACE => {
                        lexer.advance(false);
                        if lexer.lookahead == DOLLAR {
                            return has_content;
                        }
                    }
                    _ => {
                        if lexer.eof() {
                            return false;
                        }
                        lexer.advance(false);
                    }
                }
            }

            is_after_variable = false;
            has_content = true;
        }
    }
}

impl gositter::ExternalScanner for Scanner {
    /// `scan`
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let v = |t: u16| valid_symbols[t as usize];
        let is_error_recovery = v(tok::SENTINEL_ERROR);

        if is_error_recovery {
            return false;
        }

        self.has_leading_whitespace = false;

        lexer.mark_end();

        if v(tok::ENCAPSED_STRING_CHARS_AFTER_VARIABLE) {
            lexer.result_symbol = tok::ENCAPSED_STRING_CHARS_AFTER_VARIABLE;
            return self.scan_encapsed_part_string(lexer, true, false, false);
        }

        if v(tok::ENCAPSED_STRING_CHARS) {
            lexer.result_symbol = tok::ENCAPSED_STRING_CHARS;
            return self.scan_encapsed_part_string(lexer, false, false, false);
        }

        if v(tok::EXECUTION_STRING_CHARS_AFTER_VARIABLE) {
            lexer.result_symbol = tok::EXECUTION_STRING_CHARS_AFTER_VARIABLE;
            return self.scan_encapsed_part_string(lexer, true, false, true);
        }

        if v(tok::EXECUTION_STRING_CHARS) {
            lexer.result_symbol = tok::EXECUTION_STRING_CHARS;
            return self.scan_encapsed_part_string(lexer, false, false, true);
        }

        if v(tok::ENCAPSED_STRING_CHARS_AFTER_VARIABLE_HEREDOC) {
            lexer.result_symbol = tok::ENCAPSED_STRING_CHARS_AFTER_VARIABLE_HEREDOC;
            return self.scan_encapsed_part_string(lexer, true, true, false);
        }

        if v(tok::ENCAPSED_STRING_CHARS_HEREDOC) {
            lexer.result_symbol = tok::ENCAPSED_STRING_CHARS_HEREDOC;
            return self.scan_encapsed_part_string(lexer, false, true, false);
        }

        if v(tok::NOWDOC_STRING) {
            lexer.result_symbol = tok::NOWDOC_STRING;
            return self.scan_nowdoc_string(lexer);
        }

        if v(tok::HEREDOC_END) {
            lexer.result_symbol = tok::HEREDOC_END;
            if self.heredocs.is_empty() {
                return false;
            }

            let want = &self.heredocs[self.heredocs.len() - 1].word;

            while iswspace(lexer.lookahead) {
                lexer.advance(true);
            }

            // string_eq. An emptied heredoc (see deserialize) matches an
            // empty word, as in C.
            if scan_heredoc_word(lexer) != *want {
                return false;
            }

            lexer.mark_end();
            self.heredocs.pop();
            return true;
        }

        if !scan_whitespace(lexer) {
            return false;
        }

        if v(tok::EOF_TOKEN) && lexer.eof() {
            lexer.result_symbol = tok::EOF_TOKEN;
            return true;
        }

        if v(tok::HEREDOC_START) {
            lexer.result_symbol = tok::HEREDOC_START;

            while iswspace(lexer.lookahead) {
                lexer.advance(true);
            }

            let word = scan_heredoc_word(lexer);
            if word.is_empty() {
                return false;
            }
            lexer.mark_end();

            self.heredocs.push(Heredoc {
                word,
                ..Heredoc::default()
            });
            return true;
        }

        if v(tok::AUTOMATIC_SEMICOLON) {
            lexer.result_symbol = tok::AUTOMATIC_SEMICOLON;

            if lexer.lookahead != '?' as i32 {
                return false;
            }

            lexer.advance(false);

            return lexer.lookahead == '>' as i32;
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
