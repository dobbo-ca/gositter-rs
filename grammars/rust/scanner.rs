//! Port of gositter `grammars/rust/scanner.go` (tree-sitter-rust v0.24.2 `src/scanner.c`): the
//! scanner state, `serialize`, `deserialize` and `scan`.

use gositter::{Lexer, Symbol};

/// enum TokenType
#[derive(Clone, Copy)]
enum Tok {
    StringContent,
    StringClose,
    RawStringLiteralStart,
    RawStringLiteralContent,
    RawStringLiteralEnd,
    FloatLiteral,
    BlockOuterDocMarker,
    BlockInnerDocMarker,
    BlockCommentContent,
    LineDocContent,
    ErrorSentinel,
}

// The isw* functions run in the C locale (nothing calls setlocale), where they are true for
// ASCII only; -1 (invalid UTF-8) is WEOF and false.

fn iswdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}

fn iswalpha(c: i32) -> bool {
    (b'a' as i32..=b'z' as i32).contains(&c) || (b'A' as i32..=b'Z' as i32).contains(&c)
}

fn iswspace(c: i32) -> bool {
    c == b' ' as i32 || (b'\t' as i32..=b'\r' as i32).contains(&c)
}

/// is_num_char
fn is_num_char(c: i32) -> bool {
    c == b'_' as i32 || iswdigit(c)
}

/// process_string
fn process_string(l: &mut Lexer<'_>) -> bool {
    let mut has_content = false;
    loop {
        if l.lookahead == b'"' as i32 || l.lookahead == b'\\' as i32 {
            break;
        }
        if l.eof() {
            return false;
        }
        has_content = true;
        l.advance(false);
    }
    l.result_symbol = Tok::StringContent as Symbol;
    l.mark_end();
    has_content
}

/// process_float_literal
fn process_float_literal(l: &mut Lexer<'_>) -> bool {
    l.result_symbol = Tok::FloatLiteral as Symbol;

    l.advance(false);
    while is_num_char(l.lookahead) {
        l.advance(false);
    }

    let (mut has_fraction, mut has_exponent) = (false, false);

    if l.lookahead == b'.' as i32 {
        has_fraction = true;
        l.advance(false);
        if iswalpha(l.lookahead) {
            // 1.max(2) is an integer and a method call.
            return false;
        }

        if l.lookahead == b'.' as i32 {
            return false;
        }
        while is_num_char(l.lookahead) {
            l.advance(false);
        }
    }

    l.mark_end();

    if l.lookahead == b'e' as i32 || l.lookahead == b'E' as i32 {
        has_exponent = true;
        l.advance(false);
        if l.lookahead == b'+' as i32 || l.lookahead == b'-' as i32 {
            l.advance(false);
        }
        if !is_num_char(l.lookahead) {
            return true;
        }
        l.advance(false);
        while is_num_char(l.lookahead) {
            l.advance(false);
        }

        l.mark_end();
    }

    if !has_exponent && !has_fraction {
        return false;
    }

    if l.lookahead != b'u' as i32 && l.lookahead != b'i' as i32 && l.lookahead != b'f' as i32 {
        return true;
    }
    l.advance(false);
    if !iswdigit(l.lookahead) {
        return true;
    }

    while iswdigit(l.lookahead) {
        l.advance(false);
    }

    l.mark_end();
    true
}

/// process_line_doc_content
fn process_line_doc_content(l: &mut Lexer<'_>) -> bool {
    l.result_symbol = Tok::LineDocContent as Symbol;
    loop {
        if l.eof() {
            return true;
        }
        if l.lookahead == b'\n' as i32 {
            l.advance(false);
            return true;
        }
        l.advance(false);
    }
}

/// BlockCommentState
#[derive(Clone, Copy)]
enum BlockState {
    LeftForwardSlash,
    LeftAsterisk,
    Continuing,
}

/// BlockCommentProcessing
struct BlockCommentProcessing {
    state: BlockState,
    nesting_depth: u32,
}

impl BlockCommentProcessing {
    /// process_left_forward_slash
    fn process_left_forward_slash(&mut self, current: u8) {
        if current == b'*' {
            self.nesting_depth = self.nesting_depth.wrapping_add(1);
        }
        self.state = BlockState::Continuing;
    }

    /// process_left_asterisk
    fn process_left_asterisk(&mut self, current: u8, l: &mut Lexer<'_>) {
        if current == b'*' {
            l.mark_end();
            self.state = BlockState::LeftAsterisk;
            return;
        }

        if current == b'/' {
            self.nesting_depth = self.nesting_depth.wrapping_sub(1);
        }

        self.state = BlockState::Continuing;
    }

    /// process_continuing
    fn process_continuing(&mut self, current: u8) {
        match current {
            b'/' => self.state = BlockState::LeftForwardSlash,
            b'*' => self.state = BlockState::LeftAsterisk,
            _ => {}
        }
    }
}

/// process_block_comment
fn process_block_comment(l: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool {
    // (char)lookahead: C keeps the low byte only, so U+012A acts as '*'.
    // Only compared for equality with ASCII, so signedness does not matter.
    let mut first = l.lookahead as u8;
    if valid_symbols[Tok::BlockInnerDocMarker as usize] && first == b'!' {
        l.result_symbol = Tok::BlockInnerDocMarker as Symbol;
        l.advance(false);
        return true;
    }
    if valid_symbols[Tok::BlockOuterDocMarker as usize] && first == b'*' {
        l.advance(false);
        l.mark_end();
        // "/**/" is an empty block comment.
        if l.lookahead == b'/' as i32 {
            return false;
        }
        // Three or more stars is not a doc marker.
        if l.lookahead != b'*' as i32 {
            l.result_symbol = Tok::BlockOuterDocMarker as Symbol;
            return true;
        }
    } else {
        l.advance(false);
    }

    if valid_symbols[Tok::BlockCommentContent as usize] {
        let mut processing = BlockCommentProcessing {
            state: BlockState::Continuing,
            nesting_depth: 1,
        };
        match first {
            b'*' => {
                processing.state = BlockState::LeftAsterisk;
                if l.lookahead == b'/' as i32 {
                    // Empty doc block comment such as /*!*/.
                    return false;
                }
            }
            b'/' => processing.state = BlockState::LeftForwardSlash,
            _ => processing.state = BlockState::Continuing,
        }

        // An unterminated comment still succeeds at EOF.
        while !l.eof() && processing.nesting_depth != 0 {
            first = l.lookahead as u8;
            match processing.state {
                BlockState::LeftForwardSlash => processing.process_left_forward_slash(first),
                BlockState::LeftAsterisk => processing.process_left_asterisk(first, l),
                BlockState::Continuing => {
                    l.mark_end();
                    processing.process_continuing(first);
                }
            }
            l.advance(false);
            if first == b'/' && processing.nesting_depth != 0 {
                l.mark_end();
            }
        }
        l.result_symbol = Tok::BlockCommentContent as Symbol;
        return true;
    }

    false
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    opening_hash_count: u8,
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.opening_hash_count;
        1
    }

    /// C resets, then reads only when the length is exactly 1.
    pub fn deserialize(&mut self, buf: &[u8]) {
        self.opening_hash_count = 0;
        if let [b] = buf {
            self.opening_hash_count = *b;
        }
    }

    /// scan_raw_string_start
    fn scan_raw_string_start(&mut self, l: &mut Lexer<'_>) -> bool {
        if l.lookahead == b'b' as i32 || l.lookahead == b'c' as i32 {
            l.advance(false);
        }
        if l.lookahead != b'r' as i32 {
            return false;
        }
        l.advance(false);

        // uint8_t: 256 hashes wrap to 0, as in C.
        let mut opening_hash_count: u8 = 0;
        while l.lookahead == b'#' as i32 {
            l.advance(false);
            opening_hash_count = opening_hash_count.wrapping_add(1);
        }

        if l.lookahead != b'"' as i32 {
            return false;
        }
        l.advance(false);
        self.opening_hash_count = opening_hash_count;

        l.result_symbol = Tok::RawStringLiteralStart as Symbol;
        true
    }

    /// scan_raw_string_content
    fn scan_raw_string_content(&mut self, l: &mut Lexer<'_>) -> bool {
        loop {
            if l.eof() {
                return false;
            }
            if l.lookahead == b'"' as i32 {
                l.mark_end();
                l.advance(false);
                let mut hash_count = 0u32;
                while l.lookahead == b'#' as i32 && hash_count < self.opening_hash_count as u32 {
                    l.advance(false);
                    hash_count += 1;
                }
                if hash_count == self.opening_hash_count as u32 {
                    l.result_symbol = Tok::RawStringLiteralContent as Symbol;
                    return true;
                }
            } else {
                l.advance(false);
            }
        }
    }

    /// scan_raw_string_end
    fn scan_raw_string_end(&mut self, l: &mut Lexer<'_>) -> bool {
        l.advance(false);
        for _ in 0..self.opening_hash_count as u32 {
            l.advance(false);
        }
        l.result_symbol = Tok::RawStringLiteralEnd as Symbol;
        true
    }

    /// tree_sitter_rust_external_scanner_scan
    fn scan(&mut self, l: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool {
        // Error recovery marks every token valid; the scanner gives up then.
        if valid_symbols[Tok::ErrorSentinel as usize] {
            return false;
        }

        if valid_symbols[Tok::BlockCommentContent as usize]
            || valid_symbols[Tok::BlockInnerDocMarker as usize]
            || valid_symbols[Tok::BlockOuterDocMarker as usize]
        {
            return process_block_comment(l, valid_symbols);
        }

        if valid_symbols[Tok::StringContent as usize]
            && !valid_symbols[Tok::FloatLiteral as usize]
            && process_string(l)
        {
            return true;
        }
        // No content before '"' or '\': fall through to STRING_CLOSE.

        if valid_symbols[Tok::StringClose as usize] && l.lookahead == b'"' as i32 {
            l.advance(false);
            l.result_symbol = Tok::StringClose as Symbol;
            l.mark_end();
            return true;
        }

        if valid_symbols[Tok::LineDocContent as usize] {
            return process_line_doc_content(l);
        }

        while iswspace(l.lookahead) {
            l.advance(true);
        }

        if valid_symbols[Tok::RawStringLiteralStart as usize]
            && (l.lookahead == b'r' as i32
                || l.lookahead == b'b' as i32
                || l.lookahead == b'c' as i32)
        {
            return self.scan_raw_string_start(l);
        }

        if valid_symbols[Tok::RawStringLiteralContent as usize] {
            return self.scan_raw_string_content(l);
        }

        if valid_symbols[Tok::RawStringLiteralEnd as usize] && l.lookahead == b'"' as i32 {
            return self.scan_raw_string_end(l);
        }

        if valid_symbols[Tok::FloatLiteral as usize] && iswdigit(l.lookahead) {
            return process_float_literal(l);
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
        let n = Scanner::new().serialize(&mut buf);
        assert_eq!(&buf[..n], [0], "fresh scanner");
        for i in 0..=255u8 {
            let s = Scanner {
                opening_hash_count: i,
            };
            let n = s.serialize(&mut buf);
            assert_eq!(&buf[..n], [i], "serialize({i})");
            let mut got = Scanner {
                opening_hash_count: i.wrapping_add(1),
            };
            got.deserialize(&buf[..n]);
            assert_eq!(got, s, "round trip {i}");
        }
    }

    // TestDeserializeOtherLengths
    #[test]
    fn deserialize_other_lengths() {
        // C resets, then reads only when length is exactly 1.
        let cases: [&[u8]; 4] = [&[], &[], &[7, 7], &[7, 7, 7]];
        for b in cases {
            let mut s = Scanner {
                opening_hash_count: 5,
            };
            s.deserialize(b);
            assert_eq!(s.opening_hash_count, 0, "deserialize({b:?})");
        }
    }
}
