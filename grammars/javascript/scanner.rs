//! Port of tree-sitter-javascript v0.25.0 `src/scanner.c` (gositter `grammars/javascript/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
//! deserialize ignores its input. `scan` waits for the runtime Lexer (Task 12).

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner;

// enum TokenType
#[allow(dead_code)] // used by scan (Task 12)
const TOK_AUTOMATIC_SEMICOLON: usize = 0;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_TEMPLATE_CHARS: usize = 1;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_TERNARY_QMARK: usize = 2;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_HTML_COMMENT: usize = 3;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_LOGICAL_OR: usize = 4;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_ESCAPE_SEQUENCE: usize = 5;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_REGEX_PATTERN: usize = 6;
#[allow(dead_code)] // used by scan (Task 12)
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

// gositter-rs-vjz.12: compiled when the runtime parser exists (Task 12).
#[cfg(any())]
mod parser_tests {
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

    // scanner_test.go TestScanSmoke
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn scan_smoke() {
        let mut p = gositter::Parser::new();
        p.set_language(super::super::language()).unwrap();
        let src = "let a = 1\nlet b = `x${a}y`\nconst c = a ? b : /re/g\n";
        let tree = p.parse(src.as_bytes());
        assert!(
            !tree.root_node().has_error(),
            "unexpected error: {}",
            tree.root_node().to_string()
        );
    }
}
