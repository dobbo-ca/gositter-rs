//! Port of tree-sitter-kotlin v1.1.0 `src/scanner.c` (gositter `grammars/kotlin/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
//! deserialize ignores its input. `scan` waits for the runtime Lexer (Task 12).

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner;

// enum TokenType
#[allow(dead_code)] // used by scan (Task 12)
const TOK_SEMI: usize = 0;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CLASS_MEMBER_SEMI: usize = 1;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_BLOCK_COMMENT: usize = 2;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_NOT_IS: usize = 3;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_IN: usize = 4;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_Q_DOT: usize = 5;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_MULTILINE_STRING_CONTENT: usize = 6;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONSTRUCTOR: usize = 7;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_GET: usize = 8;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_SET: usize = 9;
#[allow(dead_code)] // used by scan (Task 12)
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

// gositter-rs-vjz.12: compiled when the runtime parser exists (Task 12).
#[cfg(any())]
mod parser_tests {
    // scanner_test.go TestScan
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
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
            let tree = p.parse(src.as_bytes());
            let got = tree.root_node().to_string();
            assert_eq!(got, want, "{src:?}");
        }
    }
}
