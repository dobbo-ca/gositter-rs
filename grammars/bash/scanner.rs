//! Port of `grammars/bash/scanner.go` (tree-sitter-bash v0.25.1 `src/scanner.c`): the scanner
//! state (`new`, `serialize`, `deserialize`, `reset`) and `scan` with the helpers it uses.

use crate::{SERIALIZATION_BUFFER_SIZE, cchar};

/// Heredoc. The two strings are C `Array(char)`: lookahead is cut to its low byte on push and
/// read back as a char (`cchar::value`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heredoc {
    pub is_raw: bool,
    pub started: bool,
    pub allows_indent: bool,
    pub delimiter: Vec<u8>,
    pub current_leading_word: Vec<u8>,
}

impl Heredoc {
    /// `reset_heredoc`.
    fn reset(&mut self) {
        self.is_raw = false;
        self.started = false;
        self.allows_indent = false;
        self.delimiter.clear(); // reset_string
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scanner {
    pub last_glob_paren_depth: u8,
    pub ext_was_in_double_quote: bool,
    pub ext_saw_outside_quote: bool,
    pub heredocs: Vec<Heredoc>,
}

impl Scanner {
    /// `tree_sitter_bash_external_scanner_create`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `heredoc_new` + `array_push`.
    pub fn push_heredoc(&mut self, allows_indent: bool) {
        self.heredocs.push(Heredoc {
            allows_indent,
            ..Heredoc::default()
        });
    }

    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.last_glob_paren_depth;
        buf[1] = u8::from(self.ext_was_in_double_quote);
        buf[2] = u8::from(self.ext_saw_outside_quote);
        buf[3] = self.heredocs.len() as u8; // (char) of the count, wraps at 256
        let mut size = 4;

        for h in &self.heredocs {
            if size + 3 + 4 + h.delimiter.len() >= SERIALIZATION_BUFFER_SIZE {
                return 0;
            }

            buf[size] = u8::from(h.is_raw);
            buf[size + 1] = u8::from(h.started);
            buf[size + 2] = u8::from(h.allows_indent);
            size += 3;

            buf[size..size + 4].copy_from_slice(&(h.delimiter.len() as u32).to_le_bytes());
            size += 4;
            buf[size..size + h.delimiter.len()].copy_from_slice(&h.delimiter);
            size += h.delimiter.len();
        }
        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            // reset: C clears each heredoc and keeps the count.
            for h in &mut self.heredocs {
                h.reset();
            }
            return;
        }
        self.last_glob_paren_depth = buf[0];
        self.ext_was_in_double_quote = buf[1] != 0;
        self.ext_saw_outside_quote = buf[2] != 0;
        let heredoc_count = usize::from(buf[3]);
        let mut size = 4;
        // C never shrinks heredocs here: slots past heredoc_count stay as they are.
        while self.heredocs.len() < heredoc_count {
            self.push_heredoc(false);
        }
        for h in self.heredocs.iter_mut().take(heredoc_count) {
            h.is_raw = buf[size] != 0;
            h.started = buf[size + 1] != 0;
            h.allows_indent = buf[size + 2] != 0;
            size += 3;

            let n = u32::from_le_bytes([buf[size], buf[size + 1], buf[size + 2], buf[size + 3]])
                as usize;
            size += 4;
            h.delimiter.clear();
            h.delimiter.extend_from_slice(&buf[size..size + n]);
            size += n;
        }
    }

    /// The state `new` returns, with the heredoc slots kept.
    pub fn reset(&mut self) {
        let mut heredocs = std::mem::take(&mut self.heredocs);
        // Give back what a huge input grew.
        if heredocs.capacity() > 16
            || heredocs
                .iter()
                .any(|d| d.delimiter.capacity() > 1024 || d.current_leading_word.capacity() > 1024)
        {
            heredocs = Vec::new();
        } else {
            heredocs.clear();
        }
        *self = Scanner {
            heredocs,
            ..Scanner::default()
        };
    }
}

/// enum TokenType, in the order of the grammar's externals.
#[derive(Clone, Copy)]
#[allow(dead_code)] // Esac is never read or set by scan
enum Tok {
    HeredocStart,
    SimpleHeredocBody,
    HeredocBodyBeginning,
    HeredocContent,
    HeredocEnd,
    FileDescriptor,
    EmptyValue,
    Concat,
    VariableName,
    TestOperator,
    Regex,
    RegexNoSlash,
    RegexNoSpace,
    ExpansionWord,
    ExtglobPattern,
    BareDollar,
    BraceStart,
    ImmediateDoubleHash,
    ExternalExpansionSymHash,
    ExternalExpansionSymBang,
    ExternalExpansionSymEqual,
    ClosingBrace,
    ClosingBracket,
    HeredocArrow,
    HeredocArrowDash,
    Newline,
    OpeningParen,
    Esac,
    ErrorRecovery,
}

/// A char literal as the lexer's lookahead.
const fn c(ch: char) -> i32 {
    ch as i32
}

// Patterns of the `switch (lexer->lookahead)` statements.
const BSLASH: i32 = '\\' as i32;
const DOLLAR: i32 = '$' as i32;
const NL: i32 = '\n' as i32;
const LPAREN: i32 = '(' as i32;
const RPAREN: i32 = ')' as i32;
const LBRACKET: i32 = '[' as i32;
const RBRACKET: i32 = ']' as i32;
const LBRACE: i32 = '{' as i32;
const RBRACE: i32 = '}' as i32;
const SQUOTE: i32 = '\'' as i32;

// The isw* calls of C run in the "C" locale (nothing calls setlocale), where
// macOS libc classifies ASCII only. -1 (invalid UTF-8) and 0 are in no class.

/// iswspace
fn iswspace(ch: i32) -> bool {
    ch == c(' ') || (c('\t')..=c('\r')).contains(&ch)
}

/// iswalpha
fn iswalpha(ch: i32) -> bool {
    (c('a')..=c('z')).contains(&ch) || (c('A')..=c('Z')).contains(&ch)
}

/// iswdigit, isdigit
fn iswdigit(ch: i32) -> bool {
    (c('0')..=c('9')).contains(&ch)
}

/// iswalnum
fn iswalnum(ch: i32) -> bool {
    iswalpha(ch) || iswdigit(ch)
}

/// advance_word
fn advance_word(l: &mut gositter::Lexer<'_>, unquoted_word: &mut Vec<u8>) -> bool {
    let mut empty = true;

    let mut quote = 0;
    if l.lookahead == c('\'') || l.lookahead == c('"') {
        quote = l.lookahead;
        l.advance(false);
    }

    while l.lookahead != 0 {
        if quote != 0 {
            if l.lookahead == quote || l.lookahead == c('\r') || l.lookahead == c('\n') {
                break;
            }
        } else if iswspace(l.lookahead) {
            break;
        }
        if l.lookahead == c('\\') {
            l.advance(false);
            if l.lookahead == 0 {
                return false;
            }
        }
        empty = false;
        unquoted_word.push(l.lookahead as u8);
        l.advance(false);
    }
    unquoted_word.push(0);

    if quote != 0 && l.lookahead == quote {
        l.advance(false);
    }

    !empty
}

/// scan_bare_dollar
fn scan_bare_dollar(l: &mut gositter::Lexer<'_>) -> bool {
    while iswspace(l.lookahead) && l.lookahead != c('\n') && !l.eof() {
        l.advance(true);
    }

    if l.lookahead == c('$') {
        l.advance(false);
        l.result_symbol = Tok::BareDollar as gositter::Symbol;
        l.mark_end();
        return iswspace(l.lookahead) || l.eof() || l.lookahead == c('"');
    }

    false
}

/// scan_heredoc_start
fn scan_heredoc_start(h: &mut Heredoc, l: &mut gositter::Lexer<'_>) -> bool {
    while iswspace(l.lookahead) {
        l.advance(true);
    }

    l.result_symbol = Tok::HeredocStart as gositter::Symbol;
    h.is_raw = l.lookahead == c('\'') || l.lookahead == c('"') || l.lookahead == c('\\');

    // Appends to whatever the delimiter already holds, as C does.
    let found_delimiter = advance_word(l, &mut h.delimiter);
    if !found_delimiter {
        h.delimiter.clear();
        return false;
    }
    found_delimiter
}

/// strcmp(a, b) == 0
fn cstr_equal(a: &[u8], b: &[u8]) -> bool {
    for (x, y) in a.iter().zip(b) {
        if x != y {
            return false;
        }
        if *x == 0 {
            return true;
        }
    }
    false
}

/// scan_heredoc_end_identifier
fn scan_heredoc_end_identifier(h: &mut Heredoc, l: &mut gositter::Lexer<'_>) -> bool {
    h.current_leading_word.clear();
    // Scan the first 'n' characters on this line, to see if they match the
    // heredoc delimiter
    let mut size = 0;
    if !h.delimiter.is_empty() {
        // A non-empty delimiter ends in NUL and lookahead is not 0 here, so
        // size stays in bounds. Where char is signed, 0xFF equals the -1 of
        // an invalid UTF-8 byte and 0x80..0xFE equal nothing; where it is
        // unsigned (linux/arm64), 0x80..0xFF equal U+0080..U+00FF.
        while l.lookahead != 0
            && l.lookahead != c('\n')
            && size < h.delimiter.len()
            && cchar::value(h.delimiter[size]) == l.lookahead
            && h.current_leading_word.len() < h.delimiter.len()
        {
            h.current_leading_word.push(l.lookahead as u8);
            l.advance(false);
            size += 1;
        }
    }
    h.current_leading_word.push(0);
    if h.delimiter.is_empty() {
        return false;
    }
    cstr_equal(&h.current_leading_word, &h.delimiter)
}

impl Scanner {
    /// scan_heredoc_content
    fn scan_heredoc_content(
        &mut self,
        l: &mut gositter::Lexer<'_>,
        middle_type: gositter::Symbol,
        end_type: gositter::Symbol,
    ) -> bool {
        let mut did_advance = false;
        let h = self.heredocs.last_mut().unwrap();

        loop {
            match l.lookahead {
                0 => {
                    if l.eof() && did_advance {
                        h.reset();
                        l.result_symbol = end_type;
                        return true;
                    }
                    return false;
                }

                BSLASH => {
                    did_advance = true;
                    l.advance(false);
                    l.advance(false);
                }

                // `break` of the C switch leaves the labeled block.
                DOLLAR => 'sw: {
                    if h.is_raw {
                        did_advance = true;
                        l.advance(false);
                        break 'sw;
                    }
                    if did_advance {
                        l.mark_end();
                        l.result_symbol = middle_type;
                        h.started = true;
                        l.advance(false);
                        if iswalpha(l.lookahead) || l.lookahead == c('{') || l.lookahead == c('(') {
                            return true;
                        }
                        break 'sw;
                    }
                    if middle_type == Tok::HeredocBodyBeginning as gositter::Symbol
                        && l.get_column() == 0
                    {
                        l.result_symbol = middle_type;
                        h.started = true;
                        return true;
                    }
                    return false;
                }

                NL => {
                    if !did_advance {
                        l.advance(true);
                    } else {
                        l.advance(false);
                    }
                    did_advance = true;
                    if h.allows_indent {
                        while iswspace(l.lookahead) {
                            l.advance(false);
                        }
                    }
                    if h.started {
                        l.result_symbol = middle_type;
                    } else {
                        l.result_symbol = end_type;
                    }
                    l.mark_end();
                    if scan_heredoc_end_identifier(h, l) {
                        if l.result_symbol == Tok::HeredocEnd as gositter::Symbol {
                            self.heredocs.pop();
                        }
                        return true;
                    }
                }

                _ => {
                    if l.get_column() == 0 {
                        // an alternative is to check the starting column of the
                        // heredoc body and track that statefully
                        while iswspace(l.lookahead) {
                            if did_advance {
                                l.advance(false);
                            } else {
                                l.advance(true);
                            }
                        }
                        if end_type != Tok::SimpleHeredocBody as gositter::Symbol {
                            l.result_symbol = middle_type;
                            if scan_heredoc_end_identifier(h, l) {
                                return true;
                            }
                        }
                        if end_type == Tok::SimpleHeredocBody as gositter::Symbol {
                            l.result_symbol = end_type;
                            l.mark_end();
                            if scan_heredoc_end_identifier(h, l) {
                                return true;
                            }
                        }
                    }
                    did_advance = true;
                    l.advance(false);
                }
            }
        }
    }
}

impl gositter::ExternalScanner for Scanner {
    /// scan, tree_sitter_bash_external_scanner_scan. The gotos of C jump forward to labels:
    /// `target` is the section a goto enters (2 regex, 3 extglob_pattern, 4 expansion_word,
    /// 5 brace_start), 0 is the normal flow, and `break 'pre` is the jump.
    fn scan(&mut self, l: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let v = |t: Tok| valid_symbols[t as usize];
        let mut target = 0u8;

        'pre: {
            if v(Tok::Concat) && !v(Tok::ErrorRecovery) {
                let at_boundary = l.lookahead == 0
                    || iswspace(l.lookahead)
                    || l.lookahead == c('>')
                    || l.lookahead == c('<')
                    || l.lookahead == c(')')
                    || l.lookahead == c('(')
                    || l.lookahead == c(';')
                    || l.lookahead == c('&')
                    || l.lookahead == c('|')
                    || (l.lookahead == c('}') && v(Tok::ClosingBrace))
                    || (l.lookahead == c(']') && v(Tok::ClosingBracket));
                if !at_boundary {
                    l.result_symbol = Tok::Concat as gositter::Symbol;
                    // So for a`b`, we want to return a concat. We check if the
                    // 2nd backtick has whitespace after it, and if it does we
                    // return concat.
                    if l.lookahead == c('`') {
                        l.mark_end();
                        l.advance(false);
                        while l.lookahead != c('`') && !l.eof() {
                            l.advance(false);
                        }
                        if l.eof() {
                            return false;
                        }
                        if l.lookahead == c('`') {
                            l.advance(false);
                        }
                        return iswspace(l.lookahead) || l.eof();
                    }
                    // strings w/ expansions that contains escaped quotes or
                    // backslashes need this to return a concat
                    if l.lookahead == c('\\') {
                        l.mark_end();
                        l.advance(false);
                        if l.lookahead == c('"') || l.lookahead == c('\'') || l.lookahead == c('\\')
                        {
                            return true;
                        }
                        if l.eof() {
                            return false;
                        }
                    } else {
                        return true;
                    }
                }
                if iswspace(l.lookahead) && v(Tok::ClosingBrace) && !v(Tok::ExpansionWord) {
                    l.result_symbol = Tok::Concat as gositter::Symbol;
                    return true;
                }
            }

            if v(Tok::ImmediateDoubleHash) && !v(Tok::ErrorRecovery) {
                // advance two # and ensure not } after
                if l.lookahead == c('#') {
                    l.mark_end();
                    l.advance(false);
                    if l.lookahead == c('#') {
                        l.advance(false);
                        if l.lookahead != c('}') {
                            l.result_symbol = Tok::ImmediateDoubleHash as gositter::Symbol;
                            l.mark_end();
                            return true;
                        }
                    }
                }
            }

            if v(Tok::ExternalExpansionSymHash)
                && !v(Tok::ErrorRecovery)
                && (l.lookahead == c('#') || l.lookahead == c('=') || l.lookahead == c('!'))
            {
                if l.lookahead == c('#') {
                    l.result_symbol = Tok::ExternalExpansionSymHash as gositter::Symbol;
                } else if l.lookahead == c('!') {
                    l.result_symbol = Tok::ExternalExpansionSymBang as gositter::Symbol;
                } else {
                    l.result_symbol = Tok::ExternalExpansionSymEqual as gositter::Symbol;
                }
                l.advance(false);
                l.mark_end();
                while l.lookahead == c('#') || l.lookahead == c('=') || l.lookahead == c('!') {
                    l.advance(false);
                }
                while iswspace(l.lookahead) {
                    l.advance(true);
                }
                if l.lookahead == c('}') {
                    return true;
                }
                return false;
            }

            if v(Tok::EmptyValue)
                && (iswspace(l.lookahead)
                    || l.eof()
                    || l.lookahead == c(';')
                    || l.lookahead == c('&'))
            {
                l.result_symbol = Tok::EmptyValue as gositter::Symbol;
                return true;
            }

            if (v(Tok::HeredocBodyBeginning) || v(Tok::SimpleHeredocBody))
                && self.heredocs.last().is_some_and(|h| !h.started)
                && !v(Tok::ErrorRecovery)
            {
                return self.scan_heredoc_content(
                    l,
                    Tok::HeredocBodyBeginning as gositter::Symbol,
                    Tok::SimpleHeredocBody as gositter::Symbol,
                );
            }

            if v(Tok::HeredocEnd) && !self.heredocs.is_empty() {
                let h = self.heredocs.last_mut().unwrap();
                if scan_heredoc_end_identifier(h, l) {
                    self.heredocs.pop();
                    l.result_symbol = Tok::HeredocEnd as gositter::Symbol;
                    return true;
                }
            }

            if v(Tok::HeredocContent)
                && self.heredocs.last().is_some_and(|h| h.started)
                && !v(Tok::ErrorRecovery)
            {
                return self.scan_heredoc_content(
                    l,
                    Tok::HeredocContent as gositter::Symbol,
                    Tok::HeredocEnd as gositter::Symbol,
                );
            }

            if v(Tok::HeredocStart) && !v(Tok::ErrorRecovery) && !self.heredocs.is_empty() {
                return scan_heredoc_start(self.heredocs.last_mut().unwrap(), l);
            }

            if v(Tok::TestOperator) && !v(Tok::ExpansionWord) {
                while iswspace(l.lookahead) && l.lookahead != c('\n') {
                    l.advance(true);
                }

                if l.lookahead == c('\\') {
                    if v(Tok::ExtglobPattern) {
                        target = 3; // goto extglob_pattern
                        break 'pre;
                    }
                    if v(Tok::RegexNoSpace) {
                        target = 2; // goto regex
                        break 'pre;
                    }
                    l.advance(true);

                    if l.eof() {
                        return false;
                    }

                    if l.lookahead == c('\r') {
                        l.advance(true);
                        if l.lookahead == c('\n') {
                            l.advance(true);
                        }
                    } else if l.lookahead == c('\n') {
                        l.advance(true);
                    } else {
                        return false;
                    }

                    while iswspace(l.lookahead) {
                        l.advance(true);
                    }
                }

                if l.lookahead == c('\n') && !v(Tok::Newline) {
                    l.advance(true);

                    while iswspace(l.lookahead) {
                        l.advance(true);
                    }
                }

                if l.lookahead == c('-') {
                    l.advance(false);

                    let mut advanced_once = false;
                    while iswalpha(l.lookahead) {
                        advanced_once = true;
                        l.advance(false);
                    }

                    if iswspace(l.lookahead) && advanced_once {
                        l.mark_end();
                        l.advance(false);
                        if l.lookahead == c('}') && v(Tok::ClosingBrace) {
                            if v(Tok::ExpansionWord) {
                                l.mark_end();
                                l.result_symbol = Tok::ExpansionWord as gositter::Symbol;
                                return true;
                            }
                            return false;
                        }
                        l.result_symbol = Tok::TestOperator as gositter::Symbol;
                        return true;
                    }
                    if iswspace(l.lookahead) && v(Tok::ExtglobPattern) {
                        l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                        return true;
                    }
                }

                if v(Tok::BareDollar) && !v(Tok::ErrorRecovery) && scan_bare_dollar(l) {
                    return true;
                }
            }

            if (v(Tok::VariableName) || v(Tok::FileDescriptor) || v(Tok::HeredocArrow))
                && !v(Tok::RegexNoSlash)
                && !v(Tok::ErrorRecovery)
            {
                loop {
                    if (l.lookahead == c(' ')
                        || l.lookahead == c('\t')
                        || l.lookahead == c('\r')
                        || (l.lookahead == c('\n') && !v(Tok::Newline)))
                        && !v(Tok::ExpansionWord)
                    {
                        l.advance(true);
                    } else if l.lookahead == c('\\') {
                        l.advance(true);

                        if l.eof() {
                            l.mark_end();
                            l.result_symbol = Tok::VariableName as gositter::Symbol;
                            return true;
                        }

                        if l.lookahead == c('\r') {
                            l.advance(true);
                        }
                        if l.lookahead == c('\n') {
                            l.advance(true);
                        } else {
                            if l.lookahead == c('\\') && v(Tok::ExpansionWord) {
                                target = 4; // goto expansion_word
                                break 'pre;
                            }
                            return false;
                        }
                    } else {
                        break;
                    }
                }

                // no '*', '@', '?', '-', '$', '0', '_'
                if !v(Tok::ExpansionWord)
                    && (l.lookahead == c('*')
                        || l.lookahead == c('@')
                        || l.lookahead == c('?')
                        || l.lookahead == c('-')
                        || l.lookahead == c('0')
                        || l.lookahead == c('_'))
                {
                    l.mark_end();
                    l.advance(false);
                    if l.lookahead == c('=')
                        || l.lookahead == c('[')
                        || l.lookahead == c(':')
                        || l.lookahead == c('-')
                        || l.lookahead == c('%')
                        || l.lookahead == c('#')
                        || l.lookahead == c('/')
                    {
                        return false;
                    }
                    if v(Tok::ExtglobPattern) && iswspace(l.lookahead) {
                        l.mark_end();
                        l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                        return true;
                    }
                }

                if v(Tok::HeredocArrow) && l.lookahead == c('<') {
                    l.advance(false);
                    if l.lookahead == c('<') {
                        l.advance(false);
                        if l.lookahead == c('-') {
                            l.advance(false);
                            self.push_heredoc(true);
                            l.result_symbol = Tok::HeredocArrowDash as gositter::Symbol;
                        } else if l.lookahead == c('<') || l.lookahead == c('=') {
                            return false;
                        } else {
                            self.push_heredoc(false);
                            l.result_symbol = Tok::HeredocArrow as gositter::Symbol;
                        }
                        return true;
                    }
                    return false;
                }

                let mut is_number = true;
                if iswdigit(l.lookahead) {
                    l.advance(false);
                } else if iswalpha(l.lookahead) || l.lookahead == c('_') {
                    is_number = false;
                    l.advance(false);
                } else {
                    if l.lookahead == c('{') {
                        target = 5; // goto brace_start
                        break 'pre;
                    }
                    if v(Tok::ExpansionWord) {
                        target = 4; // goto expansion_word
                        break 'pre;
                    }
                    if v(Tok::ExtglobPattern) {
                        target = 3; // goto extglob_pattern
                        break 'pre;
                    }
                    return false;
                }

                loop {
                    if iswdigit(l.lookahead) {
                        l.advance(false);
                    } else if iswalpha(l.lookahead) || l.lookahead == c('_') {
                        is_number = false;
                        l.advance(false);
                    } else {
                        break;
                    }
                }

                if is_number
                    && v(Tok::FileDescriptor)
                    && (l.lookahead == c('>') || l.lookahead == c('<'))
                {
                    l.result_symbol = Tok::FileDescriptor as gositter::Symbol;
                    return true;
                }

                if v(Tok::VariableName) {
                    if l.lookahead == c('+') {
                        l.mark_end();
                        l.advance(false);
                        if l.lookahead == c('=') || l.lookahead == c(':') || v(Tok::ClosingBrace) {
                            l.result_symbol = Tok::VariableName as gositter::Symbol;
                            return true;
                        }
                        return false;
                    }
                    if l.lookahead == c('/') {
                        return false;
                    }
                    if l.lookahead == c('=')
                        || l.lookahead == c('[')
                        || (l.lookahead == c(':') && !v(Tok::ClosingBrace) && !v(Tok::OpeningParen))
                        || l.lookahead == c('%')
                        || (l.lookahead == c('#') && !is_number)
                        || l.lookahead == c('@')
                        || (l.lookahead == c('-') && v(Tok::ClosingBrace))
                    {
                        l.mark_end();
                        l.result_symbol = Tok::VariableName as gositter::Symbol;
                        return true;
                    }

                    if l.lookahead == c('?') {
                        l.mark_end();
                        l.advance(false);
                        l.result_symbol = Tok::VariableName as gositter::Symbol;
                        return iswalpha(l.lookahead);
                    }
                }

                return false;
            }

            if v(Tok::BareDollar) && !v(Tok::ErrorRecovery) && scan_bare_dollar(l) {
                return true;
            }
        }

        // regex:
        if target <= 2
            && (v(Tok::Regex) || v(Tok::RegexNoSlash) || v(Tok::RegexNoSpace))
            && !v(Tok::ErrorRecovery)
        {
            if v(Tok::Regex) || v(Tok::RegexNoSpace) {
                while iswspace(l.lookahead) {
                    l.advance(true);
                }
            }

            if (l.lookahead != c('"') && l.lookahead != c('\''))
                || ((l.lookahead == c('$') || l.lookahead == c('\'')) && v(Tok::RegexNoSlash))
                || (l.lookahead == c('\'') && v(Tok::RegexNoSpace))
            {
                if l.lookahead == c('$') && v(Tok::RegexNoSlash) {
                    l.mark_end();
                    l.advance(false);
                    if l.lookahead == c('(') {
                        return false;
                    }
                }

                l.mark_end();

                // State. The depths are uint32 and wrap below zero as in C.
                let mut done = false;
                let mut advanced_once = false;
                let mut found_non_alnumdollarunderdash = false;
                let mut last_was_escape = false;
                let mut in_single_quote = false;
                let mut paren_depth = 0u32;
                let mut bracket_depth = 0u32;
                let mut brace_depth = 0u32;
                while !done {
                    if in_single_quote && l.lookahead == c('\'') {
                        in_single_quote = false;
                        l.advance(false);
                        l.mark_end();
                    }
                    match l.lookahead {
                        BSLASH => last_was_escape = true,
                        0 => return false,
                        LPAREN => {
                            paren_depth = paren_depth.wrapping_add(1);
                            last_was_escape = false;
                        }
                        LBRACKET => {
                            bracket_depth = bracket_depth.wrapping_add(1);
                            last_was_escape = false;
                        }
                        LBRACE => {
                            if !last_was_escape {
                                brace_depth = brace_depth.wrapping_add(1);
                            }
                            last_was_escape = false;
                        }
                        RPAREN => {
                            if paren_depth == 0 {
                                done = true;
                            }
                            paren_depth = paren_depth.wrapping_sub(1);
                            last_was_escape = false;
                        }
                        RBRACKET => {
                            if bracket_depth == 0 {
                                done = true;
                            }
                            bracket_depth = bracket_depth.wrapping_sub(1);
                            last_was_escape = false;
                        }
                        RBRACE => {
                            if brace_depth == 0 {
                                done = true;
                            }
                            brace_depth = brace_depth.wrapping_sub(1);
                            last_was_escape = false;
                        }
                        SQUOTE => {
                            // Enter or exit a single-quoted string.
                            in_single_quote = !in_single_quote;
                            l.advance(false);
                            advanced_once = true;
                            last_was_escape = false;
                            continue;
                        }
                        _ => last_was_escape = false,
                    }

                    if !done {
                        if v(Tok::Regex) {
                            let was_space = !in_single_quote && iswspace(l.lookahead);
                            l.advance(false);
                            advanced_once = true;
                            if !was_space || paren_depth > 0 {
                                l.mark_end();
                            }
                        } else if v(Tok::RegexNoSlash) {
                            if l.lookahead == c('/') {
                                l.mark_end();
                                l.result_symbol = Tok::RegexNoSlash as gositter::Symbol;
                                return advanced_once;
                            }
                            if l.lookahead == c('\\') {
                                l.advance(false);
                                advanced_once = true;
                                if !l.eof() && l.lookahead != c('[') && l.lookahead != c('/') {
                                    l.advance(false);
                                    l.mark_end();
                                }
                            } else {
                                let was_space = !in_single_quote && iswspace(l.lookahead);
                                l.advance(false);
                                advanced_once = true;
                                if !was_space {
                                    l.mark_end();
                                }
                            }
                        } else if v(Tok::RegexNoSpace) {
                            if l.lookahead == c('\\') {
                                found_non_alnumdollarunderdash = true;
                                l.advance(false);
                                if !l.eof() {
                                    l.advance(false);
                                }
                            } else if l.lookahead == c('$') {
                                l.mark_end();
                                l.advance(false);
                                // do not parse a command
                                // substitution
                                if l.lookahead == c('(') {
                                    return false;
                                }
                                // end $ always means regex, e.g.
                                // 99999999$
                                if iswspace(l.lookahead) {
                                    l.result_symbol = Tok::RegexNoSpace as gositter::Symbol;
                                    l.mark_end();
                                    return true;
                                }
                            } else {
                                let was_space = !in_single_quote && iswspace(l.lookahead);
                                if was_space && paren_depth == 0 {
                                    l.mark_end();
                                    l.result_symbol = Tok::RegexNoSpace as gositter::Symbol;
                                    return found_non_alnumdollarunderdash;
                                }
                                if !iswalnum(l.lookahead)
                                    && l.lookahead != c('$')
                                    && l.lookahead != c('-')
                                    && l.lookahead != c('_')
                                {
                                    found_non_alnumdollarunderdash = true;
                                }
                                l.advance(false);
                            }
                        }
                    }
                }

                if v(Tok::RegexNoSlash) {
                    l.result_symbol = Tok::RegexNoSlash as gositter::Symbol;
                } else if v(Tok::RegexNoSpace) {
                    l.result_symbol = Tok::RegexNoSpace as gositter::Symbol;
                } else {
                    l.result_symbol = Tok::Regex as gositter::Symbol;
                }
                if v(Tok::Regex) && !advanced_once {
                    return false;
                }
                return true;
            }
        }

        // extglob_pattern:
        if target <= 3 && v(Tok::ExtglobPattern) && !v(Tok::ErrorRecovery) {
            // first skip ws, then check for ? * + @ !
            while iswspace(l.lookahead) {
                l.advance(true);
            }

            if l.lookahead == c('?')
                || l.lookahead == c('*')
                || l.lookahead == c('+')
                || l.lookahead == c('@')
                || l.lookahead == c('!')
                || l.lookahead == c('-')
                || l.lookahead == c(')')
                || l.lookahead == c('\\')
                || l.lookahead == c('.')
                || l.lookahead == c('[')
                || iswalpha(l.lookahead)
            {
                if l.lookahead == c('\\') {
                    l.advance(false);
                    if (iswspace(l.lookahead) || l.lookahead == c('"'))
                        && l.lookahead != c('\r')
                        && l.lookahead != c('\n')
                    {
                        l.advance(false);
                    } else {
                        return false;
                    }
                }

                if l.lookahead == c(')') && self.last_glob_paren_depth == 0 {
                    l.mark_end();
                    l.advance(false);

                    if iswspace(l.lookahead) {
                        return false;
                    }
                }

                l.mark_end();
                let was_non_alpha = !iswalpha(l.lookahead);
                if l.lookahead != c('[') {
                    // no esac
                    if l.lookahead == c('e') {
                        l.mark_end();
                        l.advance(false);
                        if l.lookahead == c('s') {
                            l.advance(false);
                            if l.lookahead == c('a') {
                                l.advance(false);
                                if l.lookahead == c('c') {
                                    l.advance(false);
                                    if iswspace(l.lookahead) {
                                        return false;
                                    }
                                }
                            }
                        }
                    } else {
                        l.advance(false);
                    }
                }

                // -\w is just a word, find something else special
                if l.lookahead == c('-') {
                    l.mark_end();
                    l.advance(false);
                    while iswalnum(l.lookahead) {
                        l.advance(false);
                    }

                    if l.lookahead == c(')') || l.lookahead == c('\\') || l.lookahead == c('.') {
                        return false;
                    }
                    l.mark_end();
                }

                // case item -) or *)
                if l.lookahead == c(')') && self.last_glob_paren_depth == 0 {
                    l.mark_end();
                    l.advance(false);
                    if iswspace(l.lookahead) {
                        l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                        return was_non_alpha;
                    }
                }

                if iswspace(l.lookahead) {
                    l.mark_end();
                    l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                    self.last_glob_paren_depth = 0;
                    return true;
                }

                if l.lookahead == c('$') {
                    l.mark_end();
                    l.advance(false);
                    if l.lookahead == c('{') || l.lookahead == c('(') {
                        l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                        return true;
                    }
                }

                if l.lookahead == c('|') {
                    l.mark_end();
                    l.advance(false);
                    l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                    return true;
                }

                if !iswalnum(l.lookahead)
                    && l.lookahead != c('(')
                    && l.lookahead != c('"')
                    && l.lookahead != c('[')
                    && l.lookahead != c('?')
                    && l.lookahead != c('/')
                    && l.lookahead != c('\\')
                    && l.lookahead != c('_')
                    && l.lookahead != c('*')
                {
                    return false;
                }

                // State. The depths are uint32 and wrap below zero as in C.
                let mut done = false;
                let mut saw_non_alphadot = was_non_alpha;
                let mut paren_depth = u32::from(self.last_glob_paren_depth);
                let mut bracket_depth = 0u32;
                let mut brace_depth = 0u32;
                while !done {
                    match l.lookahead {
                        0 => return false,
                        LPAREN => paren_depth = paren_depth.wrapping_add(1),
                        LBRACKET => bracket_depth = bracket_depth.wrapping_add(1),
                        LBRACE => brace_depth = brace_depth.wrapping_add(1),
                        RPAREN => {
                            if paren_depth == 0 {
                                done = true;
                            }
                            paren_depth = paren_depth.wrapping_sub(1);
                        }
                        RBRACKET => {
                            if bracket_depth == 0 {
                                done = true;
                            }
                            bracket_depth = bracket_depth.wrapping_sub(1);
                        }
                        RBRACE => {
                            if brace_depth == 0 {
                                done = true;
                            }
                            brace_depth = brace_depth.wrapping_sub(1);
                        }
                        _ => {}
                    }

                    if l.lookahead == c('|') {
                        l.mark_end();
                        l.advance(false);
                        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                            l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                            return true;
                        }
                    }

                    if !done {
                        let was_space = iswspace(l.lookahead);
                        if l.lookahead == c('$') {
                            l.mark_end();
                            if !iswalpha(l.lookahead)
                                && l.lookahead != c('.')
                                && l.lookahead != c('\\')
                            {
                                saw_non_alphadot = true;
                            }
                            l.advance(false);
                            if l.lookahead == c('(') || l.lookahead == c('{') {
                                l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                                self.last_glob_paren_depth = paren_depth as u8; // uint32 cut to uint8
                                return saw_non_alphadot;
                            }
                        }
                        if was_space {
                            l.mark_end();
                            l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                            self.last_glob_paren_depth = 0;
                            return saw_non_alphadot;
                        }
                        if l.lookahead == c('"') {
                            l.mark_end();
                            l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                            self.last_glob_paren_depth = 0;
                            return saw_non_alphadot;
                        }
                        if l.lookahead == c('\\') {
                            if !iswalpha(l.lookahead)
                                && l.lookahead != c('.')
                                && l.lookahead != c('\\')
                            {
                                saw_non_alphadot = true;
                            }
                            l.advance(false);
                            if iswspace(l.lookahead) || l.lookahead == c('"') {
                                l.advance(false);
                            }
                        } else {
                            if !iswalpha(l.lookahead)
                                && l.lookahead != c('.')
                                && l.lookahead != c('\\')
                            {
                                saw_non_alphadot = true;
                            }
                            l.advance(false);
                        }
                        if !was_space {
                            l.mark_end();
                        }
                    }
                }

                l.result_symbol = Tok::ExtglobPattern as gositter::Symbol;
                self.last_glob_paren_depth = 0;
                return saw_non_alphadot;
            }
            self.last_glob_paren_depth = 0;

            return false;
        }

        // expansion_word:
        if target <= 4 && v(Tok::ExpansionWord) {
            let mut advanced_once = false;
            let mut advance_once_space = false;
            loop {
                if l.lookahead == c('"') {
                    return false;
                }
                if l.lookahead == c('$') {
                    l.mark_end();
                    l.advance(false);
                    if l.lookahead == c('{')
                        || l.lookahead == c('(')
                        || l.lookahead == c('\'')
                        || iswalnum(l.lookahead)
                    {
                        l.result_symbol = Tok::ExpansionWord as gositter::Symbol;
                        return advanced_once;
                    }
                    advanced_once = true;
                }

                if l.lookahead == c('}') {
                    l.mark_end();
                    l.result_symbol = Tok::ExpansionWord as gositter::Symbol;
                    return advanced_once || advance_once_space;
                }

                if l.lookahead == c('(') && !(advanced_once || advance_once_space) {
                    l.mark_end();
                    l.advance(false);
                    while l.lookahead != c(')') && !l.eof() {
                        // if we find a $( or ${ assume this is valid and is
                        // a garbage concatenation of some weird word + an
                        // expansion
                        if l.lookahead == c('$') {
                            l.mark_end();
                            l.advance(false);
                            if l.lookahead == c('{')
                                || l.lookahead == c('(')
                                || l.lookahead == c('\'')
                                || iswalnum(l.lookahead)
                            {
                                l.result_symbol = Tok::ExpansionWord as gositter::Symbol;
                                return advanced_once;
                            }
                            advanced_once = true;
                        } else {
                            advanced_once = advanced_once || !iswspace(l.lookahead);
                            advance_once_space = advance_once_space || iswspace(l.lookahead);
                            l.advance(false);
                        }
                    }
                    l.mark_end();
                    if l.lookahead == c(')') {
                        advanced_once = true;
                        l.advance(false);
                        l.mark_end();
                        if l.lookahead == c('}') {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                if l.lookahead == c('\'') {
                    return false;
                }

                if l.eof() {
                    return false;
                }
                advanced_once = advanced_once || !iswspace(l.lookahead);
                advance_once_space = advance_once_space || iswspace(l.lookahead);
                l.advance(false);
            }
        }

        // brace_start:
        if v(Tok::BraceStart) && !v(Tok::ErrorRecovery) {
            while iswspace(l.lookahead) {
                l.advance(true);
            }

            if l.lookahead != c('{') {
                return false;
            }

            l.advance(false);
            l.mark_end();

            while iswdigit(l.lookahead) {
                l.advance(false);
            }

            if l.lookahead != c('.') {
                return false;
            }
            l.advance(false);

            if l.lookahead != c('.') {
                return false;
            }
            l.advance(false);

            while iswdigit(l.lookahead) {
                l.advance(false);
            }

            if l.lookahead != c('}') {
                return false;
            }

            l.result_symbol = Tok::BraceStart as gositter::Symbol;
            return true;
        }

        false
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

    // scanner_test.go TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(serialize(&s), [0, 0, 0, 0], "fresh scanner");

        s.last_glob_paren_depth = 0x81;
        s.ext_saw_outside_quote = true;
        s.push_heredoc(true);
        s.heredocs[0].is_raw = true;
        s.heredocs[0].delimiter.extend_from_slice(&[b'E', 0xFF, 0]);
        s.push_heredoc(false);
        s.heredocs[1].started = true;
        let b = serialize(&s);
        // header, then per heredoc: is_raw, started, allows_indent, uint32 LE size, bytes.
        let want = [
            0x81, 0, 1, 2, 1, 0, 1, 3, 0, 0, 0, b'E', 0xFF, 0, 0, 1, 0, 0, 0, 0, 0,
        ];
        assert_eq!(b, want, "bytes");

        // Dirty target with one stale heredoc: every serialized field is replaced.
        let mut got = Scanner {
            last_glob_paren_depth: 7,
            ext_was_in_double_quote: true,
            ..Scanner::default()
        };
        got.push_heredoc(false);
        got.heredocs[0].started = true;
        got.heredocs[0]
            .delimiter
            .extend_from_slice(b"STALESTALE\x00");
        got.deserialize(&b);
        assert_eq!(serialize(&got), want, "after round trip");
        let h = &got.heredocs[0];
        assert!(
            got.last_glob_paren_depth == 0x81
                && !got.ext_was_in_double_quote
                && got.ext_saw_outside_quote
                && h.is_raw
                && !h.started
                && h.allows_indent
                && h.delimiter == [b'E', 0xFF, 0],
            "state = {got:?}"
        );
    }

    // C resets each heredoc on an empty state but keeps heredocs.size, and a state with fewer
    // heredocs does not shrink the array either.
    // scanner_test.go TestDeserializeKeepsCount
    #[test]
    fn deserialize_keeps_count() {
        let mut s = Scanner {
            last_glob_paren_depth: 3,
            ext_was_in_double_quote: true,
            ..Scanner::default()
        };
        for _ in 0..3 {
            s.push_heredoc(true);
            let h = s.heredocs.last_mut().unwrap();
            h.is_raw = true;
            h.started = true;
            h.delimiter.extend_from_slice(&[b'X', 0]);
        }
        let one = [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, b'Y', 0];
        s.deserialize(&one);
        assert!(
            s.heredocs.len() == 3
                && s.heredocs[0].delimiter == b"Y\x00"
                && s.heredocs[2].delimiter == b"X\x00",
            "state = {s:?}"
        );

        s.deserialize(&[]);
        let want = [
            0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(serialize(&s), want, "after reset");
    }

    // scanner_test.go TestSerializeLimit
    #[test]
    fn serialize_limit() {
        // 4 + 3 + 4 + len must stay below 1024, else C returns 0.
        for (n, want) in [(1012usize, 1023usize), (1013, 0), (5000, 0)] {
            let mut s = Scanner::new();
            s.push_heredoc(false);
            s.heredocs[0].delimiter = vec![b'D'; n];
            let b = serialize(&s);
            assert_eq!(b.len(), want, "delimiter {n}: size");
            if want > 0 {
                let mut got = Scanner::new();
                got.deserialize(&b);
                assert_eq!(serialize(&got), b, "delimiter {n}: round trip differs");
            }
        }
        // 147 empty heredocs: 4 + 146*7 = 1026 would not fit.
        let mut s = Scanner::new();
        for _ in 0..145 {
            s.push_heredoc(false);
        }
        assert_eq!(serialize(&s).len(), 4 + 145 * 7, "145 heredocs");
        s.push_heredoc(false);
        assert_eq!(serialize(&s).len(), 0, "146 heredocs");
    }

    // reset_test.go checkReset. Capacity is the only allowed difference, and `==` ignores it.
    pub(super) fn check_reset(s: &mut Scanner, what: &str) {
        s.reset();
        assert_eq!(*s, Scanner::new(), "{what}: after reset, new gives");
        assert_eq!(
            serialize(s),
            serialize(&Scanner::new()),
            "{what}: reset and new serialize differently"
        );
    }

    // reset_test.go TestResetIsNew, the hand-made half (no parser needed)
    #[test]
    fn reset_hand_made() {
        // Fields deserialize(&[]) leaves alone.
        let mut s = Scanner {
            last_glob_paren_depth: 3,
            ext_was_in_double_quote: true,
            ext_saw_outside_quote: true,
            ..Scanner::default()
        };
        s.push_heredoc(true);
        s.heredocs[0].is_raw = true;
        s.heredocs[0].started = true;
        s.heredocs[0].delimiter = b"EOF".to_vec();
        s.heredocs[0].current_leading_word = b"lead".to_vec();
        s.push_heredoc(false);
        s.heredocs[1].delimiter = b"X".to_vec();
        check_reset(&mut s, "hand-made");
    }
}

#[cfg(test)]
mod parser_tests {
    use super::Scanner;
    use super::tests::check_reset;
    use crate::cchar;

    pub(super) const RESET_INPUTS: [&str; 12] = [
        "",
        "echo hi\n",
        "cat <<EOF\nbody\nEOF\n",
        "cat <<-A <<'B'\n\tone\n\tA\ntwo\nB\n",
        "cat <<EOF\nbody $(echo <<X\nin\nX\n) more\nEOF\n",
        "cat <<EOF\nbody without an end",
        "cat <<A <<B <<C\nx\nA\n",
        "echo ${x:-$(cat <<E\n",
        "echo \"$(( 1 + \n",
        "if [[ $x =~ ^a ]]; then echo 'unclosed\n",
        "echo ${a[@]#*} <<<\"s\"\nfoo=bar\n",
        "cat <<EOF >out; echo $(\n",
    ];

    // The delimiter is compared as C chars against lookahead, so the two non-ASCII cases swap
    // where char is unsigned.
    // scanner_test.go TestHeredocEndChar
    #[test]
    fn heredoc_end_char() {
        let mut p = gositter::Parser::new();
        p.set_language(super::super::language()).unwrap();
        for (src, want_err) in [
            (&b"cat <<E\nx\nE\n"[..], false),
            // signed: 0xFF is -1, as is an invalid byte
            (&b"cat <<\xff\nx\n\xff\n"[..], cchar::UNSIGNED),
            // signed: U+00E9 is cut to 0xE9 = -23, never matches
            (&b"cat <<\xc3\xa9\nx\n\xc3\xa9\n"[..], !cchar::UNSIGNED),
        ] {
            let tree = p.parse(src).unwrap();
            assert_eq!(
                tree.root_node().has_error(),
                want_err,
                "{src:?}: has_error\n{}",
                tree.root_node().to_sexp()
            );
        }
    }

    // B parsed after A on one parser has the tree of B on a fresh parser.
    // reset_test.go TestReusedScannerSameTree
    #[test]
    fn reused_scanner_same_tree() {
        let mut reused = gositter::Parser::new();
        reused.set_language(super::super::language()).unwrap();
        for a in RESET_INPUTS {
            for b in RESET_INPUTS {
                reused.parse(a.as_bytes()).unwrap();
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

    // TestResetDropsBigBuffers (reset_test.go). Go counts the capacity of the kept slots and of
    // their buffers; a Vec keeps no readable spare slots, so the Vec and the buffers of its
    // elements are counted.
    #[test]
    fn reset_drops_big_buffers() {
        for input in [
            "cat <<A; ".repeat(20000),
            format!("cat <<{}\nbody\n", "A".repeat(2000000)),
        ] {
            let mut p = gositter::Parser::new();
            p.set_language(super::super::language()).unwrap();
            p.parse(input.as_bytes());
            p.parse(b"echo hi\n");
            let k = kept(&p);
            let held = k.heredocs.capacity()
                + k.heredocs
                    .iter()
                    .map(|h| h.delimiter.capacity() + h.current_leading_word.capacity())
                    .sum::<usize>();
            assert!(held <= 4096, "small parse kept {held} slots and bytes");
        }
    }
}
