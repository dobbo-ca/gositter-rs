//! Port of tree-sitter-javascript v0.25.0 `src/scanner.c` (gositter `grammars/javascript/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
//! deserialize ignores its input. `scan` is ported below.

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner;

// enum TokenType
const TOK_AUTOMATIC_SEMICOLON: usize = 0;
const TOK_TEMPLATE_CHARS: usize = 1;
const TOK_TERNARY_QMARK: usize = 2;
const TOK_HTML_COMMENT: usize = 3;
const TOK_LOGICAL_OR: usize = 4;
const TOK_ESCAPE_SEQUENCE: usize = 5;
const TOK_REGEX_PATTERN: usize = 6;
const TOK_JSX_TEXT: usize = 7;

// iswspace, iswalpha, iswdigit: libc in the "C" locale, which is what the cgo build runs in
// (nothing calls setlocale): ASCII only, so U+0085, U+00A0 and U+2028 are not spaces here.

fn iswspace(c: i32) -> bool {
    c == 32 || (9..=13).contains(&c) // " ", \t..\r
}

fn iswalpha(c: i32) -> bool {
    (65..=90).contains(&c) || (97..=122).contains(&c) // A-Z, a-z
}

fn iswdigit(c: i32) -> bool {
    (48..=57).contains(&c) // 0-9
}

// scan_template_chars
fn scan_template_chars(lexer: &mut gositter::Lexer<'_>) -> bool {
    lexer.result_symbol = TOK_TEMPLATE_CHARS as gositter::Symbol;
    let mut has_content = false;
    loop {
        lexer.mark_end();
        match char::from_u32(lexer.lookahead as u32) {
            Some('`') => return has_content,
            Some('\0') => return false,
            Some('$') => {
                lexer.advance(false);
                if lexer.lookahead == '{' as i32 {
                    return has_content;
                }
            }
            Some('\\') => return has_content,
            _ => lexer.advance(false),
        }
        has_content = true;
    }
}

// WhitespaceResult
const WS_REJECT: i32 = 0; // Semicolon is illegal, ie a syntax error occurred
const WS_NO_NEWLINE: i32 = 1; // Unclear if semicolon will be legal, continue
const WS_ACCEPT: i32 = 2; // Semicolon is legal, assuming a comment was encountered

// scan_whitespace_and_comments
fn scan_whitespace_and_comments(
    lexer: &mut gositter::Lexer<'_>,
    scanned_comment: &mut bool,
    consume: bool,
) -> i32 {
    let mut saw_block_newline = false;

    loop {
        while iswspace(lexer.lookahead) {
            lexer.advance(true);
        }

        if lexer.lookahead == '/' as i32 {
            lexer.advance(true);

            if lexer.lookahead == '/' as i32 {
                lexer.advance(true);
                while lexer.lookahead != 0
                    && lexer.lookahead != '\n' as i32
                    && lexer.lookahead != 0x2028
                    && lexer.lookahead != 0x2029
                {
                    lexer.advance(true);
                }
                *scanned_comment = true;
            } else if lexer.lookahead == '*' as i32 {
                lexer.advance(true);
                while lexer.lookahead != 0 {
                    if lexer.lookahead == '*' as i32 {
                        lexer.advance(true);
                        if lexer.lookahead == '/' as i32 {
                            lexer.advance(true);
                            *scanned_comment = true;

                            if lexer.lookahead != '/' as i32 && !consume {
                                if saw_block_newline {
                                    return WS_ACCEPT;
                                }
                                return WS_NO_NEWLINE;
                            }

                            break;
                        }
                    } else if lexer.lookahead == '\n' as i32
                        || lexer.lookahead == 0x2028
                        || lexer.lookahead == 0x2029
                    {
                        saw_block_newline = true;
                        lexer.advance(true);
                    } else {
                        lexer.advance(true);
                    }
                }
            } else {
                return WS_REJECT;
            }
        } else {
            return WS_ACCEPT;
        }
    }
}

// scan_automatic_semicolon
fn scan_automatic_semicolon(
    lexer: &mut gositter::Lexer<'_>,
    comment_condition: bool,
    scanned_comment: &mut bool,
) -> bool {
    lexer.result_symbol = TOK_AUTOMATIC_SEMICOLON as gositter::Symbol;
    lexer.mark_end();

    loop {
        if lexer.lookahead == 0 {
            return true;
        }

        if lexer.lookahead == '/' as i32 {
            let result = scan_whitespace_and_comments(lexer, scanned_comment, false);
            if result == WS_REJECT {
                return false;
            }

            if result == WS_ACCEPT
                && comment_condition
                && lexer.lookahead != ',' as i32
                && lexer.lookahead != '=' as i32
            {
                return true;
            }
        }

        if lexer.lookahead == '}' as i32 {
            return true;
        }

        if lexer.is_at_included_range_start() {
            return true;
        }

        if lexer.lookahead == '\n' as i32 || lexer.lookahead == 0x2028 || lexer.lookahead == 0x2029
        {
            break;
        }

        if !iswspace(lexer.lookahead) {
            return false;
        }

        lexer.advance(true);
    }

    lexer.advance(true);

    if scan_whitespace_and_comments(lexer, scanned_comment, true) == WS_REJECT {
        return false;
    }

    match char::from_u32(lexer.lookahead as u32) {
        Some(
            '`' | ',' | ':' | ';' | '*' | '%' | '>' | '<' | '=' | '[' | '(' | '?' | '^' | '|' | '&'
            | '/',
        ) => return false,

        // Insert a semicolon before decimals literals but not otherwise.
        Some('.') => {
            lexer.advance(true);
            return iswdigit(lexer.lookahead);
        }

        // Insert a semicolon before `--` and `++`, but not before binary `+` or `-`.
        Some('+') => {
            lexer.advance(true);
            return lexer.lookahead == '+' as i32;
        }
        Some('-') => {
            lexer.advance(true);
            return lexer.lookahead == '-' as i32;
        }

        // Don't insert a semicolon before `!=`, but do insert one before a unary `!`.
        Some('!') => {
            lexer.advance(true);
            return lexer.lookahead != '=' as i32;
        }

        // Don't insert a semicolon before `in` or `instanceof`, but do insert one
        // before an identifier.
        Some('i') => {
            lexer.advance(true);

            if lexer.lookahead != 'n' as i32 {
                return true;
            }
            lexer.advance(true);

            if !iswalpha(lexer.lookahead) {
                return false;
            }

            for &b in b"stanceof" {
                if lexer.lookahead != i32::from(b) {
                    return true;
                }
                lexer.advance(true);
            }

            if !iswalpha(lexer.lookahead) {
                return false;
            }
        }
        _ => {}
    }

    true
}

// scan_ternary_qmark
fn scan_ternary_qmark(lexer: &mut gositter::Lexer<'_>) -> bool {
    while iswspace(lexer.lookahead) {
        lexer.advance(true);
    }

    if lexer.lookahead == '?' as i32 {
        lexer.advance(false);

        if lexer.lookahead == '?' as i32 {
            return false;
        }

        lexer.mark_end();
        lexer.result_symbol = TOK_TERNARY_QMARK as gositter::Symbol;

        if lexer.lookahead == '.' as i32 {
            lexer.advance(false);
            return iswdigit(lexer.lookahead);
        }
        return true;
    }
    false
}

// scan_html_comment
fn scan_html_comment(lexer: &mut gositter::Lexer<'_>) -> bool {
    while iswspace(lexer.lookahead) || lexer.lookahead == 0x2028 || lexer.lookahead == 0x2029 {
        lexer.advance(true);
    }

    const COMMENT_START: &[u8] = b"<!--";
    const COMMENT_END: &[u8] = b"-->";

    if lexer.lookahead == '<' as i32 {
        for &b in COMMENT_START {
            if lexer.lookahead != i32::from(b) {
                return false;
            }
            lexer.advance(false);
        }
    } else if lexer.lookahead == '-' as i32 {
        for &b in COMMENT_END {
            if lexer.lookahead != i32::from(b) {
                return false;
            }
            lexer.advance(false);
        }
    } else {
        return false;
    }

    while lexer.lookahead != 0
        && lexer.lookahead != '\n' as i32
        && lexer.lookahead != 0x2028
        && lexer.lookahead != 0x2029
    {
        lexer.advance(false);
    }

    lexer.result_symbol = TOK_HTML_COMMENT as gositter::Symbol;
    lexer.mark_end();

    true
}

// scan_jsx_text
fn scan_jsx_text(lexer: &mut gositter::Lexer<'_>) -> bool {
    // saw_text: any non-whitespace, or whitespace that is not a newline and
    // does not directly follow one.
    let mut saw_text = false;
    // at_newline: at a newline, or at whitespace that directly follows one.
    let mut at_newline = false;

    while lexer.lookahead != 0
        && lexer.lookahead != '<' as i32
        && lexer.lookahead != '>' as i32
        && lexer.lookahead != '{' as i32
        && lexer.lookahead != '}' as i32
        && lexer.lookahead != '&' as i32
    {
        let is_wspace = iswspace(lexer.lookahead);
        if lexer.lookahead == '\n' as i32 {
            at_newline = true;
        } else {
            at_newline = at_newline && is_wspace;
            if !at_newline {
                saw_text = true;
            }
        }

        lexer.advance(false);
    }

    lexer.result_symbol = TOK_JSX_TEXT as gositter::Symbol;
    saw_text
}

impl Scanner {
    // tree_sitter_javascript_external_scanner_create
    pub fn new() -> Self {
        Scanner
    }

    // tree_sitter_javascript_external_scanner_serialize
    pub fn serialize(&self, _buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        0
    }

    // tree_sitter_javascript_external_scanner_deserialize
    pub fn deserialize(&mut self, _buf: &[u8]) {}
}

impl gositter::ExternalScanner for Scanner {
    // tree_sitter_javascript_external_scanner_scan
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        if valid_symbols[TOK_TEMPLATE_CHARS] {
            if valid_symbols[TOK_AUTOMATIC_SEMICOLON] {
                return false;
            }
            return scan_template_chars(lexer);
        }

        if valid_symbols[TOK_JSX_TEXT] && scan_jsx_text(lexer) {
            return true;
        }

        if valid_symbols[TOK_AUTOMATIC_SEMICOLON] {
            let mut scanned_comment = false;
            let ret = scan_automatic_semicolon(
                lexer,
                !valid_symbols[TOK_LOGICAL_OR],
                &mut scanned_comment,
            );
            if !ret
                && !scanned_comment
                && valid_symbols[TOK_TERNARY_QMARK]
                && lexer.lookahead == '?' as i32
            {
                return scan_ternary_qmark(lexer);
            }
            return ret;
        }

        if valid_symbols[TOK_TERNARY_QMARK] {
            return scan_ternary_qmark(lexer);
        }

        if valid_symbols[TOK_HTML_COMMENT]
            && !valid_symbols[TOK_LOGICAL_OR]
            && !valid_symbols[TOK_ESCAPE_SEQUENCE]
            && !valid_symbols[TOK_REGEX_PATTERN]
        {
            return scan_html_comment(lexer);
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
        s.deserialize(&buf);
        assert_eq!(s.serialize(&mut buf), 0, "Serialize after Deserialize != 0");
    }

    // libc "C" locale on darwin: ASCII only (scratch/scan/javascript/isw.c).
    // scanner_test.go TestIsw
    #[test]
    fn isw() {
        for c in -1i32..=0x3000 {
            let sp = c == ' ' as i32
                || c == '\t' as i32
                || c == '\n' as i32
                || c == 0x0B
                || c == 0x0C
                || c == '\r' as i32;
            let al = (97..=122).contains(&(c | 0x20)) && c > 0; // 'a'..='z'
            let di = (48..=57).contains(&c); // '0'..='9'
            assert!(
                iswspace(c) == sp && iswalpha(c) == al && iswdigit(c) == di,
                "isw*({c:#x}) = {} {} {}, want {sp} {al} {di}",
                iswspace(c),
                iswalpha(c),
                iswdigit(c)
            );
        }
    }
}

#[cfg(test)]
mod parser_tests {
    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    // scanner_test.go TestScanSmoke
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn scan_smoke() {
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        let src = "let a = 1\nlet b = `x${a}y`\nconst c = a ? b : /re/g\n";
        let tree = p.parse(src.as_bytes()).unwrap();
        assert!(
            !tree.root_node().has_error(),
            "unexpected error: {}",
            tree.root_node().to_sexp()
        );
    }
}

// gositter-rs-vjz.10: needs the generated consts
#[cfg(any())]
mod const_tests {
    use super::*;

    // The token order must be the grammar's externals order.
    // scanner_test.go TestTokenOrder
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn token_order() {
        let mut want = [0u16; 8];
        want[TOK_AUTOMATIC_SEMICOLON] = super::super::SYM__AUTOMATIC_SEMICOLON;
        want[TOK_TEMPLATE_CHARS] = super::super::SYM__TEMPLATE_CHARS;
        want[TOK_TERNARY_QMARK] = super::super::SYM__TERNARY_QMARK;
        want[TOK_HTML_COMMENT] = super::super::SYM_HTML_COMMENT;
        want[TOK_LOGICAL_OR] = super::super::ANON_SYM_PIPE_PIPE;
        want[TOK_ESCAPE_SEQUENCE] = super::super::SYM_ESCAPE_SEQUENCE;
        want[TOK_REGEX_PATTERN] = super::super::SYM_REGEX_PATTERN;
        want[TOK_JSX_TEXT] = super::super::SYM_JSX_TEXT;
        for (i, w) in want.into_iter().enumerate() {
            assert_eq!(
                super::super::EXTERNAL_SCANNER_SYMBOL_MAP[i],
                w,
                "symbol_map[{i}]"
            );
        }
    }
}
