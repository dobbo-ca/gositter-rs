//! Port of tree-sitter-typescript v0.23.2 `common/scanner.h` (gositter `grammars/tsx/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
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
const TOK_FUNCTION_SIGNATURE_AUTOMATIC_SEMICOLON: usize = 8;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_ERROR_RECOVERY: usize = 9;

// iswspace, iswalpha, iswdigit: no setlocale call anywhere, so libc is in the "C" locale. Measured
// on darwin/arm64 over -2..0x110000: space is 9..13 and 32, alpha is A-Z and a-z, digit is 0-9;
// -1 (invalid UTF-8) is false.

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

// scan_whitespace_and_comments
fn scan_whitespace_and_comments(
    lexer: &mut gositter::Lexer<'_>,
    scanned_comment: &mut bool,
) -> bool {
    loop {
        while iswspace(lexer.lookahead) {
            lexer.advance(true);
        }

        if lexer.lookahead == '/' as i32 {
            lexer.advance(true);

            if lexer.lookahead == '/' as i32 {
                lexer.advance(true);
                while lexer.lookahead != 0 && lexer.lookahead != '\n' as i32 {
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
                            break;
                        }
                    } else {
                        lexer.advance(true);
                    }
                }
            } else {
                return false;
            }
        } else {
            return true;
        }
    }
}

// scan_automatic_semicolon
fn scan_automatic_semicolon(
    lexer: &mut gositter::Lexer<'_>,
    valid_symbols: &[bool],
    scanned_comment: &mut bool,
) -> bool {
    lexer.result_symbol = TOK_AUTOMATIC_SEMICOLON as gositter::Symbol;
    lexer.mark_end();

    loop {
        if lexer.lookahead == 0 {
            return true;
        }
        if lexer.lookahead == '}' as i32 {
            // No automatic semicolon before a type annotation:
            //   type F = ({a}: {a: number}) => number;
            loop {
                lexer.advance(true);
                if !iswspace(lexer.lookahead) {
                    break;
                }
            }
            if lexer.lookahead == ':' as i32 {
                return valid_symbols[TOK_LOGICAL_OR]; // true inside a ternary
            }
            return true;
        }
        if !iswspace(lexer.lookahead) {
            return false;
        }
        if lexer.lookahead == '\n' as i32 {
            break;
        }
        lexer.advance(true);
    }

    lexer.advance(true);

    if !scan_whitespace_and_comments(lexer, scanned_comment) {
        return false;
    }

    match char::from_u32(lexer.lookahead as u32) {
        Some(
            '`' | ',' | '.' | ';' | '*' | '%' | '>' | '<' | '=' | '?' | '^' | '|' | '&' | '/' | ':',
        ) => return false,

        Some('{') => {
            if valid_symbols[TOK_FUNCTION_SIGNATURE_AUTOMATIC_SEMICOLON] {
                return false;
            }
        }

        // No semicolon before '[' or '(' unless a type is being parsed; a
        // valid binary operator means an expression.
        Some('(' | '[') => {
            if valid_symbols[TOK_LOGICAL_OR] {
                return false;
            }
        }

        // Semicolon before `--` and `++`, not before binary `+` or `-`.
        Some('+') => {
            lexer.advance(true);
            return lexer.lookahead == '+' as i32;
        }
        Some('-') => {
            lexer.advance(true);
            return lexer.lookahead == '-' as i32;
        }

        // Not before `!=`, but before a unary `!`.
        Some('!') => {
            lexer.advance(true);
            return lexer.lookahead != '=' as i32;
        }

        // Not before `in` or `instanceof`, but before an identifier.
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

        // Optional chaining.
        if lexer.lookahead == '?' as i32 || lexer.lookahead == '.' as i32 {
            return false;
        }

        lexer.mark_end();
        lexer.result_symbol = TOK_TERNARY_QMARK as gositter::Symbol;

        // Optional parameters have ?: with whitespace allowed between.
        while iswspace(lexer.lookahead) {
            lexer.advance(false);
        }

        if lexer.lookahead == ':' as i32
            || lexer.lookahead == ')' as i32
            || lexer.lookahead == ',' as i32
        {
            return false;
        }

        if lexer.lookahead == '.' as i32 {
            lexer.advance(false);
            return iswdigit(lexer.lookahead);
        }
        return true;
    }
    false
}

// scan_closing_comment
fn scan_closing_comment(lexer: &mut gositter::Lexer<'_>) -> bool {
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
    // tree_sitter_tsx_external_scanner_create
    pub fn new() -> Self {
        Scanner
    }

    // tree_sitter_tsx_external_scanner_serialize
    pub fn serialize(&self, _buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        0
    }

    // tree_sitter_tsx_external_scanner_deserialize
    pub fn deserialize(&mut self, _buf: &[u8]) {}
}

impl gositter::ExternalScanner for Scanner {
    // external_scanner_scan (tree_sitter_tsx_external_scanner_scan)
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

        if valid_symbols[TOK_AUTOMATIC_SEMICOLON]
            || valid_symbols[TOK_FUNCTION_SIGNATURE_AUTOMATIC_SEMICOLON]
        {
            let mut scanned_comment = false;
            let ret = scan_automatic_semicolon(lexer, valid_symbols, &mut scanned_comment);
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
            return scan_closing_comment(lexer);
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

    // The ranges measured from darwin libc in the C locale (scratch/scan/typescript/isw.c).
    // scanner_test.go TestIsw
    #[test]
    fn isw() {
        for c in -2i32..=0x110000 {
            let sp = (9..=13).contains(&c) || c == 32;
            let al = (65..=90).contains(&c) || (97..=122).contains(&c);
            let di = (48..=57).contains(&c);
            assert!(
                iswspace(c) == sp && iswalpha(c) == al && iswdigit(c) == di,
                "isw*({c}) differs"
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

    // Parses through the scanner: template chars, ASI, ternary, html comment.
    // scanner_test.go TestParse
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn parse() {
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        assert!(
            language().tables.external_scanner.create.is_some(),
            "ExternalScanner.New is nil: rerun grammars/gen.sh"
        );
        let src = "let a = `x${b}y`\nlet c = a ? 1 : 2\n<!-- c\nfunction f(x?: number) {}\n";
        let tree = p.parse(src.as_bytes()).unwrap();
        let root = tree.root_node();
        assert!(!root.has_error(), "tree has errors: {}", root.to_sexp());
    }
}
