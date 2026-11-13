//! Port of tree-sitter-typescript v0.23.2 `common/scanner.h` (gositter `grammars/tsx/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
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
#[allow(dead_code)] // used by scan (Task 12)
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
