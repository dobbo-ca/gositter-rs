//! Port of tree-sitter-kotlin v1.1.0 `src/scanner.c` (gositter `grammars/kotlin/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
//! deserialize ignores its input. `scan` is ported below.

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner;

// enum TokenType
const TOK_SEMI: usize = 0;
const TOK_CLASS_MEMBER_SEMI: usize = 1;
const TOK_BLOCK_COMMENT: usize = 2;
const TOK_NOT_IS: usize = 3;
const TOK_IN: usize = 4;
const TOK_Q_DOT: usize = 5;
const TOK_MULTILINE_STRING_CONTENT: usize = 6;
const TOK_CONSTRUCTOR: usize = 7;
const TOK_GET: usize = 8;
const TOK_SET: usize = 9;
const TOK_DOLLAR: usize = 10;

// The wctype calls are libc and locale dependent. These are darwin in the "C" locale, which is
// what the cgo build runs in: ASCII only, checked for every code point from -2 to 0x110000. musl
// and some glibc locales also accept non-ASCII letters and spaces.

fn iswspace(c: i32) -> bool {
    c == 32 || (9..=13).contains(&c) // " ", \t..\r
}

fn iswalpha(c: i32) -> bool {
    (65..=90).contains(&c) || (97..=122).contains(&c) // A-Z, a-z
}

fn iswdigit(c: i32) -> bool {
    (48..=57).contains(&c) // 0-9
}

const MAX_WORD_SIZE: usize = 16;

// The two word lists of the C compound literals. C pads them to 16 empty entries; a scanned word
// is never empty when compared, so those never match.
const MODIFIER_WORDS: [&str; 15] = [
    "public",
    "private",
    "protected",
    "internal",
    "abstract",
    "final",
    "open",
    "override",
    "lateinit",
    "vararg",
    "noinline",
    "crossinline",
    "external",
    "suspend",
    "inline",
];
const KEYWORD_WORDS: [&str; 9] = [
    "else",
    "in",
    "instanceof",
    "get",
    "set",
    "constructor",
    "by",
    "as",
    "where",
];

// scan_word
fn scan_word(lexer: &mut gositter::Lexer<'_>, word: &str) -> bool {
    for &b in word.as_bytes() {
        if lexer.lookahead != i32::from(b) {
            return false;
        }
        lexer.advance(true);
    }
    true
}

// scan_words
fn scan_words(
    lexer: &mut gositter::Lexer<'_>,
    words: &[&str],
    scanned_word: &mut [u8; MAX_WORD_SIZE],
    index: Option<&mut u8>,
) -> bool {
    if scanned_word[0] == 0 {
        // At most 15 letters; the rest of a longer word stays unread.
        for (i, slot) in scanned_word.iter_mut().take(MAX_WORD_SIZE - 1).enumerate() {
            if !iswalpha(lexer.lookahead) {
                if i == 0 {
                    return false;
                }
                break;
            }
            *slot = lexer.lookahead as u8;
            lexer.advance(true);
        }
    }

    let mut n = 0;
    while scanned_word[n] != 0 {
        n += 1;
    }
    for (i, w) in words.iter().enumerate() {
        if scanned_word[..n] == *w.as_bytes() {
            if let Some(index) = index {
                *index = i as u8;
            }
            return true;
        }
    }

    false
}

// Where the semicolon block leaves to. The C labels, in source order.
const ENTRY_START: usize = 0; // no semicolon token valid: line 358
const ENTRY_NOT_IS: usize = 1; // continue_not_is_from_semi
const ENTRY_Q_DOT: usize = 2; // q_dot_from_semi
const ENTRY_COMMENT: usize = 3; // comment
const ENTRY_RETURN: usize = 4; // the block returned ret

// The `if (valid_symbols[SEMI] || valid_symbols[CLASS_MEMBER_SEMI])` block of
// tree_sitter_kotlin_external_scanner_scan.
fn scan_semi(lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> (usize, bool) {
    if valid_symbols[TOK_SEMI] {
        lexer.result_symbol = TOK_SEMI as gositter::Symbol;
    } else {
        lexer.result_symbol = TOK_CLASS_MEMBER_SEMI as gositter::Symbol;
    }
    lexer.mark_end();
    let mut saw_newline = false;
    loop {
        if lexer.eof() {
            return (ENTRY_RETURN, true);
        }

        if lexer.lookahead == ';' as i32 {
            lexer.advance(false);
            lexer.mark_end();
            return (ENTRY_RETURN, true);
        }

        if !iswspace(lexer.lookahead) {
            break;
        }

        if lexer.lookahead == '\n' as i32 {
            lexer.advance(true);
            saw_newline = true;
            break;
        }

        if lexer.lookahead == '\r' as i32 {
            lexer.advance(true);

            if lexer.lookahead == '\n' as i32 {
                lexer.advance(true);
            }

            saw_newline = true;
            break;
        }

        lexer.advance(true);
    }

    // Skip whitespace and comments
    while iswspace(lexer.lookahead) {
        lexer.advance(true);
    }
    if lexer.lookahead == '/' as i32 {
        return (ENTRY_COMMENT, false);
    }

    if !saw_newline {
        match char::from_u32(lexer.lookahead as u32) {
            Some('!') => {
                lexer.advance(true);
                return (ENTRY_NOT_IS, false);
            }
            Some('?') => {
                if valid_symbols[TOK_Q_DOT] {
                    return (ENTRY_Q_DOT, false);
                }
                return (ENTRY_RETURN, false);
            }
            Some('i') => return (ENTRY_RETURN, scan_word(lexer, "import")),
            Some(';') => {
                lexer.advance(false);
                lexer.mark_end();
                return (ENTRY_RETURN, true);
            }
            _ => return (ENTRY_RETURN, false),
        }
    }

    let mut scanned_word = [0u8; MAX_WORD_SIZE];
    loop {
        // _switch:
        match char::from_u32(lexer.lookahead as u32) {
            Some(',' | '.' | ':' | '*' | '%' | '>' | '<' | '=' | '{' | '[' | '|' | '&' | '/') => {
                return (ENTRY_RETURN, false);
            }
            // Insert a semicolon before `--` and `++`, but not before binary `+` or `-`.
            // Insert before +/-{float}
            Some('+') => {
                lexer.advance(true);
                if lexer.lookahead == '+' as i32 {
                    return (ENTRY_RETURN, true);
                }
                return (ENTRY_RETURN, iswdigit(lexer.lookahead));
            }
            Some('-') => {
                lexer.advance(true);
                if lexer.lookahead == '-' as i32 {
                    return (ENTRY_RETURN, true);
                }
                return (ENTRY_RETURN, iswdigit(lexer.lookahead));
            }
            // Don't insert a semicolon before `!=`, but do insert one before a unary `!`.
            Some('!') => {
                lexer.advance(true);
                if lexer.lookahead == 'i' as i32 && valid_symbols[TOK_NOT_IS] {
                    lexer.advance(true);
                    if lexer.lookahead == 's' as i32 {
                        lexer.advance(true);
                        if !iswalnum(lexer.lookahead) {
                            return (ENTRY_RETURN, true);
                        }
                    }
                }
                return (ENTRY_RETURN, lexer.lookahead != '=' as i32);
            }
            Some('?') => {
                if valid_symbols[TOK_Q_DOT] {
                    return (ENTRY_Q_DOT, false);
                }
                return (ENTRY_RETURN, true);
            }
            Some(
                'e' | 'i' | 'g' | 's' | 'p' | 'a' | 'f' | 'o' | 'l' | 'v' | 'n' | 'c' | 'b' | 'w',
            ) => {
                while scan_words(lexer, &MODIFIER_WORDS, &mut scanned_word, None) {
                    scanned_word = [0u8; MAX_WORD_SIZE];
                    while iswspace(lexer.lookahead) {
                        lexer.advance(true);
                    }
                }

                // A word that is no modifier stays in scannedWord and is
                // looked up again here without reading input.
                let mut index = 255u8; // C: uint8_t index = -1
                let res = scan_words(lexer, &KEYWORD_WORDS, &mut scanned_word, Some(&mut index));

                // If `CLASS_MEMBER_SEMI` is valid, we found a secondary constructor and so we want to insert a semi, OR
                // we found a variable named constructor whose field is being accessed
                if index == 5 {
                    while iswspace(lexer.lookahead) {
                        lexer.advance(true);
                    }
                    if valid_symbols[TOK_CLASS_MEMBER_SEMI]
                        || lexer.lookahead == '.' as i32
                        || lexer.lookahead == '=' as i32
                    {
                        return (ENTRY_RETURN, true);
                    }
                } else if index == 0 {
                    // Ordinarily, we should not insert a semicolon if there is an `else` on the next line,
                    // except for when it's a 'when entry', which has a `->` after the `else`.
                    while iswspace(lexer.lookahead) {
                        lexer.advance(true);
                    }
                    if lexer.lookahead == '-' as i32 {
                        lexer.advance(true);
                        if lexer.lookahead == '>' as i32 {
                            return (ENTRY_RETURN, true);
                        }
                    }
                } else if index == 3 && (!valid_symbols[TOK_GET] || lexer.lookahead == '[' as i32) {
                    // If `get` was found and the keyword is not valid, return a semi since it's being used as an identifier
                    return (ENTRY_RETURN, true);
                } else if index == 4
                    && (!valid_symbols[TOK_SET]
                        || lexer.lookahead == '[' as i32
                        || lexer.lookahead == '(' as i32
                        || lexer.lookahead == '.' as i32)
                {
                    // If `set` was found and the keyword is not valid, return a semi since it's being used as an identifier
                    if lexer.lookahead == '(' as i32 && valid_symbols[TOK_SET] {
                        // skip until the closing parenthesis
                        while lexer.lookahead != ')' as i32 && !lexer.eof() {
                            lexer.advance(true);
                        }
                        lexer.advance(true);

                        while iswspace(lexer.lookahead) {
                            if lexer.lookahead == '\n' as i32 {
                                return (ENTRY_RETURN, true);
                            }
                            lexer.advance(true);
                        }
                        return (ENTRY_RETURN, false);
                    }
                    return (ENTRY_RETURN, true);
                } else if index == 1 && valid_symbols[TOK_IN] {
                    // If `in` was found and this specific external keyword is valid,
                    // return a semi since it's being used in a range test
                    return (ENTRY_RETURN, true);
                }
                return (ENTRY_RETURN, !res);
            }
            Some(';') => {
                lexer.advance(false);
                lexer.mark_end();
                return (ENTRY_RETURN, true);
            }
            Some('@') => {
                if valid_symbols[TOK_CONSTRUCTOR] {
                    // C does not test eof here. Lookahead stays 0 at the end
                    // of input and Advance does nothing there, so C never
                    // returns when the input ends inside the annotation.
                    // Kept: the loop below does not return either.
                    while !iswspace(lexer.lookahead) {
                        lexer.advance(true);
                    }
                    while iswspace(lexer.lookahead) {
                        lexer.advance(true);
                    }
                    for &b in b"constructor" {
                        if lexer.lookahead != i32::from(b) {
                            return (ENTRY_RETURN, true);
                        }
                        lexer.advance(true);
                    }
                    return (ENTRY_RETURN, false);
                }
                if valid_symbols[TOK_GET] || valid_symbols[TOK_SET] {
                    let mut saw_paren = false;
                    // Same as above: no eof test, endless at the end of input.
                    while (saw_paren && lexer.lookahead != '\n' as i32)
                        || (!saw_paren && !iswspace(lexer.lookahead))
                    {
                        lexer.advance(true);
                        if lexer.lookahead == '(' as i32 {
                            saw_paren = true;
                        }
                        if lexer.lookahead == ')' as i32 {
                            saw_paren = false;
                        }
                    }
                    while iswspace(lexer.lookahead) {
                        lexer.advance(true);
                    }
                    if lexer.lookahead == '/' as i32 {
                        return (ENTRY_RETURN, true);
                    }
                    continue; // goto _switch
                }
                return (ENTRY_RETURN, true);
            }
            _ => return (ENTRY_RETURN, true),
        }
    }
}

fn iswalnum(c: i32) -> bool {
    iswalpha(c) || iswdigit(c)
}

impl Scanner {
    // tree_sitter_kotlin_external_scanner_create
    pub fn new() -> Self {
        Scanner
    }

    // tree_sitter_kotlin_external_scanner_serialize
    pub fn serialize(&self, _buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        0
    }

    // tree_sitter_kotlin_external_scanner_deserialize
    pub fn deserialize(&mut self, _buf: &[u8]) {}
}

impl gositter::ExternalScanner for Scanner {
    // tree_sitter_kotlin_external_scanner_scan
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        if valid_symbols[TOK_MULTILINE_STRING_CONTENT] {
            let mut did_advance = false;
            lexer.result_symbol = TOK_MULTILINE_STRING_CONTENT as gositter::Symbol;
            while !lexer.eof() {
                match char::from_u32(lexer.lookahead as u32) {
                    Some('$') => {
                        lexer.mark_end();
                        lexer.advance(false);
                        if iswalpha(lexer.lookahead) || lexer.lookahead == '{' as i32 {
                            return did_advance;
                        }
                        did_advance = true;
                    }
                    Some('"') => {
                        lexer.mark_end();
                        // 3 or 4 quotes means we're done
                        lexer.advance(false);
                        if lexer.lookahead == '"' as i32 {
                            lexer.advance(false);
                            if lexer.lookahead == '"' as i32 {
                                lexer.advance(false);
                                if lexer.lookahead == '"' as i32 {
                                    lexer.advance(false);
                                }
                                return did_advance;
                            }
                        }
                        did_advance = true;
                    }
                    _ => {
                        lexer.advance(false);
                        did_advance = true;
                    }
                }
            }
        }

        let mut entry = ENTRY_START;
        if valid_symbols[TOK_SEMI] || valid_symbols[TOK_CLASS_MEMBER_SEMI] {
            let ret;
            (entry, ret) = scan_semi(lexer, valid_symbols);
            if entry == ENTRY_RETURN {
                return ret;
            }
        }

        if entry == ENTRY_START {
            while iswspace(lexer.lookahead) {
                lexer.advance(true);
            }
        }

        // continue_not_is_from_semi lands inside both ifs: it has skipped the
        // `!` already and does not test valid_symbols[NOT_IS].
        if entry == ENTRY_NOT_IS
            || (entry == ENTRY_START && valid_symbols[TOK_NOT_IS] && lexer.lookahead == '!' as i32)
        {
            if entry == ENTRY_START {
                lexer.advance(false);
            }
            if lexer.lookahead == 'i' as i32 {
                lexer.advance(false);
                if lexer.lookahead == 's' as i32 {
                    lexer.advance(false);
                    lexer.result_symbol = TOK_NOT_IS as gositter::Symbol;
                    lexer.mark_end();
                    return !iswalnum(lexer.lookahead);
                }
            }
        }

        if entry <= ENTRY_NOT_IS && valid_symbols[TOK_IN] && lexer.lookahead == 'i' as i32 {
            lexer.advance(false);
            if lexer.lookahead == 'n' as i32 {
                lexer.advance(false);
                lexer.result_symbol = TOK_IN as gositter::Symbol;
                lexer.mark_end();
                return !iswalnum(lexer.lookahead);
            }
        }

        // q_dot_from_semi:
        if entry <= ENTRY_Q_DOT && valid_symbols[TOK_Q_DOT] {
            while iswspace(lexer.lookahead) {
                lexer.advance(true);
            }
            if lexer.lookahead == '?' as i32 {
                lexer.advance(false);
                while iswspace(lexer.lookahead) {
                    lexer.advance(true);
                }
                if lexer.lookahead == '.' as i32 {
                    lexer.advance(false);
                    lexer.result_symbol = TOK_Q_DOT as gositter::Symbol;
                    lexer.mark_end();
                    return true;
                }
            }
        }

        // comment:
        if valid_symbols[TOK_DOLLAR] {
            return false;
        }

        if lexer.lookahead == '/' as i32 {
            lexer.advance(false);
            if lexer.lookahead != '*' as i32 {
                return false;
            }
            lexer.advance(false);

            let mut after_star = false;
            let mut nesting_depth = 1u32;
            loop {
                match char::from_u32(lexer.lookahead as u32) {
                    Some('\0') => return false,
                    Some('*') => {
                        lexer.advance(false);
                        after_star = true;
                    }
                    Some('/') => {
                        if after_star {
                            lexer.advance(false);
                            after_star = false;
                            nesting_depth -= 1;
                            if nesting_depth == 0 {
                                lexer.result_symbol = TOK_BLOCK_COMMENT as gositter::Symbol;
                                lexer.mark_end();
                                return true;
                            }
                        } else {
                            lexer.advance(false);
                            after_star = false;
                            if lexer.lookahead == '*' as i32 {
                                nesting_depth += 1;
                                lexer.advance(false);
                            }
                        }
                    }
                    _ => {
                        lexer.advance(false);
                        after_star = false;
                    }
                }
            }
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

    // C create returns NULL and serialize returns 0: the state is always empty.
    // scanner_test.go TestSerializeRoundTrip
    #[test]
    fn serialize_round_trip() {
        let mut s = Scanner::new();
        let mut buf = [0xAAu8; crate::SERIALIZATION_BUFFER_SIZE];
        assert_eq!(s.serialize(&mut buf), 0, "Serialize != 0");
        for (i, &b) in buf.iter().enumerate() {
            assert_eq!(b, 0xAA, "Serialize wrote buf[{i}]");
        }
        s.deserialize(&[]);
        s.deserialize(&buf[..0]);
        s.deserialize(&[1, 2, 3]); // C ignores any buffer
        assert_eq!(s.serialize(&mut buf), 0, "Serialize after Deserialize != 0");
        assert_eq!(s, Scanner::new(), "scanner is not stateless");
    }

    // darwin "C" locale: ASCII only, also for -1 (invalid UTF-8) and Latin-1.
    // scanner_test.go TestWctype
    #[test]
    fn wctype() {
        for c in -2i32..0x300 {
            let sp = c == ' ' as i32
                || c == '\t' as i32
                || c == '\n' as i32
                || c == 0x0B
                || c == 0x0C
                || c == '\r' as i32;
            let al = (97..=122).contains(&(c | 0x20)) && (1..128).contains(&c); // 'a'..='z'
            let dg = (48..=57).contains(&c); // '0'..='9'
            assert!(
                iswspace(c) == sp
                    && iswalpha(c) == al
                    && iswdigit(c) == dg
                    && iswalnum(c) == (al || dg),
                "code point {c}"
            );
        }
    }
}

#[cfg(test)]
mod parser_tests {
    // scanner_test.go TestScan
    #[test]
    fn scan() {
        let mut p = gositter::Parser::new();
        p.set_language(super::super::language()).unwrap();
        let cases = [
            // automatic semicolon, none before `else` or `.`
            (
                "val a = 1\nval b = 2\n",
                "(source_file (property_declaration (variable_declaration (identifier)) (number_literal)) (property_declaration (variable_declaration (identifier)) (number_literal)))",
            ),
            (
                "val s = \"\"\"a$b\"\"\"\n",
                "(source_file (property_declaration (variable_declaration (identifier)) (multiline_string_literal (string_content) (interpolation (identifier)))))",
            ),
            ("/* a /* b */ c */\n", "(source_file (block_comment))"),
        ];
        for (src, want) in cases {
            let tree = p.parse(src.as_bytes()).unwrap();
            let got = tree.root_node().to_sexp();
            assert_eq!(got, want, "{src:?}");
        }
    }
}
